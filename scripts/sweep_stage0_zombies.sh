#!/bin/sh
# stage0 좀비 sweep — hexa stage1 dispatcher 가 stage0 를 자식으로 spawn 후
# Claude Code hook timeout 시 부모만 죽고 자식 stage0 가 orphan 으로 누적.
# 5분 이상 살아있는 hexa_stage0 SIGKILL.

ps -eo pid,etime,command | awk '
/hexa_stage0/ && !/awk/ {
    e = $2
    n = split(e, a, ":")
    # dd-hh:mm:ss 또는 hh:mm:ss → 5분 초과 확실
    if (e ~ /-/) { print $1; next }
    if (n == 3) { print $1; next }
    # mm:ss → 분 ≥ 5
    if (n == 2 && a[1]+0 >= 5) print $1
}' | xargs kill -9 2>/dev/null

exit 0
