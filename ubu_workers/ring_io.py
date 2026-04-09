#!/usr/bin/env python3
"""
AG3 게놈 링버퍼 I/O
- 단일 writer: append_genome()
- 다중 reader: iter_recent(n), stats()
- 파일: /mnt/ramdisk/airgenome/genome.ring (기본)
"""
import struct, os, time, zlib, argparse, json, sys

MAGIC = b"AG3R"
VERSION = 1
HEADER_SIZE = 64
DEFAULT_SLOT_SIZE = 128
DEFAULT_SLOT_COUNT = 65536

HDR_FMT = "<4sHHIII44x"  # magic, version, slot_size, slot_count, write_idx, wrap_count, reserved
SLOT_FMT = "<di I 60s i I 44x"  # ts, pid, name_hash, genome(60B), cluster_id, flags

def ring_path_default():
    return os.environ.get("AG3_RING", "/mnt/ramdisk/airgenome/genome.ring")

def init_ring(path=None, slot_count=DEFAULT_SLOT_COUNT, slot_size=DEFAULT_SLOT_SIZE):
    path = path or ring_path_default()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        hdr = struct.pack(HDR_FMT, MAGIC, VERSION, slot_size, slot_count, 0, 0)
        f.write(hdr)
        f.write(b"\x00" * (slot_count * slot_size))
    return path

def read_header(path=None):
    path = path or ring_path_default()
    with open(path, "rb") as f:
        hdr = f.read(HEADER_SIZE)
    magic, ver, ss, sc, wi, wc = struct.unpack(HDR_FMT, hdr)
    assert magic == MAGIC, f"bad magic {magic!r}"
    return {"version": ver, "slot_size": ss, "slot_count": sc,
            "write_idx": wi, "wrap_count": wc, "path": path}

def append_genome(pid, name, genome60, cluster_id=-1, flags=0, path=None):
    path = path or ring_path_default()
    h = read_header(path)
    idx = h["write_idx"] % h["slot_count"]
    offset = HEADER_SIZE + idx * h["slot_size"]
    name_hash = zlib.crc32(name.encode("utf-8")) & 0xffffffff
    assert len(genome60) == 60, f"genome must be 60 bytes, got {len(genome60)}"
    slot = struct.pack(SLOT_FMT, time.time(), pid, name_hash, genome60, cluster_id, flags)
    with open(path, "r+b") as f:
        f.seek(offset)
        f.write(slot)
        new_wi = (h["write_idx"] + 1) % h["slot_count"]
        new_wc = h["wrap_count"] + (1 if new_wi == 0 else 0)
        f.seek(0)
        f.write(struct.pack(HDR_FMT, MAGIC, VERSION, h["slot_size"],
                            h["slot_count"], new_wi, new_wc))
        f.flush()
        os.fsync(f.fileno())

def iter_recent(n=100, path=None):
    path = path or ring_path_default()
    h = read_header(path)
    total_written = h["wrap_count"] * h["slot_count"] + h["write_idx"]
    count = min(n, total_written)
    with open(path, "rb") as f:
        for k in range(count):
            idx = (h["write_idx"] - 1 - k) % h["slot_count"]
            f.seek(HEADER_SIZE + idx * h["slot_size"])
            slot = f.read(h["slot_size"])
            ts, pid, nh, genome, cid, flags = struct.unpack(SLOT_FMT, slot)
            yield {"ts": ts, "pid": pid, "name_hash": nh,
                   "genome": genome.hex(), "cluster_id": cid, "flags": flags}

def cli():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["init", "stats", "head", "tail", "probe"])
    ap.add_argument("-n", type=int, default=10)
    ap.add_argument("--path")
    args = ap.parse_args()
    if args.cmd == "init":
        p = init_ring(args.path)
        print(f"initialized {p}")
    elif args.cmd == "stats":
        print(json.dumps(read_header(args.path)))
    elif args.cmd in ("tail", "head"):
        for e in iter_recent(args.n, args.path):
            print(json.dumps(e))
    elif args.cmd == "probe":
        genome = bytes(range(60))
        append_genome(os.getpid(), "probe_test", genome, path=args.path)
        print("appended probe genome")

if __name__ == "__main__":
    cli()
