#!/bin/bash
# infra_probe.sh — Mac/Ubuntu/Hetzner/Vast 상태 수집 → infra_state.json
set -euo pipefail

OUT="$HOME/Dev/nexus/shared/infra_state.json"
TMP="${OUT}.tmp"
TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

SSH_OPTS="-o ConnectTimeout=5 -o BatchMode=yes"

# ── Mac ──────────────────────────────────────────
mac_top=$(top -l1 -n0 2>/dev/null || true)
mac_cpu=$(echo "$mac_top" | awk '/CPU usage/{gsub(/%/,""); print $3+$5}' || echo 0)
mac_gpu=$(ioreg -r -d1 -c IOAccelerator 2>/dev/null | grep -o '"Device Utilization %"=[0-9]*' | head -1 | awk -F= '{print $2}' || echo 0)
mac_gpu=${mac_gpu:-0}
mac_status="active"

# ── Ubuntu ───────────────────────────────────────
ubu_status="offline"
ubu_load="" ubu_ram_used="" ubu_ram_total="" ubu_ram_avail=""
ubu_gpu_util="" ubu_gpu_vram_used="" ubu_gpu_vram_total=""

if ubu_raw=$(ssh $SSH_OPTS ubu "LANG=C free -m; echo '---'; cat /proc/loadavg; echo '---'; nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits" 2>/dev/null); then
  ubu_status="active"
  ubu_mem=$(echo "$ubu_raw" | awk '/^Mem:/{print $3, $2, $7}')
  ubu_ram_used=$(echo "$ubu_mem" | awk '{print $1}')
  ubu_ram_total=$(echo "$ubu_mem" | awk '{print $2}')
  ubu_ram_avail=$(echo "$ubu_mem" | awk '{print $3}')
  ubu_load=$(echo "$ubu_raw" | awk '/^---$/{n++; next} n==1{print $1}')
  ubu_gpu_line=$(echo "$ubu_raw" | tail -1)
  ubu_gpu_util=$(echo "$ubu_gpu_line" | awk -F', ' '{print $1}')
  ubu_gpu_vram_used=$(echo "$ubu_gpu_line" | awk -F', ' '{print $2}')
  ubu_gpu_vram_total=$(echo "$ubu_gpu_line" | awk -F', ' '{print $3}')
fi

# ── Hetzner ──────────────────────────────────────
htz_status="offline"
htz_load="" htz_threads="" htz_ram_used="" htz_ram_total=""

if htz_raw=$(ssh $SSH_OPTS hetzner "LANG=C free -m; echo '---'; cat /proc/loadavg; echo '---'; nproc" 2>/dev/null); then
  htz_status="active"
  htz_mem=$(echo "$htz_raw" | awk '/^Mem:/{print $3, $2}')
  htz_ram_used=$(echo "$htz_mem" | awk '{print $1}')
  htz_ram_total=$(echo "$htz_mem" | awk '{print $2}')
  htz_load=$(echo "$htz_raw" | awk '/^---$/{n++; next} n==1{print $1}')
  htz_threads=$(echo "$htz_raw" | tail -1)
fi

# ── Vast ─────────────────────────────────────────
vast_status="offline"
vast_gpu="4x RTX 4090"
vast_vram=96
vast_price="\$1.098"

# ── recommendation ───────────────────────────────
gpu_task="ubu"
cpu_task="htz"
avoid="mac"
[ "$ubu_status" = "offline" ] && gpu_task="htz"
[ "$htz_status" = "offline" ] && cpu_task="ubu"

# ── JSON 출력 ────────────────────────────────────
cat > "$TMP" <<EOJSON
{"ts":"${TS}","hosts":{"mac":{"status":"${mac_status}","cpu":${mac_cpu:-0},"gpu":${mac_gpu:-0}},"ubu":{"status":"${ubu_status}","load":"${ubu_load}","ram_used_mb":${ubu_ram_used:-0},"ram_total_mb":${ubu_ram_total:-0},"ram_avail_mb":${ubu_ram_avail:-0},"gpu_util":${ubu_gpu_util:-0},"gpu_vram_used_mb":${ubu_gpu_vram_used:-0},"gpu_vram_total_mb":${ubu_gpu_vram_total:-0}},"htz":{"status":"${htz_status}","load":"${htz_load}","cpu_threads":${htz_threads:-0},"ram_used_mb":${htz_ram_used:-0},"ram_total_mb":${htz_ram_total:-0}},"vast":{"status":"${vast_status}","gpu":"${vast_gpu}","vram_gb":${vast_vram},"price_hr":"${vast_price}"}},"recommendation":{"gpu_task":"${gpu_task}","cpu_task":"${cpu_task}","avoid":"${avoid}"}}
EOJSON

mv "$TMP" "$OUT"
