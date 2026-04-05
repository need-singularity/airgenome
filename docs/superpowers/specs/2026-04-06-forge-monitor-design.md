# airgenome/forge — Claude Code 해킹 모니터 + 대화 압축

**Date**: 2026-04-06
**Status**: Design approved

## Problem

Claude Code CLI의 내부 데이터(채팅 히스토리 3.3GB+, OAuth 토큰 10개, 실시간 사용량 API)는
공식 API로 노출되지 않는다. 이 데이터에 비정식 경로로 접근하여:

1. 모든 세션을 airgenome 6-axis genome으로 투영
2. 과거 대화를 token-forge 방식으로 압축
3. CLI + macOS 메뉴바 아이콘 두 인터페이스로 모니터링 on/off 제공

hexa-lang으로 전체 구현. ccmon(Python)과 token-forge(Python)를 hexa 모듈로 흡수.

## Attack Surface Map

| Vector | Path | Method |
|--------|------|--------|
| Conversations | `~/.claude*/projects/**/*.jsonl` | Plaintext JSONL parse |
| OAuth tokens (10 accounts) | macOS Keychain | `security find-generic-password` |
| Usage API | `api.anthropic.com/api/oauth/usage` | Bearer token + curl |
| Token refresh | Keychain read → refresh → Keychain write | `security` add/delete |
| Process state | `/Users/ghost/.local/bin/claude` (Mach-O arm64) | `lsof`, `ps` |
| Session metadata | `~/.claude*/sessions/*.json` | PID, cwd, startedAt |
| Input history | `~/.claude*/history.jsonl` | All prompts (32K+ lines) |
| Stats cache | `~/.claude/stats-cache.json` | Daily message/session/tool counts |

## Architecture

```
airgenome/
├── docs/
│   ├── gates.hexa              # Existing genome spec (untouched)
│   ├── forge.hexa              # NEW — main module
│   └── menubar.jxa.template    # NEW — JXA template for menu bar
├── src/                        # Existing Rust stubs (untouched)
├── forge/                      # NEW — forge runtime data
│   ├── genomes/                # Per-session genome signatures
│   ├── compressed/             # Compressed conversation summaries
│   └── state.json              # Daemon state (pid, on/off, last scan)
└── Cargo.toml                  # Existing
```

## Module: forge.hexa

### New Effects

```hexa
effect KeychainSensor {
    fn read_credential(label: str) -> str     // security find-generic-password -l <label> -w
    fn write_credential(label: str, service: str, account: str, data: str) -> ()
}

effect UsageAPI {
    fn fetch_usage(token: str) -> str         // curl usage API
    fn refresh_token(refresh: str) -> str     // curl token endpoint
}

effect ConversationScanner {
    fn list_sessions(config_dir: str) -> str  // glob projects/**/*.jsonl
    fn read_session(path: str) -> str         // read JSONL file
}
```

All effects implemented via `exec()` — no FFI, no external dependencies.

### 6-Axis Mapping (Claude Code Sessions)

| Axis | Claude Code Metric | Source |
|------|-------------------|--------|
| Cpu | Token throughput (input+output / elapsed time) | JSONL `usage.input_tokens`, `usage.output_tokens` |
| Ram | Context size (cache_read + uncached input) | JSONL `usage.cache_read_input_tokens` |
| Gpu | Tool call density (calls / messages) | JSONL `type: "tool_use"` count |
| Npu | Thinking depth (thinking block char length) | JSONL `thinking` field length |
| Power | Cost in USD | Model pricing: Opus $15/$75, Sonnet $3/$15, Haiku $0.80/$4 |
| Io | File operations (Read/Write/Edit/Glob/Grep) | JSONL tool_use name parsing |

Each session produces a 60-byte genome via the existing 15-pair gate mechanism.

### Keychain Hack (10 Accounts)

Absorb from ccmon `usage_api.py`:

```hexa
let KEYCHAIN_LABELS = {
    "claude1": "Claude Code-credentials-2b949392",
    "claude2": "Claude Code-credentials-26492e81",
    "claude3": "Claude Code-credentials-200a5e32",
    "claude4": "Claude Code-credentials-2adfc64a",
    "claude5": "Claude Code-credentials-7f9319ca",
    "claude6": "Claude Code-credentials-71a31b2a",
    "claude7": "Claude Code-credentials-02580ebb",
    "claude8": "Claude Code-credentials-156d23f8",
    "claude9": "Claude Code-credentials-90c36814",
    "claude10": "Claude Code-credentials-90defae3",
}

let TOKEN_URL = "https://platform.claude.com/v1/oauth/token"
let USAGE_URL = "https://api.anthropic.com/api/oauth/usage"
let CLIENT_ID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
```

Flow:
1. `exec("security find-generic-password -l '<label>' -w")` → JSON with `claudeAiOauth.accessToken`
2. Parse JSON, extract `accessToken` + `refreshToken` + `expiresAt`
3. If expired: `exec("curl -s TOKEN_URL -d '{refresh_token: ...}'")` → new tokens
4. Save back: `exec("security delete-generic-password ...")` then `exec("security add-generic-password ...")`
5. Call usage API: `exec("curl -s USAGE_URL -H 'Authorization: Bearer <token>'")` → `five_hour.utilization`, `seven_day.utilization`

### Conversation Compression (token-forge Absorption)

Absorb from token-forge `tf/forge.py`:

**Phase 1 — Genome Extraction** (per session):
- Parse JSONL → extract `type: "assistant"` messages with `usage` fields
- Compute 6-axis values per message window (sliding window of 10 messages)
- Project through 15-pair gates → 60-byte genome per window
- Store: `forge/genomes/<session-id>.genome` (compact binary)

**Phase 2 — History Compression** (per session):
- Extract user messages + assistant summaries (strip tool results, thinking signatures)
- Iterative self-compression: hexa generates compression prompt → `exec("claude -p '...'")` or direct JSONL truncation
- Convergence: stop when delta_ratio < 0.01
- Store: `forge/compressed/<session-id>.tf` (compressed text)
- Target: 3.3GB → ~100MB (97% reduction)

**Phase 3 — Pattern Analysis**:
- Accumulate per-project genome signatures over time
- Cross-project signature comparison (which project costs most? which has deepest thinking?)
- Temporal patterns (daily/weekly usage cycles)

### Monitoring Daemon

```hexa
fn monitor_toggle(action: str) -> () {
    // action = "on" | "off" | "status"
    let state_path = "forge/state.json"

    if action == "on" {
        // Write PID file, start background loop
        let pid = exec("echo $$").trim()
        write_file(state_path, '{"pid": ' + pid + ', "active": true}')
        monitor_loop()
    }
    if action == "off" {
        let state = read_file(state_path)
        // Parse PID, send SIGTERM
        write_file(state_path, '{"active": false}')
    }
}

fn monitor_loop() -> () {
    // Infinite loop with adaptive interval (like gates.hexa consciousness block)
    // 1. Scan active sessions (lsof + sessions/*.json)
    // 2. For each active session: compute live genome
    // 3. Call usage API for all accounts
    // 4. Update menubar via JXA
    // 5. If genomes.log grows > threshold: trigger compression
    // 6. Sleep adaptive interval (30s idle, 5s active)
}
```

### CLI Interface

```
hexa run docs/forge.hexa [command]

Commands:
  on              Start monitoring daemon
  off             Stop monitoring daemon
  status          Show current state (accounts, active sessions, genome grade)
  scan            One-shot: scan all conversations, generate genomes
  compress        One-shot: compress old sessions
  usage           Show 10-account usage dashboard
  menubar         Launch macOS menu bar icon
```

Invocation: the consciousness block in `forge.hexa` reads `argv` to dispatch.

### Menu Bar (hexa → JXA)

hexa generates and executes JavaScript for Automation (JXA) via `exec("osascript -l JavaScript ...")`.

**Template** (`menubar.jxa.template`):
```javascript
ObjC.import('Cocoa');
ObjC.import('WebKit');

// Create status bar item
const statusBar = $.NSStatusBar.systemStatusBar;
const statusItem = statusBar.statusItemWithLength($.NSVariableStatusItemLength);
statusItem.button.title = $('%ICON%');

// Create menu
const menu = $.NSMenu.alloc.init;

// Toggle item
const toggleItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(
    $('%TOGGLE_LABEL%'), 'toggle:', $(''));
menu.addItem(toggleItem);

// Separator
menu.addItem($.NSMenuItem.separatorItem);

// Account usage items (10 accounts)
%ACCOUNT_ITEMS%

// Separator
menu.addItem($.NSMenuItem.separatorItem);

// Hexagon WebView item (mini visualization)
%HEXAGON_VIEW%

// Quit
const quitItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(
    $('Quit'), 'terminate:', $('q'));
menu.addItem(quitItem);

statusItem.menu = menu;

// Run loop
$.NSApplication.sharedApplication;
$.NSApp.run;
```

hexa fills `%ICON%`, `%TOGGLE_LABEL%`, `%ACCOUNT_ITEMS%`, `%HEXAGON_VIEW%` at runtime.

**Hexagon Mini Visualization**:
- hexa writes SVG to `/tmp/airgenome-hexagon.html`
- JXA menu contains WKWebView item loading the local HTML
- SVG shows 6-axis radar chart with current genome values
- Color coding: green (breakthrough), yellow (approaching singularity), red (below threshold)

**Update cycle**:
- Timer fires every 5s in JXA run loop
- Reads `forge/state.json` for latest data
- Updates icon color: green=running, gray=off, yellow=warning (high usage)
- Updates menu item labels with live data

### Data Flow

```
macOS Keychain ──┐
                 ├──→ forge.hexa ──→ forge/state.json ──→ JXA menubar
Claude JSONL ────┤       │
ps / lsof ───────┘       ├──→ forge/genomes/*.genome
                          ├──→ forge/compressed/*.tf
                          ├──→ genomes.log (append)
                          └──→ stdout (CLI output)
```

### Absorbed Components

| Source | What | Destination in forge.hexa |
|--------|------|--------------------------|
| ccmon/usage_api.py | Keychain read, token refresh, usage API | `KeychainSensor` effect + `fetch_usage()` |
| ccmon/pricing.py | Model pricing table | `let PRICING = {...}` constant |
| ccmon/parser.py | JSONL parsing, date aggregation | `ConversationScanner` effect + `scan_sessions()` |
| ccmon/report.py | Dashboard rendering, account ranking | `render_status()` + `rank_accounts()` |
| ccmon/launcher.sh | Multi-account launcher | Not absorbed (remains separate orchestration) |
| tf/forge.py | Iterative compression engine | `compress_session()` |
| tf/anvil.py | Semantic validation (Q&A test) | `verify_compression()` |
| tf/profiler.py | Section-level density analysis | `profile_session()` |

### Policy (Prime Directive Compliance)

- **READ-ONLY** on Claude Code process — no modification, no injection
- **READ-ONLY** on Keychain — only reads existing credentials (refresh is write-back of same credential)
- **No process killing** — monitoring observes, never controls
- **File writes** only to `forge/` directory and `genomes.log`
- Usage API calls are **read-only** GET requests

### Success Criteria

1. `hexa run docs/forge.hexa on` starts background monitoring
2. `hexa run docs/forge.hexa off` stops it cleanly
3. `hexa run docs/forge.hexa status` shows 10-account usage + active session genomes
4. `hexa run docs/forge.hexa menubar` launches macOS menu bar with live hexagon
5. `hexa run docs/forge.hexa compress` reduces conversation history by >90%
6. `hexa run docs/forge.hexa scan` generates genome signatures for all historical sessions
7. Menu bar icon shows real-time: on/off state, account usage bars, hexagon visualization
8. All implemented in hexa-lang, no Python/Swift/external runtime required
