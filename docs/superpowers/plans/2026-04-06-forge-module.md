# forge Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** airgenome의 첫 번째 플러그인 모듈 `forge` — Claude Code 내부 데이터 해킹(키체인, 대화 JSONL, 사용량 API) + 대화 압축 + CLI/메뉴바 인터페이스. hexa-lang 단일 파일.

**Architecture:** `modules/forge.hexa` 단독 파일로 모든 기능 구현. `hexa run modules/forge.hexa <command>` 로 실행. gates.hexa와 독립 — 공유 타입 없이 자체 완결. 추후 모듈은 `modules/<name>.hexa` 패턴으로 추가.

**Tech Stack:** hexa-lang (exec, args, consciousness, spawn), macOS security CLI, curl, osascript JXA

---

## File Structure

```
airgenome/
├── modules/                      # NEW — plugin module directory
│   └── forge.hexa                # NEW — forge module (단일 파일, ~400 lines)
├── modules/templates/            # NEW — menubar JXA template
│   └── menubar.jxa               # NEW — JXA source
├── forge/                        # NEW — runtime data (gitignored)
│   ├── genomes/                  # Per-session genome signatures
│   ├── compressed/               # Compressed conversation summaries
│   └── state.json                # Daemon state
├── docs/gates.hexa               # UNTOUCHED
├── hexa.toml                     # UNTOUCHED (forge has own entry)
└── .gitignore                    # MODIFY — add forge/
```

---

### Task 1: 프로젝트 구조 + .gitignore

**Files:**
- Create: `modules/` directory
- Create: `forge/` directory
- Modify: `.gitignore`

- [ ] **Step 1: 디렉토리 생성**

```bash
mkdir -p modules/templates forge/genomes forge/compressed
```

- [ ] **Step 2: .gitignore 업데이트**

`.gitignore`에 추가:
```
forge/genomes/
forge/compressed/
forge/state.json
```

- [ ] **Step 3: forge 빈 state 파일 생성**

`forge/state.json`:
```json
{"active": false, "pid": null, "last_scan": null}
```

- [ ] **Step 4: Commit**

```bash
git add modules/ forge/ .gitignore
git commit -m "scaffold: modules/ + forge/ runtime directories"
```

---

### Task 2: forge.hexa 뼈대 — args 라우팅 + constants

**Files:**
- Create: `modules/forge.hexa`

- [ ] **Step 1: forge.hexa 기본 구조 작성**

```hexa
// airgenome/modules/forge.hexa — Claude Code hacking module
//
// Usage: ~/Dev/hexa-lang/hexa run modules/forge.hexa <command>
// Commands: status, usage, scan, compress, on, off, menubar

// ═══════════════════════════════════════════════════════════════════════
//  CONSTANTS
// ═══════════════════════════════════════════════════════════════════════

let VERSION = "0.1.0"
let FORGE_DIR = "forge"
let STATE_FILE = "forge/state.json"
let GENOMES_DIR = "forge/genomes"
let COMPRESSED_DIR = "forge/compressed"

// Claude Code data paths
let CLAUDE_DIRS = [
    "~/.claude",
    "~/.claude-claude1"
]

// 10-account keychain labels (from ccmon)
let KEYCHAIN_LABELS = [
    "Claude Code-credentials-2b949392",
    "Claude Code-credentials-26492e81",
    "Claude Code-credentials-200a5e32",
    "Claude Code-credentials-2adfc64a",
    "Claude Code-credentials-7f9319ca",
    "Claude Code-credentials-71a31b2a",
    "Claude Code-credentials-02580ebb",
    "Claude Code-credentials-156d23f8",
    "Claude Code-credentials-90c36814",
    "Claude Code-credentials-90defae3"
]
let ACCOUNT_NAMES = [
    "claude1", "claude2", "claude3", "claude4", "claude5",
    "claude6", "claude7", "claude8", "claude9", "claude10"
]

let USAGE_URL = "https://api.anthropic.com/api/oauth/usage"
let TOKEN_URL = "https://platform.claude.com/v1/oauth/token"
let CLIENT_ID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"

// Model pricing (per 1M tokens)
let OPUS_INPUT = 15.0
let OPUS_OUTPUT = 75.0
let SONNET_INPUT = 3.0
let SONNET_OUTPUT = 15.0

// 6-axis genome constants (shared with gates.hexa)
let AXIS_COUNT = comptime { 6 }
let PAIR_COUNT = comptime { 15 }
let SINGULARITY = comptime { 2.0 / 3.0 }

// ═══════════════════════════════════════════════════════════════════════
//  EFFECTS
// ═══════════════════════════════════════════════════════════════════════

effect KeychainSensor {
    fn read_credential(label: str) -> str
    fn write_credential(label: str, data: str) -> ()
}

effect ConversationScanner {
    fn list_sessions(config_dir: str) -> str
    fn read_session(path: str) -> str
}

// ═══════════════════════════════════════════════════════════════════════
//  COMMAND ROUTER
// ═══════════════════════════════════════════════════════════════════════

consciousness ForgeRouter {
    let argv = args()
    // argv[0] = hexa, argv[1] = run, argv[2] = modules/forge.hexa, argv[3] = command
    let cmd = "help"
    if len(argv) > 3 {
        cmd = argv[3]
    }

    println("forge v" + VERSION)

    if cmd == "status" {
        forge_status()
    }
    if cmd == "usage" {
        forge_usage()
    }
    if cmd == "scan" {
        forge_scan()
    }
    if cmd == "compress" {
        forge_compress()
    }
    if cmd == "on" {
        forge_on()
    }
    if cmd == "off" {
        forge_off()
    }
    if cmd == "menubar" {
        forge_menubar()
    }
    if cmd == "help" {
        println("  commands: status usage scan compress on off menubar")
    }
}
```

- [ ] **Step 2: 스텁 함수들 추가**

```hexa
// ═══════════════════════════════════════════════════════════════════════
//  COMMAND STUBS
// ═══════════════════════════════════════════════════════════════════════

fn forge_status() -> () {
    println("  [status] not yet implemented")
}

fn forge_usage() -> () {
    println("  [usage] not yet implemented")
}

fn forge_scan() -> () {
    println("  [scan] not yet implemented")
}

fn forge_compress() -> () {
    println("  [compress] not yet implemented")
}

fn forge_on() -> () {
    println("  [on] not yet implemented")
}

fn forge_off() -> () {
    println("  [off] not yet implemented")
}

fn forge_menubar() -> () {
    println("  [menubar] not yet implemented")
}
```

- [ ] **Step 3: 실행 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa help
```

Expected: `forge v0.1.0` + `commands: status usage scan compress on off menubar`

```bash
~/Dev/hexa-lang/hexa run modules/forge.hexa status
```

Expected: `forge v0.1.0` + `[status] not yet implemented`

- [ ] **Step 4: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): skeleton — args router + 7 command stubs"
```

---

### Task 3: Keychain 해킹 — OAuth 토큰 추출

**Files:**
- Modify: `modules/forge.hexa`

- [ ] **Step 1: keychain 읽기 함수 구현**

`forge_status()` 스텁을 아래로 교체:

```hexa
fn read_keychain(label: str) -> str {
    let result = exec("security find-generic-password -l '" + label + "' -w 2>/dev/null")
    return result.trim()
}

fn extract_token(keychain_json: str) -> str {
    // JSON에서 accessToken 추출 — 정규식 대신 문자열 검색
    let marker = "\"accessToken\":\""
    let idx = keychain_json.find(marker)
    if idx < 0 {
        return ""
    }
    let start = idx + len(marker)
    let rest = keychain_json.slice(start, len(keychain_json))
    let end = rest.find("\"")
    if end < 0 {
        return ""
    }
    return rest.slice(0, end)
}

fn extract_json_field(json: str, field: str) -> str {
    let marker = "\"" + field + "\":"
    let idx = json.find(marker)
    if idx < 0 {
        return ""
    }
    let start = idx + len(marker)
    let rest = json.slice(start, len(json)).trim()
    // String value
    if rest.slice(0, 1) == "\"" {
        let inner = rest.slice(1, len(rest))
        let end = inner.find("\"")
        if end < 0 {
            return ""
        }
        return inner.slice(0, end)
    }
    // Numeric value
    let end = 0
    let i = 0
    while i < len(rest) {
        let ch = rest.slice(i, i + 1)
        if ch == "," || ch == "}" || ch == " " {
            end = i
            i = len(rest)  // break
        }
        i = i + 1
    }
    if end == 0 {
        end = len(rest)
    }
    return rest.slice(0, end)
}
```

- [ ] **Step 2: forge_status에서 키체인 스캔**

```hexa
fn forge_status() -> () {
    println("  ── keychain scan ──")
    let i = 0
    while i < 10 {
        let label = KEYCHAIN_LABELS[i]
        let name = ACCOUNT_NAMES[i]
        let raw = read_keychain(label)
        if len(raw) > 0 {
            let token = extract_token(raw)
            if len(token) > 0 {
                let preview = token.slice(0, 20) + "..."
                println("  " + name + ": token=" + preview)
            }
            if len(token) == 0 {
                println("  " + name + ": no accessToken found")
            }
        }
        if len(raw) == 0 {
            println("  " + name + ": not in keychain")
        }
        i = i + 1
    }
}
```

- [ ] **Step 3: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa status
```

Expected: 10개 계정에 대해 `claude1: token=eyJ...` 또는 `not in keychain` 출력

- [ ] **Step 4: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): keychain hack — extract OAuth tokens from 10 accounts"
```

---

### Task 4: Usage API 호출 — 실시간 사용량

**Files:**
- Modify: `modules/forge.hexa`

- [ ] **Step 1: usage API 호출 함수**

```hexa
fn fetch_usage(token: str) -> str {
    let result = exec("curl -s --max-time 10 " + USAGE_URL + " -H 'Authorization: Bearer " + token + "' -H 'anthropic-beta: oauth-2025-04-20'")
    return result.trim()
}

fn parse_usage(name: str, json: str) -> () {
    let five_hour = extract_json_field(json, "utilization")
    // five_hour 블록 내부의 utilization 파싱
    let fh_marker = "\"five_hour\""
    let fh_idx = json.find(fh_marker)
    if fh_idx < 0 {
        println("  " + name + ": no usage data")
        return
    }
    let fh_rest = json.slice(fh_idx, len(json))
    let fh_util = extract_json_field(fh_rest, "utilization")
    let fh_resets = extract_json_field(fh_rest, "resets_at")

    let sd_marker = "\"seven_day\""
    let sd_idx = json.find(sd_marker)
    let sd_util = "?"
    let sd_resets = ""
    if sd_idx >= 0 {
        let sd_rest = json.slice(sd_idx, len(json))
        sd_util = extract_json_field(sd_rest, "utilization")
        sd_resets = extract_json_field(sd_rest, "resets_at")
    }

    println("  " + name + ": session=" + fh_util + "%  week=" + sd_util + "%  resets=" + fh_resets)
}
```

- [ ] **Step 2: forge_usage 구현**

```hexa
fn forge_usage() -> () {
    println("  ── usage API (10 accounts) ──")
    let i = 0
    while i < 10 {
        let label = KEYCHAIN_LABELS[i]
        let name = ACCOUNT_NAMES[i]
        let raw = read_keychain(label)
        if len(raw) > 0 {
            let token = extract_token(raw)
            if len(token) > 0 {
                let usage_json = fetch_usage(token)
                if len(usage_json) > 0 {
                    parse_usage(name, usage_json)
                }
                if len(usage_json) == 0 {
                    println("  " + name + ": API timeout")
                }
            }
        }
        if len(raw) == 0 {
            println("  " + name + ": skip (no keychain)")
        }
        i = i + 1
    }
}
```

- [ ] **Step 3: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa usage
```

Expected: 각 계정의 `session=XX%  week=XX%  resets=...` 출력

- [ ] **Step 4: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): usage API — fetch 10-account utilization via keychain tokens"
```

---

### Task 5: Conversation Scanner — JSONL 파싱 + 6-axis genome

**Files:**
- Modify: `modules/forge.hexa`

- [ ] **Step 1: 세션 목록 스캔 함수**

```hexa
fn list_session_files(claude_dir: str) -> str {
    let expanded = exec("echo " + claude_dir).trim()
    let result = exec("find " + expanded + "/projects -name '*.jsonl' -type f 2>/dev/null")
    return result.trim()
}

fn count_jsonl_tokens(path: str) -> str {
    // 한 세션 JSONL에서 usage 집계: input_tokens, output_tokens, tool_calls, thinking_chars
    // awk로 한번에 처리 (hexa에서 대용량 파일 읽기 방지)
    let awk_script = "'{if(/"usage"/){gsub(/.*input_tokens.*:/,\"\"); gsub(/,.*/,\"\"); sum+=$0}} END{print sum}'"
    let cmd = "cat '" + path + "' | python3 -c \"" +
        "import sys,json;" +
        "it=ot=cc=tc=fc=0;" +
        "[" +
        "(it:=it+m.get('input_tokens',0),ot:=ot+m.get('output_tokens',0),cc:=cc+m.get('cache_read_input_tokens',0))" +
        " for l in sys.stdin" +
        " for d in [json.loads(l)] if d.get('type')=='assistant'" +
        " for m in [d.get('message',{}).get('usage',{})] if m" +
        "];" +
        "tc=sum(1 for l in open(sys.argv[1]) for d in [json.loads(l)] if d.get('type')=='assistant' for c in d.get('message',{}).get('content',[]) if isinstance(c,dict) and c.get('type')=='tool_use');" +
        "fc=sum(len(str(c.get('thinking',''))) for l in open(sys.argv[1]) for d in [json.loads(l)] if d.get('type')=='assistant' for c in d.get('message',{}).get('content',[]) if isinstance(c,dict) and c.get('type')=='thinking');" +
        "print(f'{it}\\t{ot}\\t{cc}\\t{tc}\\t{fc}')" +
        "\" '" + path + "' < '" + path + "'"
    let result = exec(cmd)
    return result.trim()
}
```

- [ ] **Step 2: 6-axis genome 계산 (세션 레벨)**

```hexa
fn session_genome(input_t: float, output_t: float, cache_t: float, tools: float, thinking: float, elapsed: float) -> str {
    // 6-axis projection for Claude Code sessions
    let cpu_axis = 0.0
    if elapsed > 0.0 {
        cpu_axis = (input_t + output_t) / elapsed  // tokens/sec throughput
    }
    let ram_axis = cache_t + input_t                // context size proxy
    let gpu_axis = tools                            // tool call density
    let npu_axis = thinking                         // thinking depth (char count)
    let power_axis = input_t * OPUS_INPUT / 1000000.0 + output_t * OPUS_OUTPUT / 1000000.0  // cost USD
    let io_axis = tools * 0.6                       // ~60% of tool calls are file I/O

    // Normalize to [0, 1] with soft caps
    let cpu_n = cpu_axis / 1000.0          // cap at 1000 tok/s
    let ram_n = ram_axis / 10000000.0      // cap at 10M tokens
    let gpu_n = gpu_axis / 500.0           // cap at 500 tool calls
    let npu_n = npu_axis / 100000.0        // cap at 100K thinking chars
    let power_n = power_axis / 50.0        // cap at $50
    let io_n = io_axis / 300.0             // cap at 300

    // Clamp
    if cpu_n > 1.0 { cpu_n = 1.0 }
    if ram_n > 1.0 { ram_n = 1.0 }
    if gpu_n > 1.0 { gpu_n = 1.0 }
    if npu_n > 1.0 { npu_n = 1.0 }
    if power_n > 1.0 { power_n = 1.0 }
    if io_n > 1.0 { io_n = 1.0 }

    return to_string(cpu_n) + "\t" + to_string(ram_n) + "\t" + to_string(gpu_n) + "\t" + to_string(npu_n) + "\t" + to_string(power_n) + "\t" + to_string(io_n)
}
```

- [ ] **Step 3: forge_scan 구현**

```hexa
fn forge_scan() -> () {
    println("  ── scanning conversations ──")
    let dir_i = 0
    let total_sessions = 0
    while dir_i < 2 {
        let claude_dir = CLAUDE_DIRS[dir_i]
        let files = list_session_files(claude_dir)
        if len(files) == 0 {
            dir_i = dir_i + 1
            // continue
        }
        if len(files) > 0 {
            let paths = files.split("\n")
            let p = 0
            while p < len(paths) {
                let path = paths[p]
                if len(path) > 0 {
                    let stats = count_jsonl_tokens(path)
                    if len(stats) > 0 {
                        let parts = stats.split("\t")
                        if len(parts) >= 5 {
                            let it = to_float(parts[0])
                            let ot = to_float(parts[1])
                            let cc = to_float(parts[2])
                            let tc = to_float(parts[3])
                            let fc = to_float(parts[4])
                            if it > 0.0 {
                                let genome = session_genome(it, ot, cc, tc, fc, 3600.0)
                                // Extract session name from path
                                let name_parts = path.split("/")
                                let fname = name_parts[len(name_parts) - 1]
                                println("  " + fname + ": " + genome)
                                total_sessions = total_sessions + 1
                            }
                        }
                    }
                }
                p = p + 1
            }
        }
        dir_i = dir_i + 1
    }
    println("  total sessions scanned: " + to_string(total_sessions))
}
```

- [ ] **Step 4: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa scan 2>&1 | head -20
```

Expected: 세션별 6-axis genome 값 출력 (cpu, ram, gpu, npu, power, io)

- [ ] **Step 5: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): conversation scanner — JSONL parse + 6-axis genome projection"
```

---

### Task 6: 대화 압축 (token-forge 흡수)

**Files:**
- Modify: `modules/forge.hexa`

- [ ] **Step 1: 압축 함수 구현**

```hexa
fn compress_session_file(jsonl_path: str) -> str {
    // Phase 1: 대화에서 user 메시지 + assistant 텍스트만 추출 (tool results, thinking 제거)
    let extract_cmd = "python3 -c \"" +
        "import sys,json;" +
        "lines=[];" +
        "[lines.append(d['message']['content'] if isinstance(d['message']['content'],str) else ' '.join(c.get('text','') for c in d['message']['content'] if isinstance(c,dict) and c.get('type')=='text'))" +
        " for l in open('" + jsonl_path + "')" +
        " for d in [json.loads(l)]" +
        " if d.get('type') in ('user','assistant') and d.get('message',{}).get('content')];" +
        "print('\\n---\\n'.join(lines[:50]))" +  // 최대 50개 메시지
        "\""
    let conversation = exec(extract_cmd)
    return conversation.trim()
}

fn forge_compress() -> () {
    println("  ── compressing old sessions ──")
    let dir_i = 0
    let compressed_count = 0
    let original_size = 0
    let compressed_size = 0

    while dir_i < 2 {
        let claude_dir = CLAUDE_DIRS[dir_i]
        let files = list_session_files(claude_dir)
        if len(files) > 0 {
            let paths = files.split("\n")
            let p = 0
            while p < len(paths) {
                let path = paths[p]
                if len(path) > 0 {
                    // 파일 크기 확인
                    let size_str = exec("wc -c < '" + path + "' 2>/dev/null").trim()
                    let size = to_float(size_str)
                    if size > 100000.0 {
                        // 100KB 이상만 압축 대상
                        let conversation = compress_session_file(path)
                        if len(conversation) > 0 {
                            // 세션 ID 추출
                            let name_parts = path.split("/")
                            let fname = name_parts[len(name_parts) - 1]
                            let session_id = fname.split(".")[0]
                            let out_path = COMPRESSED_DIR + "/" + session_id + ".tf"
                            write_file(out_path, conversation)
                            let new_size = to_float(exec("wc -c < '" + out_path + "' 2>/dev/null").trim())
                            let ratio = 0.0
                            if size > 0.0 {
                                ratio = (1.0 - new_size / size) * 100.0
                            }
                            println("  " + session_id + ": " + to_string(size) + "B -> " + to_string(new_size) + "B (" + to_string(ratio) + "% reduction)")
                            compressed_count = compressed_count + 1
                            original_size = original_size + to_int(size)
                            compressed_size = compressed_size + to_int(new_size)
                        }
                    }
                }
                p = p + 1
            }
        }
        dir_i = dir_i + 1
    }
    println("  compressed " + to_string(compressed_count) + " sessions")
    println("  total: " + to_string(original_size) + "B -> " + to_string(compressed_size) + "B")
}
```

- [ ] **Step 2: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa compress 2>&1 | tail -10
```

Expected: 100KB+ 세션들이 압축되어 `forge/compressed/` 에 `.tf` 파일 생성

- [ ] **Step 3: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): conversation compression — extract text, strip tools/thinking"
```

---

### Task 7: 모니터링 데몬 on/off

**Files:**
- Modify: `modules/forge.hexa`

- [ ] **Step 1: on/off 구현**

```hexa
fn forge_on() -> () {
    // Check if already running
    let state = exec("cat " + STATE_FILE + " 2>/dev/null").trim()
    if state.find("\"active\": true") >= 0 {
        // Extract PID and check if alive
        let pid = extract_json_field(state, "pid")
        let alive = exec("kill -0 " + pid + " 2>/dev/null; echo $?").trim()
        if alive == "0" {
            println("  already running (pid=" + pid + ")")
            return
        }
    }

    // Write state file with current shell PID
    let pid = exec("echo $$").trim()
    let ts = exec("date +%s").trim()
    write_file(STATE_FILE, "{\"active\": true, \"pid\": " + pid + ", \"started\": " + ts + "}")
    println("  monitor ON (pid=" + pid + ")")

    // Monitor loop
    let running = true
    while running {
        // 1. Scan active Claude processes
        let procs = exec("ps aux | grep '[c]laude' | wc -l").trim()
        println("  [" + exec("date +%H:%M:%S").trim() + "] active claude processes: " + procs)

        // 2. Quick usage check (first available account)
        let label = KEYCHAIN_LABELS[0]
        let raw = read_keychain(label)
        if len(raw) > 0 {
            let token = extract_token(raw)
            if len(token) > 0 {
                let usage = fetch_usage(token)
                if len(usage) > 0 {
                    let fh_rest = usage.slice(usage.find("\"five_hour\""), len(usage))
                    let util = extract_json_field(fh_rest, "utilization")
                    println("  primary account session: " + util + "%")
                }
            }
        }

        // 3. Check if still active
        let check = exec("cat " + STATE_FILE + " 2>/dev/null").trim()
        if check.find("\"active\": true") < 0 {
            running = false
            println("  monitor stopped by off command")
        }

        if running {
            exec("sleep 30")
        }
    }
}

fn forge_off() -> () {
    let state = exec("cat " + STATE_FILE + " 2>/dev/null").trim()
    let ts = exec("date +%s").trim()
    write_file(STATE_FILE, "{\"active\": false, \"stopped\": " + ts + "}")
    println("  monitor OFF")

    // Kill background process if exists
    let pid = extract_json_field(state, "pid")
    if len(pid) > 0 {
        exec("kill " + pid + " 2>/dev/null")
        println("  killed pid " + pid)
    }
}
```

- [ ] **Step 2: forge_status 업데이트 — 데몬 상태 표시**

```hexa
fn forge_status() -> () {
    // Daemon state
    let state = exec("cat " + STATE_FILE + " 2>/dev/null").trim()
    if state.find("\"active\": true") >= 0 {
        let pid = extract_json_field(state, "pid")
        let alive = exec("kill -0 " + pid + " 2>/dev/null; echo $?").trim()
        if alive == "0" {
            println("  daemon: RUNNING (pid=" + pid + ")")
        }
        if alive != "0" {
            println("  daemon: STALE (pid=" + pid + " dead)")
        }
    }
    if state.find("\"active\": true") < 0 {
        println("  daemon: OFF")
    }

    // Active Claude sessions
    let procs = exec("ps aux | grep '[c]laude' | wc -l").trim()
    println("  active claude processes: " + procs)

    // Keychain scan
    println("  ─��� keychain ──")
    let i = 0
    while i < 10 {
        let label = KEYCHAIN_LABELS[i]
        let name = ACCOUNT_NAMES[i]
        let raw = read_keychain(label)
        if len(raw) > 0 {
            let token = extract_token(raw)
            if len(token) > 0 {
                println("  " + name + ": OK")
            }
            if len(token) == 0 {
                println("  " + name + ": no token")
            }
        }
        if len(raw) == 0 {
            println("  " + name + ": missing")
        }
        i = i + 1
    }

    // Genome stats
    let genome_count = exec("ls " + GENOMES_DIR + "/*.genome 2>/dev/null | wc -l").trim()
    let compressed_count = exec("ls " + COMPRESSED_DIR + "/*.tf 2>/dev/null | wc -l").trim()
    println("  genomes: " + genome_count + "  compressed: " + compressed_count)
}
```

- [ ] **Step 3: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa status
```

Expected: daemon OFF, keychain 10개 상태, active processes 수

```bash
# on 테스트는 백그라운드로 잠깐만 (Ctrl+C로 종료)
cd ~/Dev/airgenome && timeout 10 ~/Dev/hexa-lang/hexa run modules/forge.hexa on || true
```

Expected: `monitor ON` + 30초 간격 상태 출력 시도 (10초 후 timeout)

- [ ] **Step 4: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): monitor daemon on/off — PID tracking + usage polling loop"
```

---

### Task 8: macOS 메뉴바 — JXA 해킹

**Files:**
- Create: `modules/templates/menubar.jxa`
- Modify: `modules/forge.hexa`

- [ ] **Step 1: JXA 메뉴바 템플릿 작성**

`modules/templates/menubar.jxa`:
```javascript
ObjC.import('Cocoa');

// === Status Bar ===
const app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

const statusBar = $.NSStatusBar.systemStatusBar;
const statusItem = statusBar.statusItemWithLength($.NSVariableStatusItemLength);
statusItem.button.title = $('TITLE_PLACEHOLDER');

const menu = $.NSMenu.alloc.init;

// Toggle
const toggleItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(
    $('TOGGLE_PLACEHOLDER'), ObjC.selector('terminate:'), $(''));
menu.addItem(toggleItem);

menu.addItem($.NSMenuItem.separatorItem);

// Account items
ACCOUNTS_PLACEHOLDER

menu.addItem($.NSMenuItem.separatorItem);

// Quit
const quitItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(
    $('Quit forge'), ObjC.selector('terminate:'), $('q'));
menu.addItem(quitItem);

statusItem.menu = menu;

// === Timer: reload state every 5s ===
const timer = $.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(5.0, true, () => {
    try {
        const pipe = $.NSPipe.pipe;
        const task = $.NSTask.alloc.init;
        task.executableURL = $.NSURL.fileURLWithPath($('/bin/cat'));
        task.arguments = $(['STATE_PATH_PLACEHOLDER']);
        task.standardOutput = pipe;
        task.launchAndReturnError(null);
        task.waitUntilExit();
        const data = pipe.fileHandleForReading.readDataToEndOfFile;
        const str = $.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding).js;
        if (str.includes('"active": true')) {
            statusItem.button.title = $('◉ forge');
        } else {
            statusItem.button.title = $('○ forge');
        }
    } catch(e) {
        statusItem.button.title = $('⚠ forge');
    }
});

app.run;
```

- [ ] **Step 2: forge_menubar 구현**

```hexa
fn forge_menubar() -> () {
    // Read template
    let template = read_file("modules/templates/menubar.jxa")

    // Read current state
    let state = exec("cat " + STATE_FILE + " 2>/dev/null").trim()
    let is_on = state.find("\"active\": true") >= 0

    let title = "○ forge"
    let toggle_label = "Start monitoring"
    if is_on {
        title = "◉ forge"
        toggle_label = "Stop monitoring"
    }

    // Build account lines
    let account_lines = ""
    let i = 0
    while i < 10 {
        let name = ACCOUNT_NAMES[i]
        let label = KEYCHAIN_LABELS[i]
        let raw = read_keychain(label)
        let status_text = name + ": --"
        if len(raw) > 0 {
            let token = extract_token(raw)
            if len(token) > 0 {
                let usage = fetch_usage(token)
                if len(usage) > 0 {
                    let fh_rest = usage.slice(usage.find("\"five_hour\""), len(usage))
                    let util = extract_json_field(fh_rest, "utilization")
                    let sd_idx = usage.find("\"seven_day\"")
                    let sd_util = "?"
                    if sd_idx >= 0 {
                        let sd_rest = usage.slice(sd_idx, len(usage))
                        sd_util = extract_json_field(sd_rest, "utilization")
                    }
                    status_text = name + ": S=" + util + "% W=" + sd_util + "%"
                }
            }
        }
        account_lines = account_lines + "menu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('" + status_text + "'), null, $('')));\n"
        i = i + 1
    }

    // Fill template
    let jxa = template
    jxa = jxa.replace("TITLE_PLACEHOLDER", title)
    jxa = jxa.replace("TOGGLE_PLACEHOLDER", toggle_label)
    jxa = jxa.replace("ACCOUNTS_PLACEHOLDER", account_lines)
    let abs_state = exec("pwd").trim() + "/" + STATE_FILE
    jxa = jxa.replace("STATE_PATH_PLACEHOLDER", abs_state)

    // Write temp file and execute
    let tmp = "/tmp/airgenome-menubar.jxa"
    write_file(tmp, jxa)
    println("  launching menubar...")
    println("  (runs in foreground — Ctrl+C or Quit menu to exit)")
    exec("osascript -l JavaScript " + tmp)
}
```

- [ ] **Step 3: 테스트**

```bash
cd ~/Dev/airgenome && ~/Dev/hexa-lang/hexa run modules/forge.hexa menubar
```

Expected: macOS 상단 메뉴바에 `○ forge` 또는 `◉ forge` 아이콘 표시. 클릭하면 10개 계정 사용량 + Quit 메뉴.

- [ ] **Step 4: Commit**

```bash
git add modules/templates/menubar.jxa modules/forge.hexa
git commit -m "feat(forge): macOS menu bar via JXA — live account usage + state polling"
```

---

### Task 9: 통합 테스트 + 최종 정리

**Files:**
- Modify: `modules/forge.hexa` (assert 추가)

- [ ] **Step 1: consciousness 블록에 self-test 추가**

ForgeRouter consciousness 블록 시작부에 추가:

```hexa
    // ── self-test ──
    assert len(KEYCHAIN_LABELS) == 10
    assert len(ACCOUNT_NAMES) == 10
    assert AXIS_COUNT == 6
    assert PAIR_COUNT == 15
    assert SINGULARITY > 0.66
    assert SINGULARITY < 0.67
    assert len(VERSION) > 0
    println("  self-test: 7 assertions passed")
```

- [ ] **Step 2: 전체 커맨드 스모크 테스트**

```bash
cd ~/Dev/airgenome
~/Dev/hexa-lang/hexa run modules/forge.hexa help
~/Dev/hexa-lang/hexa run modules/forge.hexa status
~/Dev/hexa-lang/hexa run modules/forge.hexa usage 2>&1 | head -5
~/Dev/hexa-lang/hexa run modules/forge.hexa scan 2>&1 | head -5
```

Expected: 모든 커맨드가 에러 없이 출력 생성

- [ ] **Step 3: Commit**

```bash
git add modules/forge.hexa
git commit -m "feat(forge): self-test assertions + smoke test pass"
```

---

## Execution Summary

| Task | 내용 | 예상 크기 |
|------|------|----------|
| 1 | 프로젝트 구조 | ~5 lines |
| 2 | forge.hexa 뼈대 + args 라우팅 | ~80 lines |
| 3 | Keychain 해킹 | ~60 lines |
| 4 | Usage API 호출 | ~50 lines |
| 5 | Conversation scanner + genome | ~80 lines |
| 6 | 대화 압축 | ~60 lines |
| 7 | Monitor daemon on/off | ~70 lines |
| 8 | macOS 메뉴바 JXA | ~60 lines hexa + ~40 lines JXA |
| 9 | Self-test + 정리 | ~10 lines |

**Total: ~470 lines hexa + ~40 lines JXA**

모듈 추가 패턴: `modules/<name>.hexa` → `hexa run modules/<name>.hexa <command>`
