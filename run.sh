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
    eval "$(python3 << PYEOF
import json
with open('$PROFILE_JSON') as f: data=json.load(f)
chip='$CHIP'
ram=$TOTAL_RAM_GB
fan=$( [ "$HAS_FAN" = "true" ] && echo "True" || echo "False" )
best=data['profiles']['default']
for k,v in data['profiles'].items():
    if k=='default': continue
    m=v.get('match',{})
    mc=m.get('chip','')
    mr=m.get('ram_gb',0)
    mf=m.get('fan',None)
    if mc and mc in chip and mr==ram and (mf is None or mf==fan):
        best=v
        break
print(f"CPU_C={best['cpu_ceil']}")
print(f"RAM_C={best['ram_ceil']}")
print(f"SWAP_C={best['swap_ceil']}")
print(f"PROFILE_NOTE='{best['note']}'")
PYEOF
)" || { CPU_C=75; RAM_C=70; SWAP_C=30; PROFILE_NOTE="default"; }
  else
    CPU_C=75; RAM_C=70; SWAP_C=30; PROFILE_NOTE="default"
  fi
  echo "⬡ profile: $CHIP ${TOTAL_RAM_GB}GB → CPU ${CPU_C}% RAM ${RAM_C}% Swap ${SWAP_C}%"
  echo "  $PROFILE_NOTE"
  cat > "$CONFIG" <<CJSON
{"cpu_ceil": $CPU_C, "ram_ceil": $RAM_C, "swap_ceil": $SWAP_C, "bridge_max": 4}
CJSON
fi

# read back config for initial state
CPU_C=$(python3 -c "import json;print(json.load(open('$CONFIG'))['cpu_ceil'])" 2>/dev/null || echo 75)
RAM_C=$(python3 -c "import json;print(json.load(open('$CONFIG'))['ram_ceil'])" 2>/dev/null || echo 70)
SWAP_C=$(python3 -c "import json;print(json.load(open('$CONFIG'))['swap_ceil'])" 2>/dev/null || echo 30)

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
    [ "${CPU:-0}" -eq 0 ] && { CPU_TOTAL=$(ps -A -o %cpu= | awk '{s+=$1}END{printf "%.0f",s}'); CPU=$((CPU_TOTAL / NCPU)); }

    # RAM: PhysMem used (what user actually sees)
    RAM_USED_MB=$(echo "$TOP_OUT" | /opt/homebrew/bin/python3.12 -c "
import sys,re
l=sys.stdin.read()
m=re.search(r'(\d+)([GM])\s+used',l)
if m:
  v=int(m.group(1))
  if m.group(2)=='G': v*=1024
  print(v)
else: print(0)" 2>/dev/null || echo 0)
    FREE_MB=$((TOTAL_RAM_MB - RAM_USED_MB))
    [ "$FREE_MB" -lt 0 ] && FREE_MB=0
    RAM=$((RAM_USED_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))

    # Swap
    SWAP_MB=$(sysctl -n vm.swapusage 2>/dev/null | awk '{gsub(/M/,"",$3); printf "%.0f",$3}')
    SWAP=$((SWAP_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))

    # Load average (1-min)
    LOAD=$(sysctl -n vm.loadavg 2>/dev/null | awk '{gsub(/[{}]/,""); printf "%.0f",$1}')

    # ── CONFIG ───────────────────────────────────────────────
    eval "$(/opt/homebrew/bin/python3.12 -c "
import json
try:
    c=json.load(open('$CONFIG'))
    print(f'CPU_CEIL={c.get(\"cpu_ceil\",90)}')
    print(f'RAM_CEIL={c.get(\"ram_ceil\",80)}')
    print(f'SWAP_CEIL={c.get(\"swap_ceil\",50)}')
    g=c.get('guard',False)
    print(f'GUARD_ON={1 if g else 0}')
    print(f'BRIDGE_MAX={c.get(\"bridge_max\",4)}')
except: print('CPU_CEIL=90\nRAM_CEIL=80\nSWAP_CEIL=50\nGUARD_ON=0\nBRIDGE_MAX=4')
" 2>/dev/null)" || { CPU_CEIL=90; RAM_CEIL=80; SWAP_CEIL=50; GUARD_ON=0; BRIDGE_MAX=4; }

    # ── BRIDGE LIMITER ───────────────────────────────────────
    if [ "$GUARD_ON" = "1" ] && [ "${BRIDGE_MAX:-0}" -gt 0 ]; then
      BRIDGE_PIDS=$(ps -eo pid=,lstart=,command= | grep 'gap_finder.hexa bridge' | grep -v grep | sort -k2,5 | awk '{print $1}')
      BRIDGE_COUNT=$(echo "$BRIDGE_PIDS" | grep -c . 2>/dev/null || echo 0)
      if [ "$BRIDGE_COUNT" -gt "$BRIDGE_MAX" ]; then
        KILL_N=$((BRIDGE_COUNT - BRIDGE_MAX))
        echo "$BRIDGE_PIDS" | head -"$KILL_N" | while read BPID; do
          kill "$BPID" 2>/dev/null || true
        done
      fi
      # RAM critical (< 512MB free): reduce bridges to 1
      if [ "$FREE_MB" -lt 512 ] && [ "$BRIDGE_COUNT" -gt 1 ]; then
        echo "$BRIDGE_PIDS" | head -$((BRIDGE_COUNT - 1)) | while read BPID; do
          kill "$BPID" 2>/dev/null || true
        done
      fi
    fi

    # ── LEVEL ASSESSMENT (multi-signal) ──────────────────────
    LEVEL="ok"
    THROTTLED="false"
    CPU_OVER=0; RAM_OVER=0; SWAP_OVER=0; RAM_CRIT=0
    [ "$CPU"  -gt "$CPU_CEIL"  ] 2>/dev/null && CPU_OVER=1
    [ "$RAM"  -gt "$RAM_CEIL"  ] 2>/dev/null && RAM_OVER=1
    [ "$SWAP" -gt "$SWAP_CEIL" ] 2>/dev/null && SWAP_OVER=1
    [ "$FREE_MB" -lt 256 ] 2>/dev/null && RAM_CRIT=1

    # Emergency: RAM < 256MB free → immediate action regardless of ceiling
    if [ "$RAM_CRIT" = "1" ]; then
      LEVEL="critical"
      THROTTLED="true"
      if [ "$GUARD_ON" = "1" ]; then
        purge 2>/dev/null || true
        # background QoS on top 5 RAM consumers (non-system)
        ps -eo pid=,rss=,comm= | sort -rnk2 | head -5 | while read TPID TRSS TNAME; do
          [ "${TPID:-0}" -lt 200 ] 2>/dev/null && continue
          case "$TNAME" in kernel_task|WindowServer|launchd|loginwindow) continue ;; esac
          taskpolicy -b -p "$TPID" 2>/dev/null || true
          echo "$TPID" >> "$THROTTLE_FILE"
        done
      fi

    elif [ "$CPU_OVER" = "1" ] || [ "$RAM_OVER" = "1" ] || [ "$SWAP_OVER" = "1" ]; then
      LEVEL="danger"
      THROTTLED="true"
      if [ "$GUARD_ON" = "1" ]; then
        # CPU enforcement: taskpolicy -b on top consumers
        if [ "$CPU_OVER" = "1" ]; then
          ps -Ao pid=,%cpu=,comm= | sort -rnk2 | head -8 | while read TPID TCPU TNAME; do
            [ "${TPID:-0}" -lt 200 ] 2>/dev/null && continue
            TCPU_INT=${TCPU%.*}
            [ "${TCPU_INT:-0}" -lt 15 ] 2>/dev/null && continue
            case "$TNAME" in kernel_task|WindowServer|launchd|loginwindow|opendirectoryd) continue ;; esac
            taskpolicy -b -p "$TPID" 2>/dev/null || true
            echo "$TPID" >> "$THROTTLE_FILE"
          done

          # Duty-cycle if >30% over ceiling
          OVERSHOOT=$((CPU - CPU_CEIL))
          if [ "$OVERSHOOT" -gt $((CPU_CEIL * 30 / 100)) ]; then
            TOP_PID=$(ps -Ao pid=,%cpu= -r | head -1 | awk '{print $1}')
            TOP_NAME=$(ps -p "$TOP_PID" -o comm= 2>/dev/null || echo "")
            case "$TOP_NAME" in kernel_task|WindowServer|launchd|loginwindow) ;; *)
              if [ "${TOP_PID:-0}" -gt 200 ] 2>/dev/null; then
                kill -STOP "$TOP_PID" 2>/dev/null || true
                PAUSE_MS=$((OVERSHOOT * 10))
                [ "$PAUSE_MS" -gt 1000 ] && PAUSE_MS=1000
                [ "$PAUSE_MS" -lt 300 ] && PAUSE_MS=300
                perl -e "select(undef,undef,undef,$PAUSE_MS/1000)" 2>/dev/null || sleep 1
                kill -CONT "$TOP_PID" 2>/dev/null || true
              fi
            ;; esac
          fi
        fi

        # RAM enforcement: purge
        if [ "$RAM_OVER" = "1" ]; then
          purge 2>/dev/null || true
        fi
      fi

    elif [ "$CPU" -gt $((CPU_CEIL * 90 / 100)) ] 2>/dev/null || [ "$LOAD" -gt $((NCPU * 3)) ] 2>/dev/null; then
      LEVEL="warn"
      # Warn: bridge에만 경량 taskpolicy (user 앱은 안 건드림)
      if [ "$GUARD_ON" = "1" ]; then
        ps -eo pid=,%cpu=,comm= | grep 'hexa' | sort -rnk2 | head -3 | while read TPID TCPU TNAME; do
          TCPU_INT=${TCPU%.*}
          [ "${TCPU_INT:-0}" -lt 20 ] 2>/dev/null && continue
          taskpolicy -b -p "$TPID" 2>/dev/null || true
          echo "$TPID" >> "$THROTTLE_FILE"
        done
      fi

    else
      # OK — restore all throttled processes
      if [ -f "$THROTTLE_FILE" ]; then
        sort -u "$THROTTLE_FILE" | while read RPID; do
          taskpolicy -d default -p "$RPID" 2>/dev/null || true
        done
        rm -f "$THROTTLE_FILE"
      fi
    fi

    # Track level transitions for smarter decisions
    PREV_LEVEL="$LEVEL"

    cat > "$STATE" <<EOF
{"active":true,"cpu":$CPU,"ram":$RAM,"swap":$SWAP,"free_mb":$FREE_MB,"load":$LOAD,"level":"$LEVEL","throttled":$THROTTLED,"cpu_ceil":$CPU_CEIL,"ram_ceil":$RAM_CEIL,"swap_ceil":$SWAP_CEIL}
EOF
    sleep 3
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
    statusItem.button.title = $(levelIcon(lv) + ' ' + j.cpu + '% \u00B7 ' + j.ram + '%' + swapTag);

    var bw = 16;
    cpuItem.title  = $('CPU  ' + bar(j.cpu, cc, bw)  + '  ' + j.cpu  + '/' + cc + '%');
    ramItem.title  = $('RAM  ' + bar(j.ram, rc, bw)   + '  ' + j.ram  + '/' + rc + '%');
    var swapIcon = swapHigh ? '\u26D4 ' : (swapMid ? '\u26A0 ' : '');
    swapItem.title = $(swapIcon + 'Swap ' + bar(j.swap, sc, bw)  + '  ' + j.swap + '/' + sc + '%');

    var swapNote = swapHigh ? ' [swap]' : (swapMid ? ' [swap]' : '');
    var freeMB = j.free_mb || 0;
    var loadAvg = j.load || 0;
    var ramNote = freeMB < 512 ? ' [' + freeMB + 'MB free]' : '';
    if (lv === 'critical') {
        safetyItem.title = $('\uD83D\uDD34 CRITICAL \u2014 RAM ' + freeMB + 'MB free' + swapNote);
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
