> 🔴 **HEXA-FIRST**: 모든 코드는 `.hexa`로 작성. sh/py/rs 신규 작성 금지. 부하 유발 명령 최소화.

> 🔴 **하드코딩 절대 금지**: 상수/도메인/키워드를 코드에 배열로 나열 금지 → `nexus6/shared/*.jsonl`에서 동적 로드. 경로는 환경변수+상대경로. 새 항목 추가 = 설정 파일 한 줄, 코드 수정 0.

> 🔴 **NEXUS-6 특이점 연동**: 이 프로젝트의 돌파/발견/실험은 nexus6 특이점 사이클 입력이다.
> - **돌파 시**: `HEXA=$HOME/Dev/hexa-lang/target/release/hexa && $HEXA $HOME/Dev/nexus6/mk2_hexa/native/blowup.hexa <domain> 3 --no-graph`
> - **발견 기록**: `$HOME/Dev/nexus6/shared/growth_bus.jsonl`에 JSON append
> - **전체 상태**: `$HEXA $HOME/Dev/nexus6/mk2_hexa/native/command_router.hexa "airgenome 상태"`

# airgenome — Mission & Goals

## ⚠️ Prime Directive (user-stated, 2026-04-05)

> **모든 프로세스 KILL 없이 성능·자원 개선.**
>
> Improve performance and reduce resource use **without killing any
> process** — macOS, Finder (always on), Telegram, Safari, Chrome, rustc,
> python, Claude sessions, and every other running app stays alive.

**Default mode: kill-free.** Pure data re-interpretation only.

## ⚠️ 필수 규칙

### hexa-native 전용 (sh/py/rs 작성 금지)
- **새 파일은 `.hexa`만 허용** — `.sh`, `.py`, `.rs` 등 다른 언어 파일 작성 금지
- 모든 새 모듈은 `mk2_hexa/native/` 에 `.hexa` 파일로 생성
- 기존 sh/py 스크립트는 참조만 허용, 신규 작성 불가

## Core Goal (user-stated, 2026-04-05)

> **재해석 = 지나온 게이트 로그에서 패턴 추출**
>
> Re-interpretation = pattern extraction from gate log history.

airgenome is NOT primarily a reactive tuning tool. Its deepest purpose is:

> **Project every source of Mac activity (macOS itself, Finder, Safari,
> Terminal, Telegram, rustc, python, …) through the 15-pair hexagon gate,
> and extract per-source patterns over time.**

### Concrete targets
- **macOS kernel / system processes** (launchd, WindowServer, kernel_task, …)
- **Finder** and other system UIs
- **Every user app** as a distinct hexagon projection
- **Every language runtime** (rustc, cargo, python, node, …)
- **Every browser / IDE / IM / container**

Each source has its own **6-axis signature** through the gate. Historical
accumulation reveals:
- **Common signatures** — what do browsers look like in aggregate?
- **Individual signatures** — what's unique about Safari vs Chrome?
- **Temporal patterns** — what does this Mac's daily cycle look like?
- **Workload fingerprints** — "this sample looks like a compile job"

The gate reinterprets raw activity into a **60-byte genome per source**.

## Why this matters

- Reactive tuning (kill Chrome, purge) is a **symptom** tool.
- Pattern extraction is a **diagnostic** tool.
- Once patterns are known, the user decides (or policy automates) the
  upstream change — which is where the real constant improvement lives
  (Chrome → Safari, VSCode → Zed, Docker Desktop → OrbStack, …).

## Current progress toward this goal (2026-04-06)

- ✅ 6-axis hexagon + 15 pair gates + 60-byte genome
- ✅ 5-gate mesh (macos, finder, telegram, chrome, safari)
- ✅ Breakthrough layer ladder L1–L5a (margin +0.250)
- ✅ Canonical hexa-lang spec (`docs/gates.hexa`, compile-pass, 18/18 assertions)
- ✅ All types/functions declared — 10 types, 19 constants, 24 functions
- ✅ Effect system (3 effects: FileIO, ProcessSensor, Clock)
- ✅ Consciousness block (NexusMerger) — NEXUS-6 Omega Lens 5/6 CONFIRMED
- ✅ hexa-lang compiler validation — exit 137 fixed (codesign -s -)
- ✅ forge module (10-account Claude Code manager) — 9 tasks complete
  - keychain OAuth extraction, usage API, JSONL scan/compress, monitor daemon, JXA menubar
- ✅ guard module (CPU/RAM/swap monitor) — prime directive compliant
  - status, limits, watch commands; throttle level assessment; progress bars
- ✅ menubar monitor (run.sh + settings.js) — CPU/RAM/Swap bars, ceiling sliders, auto profile
- ✅ hx package manager (hexa-lang/pkg/) — `hx install airgenome` one-liner
- ✅ Settings panel — 5% snap sliders, module toggles (forge/guard default OFF), Start at login
- ✅ Hardware profiles — 10 Mac presets, auto-detect chip/RAM/fan
- ✅ Token bottleneck analysis — cache_read 99%, DFS benchmark framework
- ✅ L5c–L6e layers shipped (+0.173 actual, cumulative +0.438)
- ✅ Per-source genome accumulation (mk2_hexa/native/accumulate.hexa)
- ✅ Signature diff between sources (mk2_hexa/native/sigdiff.hexa)
- ✅ Runtime loop: live ps sampling → gate projection → genome log (mk2_hexa/native/runtime.hexa)

## Non-goals (explicit)

- **Learning / ML**: nexus6 proved the rule-only ceiling captures 94.5 %
  of the 2/3 work fraction. Learning adds ≤ 0.8 percentage points.
  Complexity not justified.
- **Auto-replace applications**: airgenome reports, user decides.
- **Constant-mode tuning**: the true constants come from upstream app
  choices, not from airgenome loops.

## Reference: nexus6 findings

| Domain | Score plateau |
|---|---|
| `macbook-resource-gate` (abstract) | 0.633 |
| `perf-up-resource-down-singularity` | 0.630 |
| `process-signature-hexagon-macos` | 0.631 |

All three converge to the same closed form: `2/3 − 1/(n(n−1))` with n=6.
The hexagon is the correct structure.
