# Gate 트러블슈팅 & 수렴 원칙

## 아키텍처

```
Mac (hexa/python3/cargo/rustc/sh)
  ↓ ~/.hx/bin/ wrapper
  ↓ nc -z 체크
  ├─ Wi-Fi (192.168.50.119:9900) ← 1순위
  ├─ Tailscale (100.96.193.56:9900) ← 2순위 (외부/핫스팟)
  └─ 로컬 fallback ← gate 없을 때
  ↓
Ubuntu socat daemon (gate_daemon.hexa)
  ↓ gate_handler.sh
  ↓ 명령 실행 → 결과 반환
```

## Wrapper 목록

| 파일 | 위치 | Ubuntu 실행 | 로컬 fallback |
|---|---|---|---|
| `hexa` | `~/Dev/hexa-lang/target/release/hexa` | 모든 `hexa run` | gate 없음 / `HEXA_LOCAL=1` |
| `python3` | `~/.hx/bin/python3` | `.py` 파일 실행 | `-c` 인라인 / REPL |
| `python` | `~/.hx/bin/python` → python3 | 동일 | 동일 |
| `cargo` | `~/.hx/bin/cargo` | 모든 cargo 명령 | gate 없음 |
| `rustc` | `~/.hx/bin/rustc` | `.rs` 컴파일+실행 | gate 없음 |
| `sh-run` | `~/.hx/bin/sh-run` | `.sh` 파일 실행 | gate 없음 |

## 설정 파일

- `nexus/shared/gate_config.jsonl` — IP, 포트, SSH alias, Tailscale IP
- `nexus/shared/gate_offload.jsonl` — (레거시, 현재 미사용: 모든 hexa run 오프로드)

## 강제 로컬 실행

```bash
HEXA_LOCAL=1 hexa run script.hexa   # hexa만
GATE_LOCAL=1 python3 script.py      # python3/cargo/rustc/sh-run
```

## 트러블슈팅

### 1. `command not found: _ubu_with_file`

**원인**: zshrc에서 `python3()`, `cargo()` 등 함수를 정의했는데, Claude Code shell snapshot에 함수만 복사되고 `_ubu_with_file` 헬퍼는 복사 안 됨.

**해결**: zshrc에서 함수 제거, `~/.hx/bin/` wrapper에 위임.

```bash
# zshrc에서 이것들 제거:
# cargo()    { _ubu_with_file ... }
# python3()  { _ubu_with_file ... }
# rustc()    { _ubu_with_file ... }
# hexa()     { _ubu_with_file ... }

# 기존 스냅샷 정리:
for f in ~/.claude-claude6/shell-snapshots/snapshot-zsh-*.sh; do
  sed -i '' '/^python3 ()/,/^}/d; /^cargo ()/,/^}/d; /^rustc ()/,/^}/d; /^hexa ()/,/^}/d' "$f"
done
```

### 2. 절대경로 호출 시 gate 우회

**원인**: `$HEXA=~/Dev/hexa-lang/target/release/hexa && $HEXA run ...` 형태로 호출하면 zshrc 함수 우회.

**해결**: 바이너리를 `hexa-bin`으로 이름변경, 원래 경로에 wrapper 설치. 절대경로 호출도 wrapper를 탐.

### 3. Tailscale 연결 안 됨

```bash
# Ubuntu에서:
sudo tailscale up         # 로그인
tailscale ip -4           # IP 확인

# Mac에서:
tailscale status          # 양쪽 보이는지 확인
nc -z -w 2 <TS_IP> 9900  # gate 포트 접근 확인
```

### 4. gate_daemon 안 돌아감

```bash
# Ubuntu에서:
ssh ubu 'pgrep -fl socat'                    # socat 프로세스 확인
ssh ubu 'cat /tmp/airgenome/gate.pid'        # PID 확인

# 재시작:
ssh ubu 'cd /tmp/airgenome && /tmp/hexa-build/hexa-lang/target/release/hexa run gate_daemon.hexa start 9900'
```

### 5. Ubuntu hexa 빌드 오래됨

```bash
ssh ubu 'source ~/.cargo/env && cd /tmp/hexa-build/hexa-lang && git pull && cargo build --release'
```

## 수렴 원칙

1. **단일 진입점**: 모든 언어 실행은 `~/.hx/bin/` wrapper 하나로 수렴. zshrc 함수 중복 금지.
2. **설정은 jsonl**: 하드코딩 없음. `gate_config.jsonl`에서 동적 로드.
3. **자동 전환**: Wi-Fi → Tailscale → 로컬. 사용자 개입 없음.
4. **프로세스 kill 없음**: Prime Directive 준수. 양쪽 모두.
5. **스냅샷 독립**: wrapper는 독립 실행 가능한 bash 스크립트. 외부 함수 의존 없음. Claude Code shell snapshot 문제 원천 차단.
