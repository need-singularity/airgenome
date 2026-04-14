#!/bin/bash
# M8 e2e soak evaluator — forge/e2e_samples.jsonl + shared/config/e2e_acceptance.jsonl
# 24h 경과 후 수동 실행. 출력: PASS/FAIL per criterion + overall verdict

set -u
AG=/Users/ghost/Dev/airgenome
SAMPLES=$AG/forge/e2e_samples.jsonl
CRITERIA=$AG/shared/config/e2e_acceptance.jsonl

[ -f "$SAMPLES" ] || { echo "FAIL: $SAMPLES missing"; exit 1; }
[ -f "$CRITERIA" ] || { echo "FAIL: $CRITERIA missing"; exit 1; }

n=$(wc -l < "$SAMPLES" | tr -d ' ')
first_ts=$(head -1 "$SAMPLES" | jq -r .ts)
last_ts=$(tail -1 "$SAMPLES" | jq -r .ts)
first_epoch=$(date -jf '%Y-%m-%dT%H:%M:%SZ' "$first_ts" +%s 2>/dev/null)
last_epoch=$(date -jf '%Y-%m-%dT%H:%M:%SZ' "$last_ts" +%s 2>/dev/null)
span_s=$((last_epoch - first_epoch))
span_h=$(awk -v s=$span_s 'BEGIN{printf "%.2f", s/3600}')

echo "=== M8 e2e eval ==="
echo "samples: $n · span: ${span_h}h ($first_ts → $last_ts)"

fails=0

# 1. per-stage freshness — 최근 1h 샘플에서 max age 확인
recent_cutoff=$((last_epoch - 3600))
recent=$(jq -c --arg cut "$recent_cutoff" 'select((.ts | strptime("%Y-%m-%dT%H:%M:%SZ") | mktime) >= ($cut|tonumber))' "$SAMPLES")

for stage in probe dispatch harvest label forecast; do
  max_age_s=$(jq -r --arg s "${stage}_freshness" 'select(.id==$s) | .max_age_s' "$CRITERIA")
  max_seen=$(echo "$recent" | jq -r ".${stage}_age_s" | awk 'max<$1{max=$1} END{print max+0}')
  if [ "$max_seen" -le "$max_age_s" ] 2>/dev/null; then
    echo "PASS  ${stage}_freshness (max_seen=${max_seen}s ≤ ${max_age_s}s)"
  else
    echo "FAIL  ${stage}_freshness (max_seen=${max_seen}s > ${max_age_s}s)"
    fails=$((fails+1))
  fi
done

# 2. stderr clean — 마지막 샘플의 stderr.total = 0
stderr_total=$(tail -1 "$SAMPLES" | jq -r '.stderr.total')
if [ "$stderr_total" -eq 0 ] 2>/dev/null; then
  echo "PASS  stderr_clean (total=0)"
else
  echo "FAIL  stderr_clean (total=$stderr_total)"
  fails=$((fails+1))
fi

# 3. anomaly_fired — 모든 샘플 anom_sum_1h 합계 >= 1
anom_sum=$(jq -s 'map(.anom_sum_1h) | add' "$SAMPLES")
if [ "$anom_sum" -ge 1 ] 2>/dev/null; then
  echo "PASS  anomaly_fired (cumulative_1h_sum=$anom_sum)"
else
  echo "FAIL  anomaly_fired (cumulative_1h_sum=$anom_sum)"
  fails=$((fails+1))
fi

# 4. duration — 최소 40 samples, span >= 23h (48 samples * 30min = 24h, 23h allow slack)
min_samples=$(jq -r 'select(.id=="duration_24h") | .min_samples' "$CRITERIA")
if [ "$n" -ge "$min_samples" ] && [ "$span_s" -ge $((23*3600)) ]; then
  echo "PASS  duration_24h (samples=$n ≥ $min_samples, span=${span_h}h ≥ 23h)"
else
  echo "FAIL  duration_24h (samples=$n, span=${span_h}h)"
  fails=$((fails+1))
fi

# 5. sample_gap — 최대 gap < 5400s
max_gap=$(jq -r '.ts' "$SAMPLES" | awk -F: '
BEGIN{prev=0; max=0}
{
  cmd="date -jf %Y-%m-%dT%H:%M:%SZ \"" $0 "\" +%s"; cmd | getline t; close(cmd)
  if (prev>0) { g=t-prev; if (g>max) max=g }
  prev=t
}
END{print max+0}')
max_gap_threshold=$(jq -r 'select(.id=="sample_uniformity") | .max_gap_s' "$CRITERIA")
if [ "$max_gap" -le "$max_gap_threshold" ] 2>/dev/null; then
  echo "PASS  sample_uniformity (max_gap=${max_gap}s ≤ ${max_gap_threshold}s)"
else
  echo "FAIL  sample_uniformity (max_gap=${max_gap}s > ${max_gap_threshold}s)"
  fails=$((fails+1))
fi

echo "---"
if [ $fails -eq 0 ]; then
  echo "VERDICT: PASS (6/6)"
  exit 0
else
  echo "VERDICT: FAIL ($fails criteria failed)"
  exit 1
fi
