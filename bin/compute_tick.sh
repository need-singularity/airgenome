#!/usr/bin/env bash
# bin/compute_tick.sh — 30s 주기 tick. dispatch.selection.compute 로 벤치 워크로드 송신.
#
# 목적: ubu2 같은 idle 호스트가 실제 자원 활용 대상이 되는지 live 검증.
# 워크: openssl sha256 2s → 1 core * 2s = 6%*core 정도 부하 (관찰 가능한 수준, 과부하 아님).
#
# 로그: ~/.airgenome/compute_tick.log (stdout) · executor.jsonl (structured).
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="$HOME/.airgenome/compute_tick.log"
mkdir -p "$(dirname "$LOG")"

ts=$(date -u +%FT%TZ)
# iter-4: 4-core 병렬 ~15s 벤치. `openssl speed -seconds N` 은 6 block-size * N 이라
# -bytes 16384 로 1 block 만 측정 → 실 duration = N 초.
# 기대: 15s 워크 × 4 core = ubu2 12core 의 ~40% 활용, load1 ~ 4 peak.
work='
for i in 1 2 3 4; do
  (openssl speed -seconds 15 -bytes 16384 sha256 >/dev/null 2>&1) &
done
wait
echo OK'

rc=0
"$ROOT/bin/executor.sh" compute "$work" >> "$LOG" 2>&1 || rc=$?
echo "[$ts] tick rc=$rc" >> "$LOG"
exit $rc
