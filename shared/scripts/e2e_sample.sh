#!/bin/bash
# M8 e2e soak sampler — 30min 주기 plist 에서 호출
# 5-stage 산출물 mtime/age + stderr ERROR 누적 + anomaly_total 최근치 수집
# 출력: forge/e2e_samples.jsonl append-only

set -u
AG=/Users/ghost/Dev/airgenome
NX=/Users/ghost/Dev/nexus
LOG=/Users/ghost/.airgenome
OUT=$AG/forge/e2e_samples.jsonl

age() {
  local p=$1
  [ -f "$p" ] || { echo -1; return; }
  local m
  m=$(stat -f %m "$p" 2>/dev/null || echo 0)
  [ "$m" -eq 0 ] && { echo -1; return; }
  echo $(( $(date +%s) - m ))
}

grep_count() {
  local p=$1
  [ -f "$p" ] || { echo 0; return; }
  local c
  c=$(grep -cE "ERROR|panic|PANIC|Traceback|FATAL" "$p" 2>/dev/null)
  [ -z "$c" ] && c=0
  echo "$c"
}

now=$(date -u +%Y-%m-%dT%H:%M:%SZ)
probe_age=$(age $NX/shared/infra_state.json)
dispatch_age=$(age $NX/shared/dispatch_state.json)
harvest_age=$(age $AG/forge/genomes.ring)
label_age=$(age $AG/forge/labeled_anomaly.jsonl)
forecast_age=$(age $AG/forge/forecast.jsonl)

probe_err=$(grep_count $LOG/probe.stderr.log)
dispatch_err=$(grep_count $LOG/dispatch.stderr.log)
harvest_err=$(grep_count $LOG/harvest.stderr.log)
label_err=$(grep_count $LOG/label.stderr.log)
forecast_err=$(grep_count $LOG/forecast.stderr.log)
stderr_total=$((probe_err + dispatch_err + harvest_err + label_err + forecast_err))

anom_last=0
if [ -f "$LOG/harvest.stdout.log" ]; then
  v=$(grep -oE "anomaly_total=[0-9]+" "$LOG/harvest.stdout.log" 2>/dev/null | tail -1 | cut -d= -f2)
  [ -n "$v" ] && anom_last=$v
fi

anom_sum_1h=0
if [ -f "$LOG/harvest.stdout.log" ]; then
  cutoff=$(( $(date +%s) - 3600 ))
  if [ "$(stat -f %m "$LOG/harvest.stdout.log")" -gt "$cutoff" ]; then
    anom_sum_1h=$(grep -oE "anomaly_total=[0-9]+" "$LOG/harvest.stdout.log" | tail -60 | cut -d= -f2 | awk '{s+=$1} END {print s+0}')
  fi
fi

printf '{"ts":"%s","probe_age_s":%s,"dispatch_age_s":%s,"harvest_age_s":%s,"label_age_s":%s,"forecast_age_s":%s,"stderr":{"probe":%s,"dispatch":%s,"harvest":%s,"label":%s,"forecast":%s,"total":%s},"anom_last":%s,"anom_sum_1h":%s}\n' \
  "$now" "$probe_age" "$dispatch_age" "$harvest_age" "$label_age" "$forecast_age" \
  "$probe_err" "$dispatch_err" "$harvest_err" "$label_err" "$forecast_err" "$stderr_total" \
  "$anom_last" "$anom_sum_1h" >> "$OUT"
