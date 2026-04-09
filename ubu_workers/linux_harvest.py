#!/usr/bin/env python3
"""
linux_harvest.py -- Linux /proc 기반 6축 게놈 수집기 (Ubuntu 네이티브)

6축: CPU / RAM / Swap / Net / Disk / GPU
- CPU:  /proc/<pid>/stat  (utime + stime)
- RAM:  /proc/<pid>/stat  (rss pages * page_size)
- Swap: /proc/<pid>/status (VmSwap)
- Disk: /proc/<pid>/io    (read_bytes + write_bytes)
- Net:  /proc/net/dev     (system-wide rx+tx delta, cpu비중 배분)
- GPU:  nvidia-smi        (per-process GPU memory, 없으면 proxy)

출력: 120 hex char = 60 bytes genome (15쌍 x 4바이트, Mac genome_harvest.hexa 호환)

Usage:
  python3 linux_harvest.py once              # 1회 배치
  python3 linux_harvest.py loop              # 연속 수집 (sleep_between_batches_sec 주기)
  python3 linux_harvest.py status            # 현재 수집 통계
  python3 linux_harvest.py once --ring-only  # ring_io에만 기록 (파일 미생성)
"""

import os, sys, time, json, math, subprocess, signal, random

# ─── CONFIG ────────────────────────────────────────────────────────────────
BASE_DIR = os.environ.get("AG3_BASE", os.path.expanduser("~/airgenome"))
CFG_PATH = os.environ.get("AG3_CONFIG",
    os.path.join(BASE_DIR, "nexus/shared/genome_harvest.jsonl"))
GATE_CFG = os.path.join(BASE_DIR, "nexus/shared/gate_config.jsonl")
PAGE_SIZE = os.sysconf("SC_PAGE_SIZE")  # typically 4096

# ring_io import (same directory)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    import ring_io
    HAS_RING = True
except ImportError:
    HAS_RING = False

# ─── JSONL CONFIG LOADER ──────────────────────────────────────────────────
def load_cfg_all():
    cfg = {}
    for p in (CFG_PATH, GATE_CFG):
        try:
            with open(p) as f:
                for line in f:
                    try:
                        j = json.loads(line.strip())
                        cfg[j["key"]] = j["value"]
                    except Exception:
                        pass
        except FileNotFoundError:
            pass
    return cfg

def cfg_int(cfg, key, default):
    try:
        return int(cfg.get(key, default))
    except (ValueError, TypeError):
        return default

def cfg_str(cfg, key, default):
    return str(cfg.get(key, default))

# ─── SYSTEM-WIDE SNAPSHOTS ────────────────────────────────────────────────
def read_total_ram_kb():
    try:
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    return int(line.split()[1])
    except Exception:
        pass
    return 16 * 1024 * 1024  # fallback 16GB

def read_net_totals():
    """Sum rx_bytes + tx_bytes across all non-lo interfaces."""
    rx, tx = 0, 0
    try:
        with open("/proc/net/dev") as f:
            for line in f:
                parts = line.split()
                if len(parts) < 10 or ":" not in parts[0]:
                    continue
                iface = parts[0].rstrip(":")
                if iface == "lo":
                    continue
                rx += int(parts[1])
                tx += int(parts[9])
    except Exception:
        pass
    return rx + tx

def read_gpu_pid_map():
    """nvidia-smi per-process GPU memory (MB). Returns {pid: mem_mb}."""
    result = {}
    try:
        out = subprocess.check_output(
            ["nvidia-smi",
             "--query-compute-apps=pid,used_memory",
             "--format=csv,noheader,nounits"],
            timeout=5, stderr=subprocess.DEVNULL
        ).decode().strip()
        for line in out.split("\n"):
            if not line.strip():
                continue
            parts = line.split(",")
            if len(parts) >= 2:
                pid = int(parts[0].strip())
                mem = int(parts[1].strip())
                result[pid] = mem
    except (FileNotFoundError, subprocess.SubprocessError):
        pass
    return result

# ─── PER-PROCESS /proc READING ────────────────────────────────────────────
def read_proc_stat(pid):
    """Read /proc/<pid>/stat -> (utime, stime, rss_pages, nice, ppid, comm)."""
    try:
        with open(f"/proc/{pid}/stat") as f:
            data = f.read()
        # comm is in parens; find last ')' to split safely
        i = data.rfind(")")
        if i < 0:
            return None
        comm = data[data.index("(") + 1 : i]
        fields = data[i + 2:].split()
        # fields index: 0=state, 1=ppid, 11=utime, 12=stime, 16=nice, 21=rss
        ppid = int(fields[1])
        utime = int(fields[11])
        stime = int(fields[12])
        nice = int(fields[16])
        rss_pages = int(fields[21])
        return (utime, stime, rss_pages, nice, ppid, comm)
    except (FileNotFoundError, PermissionError, IndexError, ValueError):
        return None

def read_proc_io(pid):
    """Read /proc/<pid>/io -> (read_bytes, write_bytes) or None."""
    try:
        with open(f"/proc/{pid}/io") as f:
            rb, wb = 0, 0
            for line in f:
                if line.startswith("read_bytes:"):
                    rb = int(line.split(":")[1].strip())
                elif line.startswith("write_bytes:"):
                    wb = int(line.split(":")[1].strip())
            return (rb, wb)
    except (FileNotFoundError, PermissionError, ValueError):
        return None

def read_proc_swap(pid):
    """VmSwap from /proc/<pid>/status in KB."""
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("VmSwap:"):
                    return int(line.split()[1])
    except (FileNotFoundError, PermissionError, ValueError):
        pass
    return 0

def list_pids(min_pid):
    pids = []
    try:
        for entry in os.listdir("/proc"):
            if entry.isdigit():
                pid = int(entry)
                if pid > min_pid:
                    pids.append(pid)
    except OSError:
        pass
    return pids

# ─── 6-AXIS ENCODING ──────────────────────────────────────────────────────
def log2_x16(v):
    """log2(v)*16, clipped to [0..255] -- matches Mac genome_harvest.hexa."""
    if v <= 1:
        return 0
    raw = math.log2(v) * 16
    return min(255, max(0, int(raw)))

def sample_proc_6axis(pid, gpu_map, net_total_bytes, cpu_total_ticks):
    """Returns (cpu_v, ram_v, swp_v, net_v, dsk_v, gpu_v, comm) or None."""
    stat = read_proc_stat(pid)
    if stat is None:
        return None
    utime, stime, rss_pages, nice, ppid, comm = stat
    rss_kb = rss_pages * PAGE_SIZE // 1024

    if rss_kb == 0:
        return None

    # CPU: log2_x16(utime + stime + 1)
    cpu_ticks = utime + stime
    cpu_v = log2_x16(cpu_ticks + 1)

    # RAM: log2_x16(rss_kb) with floor subtraction for spread
    ram_raw = log2_x16(rss_kb)
    ram_floor = 128
    if ram_raw > ram_floor:
        ram_v = (ram_raw - ram_floor) * 255 // (255 - ram_floor)
    else:
        ram_v = 0
    ram_v = min(255, ram_v)

    # SWAP: real VmSwap
    swap_kb = read_proc_swap(pid)
    swp_v = log2_x16(swap_kb + 1)

    # NET: cpu-proportional share of system net bytes
    if cpu_total_ticks > 0 and net_total_bytes > 0:
        share = (cpu_ticks / max(cpu_total_ticks, 1)) * net_total_bytes
        net_v = log2_x16(int(share) + 1)
    else:
        net_v = min(255, max(0, (nice + 20) * 6))

    # DISK: real I/O bytes
    io_data = read_proc_io(pid)
    if io_data is not None:
        rb, wb = io_data
        dsk_v = log2_x16(rb + wb + 1)
    else:
        dsk_v = ((pid * 2654435761) >> 6) & 63

    # GPU: real nvidia-smi per-process memory
    gpu_mb = gpu_map.get(pid, 0)
    if gpu_mb > 0:
        gpu_v = log2_x16(gpu_mb * 1024 + 1)
    else:
        gpu_combo = cpu_ticks * 256 + (nice + 20)
        gpu_v = log2_x16(gpu_combo + 1)

    return (cpu_v, ram_v, swp_v, net_v, dsk_v, gpu_v, comm)

# ─── GENOME ENCODING (60 bytes = 120 hex, identical to Mac) ───────────────
def encode_genome(axes):
    """axes: 6-tuple (cpu, ram, swp, net, dsk, gpu). Returns 120-char hex."""
    a = [axes[i] & 255 for i in range(6)]
    buf = bytearray()
    for x in range(6):
        for y in range(x + 1, 6):
            r = (x + y) % 8
            xor_v = (a[x] ^ a[y]) & 255
            hi = (xor_v << r) & 255
            lo = xor_v >> (8 - r) if r < 8 else 0
            rot = (hi | lo) & 255
            buf.extend([a[x], a[y], xor_v, rot])
    assert len(buf) == 60, f"genome must be 60 bytes, got {len(buf)}"
    return buf.hex()

# ─── CPU TOTAL TICKS (for net proportioning) ──────────────────────────────
def cpu_total_ticks_all():
    """Sum of all process CPU ticks (quick /proc scan)."""
    total = 0
    try:
        for entry in os.listdir("/proc"):
            if entry.isdigit():
                try:
                    with open(f"/proc/{entry}/stat") as f:
                        data = f.read()
                    i = data.rfind(")")
                    if i < 0:
                        continue
                    fields = data[i + 2:].split()
                    total += int(fields[11]) + int(fields[12])
                except Exception:
                    pass
    except OSError:
        pass
    return max(total, 1)

# ─── HARVEST BATCH ─────────────────────────────────────────────────────────
def harvest_batch(cfg, out_dir, index_file, batch_size, min_pid, ring_only,
                  ring_path):
    gpu_map = read_gpu_pid_map()
    net_total = read_net_totals()
    cpu_total = cpu_total_ticks_all()

    pids = list_pids(min_pid)
    random.shuffle(pids)

    if not ring_only:
        os.makedirs(out_dir, exist_ok=True)

    added = 0
    for pid in pids:
        if added >= batch_size:
            break
        result = sample_proc_6axis(pid, gpu_map, net_total, cpu_total)
        if result is None:
            continue

        cpu_v, ram_v, swp_v, net_v, dsk_v, gpu_v, comm = result
        axes_csv = f"{cpu_v},{ram_v},{swp_v},{net_v},{dsk_v},{gpu_v}"
        genome_hex = encode_genome((cpu_v, ram_v, swp_v, net_v, dsk_v, gpu_v))

        if len(genome_hex) != 120:
            continue

        ts = int(time.time_ns())

        # ring_io append
        if HAS_RING and ring_path and os.path.exists(ring_path):
            try:
                genome_bytes = bytes.fromhex(genome_hex)
                ring_io.append_genome(pid, comm, genome_bytes, path=ring_path)
            except Exception as e:
                print(f"  [ring_io] warn: {e}", file=sys.stderr)

        # file-based output
        if not ring_only:
            fname = os.path.join(out_dir, f"{ts}_{pid}.genome")
            try:
                with open(fname, "w") as f:
                    f.write(genome_hex)
            except OSError:
                pass

            safe_comm = comm.replace('"', '').replace('\\', '')
            idx_line = json.dumps({
                "ts": ts, "pid": pid, "comm": safe_comm,
                "axes": axes_csv, "file": fname,
                "encoding_ver": 2, "platform": "linux"
            }, ensure_ascii=False)
            try:
                with open(index_file, "a") as f:
                    f.write(idx_line + "\n")
            except OSError:
                pass

        added += 1

    return added

def count_existing(out_dir):
    try:
        return len([f for f in os.listdir(out_dir) if f.endswith(".genome")])
    except FileNotFoundError:
        return 0

# ─── MAIN ──────────────────────────────────────────────────────────────────
def main():
    cfg = load_cfg_all()

    target = cfg_int(cfg, "target_count", 9999)
    batch_size = cfg_int(cfg, "batch_size", 100)
    sleep_sec = cfg_int(cfg, "sleep_between_batches_sec", 2)
    out_dir = cfg_str(cfg, "output_dir", "forge/genomes")
    index_file = cfg_str(cfg, "index_file", "forge/genomes.index.jsonl")
    min_pid = cfg_int(cfg, "min_pid", 100)

    if not os.path.isabs(out_dir):
        out_dir = os.path.join(BASE_DIR, out_dir)
    if not os.path.isabs(index_file):
        index_file = os.path.join(BASE_DIR, index_file)

    ring_path = None
    if HAS_RING:
        ring_path = os.environ.get("AG3_RING",
            "/mnt/ramdisk/airgenome/genome.ring")

    mode = "once"
    ring_only = False
    for a in sys.argv[1:]:
        if a == "loop":
            mode = "loop"
        elif a == "once":
            mode = "once"
        elif a == "status":
            mode = "status"
        elif a == "--ring-only":
            ring_only = True

    print(f"  linux_harvest -- target={target} batch={batch_size} mode={mode}"
          f" ring_only={ring_only}")
    print(f"  out_dir={out_dir}")
    if ring_path:
        print(f"  ring_path={ring_path}")

    if mode == "status":
        have = count_existing(out_dir)
        print(f"  existing: {have}  remaining: {target - have}")
        return

    # graceful shutdown
    running = [True]
    def _sig(s, f):
        running[0] = False
        print("\n  shutting down...")
    signal.signal(signal.SIGTERM, _sig)
    signal.signal(signal.SIGINT, _sig)

    have = count_existing(out_dir) if not ring_only else 0
    print(f"  existing: {have}")

    loops = 0
    max_loops = 1 if mode == "once" else 999999999

    while running[0] and loops < max_loops:
        if not ring_only and have >= target:
            print(f"  reached target ({have} >= {target})")
            break

        added = harvest_batch(cfg, out_dir, index_file, batch_size, min_pid,
                              ring_only, ring_path)
        have = count_existing(out_dir) if not ring_only else (have + added)
        print(f"  batch+{added} total={have}")

        loops += 1
        if mode == "once":
            break
        time.sleep(sleep_sec)

    print(f"  done. genomes={have}")

if __name__ == "__main__":
    main()
