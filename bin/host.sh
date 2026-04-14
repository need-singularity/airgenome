#!/usr/bin/env bash
# bin/host.sh — host registry CLI (P1).
#
# SSOT: shared/config/hosts.json. 이 스크립트 외의 직접 편집 후에는 `host.sh sync` 필수.
#
# Commands:
#   list                                     — registry + ssh reachability + sudo -n 상태
#   add <name> <user> <ip> [flags...]        — ssh-copy-id → sudoers → ~/.ssh/config → slice 배포 → registry
#   remove <name> [--hard]                   — soft: enabled=false / hard: ssh_config 항목 제거 + registry 삭제
#   sync                                     — enabled 호스트 전체 slice 배포 + probe self-test
#
# add flags:
#   --no-gpu                  has_gpu=false (기본 false; GPU 있으면 --gpu)
#   --gpu                     has_gpu=true
#   --tier primary|secondary  기본 primary (LAN), remote 는 secondary 권장
#   --kind lan|remote         기본 lan
#   --threads N               기본 = ssh 로 nproc 조회
#
# 비밀번호 주입: 환경변수 SSHPASS (sshpass -e). 미설정 시 ssh-copy-id 가 prompt.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REG="$ROOT/shared/config/hosts.json"
SLICE_DIR="$ROOT/shared/systemd"
SSH_CFG="$HOME/.ssh/config"

die() { echo "error: $*" >&2; exit 2; }
info() { echo "  ▸ $*"; }

require_jq() { command -v jq >/dev/null 2>&1 || die "jq 필요 (brew install jq)"; }

# ── registry IO ──────────────────────────────────────────────────
reg_read() { jq "$@" "$REG"; }

reg_write() {
    # stdin 의 JSON 을 atomic write. jq 결과로 스키마 유지.
    local tmp="$REG.tmp.$$"
    cat > "$tmp"
    jq -e . "$tmp" >/dev/null || { rm -f "$tmp"; die "registry write: invalid JSON"; }
    mv -f "$tmp" "$REG"
}

reg_enabled_hosts() {
    # kind != self 만. ssh_alias 반환.
    reg_read -r '.hosts | to_entries[] | select(.value.enabled == true and .value.kind != "self") | .value.ssh_alias'
}

reg_has() { reg_read -e ".hosts[\"$1\"]" >/dev/null 2>&1; }

# ── commands ─────────────────────────────────────────────────────
cmd_list() {
    require_jq
    printf "%-8s %-10s %-8s %-6s %-8s %-5s %-8s %s\n" NAME ALIAS KIND GPU THREADS EN REACH SUDO
    local rows
    rows=$(reg_read -r '.hosts | to_entries[] | [.key, (.value.ssh_alias // "-"), .value.kind, (.value.has_gpu|tostring), (.value.threads|tostring), (.value.enabled|tostring)] | @tsv')
    # 배열로 받아 iterate — ssh 가 stdin 먹지 못하게 `< /dev/null` 부착.
    while IFS=$'\t' read -r name alias kind gpu threads en; do
        local reach="-" sudo="-"
        if [ "$kind" != "self" ] && [ "$alias" != "-" ]; then
            if ssh -o BatchMode=yes -o ConnectTimeout=3 "$alias" true </dev/null 2>/dev/null; then
                reach="ok"
                if ssh -o BatchMode=yes -o ConnectTimeout=3 "$alias" sudo -n true </dev/null 2>/dev/null; then sudo="ok"; else sudo="no"; fi
            else reach="fail"
            fi
        fi
        printf "%-8s %-10s %-8s %-6s %-8s %-5s %-8s %s\n" "$name" "$alias" "$kind" "$gpu" "$threads" "$en" "$reach" "$sudo"
    done <<< "$rows"
}

ssh_config_add() {
    local alias=$1 ip=$2 user=$3
    grep -qE "^Host[[:space:]]+$alias\b" "$SSH_CFG" 2>/dev/null && { info "~/.ssh/config 에 $alias 이미 있음 — 스킵"; return 0; }
    info "~/.ssh/config 에 $alias 항목 추가"
    cat >> "$SSH_CFG" <<EOF

Host $alias
    HostName $ip
    User $user
    ControlMaster auto
    ControlPath ~/.ssh/ctl-%r@%h:%p
    ControlPersist 10m
    Compression no
EOF
    chmod 600 "$SSH_CFG"
}

ssh_config_remove() {
    local alias=$1
    grep -qE "^Host[[:space:]]+$alias\b" "$SSH_CFG" 2>/dev/null || { info "~/.ssh/config 에 $alias 없음 — 스킵"; return 0; }
    info "~/.ssh/config 에서 $alias 블록 제거"
    local tmp="$SSH_CFG.tmp.$$"
    awk -v a="$alias" '
        BEGIN { skip=0 }
        /^Host[[:space:]]+/ { skip = ($2 == a) ? 1 : 0 }
        { if (!skip) print }
    ' "$SSH_CFG" > "$tmp"
    mv -f "$tmp" "$SSH_CFG"
    chmod 600 "$SSH_CFG"
}

copy_key_and_sudoers() {
    local alias=$1 user=$2
    # ssh-copy-id 는 agent 로드 키를 우선해서 잘못된 키를 복사할 수 있음 (2026-04-14 ubu2 사건).
    # 반드시 Mac 의 id_ed25519.pub 를 -i 로 명시. 없으면 id_rsa.pub 로 fallback.
    local keyfile=""
    if [ -r "$HOME/.ssh/id_ed25519.pub" ]; then
        keyfile="$HOME/.ssh/id_ed25519.pub"
    elif [ -r "$HOME/.ssh/id_rsa.pub" ]; then
        keyfile="$HOME/.ssh/id_rsa.pub"
    else
        die "ssh pubkey 없음 — ssh-keygen 으로 id_ed25519 먼저 생성"
    fi
    info "ssh-copy-id $alias (key=$keyfile, SSHPASS=${SSHPASS:+set}${SSHPASS:-prompt})"
    if [ -n "${SSHPASS:-}" ] && command -v sshpass >/dev/null 2>&1; then
        sshpass -e ssh-copy-id -i "$keyfile" -o StrictHostKeyChecking=accept-new "$alias" >/dev/null
    else
        ssh-copy-id -i "$keyfile" -o StrictHostKeyChecking=accept-new "$alias"
    fi
    info "sudoers NOPASSWD 설치"
    if [ -n "${SSHPASS:-}" ]; then
        ssh "$alias" "echo '${SSHPASS}' | sudo -S sh -c 'echo \"$user ALL=(ALL) NOPASSWD:ALL\" > /etc/sudoers.d/$user && chmod 440 /etc/sudoers.d/$user'"
    else
        ssh -t "$alias" "sudo sh -c 'echo \"$user ALL=(ALL) NOPASSWD:ALL\" > /etc/sudoers.d/$user && chmod 440 /etc/sudoers.d/$user'"
    fi
    ssh "$alias" sudo -n true || die "sudo -n 검증 실패"
}

slice_deploy() {
    local alias=$1
    info "systemd slice 3종 배포 → $alias"
    scp "$SLICE_DIR"/airgenome-real.slice "$SLICE_DIR"/airgenome-bkgnd.slice "$SLICE_DIR"/airgenome-stress.slice "$alias":/tmp/ >/dev/null
    ssh "$alias" "sudo -n mv /tmp/airgenome-*.slice /etc/systemd/system/ && sudo -n systemctl daemon-reload"
}

cmd_add() {
    require_jq
    [ $# -ge 3 ] || die "usage: host.sh add <name> <user> <ip> [flags]"
    local name=$1 user=$2 ip=$3; shift 3
    local has_gpu=false kind=lan tier=primary threads=""
    while [ $# -gt 0 ]; do
        case "$1" in
            --gpu) has_gpu=true ;;
            --no-gpu) has_gpu=false ;;
            --tier) tier=$2; shift ;;
            --kind) kind=$2; shift ;;
            --threads) threads=$2; shift ;;
            *) die "unknown flag: $1" ;;
        esac; shift
    done
    reg_has "$name" && die "host '$name' 이미 등록됨. 먼저 remove 하거나 다른 이름"

    ssh_config_add "$name" "$ip" "$user"
    copy_key_and_sudoers "$name" "$user"
    if [ -z "$threads" ]; then
        threads=$(ssh "$name" nproc 2>/dev/null || echo 0)
    fi
    slice_deploy "$name"

    info "registry 추가 — $name (gpu=$has_gpu, kind=$kind, tier=$tier, threads=$threads)"
    local tags='["compute"]'
    [ "$has_gpu" = "true" ] && tags='["compute","gpu"]'
    [ "$kind" = "remote" ] && [ "$has_gpu" = "false" ] && tags='["compute","heavy"]'
    reg_read --arg n "$name" --arg a "$name" --arg k "$kind" --argjson g "$has_gpu" --argjson t "$threads" --arg ti "$tier" --argjson tg "$tags" \
        '.hosts[$n] = {enabled: true, ssh_alias: $a, kind: $k, has_gpu: $g, threads: $t, tier: $ti, tags: $tg}' | reg_write

    info "probe 검증"
    bash "$ROOT/bin/remote_load.sh" --self-test | tail -5
    info "✅ host '$name' 추가 완료"
}

cmd_remove() {
    require_jq
    [ $# -ge 1 ] || die "usage: host.sh remove <name> [--hard]"
    local name=$1; shift
    local hard=false
    [ "${1:-}" = "--hard" ] && hard=true
    reg_has "$name" || die "host '$name' 없음"

    if [ "$hard" = "true" ]; then
        local alias
        alias=$(reg_read -r --arg n "$name" '.hosts[$n].ssh_alias // ""')
        [ -n "$alias" ] && ssh_config_remove "$alias"
        info "registry 에서 $name 삭제"
        reg_read --arg n "$name" 'del(.hosts[$n])' | reg_write
    else
        info "registry 에서 $name 비활성화 (enabled=false)"
        reg_read --arg n "$name" '.hosts[$n].enabled = false' | reg_write
    fi
    info "✅ host '$name' ${hard:+hard-}remove 완료"
}

cmd_sync() {
    require_jq
    info "enabled 원격 호스트 slice 배포"
    local aliases
    aliases=$(reg_enabled_hosts)
    while IFS= read -r alias; do
        [ -z "$alias" ] && continue
        slice_deploy "$alias"
    done <<< "$aliases"
    info "remote_load self-test"
    bash "$ROOT/bin/remote_load.sh" --self-test | tail -5
    info "probe self-test"
    hexa run "$ROOT/modules/probe.hexa" self-test | tail -3
    info "dispatch self-test"
    hexa run "$ROOT/modules/dispatch.hexa" self-test | tail -3
    info "✅ sync 완료"
}

case "${1:-}" in
    list) shift; cmd_list "$@" ;;
    add) shift; cmd_add "$@" ;;
    remove) shift; cmd_remove "$@" ;;
    sync) shift; cmd_sync "$@" ;;
    ""|-h|--help) sed -n '1,30p' "$0" | sed -n '/^# /p' >&2; exit 2 ;;
    *) die "unknown command: $1" ;;
esac
