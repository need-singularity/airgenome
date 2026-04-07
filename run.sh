#!/bin/bash
# airgenome — build & launch menubar (one command, no sudo)
set -e

SCRIPT="$(readlink -f "$0" 2>/dev/null || realpath "$0" 2>/dev/null || echo "$0")"
DIR="$(cd "$(dirname "$SCRIPT")" && pwd)"

# --settings: open settings panel only
if [ "${1:-}" = "--settings" ] || [ "${1:-}" = "-s" ]; then
  exec osascript -l JavaScript "$DIR/settings.js" "$HOME/.airgenome/config.json"
fi
CARGO="${CARGO:-$(command -v cargo || echo "$HOME/.cargo/bin/cargo")}"
CONF_DIR="$HOME/.airgenome"
mkdir -p "$CONF_DIR"
STATE="${TMPDIR:-/tmp}/airgenome-state.json"
CONFIG="$CONF_DIR/config.json"

# 0. Single instance lock
LOCKFILE="${TMPDIR:-/tmp}/airgenome.lock"
if [ -f "$LOCKFILE" ]; then
  OLD_PID=$(cat "$LOCKFILE" 2>/dev/null)
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "⬡ killing old instance (pid $OLD_PID)..."
    pkill -9 -P "$OLD_PID" 2>/dev/null || true
    kill -9 "$OLD_PID" 2>/dev/null || true
    sleep 1
  fi
fi
echo $$ > "$LOCKFILE"

# 1. Build
echo "⟐ building airgenome..."
$CARGO build --manifest-path "$DIR/Cargo.toml" --quiet 2>&1

# 2. Kill old menubar
pkill -f 'osascript.*airgenome-menubar' 2>/dev/null || true

# 3. System info
TOTAL_RAM_MB=$(sysctl -n hw.memsize 2>/dev/null | awk '{print int($1/1048576)}')
TOTAL_RAM_GB=$((TOTAL_RAM_MB / 1024))
NCPU=$(sysctl -n hw.ncpu 2>/dev/null || echo 8)
CHIP=$(sysctl -n machdep.cpu.brand_string 2>/dev/null | grep -oE 'M[0-9]+' | head -1)
MODEL_NAME=$(system_profiler SPHardwareDataType 2>/dev/null | grep "Model Name" | awk -F: '{print $2}' | xargs)
HAS_FAN="false"
echo "$MODEL_NAME" | grep -qi "Pro" && HAS_FAN="true"

# 4. Auto-detect profile → default config
if [ ! -f "$CONFIG" ]; then
  PROFILE_JSON="$DIR/profiles.json"
  if [ -f "$PROFILE_JSON" ]; then
    eval "$(awk -v chip="$CHIP" -v ram="$TOTAL_RAM_GB" -v fan="$HAS_FAN" '
    BEGIN{RS="{";best_cpu=75;best_ram=70;best_swap=30;best_note="default";found=0}
    /"chip"/{
      mc="";mr=0;mf="";cc=0;rc=0;sc=0;note=""
      for(i=1;i<=NF;i++){
        if($i~/"chip"/)  {split($i,a,"\"");mc=a[4]}
        if($i~/"ram_gb"/){gsub(/[^0-9]/,"",$i);mr=$i+0}
        if($i~/"fan"/)   {mf=($i~/true/)?"true":"false"}
        if($i~/"cpu_ceil"/){gsub(/[^0-9]/,"",$i);cc=$i+0}
        if($i~/"ram_ceil"/){gsub(/[^0-9]/,"",$i);rc=$i+0}
        if($i~/"swap_ceil"/){gsub(/[^0-9]/,"",$i);sc=$i+0}
        if($i~/"note"/)  {split($i,a,"\"");note=a[4]}
      }
      if(mc!=""&&cc>0&&!found){
        chip_ok=(index(chip,mc)>0)
        ram_ok=(mr==ram+0)
        fan_ok=(mf==""||mf==fan)
        if(chip_ok&&ram_ok&&fan_ok){
          best_cpu=cc;best_ram=rc;best_swap=sc;best_note=note;found=1
        }
      }
    }
    END{printf "CPU_C=%d\nRAM_C=%d\nSWAP_C=%d\nPROFILE_NOTE='\''%s'\''\n",best_cpu,best_ram,best_swap,best_note}
    ' "$PROFILE_JSON" 2>/dev/null)" || { CPU_C=75; RAM_C=70; SWAP_C=30; PROFILE_NOTE="default"; }
  else
    CPU_C=75; RAM_C=70; SWAP_C=30; PROFILE_NOTE="default"
  fi
  echo "⬡ profile: $CHIP ${TOTAL_RAM_GB}GB → CPU ${CPU_C}% RAM ${RAM_C}% Swap ${SWAP_C}%"
  echo "  $PROFILE_NOTE"
  cat > "$CONFIG" <<CJSON
{"cpu_ceil": $CPU_C, "ram_ceil": $RAM_C, "swap_ceil": $SWAP_C, "bridge_max": 4}
CJSON
fi

# read back config for initial state (pure awk)
eval "$(awk -F'[,:}]' '{for(i=1;i<=NF;i++){
  gsub(/["{[:space:]]/,"",$i)
  if($i=="cpu_ceil")printf "CPU_C=%d\n",$(i+1)
  if($i=="ram_ceil")printf "RAM_C=%d\n",$(i+1)
  if($i=="swap_ceil")printf "SWAP_C=%d\n",$(i+1)
}}' "$CONFIG" 2>/dev/null)" || { CPU_C=75; RAM_C=70; SWAP_C=30; }

# 5. Write initial state
cat > "$STATE" <<SJSON
{"active":true,"cpu":0,"ram":0,"swap":0,"level":"ok","throttled":false,"cpu_ceil":$CPU_C,"ram_ceil":$RAM_C,"swap_ceil":$SWAP_C}
SJSON

# 6. Background sampler + adaptive guard
THROTTLE_FILE="${TMPDIR:-/tmp}/airgenome-throttled.pids"
(
  PREV_LEVEL="ok"
  while true; do
    # ── MEASURE (single top call) ──────────────────────────────
    TOP_OUT=$(top -l1 -n0 2>/dev/null)

    CPU=$(echo "$TOP_OUT" | awk '/CPU usage/{gsub(/%/,""); printf "%d",$3+$5}')
    : "${CPU:=0}"; CPU=$((CPU + 0))
    [ "$CPU" -eq 0 ] && { CPU_TOTAL=$(ps -A -o %cpu= | awk '{s+=$1}END{printf "%.0f",s}'); CPU=$((CPU_TOTAL / NCPU)); }

    # RAM: PhysMem "XG used" or "XXXXM used" — handle both units
    RAM_USED_MB=$(echo "$TOP_OUT" | awk '/PhysMem/{v=$2;u=substr(v,length(v));n=substr(v,1,length(v)-1)+0;if(u=="G")n*=1024;else if(u!="M")n=0;printf "%d",n}')
    : "${RAM_USED_MB:=0}"; RAM_USED_MB=$((RAM_USED_MB + 0))
    FREE_MB=$((TOTAL_RAM_MB - RAM_USED_MB))
    [ "$FREE_MB" -lt 0 ] && FREE_MB=0
    RAM=$((RAM_USED_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))
    [ "$RAM" -gt 100 ] && RAM=100

    # Swap + Load — single sysctl call
    SWAP_MB=$(sysctl -n vm.swapusage 2>/dev/null | awk '{gsub(/M/,"",$3); printf "%.0f",$3}')
    : "${SWAP_MB:=0}"; SWAP_MB=$((SWAP_MB + 0))
    SWAP=$((SWAP_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))
    LOAD=$(sysctl -n vm.loadavg 2>/dev/null | awk '{gsub(/[{}]/,""); printf "%.0f",$1}')
    : "${LOAD:=0}"; LOAD=$((LOAD + 0))

    # ── CONFIG (pure shell, no python) ───────────────────────
    if [ -f "$CONFIG" ]; then
      eval "$(awk -F'[,:}]' '{for(i=1;i<=NF;i++){
        gsub(/["{[:space:]]/,"",$i)
        if($i=="cpu_ceil")printf "CPU_CEIL=%d\n",$(i+1)
        if($i=="ram_ceil")printf "RAM_CEIL=%d\n",$(i+1)
        if($i=="swap_ceil")printf "SWAP_CEIL=%d\n",$(i+1)
        if($i=="bridge_max")printf "BRIDGE_MAX=%d\n",$(i+1)
      }}' "$CONFIG" 2>/dev/null)"
    fi
    : "${CPU_CEIL:=90}" "${RAM_CEIL:=80}" "${SWAP_CEIL:=50}" "${BRIDGE_MAX:=4}"
    GUARD_ON=1  # 무조건 자동 사용

    # ── BRIDGE LIMITER ───────────────────────────────────────
    if [ "$GUARD_ON" = "1" ] && [ "${BRIDGE_MAX:-0}" -gt 0 ]; then
      BRIDGE_PIDS=$(ps -eo pid=,lstart=,command= | grep 'gap_finder.hexa bridge' | grep -v grep | sort -k2,5 | awk '{print $1}')
      BRIDGE_COUNT=$(echo "$BRIDGE_PIDS" | grep -c . 2>/dev/null || true)
      : "${BRIDGE_COUNT:=0}"; BRIDGE_COUNT=$((BRIDGE_COUNT + 0))
      if [ "$BRIDGE_COUNT" -gt "$BRIDGE_MAX" ]; then
        KILL_N=$((BRIDGE_COUNT - BRIDGE_MAX))
        echo "$BRIDGE_PIDS" | head -"$KILL_N" | while read BPID; do
          kill "$BPID" 2>/dev/null || true
        done
      fi
    fi

    # ══════════════════════════════════════════════════════════
    #  SINGULARITY GUARD — 죽지않는 선만 방어, 나머지는 자유
    #
    #  철학: ceiling은 "목표"가 아니라 "경고선"
    #        실제 개입은 시스템이 진짜 죽을 때만
    #
    #  4단계:
    #    ok       — 전부 자유, throttle 해제
    #    warn     — 표시만, 개입 없음
    #    danger   — purge만 (프로세스 안 건드림)
    #    critical — 생존 모드 (purge + bridge 축소 + taskpolicy)
    # ══════════════════════════════════════════════════════════
    LEVEL="ok"
    THROTTLED="false"

    # 실제 위험 신호 (하드웨어 한계 기반, ceiling 무관)
    SWAP_THRASH=0  # swap > 10GB → disk I/O death spiral
    [ "$SWAP_MB" -gt 10240 ] 2>/dev/null && SWAP_THRASH=1
    LOAD_SATURATED=0  # load > NCPU × 5 → 완전 포화
    [ "$LOAD" -gt $((NCPU * 5)) ] 2>/dev/null && LOAD_SATURATED=1

    # ── CRITICAL: 시스템 생존 위협 ──
    if [ "$FREE_MB" -lt 200 ] 2>/dev/null || [ "$SWAP_THRASH" = "1" ]; then
      LEVEL="critical"
      THROTTLED="true"
      if [ "$GUARD_ON" = "1" ]; then
        # 1) purge — 비파괴 캐시 해제
        purge 2>/dev/null || true
        # 2) bridge 전부 정리 (가장 큰 메모리 절약)
        if [ -n "$BRIDGE_PIDS" ] && [ "$BRIDGE_COUNT" -gt 0 ]; then
          echo "$BRIDGE_PIDS" | while read BPID; do
            kill "$BPID" 2>/dev/null || true
          done
        fi
        # 3) top 5 RAM consumer에 background QoS (시스템 프로세스 제외)
        ps -eo pid=,rss=,comm= | sort -rnk2 | head -5 | while read TPID TRSS TNAME; do
          [ "${TPID:-0}" -lt 200 ] 2>/dev/null && continue
          case "$TNAME" in kernel_task|WindowServer|launchd|loginwindow|opendirectoryd) continue ;; esac
          taskpolicy -b -p "$TPID" 2>/dev/null || true
          echo "$TPID" >> "$THROTTLE_FILE"
        done
      fi

    # ── DANGER: ceiling 초과, 아직 살 수 있음 ──
    elif [ "$FREE_MB" -lt 512 ] 2>/dev/null || [ "$LOAD_SATURATED" = "1" ]; then
      LEVEL="danger"
      THROTTLED="true"
      if [ "$GUARD_ON" = "1" ]; then
        # purge만 — 프로세스에 taskpolicy 안 건드림
        purge 2>/dev/null || true
        # bridge만 1개로 축소 (bridge가 있을 때만)
        if [ -n "$BRIDGE_PIDS" ] && [ "$BRIDGE_COUNT" -gt 1 ]; then
          echo "$BRIDGE_PIDS" | head -$((BRIDGE_COUNT - 1)) | while read BPID; do
            kill "$BPID" 2>/dev/null || true
          done
        fi
      fi

    # ── WARN: ceiling 근처, 표시만 ──
    elif [ "$RAM" -gt "$RAM_CEIL" ] 2>/dev/null || [ "$CPU" -gt "$CPU_CEIL" ] 2>/dev/null || [ "$SWAP" -gt "$SWAP_CEIL" ] 2>/dev/null; then
      LEVEL="warn"
      # 표시만, 개입 없음 — 시스템이 알아서 관리

    # ── OK: 전부 자유 ──
    else
      # throttle 걸려있던 프로세스 복구
      if [ -f "$THROTTLE_FILE" ]; then
        sort -u "$THROTTLE_FILE" | while read RPID; do
          taskpolicy -d default -p "$RPID" 2>/dev/null || true
        done
        rm -f "$THROTTLE_FILE"
      fi
    fi

    PREV_LEVEL="$LEVEL"

    # ── SAVINGS: QoS 절감률 계산 ────────────────────────────────
    # renice 대상: CPU >30% (시스템 제외) → 예상 30% 절감
    # taskpolicy 대상: RSS >2048MB (시스템 제외) → 예상 20% 절감
    SAVE_CPU=0; SAVE_RAM=0
    SYS_CPU_TOTAL=$(ps -A -o %cpu= | awk '{s+=$1}END{printf "%.0f",s}')
    SYS_RAM_TOTAL_MB=$(ps -A -o rss= | awk '{s+=$1}END{printf "%.0f",s/1024}')
    : "${SYS_CPU_TOTAL:=1}" "${SYS_RAM_TOTAL_MB:=1}"
    [ "$SYS_CPU_TOTAL" -lt 1 ] 2>/dev/null && SYS_CPU_TOTAL=1
    [ "$SYS_RAM_TOTAL_MB" -lt 1 ] 2>/dev/null && SYS_RAM_TOTAL_MB=1

    # CPU hogs: sum(cpu * 0.30) for processes >30% CPU, PID>100, not system
    HOG_CPU_SAVE=$(ps -eo pid=,%cpu=,comm= | awk '$1>100 && $2>30.0 {
      c=$3; if(c~/kernel_task|WindowServer|launchd|loginwindow|Finder|Dock/) next;
      s+=$2*0.30} END{printf "%.0f",s}')
    : "${HOG_CPU_SAVE:=0}"
    # RAM hogs: sum(rss/1024 * 0.20) for processes >1GB RSS
    HOG_RAM_SAVE=$(ps -eo pid=,rss=,comm= | awk '$1>100 && $2>1048576 {
      c=$3; if(c~/kernel_task|WindowServer|launchd|loginwindow|Finder|Dock/) next;
      s+=($2/1024)*0.20} END{printf "%.0f",s}')
    : "${HOG_RAM_SAVE:=0}"
    # Claude idle sessions: CPU<1% → taskpolicy saves 20% RAM
    CLAUDE_SAVE=$(ps -eo pid=,rss=,%cpu=,comm= | grep '[c]laude' | awk '$3<1.0 {s+=($2/1024)*0.20} END{printf "%.0f",s}')
    : "${CLAUDE_SAVE:=0}"
    # WebKit inactive tabs: CPU<0.5%, >200MB → taskpolicy saves 20%
    WEBKIT_SAVE=$(ps -eo pid=,rss=,%cpu=,comm= | grep 'WebKit.WebContent' | awk '$3<0.5 && $2>204800 {s+=($2/1024)*0.20} END{printf "%.0f",s}')
    : "${WEBKIT_SAVE:=0}"

    HOG_RAM_SAVE=$((HOG_RAM_SAVE + CLAUDE_SAVE + WEBKIT_SAVE))
    SAVE_CPU=$((HOG_CPU_SAVE * 100 / SYS_CPU_TOTAL))
    SAVE_RAM=$((HOG_RAM_SAVE * 100 / SYS_RAM_TOTAL_MB))
    [ "$SAVE_CPU" -gt 99 ] 2>/dev/null && SAVE_CPU=99
    [ "$SAVE_RAM" -gt 99 ] 2>/dev/null && SAVE_RAM=99

    # ── GATE STATUS ────────────────────────────────────────────
    GATE_CFG="$DIR/nexus/shared/gate_config.jsonl"
    UBU_HOST=$(awk -F'"' '/"remote_host"/{print $8}' "$GATE_CFG" 2>/dev/null)
    UBU_PORT=$(awk -F'"' '/"remote_port"/{print $8}' "$GATE_CFG" 2>/dev/null)
    : "${UBU_HOST:=192.168.50.119}" "${UBU_PORT:=9900}"
    if nc -z -w 1 "$UBU_HOST" "$UBU_PORT" 2>/dev/null; then
      GATE="online"
      UBU_STATUS=$(echo "STATUS" | nc -w 2 "$UBU_HOST" "$UBU_PORT" 2>/dev/null)
      UBU_LOAD=$(echo "$UBU_STATUS" | grep -o 'load=[^ ]*' | cut -d= -f2 | awk '{print $1}')
      UBU_RAM_USED=$(echo "$UBU_STATUS" | grep -o 'ram=[^ ]*' | cut -d= -f2 | cut -d/ -f1)
      UBU_RAM_TOTAL=$(echo "$UBU_STATUS" | grep -o 'ram=[^ ]*' | cut -d/ -f2 | sed 's/MB.*//')
      UBU_RAM_AVAIL=$(echo "$UBU_STATUS" | grep -o 'avail=[^ ]*' | cut -d= -f2 | sed 's/MB.*//')
      : "${UBU_LOAD:=0}" "${UBU_RAM_USED:=0}" "${UBU_RAM_TOTAL:=1}" "${UBU_RAM_AVAIL:=0}"
      UBU_RAM_PCT=$((UBU_RAM_USED * 100 / (UBU_RAM_TOTAL > 0 ? UBU_RAM_TOTAL : 1)))
      UBU_SSH=$(awk -F'"' '/"ssh_alias"/{print $8}' "$GATE_CFG" 2>/dev/null)
      : "${UBU_SSH:=ubu}"
      UBU_JOBS=$(ssh -o ConnectTimeout=1 "$UBU_SSH" "find /tmp/airgenome/gate_files -mmin -60 -type f 2>/dev/null | wc -l" 2>/dev/null | tr -d ' ')
      : "${UBU_JOBS:=0}"
    else
      GATE="offline"
      UBU_LOAD="0"; UBU_RAM_PCT=0; UBU_RAM_USED=0; UBU_RAM_TOTAL=0; UBU_RAM_AVAIL=0; UBU_JOBS=0
    fi

    cat > "$STATE" <<EOF
{"active":true,"cpu":$CPU,"ram":$RAM,"swap":$SWAP,"free_mb":$FREE_MB,"load":$LOAD,"level":"$LEVEL","throttled":$THROTTLED,"cpu_ceil":$CPU_CEIL,"ram_ceil":$RAM_CEIL,"swap_ceil":$SWAP_CEIL,"save_cpu":$SAVE_CPU,"save_ram":$SAVE_RAM,"gate":"$GATE","ubu_load":"$UBU_LOAD","ubu_ram_pct":$UBU_RAM_PCT,"ubu_ram_used":$UBU_RAM_USED,"ubu_ram_total":$UBU_RAM_TOTAL,"ubu_ram_avail":$UBU_RAM_AVAIL,"ubu_jobs":$UBU_JOBS}
EOF
    sleep 5
  done
) &
SAMPLER_PID=$!

# 7. JXA menubar with sliders
MENUBAR_JS=$(cat <<'JXAEOF'
ObjC.import('Cocoa');
ObjC.import('Foundation');

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

// --- Action handler ---
var settingsJsPath = 'SETTINGS_JS_REPLACE';

ObjC.registerSubclass({
    name: 'MenuHandler',
    methods: {
        'openSettings:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                var task = $.NSTask.alloc.init;
                task.executableURL = $.NSURL.fileURLWithPath($('/usr/bin/osascript'));
                task.arguments = $(['-l', 'JavaScript', settingsJsPath, 'CONFIG_REPLACE']);
                task.launchAndReturnError(null);
            }
        }
    }
});
var handler = $.MenuHandler.alloc.init;

var statusBar = $.NSStatusBar.systemStatusBar;
var statusItem = statusBar.statusItemWithLength($.NSVariableStatusItemLength);
statusItem.button.title = $('\u2B22 airgenome');

var menu = $.NSMenu.alloc.init;

var cpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('CPU  ...'), null, $(''));
cpuItem.enabled = false;
menu.addItem(cpuItem);

var ramItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('RAM  ...'), null, $(''));
ramItem.enabled = false;
menu.addItem(ramItem);

var swapItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Swap ...'), null, $(''));
swapItem.enabled = false;
menu.addItem(swapItem);

var saveItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Save ...'), null, $(''));
saveItem.enabled = false;
menu.addItem(saveItem);

var gateItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Gate ...'), null, $(''));
gateItem.enabled = false;
menu.addItem(gateItem);

menu.addItem($.NSMenuItem.separatorItem);

var safetyItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2705 Safe'), null, $(''));
safetyItem.enabled = false;
menu.addItem(safetyItem);

menu.addItem($.NSMenuItem.separatorItem);

var settingsItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2699 Settings...'), 'openSettings:', $(','));
settingsItem.target = handler;
menu.addItem(settingsItem);

menu.addItem($.NSMenuItem.separatorItem);

var quitItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Quit airgenome'), 'terminate:', $('q'));
menu.addItem(quitItem);

statusItem.menu = menu;

var statePath = 'STATE_REPLACE';
var configPath = 'CONFIG_REPLACE';
// --- Functions ---
function readState() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(statePath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        return JSON.parse(str.js);
    } catch(e) { return null; }
}

function readConfig() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(configPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        return JSON.parse(str.js);
    } catch(e) { return null; }
}

function bar(val, ceil, w) {
    var pct = Math.min(val / (ceil > 0 ? ceil : 1), 1.0);
    var filled = Math.round(pct * w);
    var s = '';
    for (var i = 0; i < filled; i++) s += '\u2588';
    for (var i = filled; i < w; i++) s += '\u2591';
    return s;
}

function levelIcon(lv) {
    if (lv === 'danger') return '\u26A0';
    if (lv === 'warn') return '\u2B21';
    return '\u2B22';
}


$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(2.0, true, function() {
    var j = readState();
    if (!j) { statusItem.button.title = $('\u26A0 airgenome'); return; }

    var cfg = readConfig() || {cpu_ceil: 90, ram_ceil: 80, swap_ceil: 50};
    var cc = cfg.cpu_ceil;
    var rc = cfg.ram_ceil;
    var sc = cfg.swap_ceil;

    var lv = j.level || 'ok';
    var swapHigh = j.swap >= sc;
    var swapMid  = j.swap >= (sc * 80 / 100);
    var swapTag = swapHigh ? ' \u26D4sw' : (swapMid ? ' \u26A0sw' : '');
    var saveCpuBar = j.save_cpu || 0;
    var saveRamBar = j.save_ram || 0;
    var saveTotalBar = Math.min(Math.round((saveCpuBar + saveRamBar) / 2), 99);
    var saveTag = saveTotalBar > 0 ? ' \u2193' + saveTotalBar + '%' : '';
    var gateIcon = j.gate === 'online' ? ' \u25CF' : ' \u25CB';
    statusItem.button.title = $(levelIcon(lv) + ' ' + j.cpu + '% \u00B7 ' + j.ram + '%' + swapTag + saveTag + gateIcon);

    var bw = 16;
    cpuItem.title  = $('CPU  ' + bar(j.cpu, cc, bw)  + '  ' + j.cpu  + '/' + cc + '%');
    ramItem.title  = $('RAM  ' + bar(j.ram, rc, bw)   + '  ' + j.ram  + '/' + rc + '%');
    var swapIcon = swapHigh ? '\u26D4 ' : (swapMid ? '\u26A0 ' : '');
    swapItem.title = $(swapIcon + 'Swap ' + bar(j.swap, sc, bw)  + '  ' + j.swap + '/' + sc + '%');

    var saveCpu = j.save_cpu || 0;
    var saveRam = j.save_ram || 0;
    var saveTotal = Math.min(Math.round((saveCpu + saveRam) / 2), 99);
    var saveIcon = saveTotal > 0 ? '\u2193' : '\u2500';
    saveItem.title = $(saveIcon + ' Save  CPU -' + saveCpu + '%  RAM -' + saveRam + '%  (\u2248' + saveTotal + '% \uC808\uAC10)');

    if (j.gate === 'online') {
        var uLoad = j.ubu_load || '0';
        var uRamPct = j.ubu_ram_pct || 0;
        var uRamAvail = j.ubu_ram_avail || 0;
        var uRamAvailG = Math.round(uRamAvail / 1024 * 10) / 10;
        var uJobs = j.ubu_jobs || 0;
        var jobsTag = uJobs > 0 ? '  \u2191' + uJobs + 'jobs' : '';
        gateItem.title = $('\u25CF Gate  load=' + uLoad + '  RAM ' + uRamPct + '%  (' + uRamAvailG + 'G free)' + jobsTag);
    } else {
        gateItem.title = $('\u25CB Gate \u2014 offline (local mode)');
    }

    var swapNote = swapHigh ? ' [swap]' : (swapMid ? ' [swap]' : '');
    var freeMB = j.free_mb || 0;
    var loadAvg = j.load || 0;
    var ramNote = freeMB < 512 ? ' [' + freeMB + 'MB free]' : '';
    if (lv === 'critical') {
        safetyItem.title = $('\u25CB CRITICAL \u2014 RAM ' + freeMB + 'MB free' + swapNote);
    } else if (lv === 'danger') {
        safetyItem.title = $('\u26A0 THROTTLE active' + ramNote + swapNote);
    } else if (lv === 'warn') {
        safetyItem.title = $('\u26A1 Approaching ceiling' + ramNote + swapNote);
    } else {
        safetyItem.title = $('\u2705 Safe \u2014 ' + Math.round(freeMB/1024*10)/10 + 'G free');
    }
});

app.run;
JXAEOF
)

MENUBAR_JS="${MENUBAR_JS//STATE_REPLACE/$STATE}"
MENUBAR_JS="${MENUBAR_JS//CONFIG_REPLACE/$CONFIG}"
MENUBAR_JS="${MENUBAR_JS//SETTINGS_JS_REPLACE/$DIR/settings.js}"

echo "⬡ airgenome menubar launching..."
osascript -l JavaScript -e "$MENUBAR_JS" &
MENUBAR_PID=$!

# 8. Cleanup on exit
trap "kill $SAMPLER_PID $MENUBAR_PID 2>/dev/null; rm -f '$STATE' '$LOCKFILE'" EXIT INT TERM

echo "⬡ running (sampler=$SAMPLER_PID menubar=$MENUBAR_PID)"
echo "  press Ctrl+C to stop"
wait
