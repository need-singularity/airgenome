#!/usr/bin/env bash
# bin/stress.sh — Phase R7: stress emergency pause/resume (ubu+hetzner)
#
# 목적: real workload 시작 직전 stress (ag-*.service) 를 일시정지 →
#       경쟁 원천 제거. R4 slice 격리 보완.
#
# Commands:
#   pause          ubu+hetzner 의 ag-*.service 전 프로세스에 SIGSTOP
#   resume         SIGCONT 로 재개
#   status         각 호스트 ag-* units 의 상태 + 프로세스 state(T/S/R)
#   --self-test    pause→status→resume→status 왕복 후 stopped count 검증
#
# 주의: SIGSTOP 은 OS signal 이라 process state 만 멈춤. systemd unit 은
#       여전히 active 로 표시됨. `ps -o stat` 의 'T' 가 실제 pause 증거.

set -u
cd "$(dirname "$0")/.." || exit 1

HOSTS=("ubu" "hetzner")
# ubu: user-level (--user), hetzner: system-level (빈 문자열)
ubu_scope="--user"
hetzner_scope=""

# 호스트별 scope 조회
scope_of() {
    local h="$1"
    case "$h" in
        ubu)     echo "--user" ;;
        hetzner) echo "" ;;
        *)       echo "" ;;
    esac
}

cmd_pause() {
    for h in "${HOSTS[@]}"; do
        local s; s=$(scope_of "$h")
        local n
        n=$(ssh "$h" "units=\$(systemctl $s list-units 'ag-*' --state=active --no-pager --no-legend 2>/dev/null | awk '{print \$1}'); \
            [ -z \"\$units\" ] && { echo 0; exit; }; \
            for u in \$units; do systemctl $s kill --kill-whom=all -s STOP \"\$u\" 2>/dev/null; done; \
            echo \"\$units\" | wc -l" 2>/dev/null) || n=0
        echo "paused $h: $n unit(s)"
    done
}

cmd_resume() {
    for h in "${HOSTS[@]}"; do
        local s; s=$(scope_of "$h")
        local n
        n=$(ssh "$h" "units=\$(systemctl $s list-units 'ag-*' --state=active --no-pager --no-legend 2>/dev/null | awk '{print \$1}'); \
            [ -z \"\$units\" ] && { echo 0; exit; }; \
            for u in \$units; do systemctl $s kill --kill-whom=all -s CONT \"\$u\" 2>/dev/null; done; \
            echo \"\$units\" | wc -l" 2>/dev/null) || n=0
        echo "resumed $h: $n unit(s)"
    done
}

cmd_status() {
    for h in "${HOSTS[@]}"; do
        local s; s=$(scope_of "$h")
        echo "=== $h"
        ssh "$h" "units=\$(systemctl $s list-units 'ag-*' --state=active --no-pager --no-legend 2>/dev/null | awk '{print \$1}'); \
            [ -z \"\$units\" ] && { echo '  (no ag-* units)'; exit; }; \
            for u in \$units; do \
                cg=\$(systemctl $s show -p ControlGroup --value \"\$u\" 2>/dev/null); \
                allpids=\$(cat \"/sys/fs/cgroup\$cg/cgroup.procs\" 2>/dev/null); \
                stopped=0; running=0; \
                for p in \$allpids; do \
                    st=\$(ps -o stat= -p \$p 2>/dev/null | tr -d ' '); \
                    case \"\$st\" in T*) stopped=\$((stopped+1));; *) running=\$((running+1));; esac; \
                done; \
                echo \"  \$u: running=\$running stopped=\$stopped\"; \
            done" 2>/dev/null
    done
}

self_test() {
    echo "stress.sh self-test"
    echo "--- 0. 시작 상태 (기대: 모두 running)"
    cmd_status

    echo "--- 1. pause"
    cmd_pause
    sleep 1

    echo "--- 2. pause 후 상태 (기대: stopped > 0)"
    local st; st=$(cmd_status)
    echo "$st"
    local total_stopped
    total_stopped=$(echo "$st" | grep -oE 'stopped=[0-9]+' | awk -F= '{s+=$2} END{print s+0}')
    if [ "$total_stopped" -lt 1 ]; then
        echo "  FAIL: pause 후에도 stopped=0"
        return 1
    fi
    echo "  PASS: 총 stopped=$total_stopped"

    echo "--- 3. resume"
    cmd_resume
    sleep 1

    echo "--- 4. resume 후 상태 (기대: stopped=0)"
    st=$(cmd_status)
    echo "$st"
    total_stopped=$(echo "$st" | grep -oE 'stopped=[0-9]+' | awk -F= '{s+=$2} END{print s+0}')
    if [ "$total_stopped" -gt 0 ]; then
        echo "  FAIL: resume 후에도 stopped=$total_stopped"
        return 1
    fi
    echo "  PASS: 총 stopped=0, 복구 완료"

    echo "self-test OK"
}

case "${1:-}" in
    pause)       cmd_pause ;;
    resume)      cmd_resume ;;
    status)      cmd_status ;;
    --self-test) self_test ;;
    *) cat <<USAGE
usage: $(basename "$0") <pause|resume|status|--self-test>

R7: ubu+hetzner 의 ag-*.service (openssl speed, blowup.hexa 등 stress)
    를 SIGSTOP/SIGCONT 로 일시정지/재개. 상태 보존.

  pause        모든 ag-* 프로세스에 SIGSTOP (kill -s STOP)
  resume       SIGCONT 로 재개
  status       호스트별 ag-* units + running/stopped 프로세스 수
  --self-test  pause→status→resume→status 왕복 검증
USAGE
       exit 1 ;;
esac
