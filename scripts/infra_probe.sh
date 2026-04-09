#!/bin/bash
# infra_probe.sh — Mac/Ubuntu/Hetzner/Vast 상태 수집 → infra_state.json
set -euo pipefail

OUT="$HOME/Dev/nexus/shared/infra_state.json"
TMP="${OUT}.tmp"
TS=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

SSH_OPTS="-o ConnectTimeout=5 -o BatchMode=yes"

# ── retry helper (최대 2회 시도) ─────────────────
ssh_retry() {
  local host="$1"; shift
  local result
  result=$(ssh $SSH_OPTS "$host" "$@" 2>/dev/null) && { echo "$result"; return 0; }
  sleep 2
  result=$(ssh $SSH_OPTS "$host" "$@" 2>/dev/null) && { echo "$result"; return 0; }
  return 1
}

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

if ubu_raw=$(ssh_retry ubu "LANG=C free -m; echo '---'; cat /proc/loadavg; echo '---'; nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits"); then
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

htz_gpu_util=0 htz_gpu_vram_used=0 htz_gpu_vram_total=0

if htz_raw=$(ssh_retry hetzner "LANG=C free -m; echo '---'; cat /proc/loadavg; echo '---'; nproc; echo '---'; nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null || echo 'no_gpu'"); then
  htz_status="active"
  htz_mem=$(echo "$htz_raw" | awk '/^Mem:/{print $3, $2}')
  htz_ram_used=$(echo "$htz_mem" | awk '{print $1}')
  htz_ram_total=$(echo "$htz_mem" | awk '{print $2}')
  htz_load=$(echo "$htz_raw" | awk '/^---$/{n++; next} n==1{print $1}')
  htz_threads=$(echo "$htz_raw" | awk '/^---$/{n++; next} n==2{print $1; exit}')
  # GPU (if available)
  htz_gpu_line=$(echo "$htz_raw" | tail -1)
  if [ "$htz_gpu_line" != "no_gpu" ] && echo "$htz_gpu_line" | grep -qE '^[0-9]'; then
    htz_gpu_util=$(echo "$htz_gpu_line" | awk -F', ' '{print $1}')
    htz_gpu_vram_used=$(echo "$htz_gpu_line" | awk -F', ' '{print $2}')
    htz_gpu_vram_total=$(echo "$htz_gpu_line" | awk -F', ' '{print $3}')
  fi
fi

# ── Vast.ai (34459201: 4x RTX 4090, ssh9.vast.ai:19200) ──
vast_status="offline"
vast_gpu="4x RTX 4090"
vast_vram_gb=96
vast_vram_used_gb=0
vast_gpu_util=0
vast_cpu_pct=0
vast_cpu_cores=0
vast_ram_used_gb=0
vast_ram_total_gb=0
vast_price="\$1.64"

VAST_SSH_OPTS="-o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes -p 19200"
vast_ssh_retry() {
  local result
  result=$(ssh $VAST_SSH_OPTS root@ssh9.vast.ai "$@" 2>/dev/null) && { echo "$result"; return 0; }
  sleep 3
  result=$(ssh $VAST_SSH_OPTS root@ssh9.vast.ai "$@" 2>/dev/null) && { echo "$result"; return 0; }
  return 1
}
if vast_raw=$(vast_ssh_retry "nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits; echo '---'; nproc; echo '---'; LANG=C free -g | grep Mem; echo '---'; LANG=C top -bn1 | head -3"); then
  vast_status="active"
  # GPU — 4줄 합산
  vast_gpu_util=$(echo "$vast_raw" | awk -F', ' '/^[0-9]/{s+=$1; n++} /^---/{exit} END{if(n>0) printf "%d", s/n; else print 0}')
  vast_vram_used_gb=$(echo "$vast_raw" | awk -F', ' '/^[0-9]/{s+=$2} /^---/{exit} END{printf "%d", s/1024}')
  vast_vram_gb=$(echo "$vast_raw" | awk -F', ' '/^[0-9]/{s+=$3} /^---/{exit} END{printf "%d", s/1024}')
  # CPU
  vast_cpu_cores=$(echo "$vast_raw" | awk '/^---/{n++; next} n==1{print $1; exit}')
  # RAM
  vast_ram_line=$(echo "$vast_raw" | awk '/^---/{n++; next} n==2 && /Mem/{print; exit}')
  vast_ram_used_gb=$(echo "$vast_ram_line" | awk '{print $3}')
  vast_ram_total_gb=$(echo "$vast_ram_line" | awk '{print $2}')
  # CPU %
  vast_cpu_pct=$(echo "$vast_raw" | awk '/^---/{n++; next} n==3 && /%Cpu/{gsub(/[^0-9.]/, "", $2); printf "%d", $2; exit}')
fi

# ── smart recommendation (사용률 기반) ─────────────
# GPU: VRAM 여유 + 사용률 낮은 쪽 우선
gpu_task="none"
if [ "$vast_status" = "active" ] && [ "$ubu_status" = "active" ]; then
  vast_vram_free=$((${vast_vram_gb:-0} - ${vast_vram_used_gb:-0}))
  ubu_vram_free=$(( (${ubu_gpu_vram_total:-0} - ${ubu_gpu_vram_used:-0}) / 1024 ))
  if [ "$vast_vram_free" -ge "$ubu_vram_free" ]; then gpu_task="vast"; else gpu_task="ubu"; fi
elif [ "$vast_status" = "active" ]; then gpu_task="vast"
elif [ "$ubu_status" = "active" ]; then gpu_task="ubu"
fi

# CPU: load/threads 비율 낮은 쪽 우선
cpu_task="none"
if [ "$htz_status" = "active" ] && [ "${htz_threads:-1}" -gt 0 ]; then
  htz_ratio=$(echo "${htz_load:-0} ${htz_threads}" | awk '{printf "%d", ($1/$2)*100}')
else
  htz_ratio=999
fi
if [ "$ubu_status" = "active" ]; then
  ubu_ratio=$(echo "${ubu_load:-0}" | awk '{printf "%d", ($1/8)*100}')  # 8코어 가정
else
  ubu_ratio=999
fi
if [ "$htz_ratio" -le "$ubu_ratio" ] && [ "$htz_status" = "active" ]; then
  cpu_task="htz"
elif [ "$ubu_status" = "active" ]; then
  cpu_task="ubu"
elif [ "$htz_status" = "active" ]; then
  cpu_task="htz"
fi

avoid="mac"

# ── JSON 출력 ────────────────────────────────────
cat > "$TMP" <<EOJSON
{"ts":"${TS}","hosts":{"mac":{"status":"${mac_status}","cpu":${mac_cpu:-0},"gpu":${mac_gpu:-0}},"ubu":{"status":"${ubu_status}","load":"${ubu_load}","ram_used_mb":${ubu_ram_used:-0},"ram_total_mb":${ubu_ram_total:-0},"ram_avail_mb":${ubu_ram_avail:-0},"gpu_util":${ubu_gpu_util:-0},"gpu_vram_used_mb":${ubu_gpu_vram_used:-0},"gpu_vram_total_mb":${ubu_gpu_vram_total:-0}},"htz":{"status":"${htz_status}","load":"${htz_load}","cpu_threads":${htz_threads:-0},"ram_used_mb":${htz_ram_used:-0},"ram_total_mb":${htz_ram_total:-0},"gpu_util":${htz_gpu_util:-0},"gpu_vram_used_mb":${htz_gpu_vram_used:-0},"gpu_vram_total_mb":${htz_gpu_vram_total:-0}},"vast":{"status":"${vast_status}","gpu":"${vast_gpu}","vram_gb":${vast_vram_gb:-96},"vram_used_gb":${vast_vram_used_gb:-0},"gpu_util":${vast_gpu_util:-0},"cpu_pct":${vast_cpu_pct:-0},"cpu_cores":${vast_cpu_cores:-0},"ram_used_gb":${vast_ram_used_gb:-0},"ram_total_gb":${vast_ram_total_gb:-0},"price_hr":"${vast_price}"}},"recommendation":{"gpu_task":"${gpu_task}","cpu_task":"${cpu_task}","avoid":"${avoid}"}}
EOJSON

mv "$TMP" "$OUT"
