#!/bin/bash
# airgenome — build & launch menubar (one command, no sudo)
set -e

SCRIPT="$(readlink -f "$0" 2>/dev/null || realpath "$0" 2>/dev/null || echo "$0")"
DIR="$(cd "$(dirname "$SCRIPT")" && pwd)"

# --settings: open settings panel only
if [ "${1:-}" = "--settings" ] || [ "${1:-}" = "-s" ]; then
  CONFIG="${TMPDIR:-/tmp}/airgenome-config.json"
  exec osascript -l JavaScript "$DIR/settings.js" "$CONFIG"
fi
CARGO="${CARGO:-$(command -v cargo || echo "$HOME/.cargo/bin/cargo")}"
STATE="${TMPDIR:-/tmp}/airgenome-state.json"
CONFIG="${TMPDIR:-/tmp}/airgenome-config.json"

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
    CPU_TOTAL=$(ps -A -o %cpu= | awk '{s+=$1}END{printf "%.0f",s}')
    CPU=$((CPU_TOTAL / NCPU))
    RAM_MB=$(vm_stat | awk '/Pages active/{gsub(/\./,"",$3); print int($3*4096/1048576)}')
    RAM=$((RAM_MB * 100 / TOTAL_RAM_MB))
    SWAP_MB=$(sysctl -n vm.swapusage 2>/dev/null | awk '{gsub(/M/,"",$3); printf "%.0f",$3}')
    SWAP=$((SWAP_MB * 100 / (TOTAL_RAM_MB > 0 ? TOTAL_RAM_MB : 1)))

    eval "$(python3 -c "
import json
try:
    c=json.load(open('$CONFIG'))
    print(f\"CPU_CEIL={c.get('cpu_ceil',90)}\")
    print(f\"RAM_CEIL={c.get('ram_ceil',80)}\")
    print(f\"SWAP_CEIL={c.get('swap_ceil',50)}\")
    print(f\"GUARD_ON={'1' if c.get('guard',False) else '0'}\")
except: print('CPU_CEIL=90\nRAM_CEIL=80\nSWAP_CEIL=50\nGUARD_ON=0')
" 2>/dev/null)" || { CPU_CEIL=90; RAM_CEIL=80; SWAP_CEIL=50; GUARD_ON=0; }

    LEVEL="ok"
    THROTTLED="false"
    if [ "$CPU" -gt "$CPU_CEIL" ] 2>/dev/null || [ "$RAM" -gt "$RAM_CEIL" ] 2>/dev/null || [ "$SWAP" -gt "$SWAP_CEIL" ] 2>/dev/null; then
      LEVEL="danger"
      THROTTLED="true"
      # renice only if guard module is ON
      if [ "$GUARD_ON" = "1" ]; then
        ps -Ao pid,%cpu= | sort -rnk2 | head -3 | awk '{print $1}' | while read PID; do
          renice 15 -p "$PID" 2>/dev/null || true
        done
      fi
    elif [ "$CPU" -gt $((CPU_CEIL * 80 / 100)) ] 2>/dev/null; then
      LEVEL="warn"
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
