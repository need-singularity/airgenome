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
{"cpu_ceil": $CPU_C, "ram_ceil": $RAM_C, "swap_ceil": $SWAP_C}
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

# 6. Background sampler + safety net
(
  while true; do
    # CPU: use top for accurate system-wide % (user+sys), fallback to ps
    CPU=$(top -l1 -n0 2>/dev/null | awk '/CPU usage/{gsub(/%/,""); printf "%d",$3+$5}' || echo 0)
    [ "${CPU:-0}" -eq 0 ] && { CPU_TOTAL=$(ps -A -o %cpu= | awk '{s+=$1}END{printf "%.0f",s}'); CPU=$((CPU_TOTAL / NCPU)); }
    # RAM: active + wired + compressor (matches guard.hexa)
    RAM_MB=$(vm_stat 2>/dev/null | awk '/Pages active/{a=$3}/Pages wired/{w=$4}/compressor/{c=$5}END{gsub(/\./,"",a);gsub(/\./,"",w);gsub(/\./,"",c);print int((a+w+c)*16384/1048576)}' || echo 0)
    RAM=$((RAM_MB * 100 / TOTAL_RAM_MB))
    SWAP_MB=$(sysctl -n vm.swapusage 2>/dev/null | awk '{gsub(/M/,"",$3); printf "%.0f",$3}')
    SWAP=$((SWAP_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))

    eval "$(/opt/homebrew/bin/python3.12 -c "
import json
try:
    c=json.load(open('$CONFIG'))
    print(f'CPU_CEIL={c.get(\"cpu_ceil\",90)}')
    print(f'RAM_CEIL={c.get(\"ram_ceil\",80)}')
    print(f'SWAP_CEIL={c.get(\"swap_ceil\",50)}')
    g=c.get('guard',False)
    print(f'GUARD_ON={1 if g else 0}')
except: print('CPU_CEIL=90\nRAM_CEIL=80\nSWAP_CEIL=50\nGUARD_ON=0')
" 2>/dev/null)" || { CPU_CEIL=90; RAM_CEIL=80; SWAP_CEIL=50; GUARD_ON=0; }

    LEVEL="ok"
    THROTTLED="false"
    CPU_OVER=0; RAM_OVER=0; SWAP_OVER=0
    [ "$CPU"  -gt "$CPU_CEIL"  ] 2>/dev/null && CPU_OVER=1
    [ "$RAM"  -gt "$RAM_CEIL"  ] 2>/dev/null && RAM_OVER=1
    [ "$SWAP" -gt "$SWAP_CEIL" ] 2>/dev/null && SWAP_OVER=1

    if [ "$CPU_OVER" = "1" ] || [ "$RAM_OVER" = "1" ] || [ "$SWAP_OVER" = "1" ]; then
      LEVEL="danger"
      THROTTLED="true"
      # enforce only if guard module is ON
      if [ "$GUARD_ON" = "1" ]; then
        # --- CPU enforcement ---
        if [ "$CPU_OVER" = "1" ]; then
          # Tier 1: taskpolicy -b (background QoS) + renice 20 on top consumers
          ps -Ao pid=,%cpu=,comm= | sort -rnk2 | head -8 | while read TPID TCPU TNAME; do
            # skip kernel/system (pid<200) and our own sampler+menubar
            [ "${TPID:-0}" -lt 200 ] 2>/dev/null && continue
            [ "$TPID" = "$$" ] 2>/dev/null && continue
            # skip if process CPU < 10%
            TCPU_INT=${TCPU%.*}
            [ "${TCPU_INT:-0}" -lt 10 ] 2>/dev/null && continue
            # skip critical system processes
            case "$TNAME" in kernel_task|WindowServer|launchd|loginwindow|opendirectoryd) continue ;; esac
            renice 20 -p "$TPID" 2>/dev/null || true
            taskpolicy -b -p "$TPID" 2>/dev/null || true
          done

          # Tier 2: duty-cycle throttle if >30% over ceiling (e.g. 78% at 60% ceil)
          OVERSHOOT=$((CPU - CPU_CEIL))
          if [ "$OVERSHOOT" -gt $((CPU_CEIL * 30 / 100)) ]; then
            # pause top consumer briefly (proportional to overshoot)
            TOP_PID=$(ps -Ao pid=,%cpu= -r | head -1 | awk '{print $1}')
            TOP_NAME=$(ps -p "$TOP_PID" -o comm= 2>/dev/null || echo "")
            case "$TOP_NAME" in kernel_task|WindowServer|launchd|loginwindow) ;; *)
              if [ "${TOP_PID:-0}" -gt 200 ] 2>/dev/null; then
                kill -STOP "$TOP_PID" 2>/dev/null || true
                # pause 0.3-1.0s proportional to overshoot
                PAUSE_MS=$((OVERSHOOT * 10))
                [ "$PAUSE_MS" -gt 1000 ] && PAUSE_MS=1000
                [ "$PAUSE_MS" -lt 300 ] && PAUSE_MS=300
                perl -e "select(undef,undef,undef,$PAUSE_MS/1000)" 2>/dev/null || sleep 1
                kill -CONT "$TOP_PID" 2>/dev/null || true
              fi
            ;; esac
          fi
        fi

        # --- RAM enforcement: purge if over ceiling ---
        if [ "$RAM_OVER" = "1" ]; then
          # macOS memory_pressure + purge (non-destructive cache flush)
          purge 2>/dev/null || true
        fi
      fi
    elif [ "$CPU" -gt $((CPU_CEIL * 80 / 100)) ] 2>/dev/null; then
      LEVEL="warn"
      # pre-warn: light renice on top 3
      if [ "$GUARD_ON" = "1" ]; then
        ps -Ao pid=,%cpu= | sort -rnk2 | head -3 | awk '{print $1}' | while read TPID; do
          [ "${TPID:-0}" -lt 200 ] 2>/dev/null && continue
          renice 10 -p "$TPID" 2>/dev/null || true
        done
      fi
    fi

    cat > "$STATE" <<EOF
{"active":true,"cpu":$CPU,"ram":$RAM,"swap":$SWAP,"level":"$LEVEL","throttled":$THROTTLED,"cpu_ceil":$CPU_CEIL,"ram_ceil":$RAM_CEIL,"swap_ceil":$SWAP_CEIL}
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
    if (lv === 'danger') {
        safetyItem.title = $('\u26A0 THROTTLE \u2014 renice active' + swapNote);
    } else if (lv === 'warn') {
        safetyItem.title = $('\u26A1 Approaching ceiling' + swapNote);
    } else {
        safetyItem.title = $('\u2705 Safe');
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
