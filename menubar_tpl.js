ObjC.import('Cocoa');
ObjC.import('Foundation');

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

var statePath = '__STATE__';
var configPath = '__CONFIG__';
var settingsJsPath = '__DIR__/settings.js';
var ag3StatusPath = ($.NSString.stringWithString($('~/.airgenome/ag3_status.json')).stringByExpandingTildeInPath).js;

ObjC.registerSubclass({
    name: 'MenuHandler',
    methods: {
        'openSettings:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                var task = $.NSTask.alloc.init;
                task.executableURL = $.NSURL.fileURLWithPath($('/usr/bin/osascript'));
                task.arguments = $(['-l', 'JavaScript', settingsJsPath, configPath]);
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

// ─── Mac ───
var macHeader = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2501\u2501\u2501 Mac \u2501\u2501\u2501'), null, $(''));
macHeader.enabled = false;
menu.addItem(macHeader);

var cpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('CPU  ...'), null, $(''));
cpuItem.enabled = false;
menu.addItem(cpuItem);

var ramItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('RAM  ...'), null, $(''));
ramItem.enabled = false;
menu.addItem(ramItem);

var gpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('GPU  ...'), null, $(''));
gpuItem.enabled = false;
menu.addItem(gpuItem);

var saveItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Save ...'), null, $(''));
saveItem.enabled = false;
menu.addItem(saveItem);

// ─── Ubuntu ───
var ubuHeader = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2501\u2501\u2501 Ubuntu \u2501\u2501\u2501'), null, $(''));
ubuHeader.enabled = false;
menu.addItem(ubuHeader);

var uCpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  CPU ...'), null, $(''));
uCpuItem.enabled = false;
menu.addItem(uCpuItem);

var uRamItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  RAM ...'), null, $(''));
uRamItem.enabled = false;
menu.addItem(uRamItem);

var uGpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  GPU ...'), null, $(''));
uGpuItem.enabled = false;
menu.addItem(uGpuItem);

// ─── Hetzner ───
var htzHeader = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2501\u2501\u2501 Hetzner \u2501\u2501\u2501'), null, $(''));
htzHeader.enabled = false;
menu.addItem(htzHeader);

var htzCpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  CPU ...'), null, $(''));
htzCpuItem.enabled = false;
menu.addItem(htzCpuItem);

var htzRamItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  RAM ...'), null, $(''));
htzRamItem.enabled = false;
menu.addItem(htzRamItem);

var htzGpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  GPU ...'), null, $(''));
htzGpuItem.enabled = false;
menu.addItem(htzGpuItem);

// ─── Vast.ai ───
var vastHeader = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2501\u2501\u2501 Vast.ai \u2501\u2501\u2501'), null, $(''));
vastHeader.enabled = false;
menu.addItem(vastHeader);

var vastCpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  CPU ...'), null, $(''));
vastCpuItem.enabled = false;
menu.addItem(vastCpuItem);

var vastRamItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  RAM ...'), null, $(''));
vastRamItem.enabled = false;
menu.addItem(vastRamItem);

var vastGpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  GPU ...'), null, $(''));
vastGpuItem.enabled = false;
menu.addItem(vastGpuItem);

menu.addItem($.NSMenuItem.separatorItem);

// ─── Status ───
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

// ─── Readers ───
function readState() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(statePath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        return JSON.parse(str.js);
    } catch(e) { return null; }
}

function readAg3() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(ag3StatusPath), $.NSUTF8StringEncoding, null);
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

var infraPath = ($.NSString.stringWithString($('~/Dev/nexus/shared/infra_state.json')).stringByExpandingTildeInPath).js;

function readInfra() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(infraPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        var obj = JSON.parse(str.js);
        if (obj.ts) {
            var tsDate = new Date(obj.ts);
            var now = new Date();
            if ((now - tsDate) > 60 * 60 * 1000) { obj._stale = true; }
        }
        return obj;
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

var _lastValidState = null;

$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(2.0, true, function() {
    var j = readState();
    if (!j && _lastValidState) { j = _lastValidState; }
    if (!j) { statusItem.button.title = $('\u26A0 airgenome'); return; }
    _lastValidState = j;

    var cfg = readConfig() || {cpu_ceil: 90, ram_ceil: 80, swap_ceil: 50};
    var cc = cfg.cpu_ceil;
    var rc = cfg.ram_ceil;
    var sc = cfg.swap_ceil;
    var bw = 16;

    var lv = j.level || 'ok';
    var gateIcon = j.gate === 'online' ? ' \u25CF' : ' \u25CB';
    statusItem.button.title = $(levelIcon(lv) + ' ' + j.cpu + '% \u00B7 ' + j.ram + '%' + gateIcon);

    // ═══ Mac ═══
    cpuItem.title = $('  CPU  ' + bar(j.cpu, cc, bw) + '  ' + j.cpu + '/' + cc + '%');
    ramItem.title = $('  RAM  ' + bar(j.ram, rc, bw) + '  ' + j.ram + '/' + rc + '%');
    var gpuLocal = j.gpu_local || 0;
    gpuItem.title = $('  GPU  ' + bar(gpuLocal, 100, bw) + '  ' + gpuLocal + '%');

    var saveCpu = j.save_cpu || 0;
    var saveRam = j.save_ram || 0;
    var saveTotal = Math.min(Math.round((saveCpu + saveRam) / 2), 99);
    var saveIcon = saveTotal > 0 ? '\u2193' : '\u2500';
    saveItem.title = $('  ' + saveIcon + ' Save  CPU -' + saveCpu + '%  RAM -' + saveRam + '%');

    // ═══ Ubuntu ═══
    if (j.gate === 'online') {
        var uCpu = j.ubu_cpu || 0;
        var uRamPct = j.ubu_ram_pct || 0;
        var uRamAvailG = Math.round((j.ubu_ram_avail || 0) / 1024 * 10) / 10;
        var uRamTotalG = Math.round((j.ubu_ram_total || 0) / 1024 * 10) / 10;
        var uRamUsedG = Math.round((uRamTotalG - uRamAvailG) * 10) / 10;
        var uGpuUtil = j.ubu_gpu_util || 0;
        var uGpuMem = j.ubu_gpu_mem || 0;
        var uGpuName = j.ubu_gpu_name || '';
        ubuHeader.title = $('\u2501\u2501\u2501 Ubuntu \u25CF \u2501\u2501\u2501');
        uCpuItem.title = $('  CPU  ' + bar(uCpu, 100, bw) + '  ' + uCpu + '%');
        uCpuItem.hidden = false;
        uRamItem.title = $('  RAM  ' + bar(uRamPct, 100, bw) + '  ' + uRamPct + '%  (' + uRamUsedG + 'G/' + uRamTotalG + 'G)');
        uRamItem.hidden = false;
        uGpuItem.title = $('  GPU  ' + bar(uGpuUtil, 100, bw) + '  ' + uGpuUtil + '%  VRAM ' + uGpuMem + '%  ' + uGpuName);
        uGpuItem.hidden = false;
    } else {
        ubuHeader.title = $('\u2501\u2501\u2501 Ubuntu \u25CB \u2501\u2501\u2501');
        uCpuItem.title = $('  offline'); uCpuItem.hidden = false;
        uRamItem.hidden = true;
        uGpuItem.hidden = true;
    }

    // ═══ Hetzner ═══
    var infra = readInfra();
    if (infra && !infra._stale && infra.hosts && infra.hosts.htz) {
        var hz = infra.hosts.htz;
        var hzStatus = hz.status === 'active' ? '\u25CF' : '\u25CB';
        htzHeader.title = $('\u2501\u2501\u2501 Hetzner ' + hzStatus + ' \u2501\u2501\u2501');
        var hzLoad = hz.load || '?';
        var hzThreads = hz.cpu_threads || 0;
        var hzLoadPct = hzThreads > 0 ? Math.round(parseFloat(hzLoad) * 100 / hzThreads) : 0;
        htzCpuItem.title = $('  CPU  ' + bar(hzLoadPct, 100, bw) + '  load=' + hzLoad + '  ' + hzThreads + 'T');
        htzCpuItem.hidden = false;
        var hzRamUsedG = Math.round((hz.ram_used_mb || 0) / 1024 * 10) / 10;
        var hzRamTotalG = Math.round((hz.ram_total_mb || 0) / 1024 * 10) / 10;
        var hzRamPct = hzRamTotalG > 0 ? Math.round(hzRamUsedG * 100 / hzRamTotalG) : 0;
        htzRamItem.title = $('  RAM  ' + bar(hzRamPct, 100, bw) + '  ' + hzRamUsedG + '/' + hzRamTotalG + 'GB');
        htzRamItem.hidden = false;
        htzGpuItem.title = $('  GPU  \u2014 CPU-only (EPYC)');
        htzGpuItem.hidden = false;
    } else {
        var htzTag = (infra && infra._stale) ? '\u25CB stale' : '\u25CB';
        htzHeader.title = $('\u2501\u2501\u2501 Hetzner ' + htzTag + ' \u2501\u2501\u2501');
        htzCpuItem.title = $('  CPU  ' + bar(0, 100, bw) + '  0%'); htzCpuItem.hidden = false;
        htzRamItem.title = $('  RAM  ' + bar(0, 100, bw) + '  0/0GB'); htzRamItem.hidden = false;
        htzGpuItem.title = $('  GPU  \u2014 CPU-only (EPYC)'); htzGpuItem.hidden = false;
    }

    // ═══ Vast.ai ═══
    if (infra && !infra._stale && infra.hosts && infra.hosts.vast) {
        var v = infra.hosts.vast;
        var vActive = v.status === 'active';
        var vStatus = vActive ? '\u25CF' : '\u25CB';
        vastHeader.title = $('\u2501\u2501\u2501 Vast.ai ' + vStatus + ' \u2501\u2501\u2501');
        var vGpuUtil = v.gpu_util || 0;
        var vVramUsed = v.vram_used_gb || 0;
        var vVramTotal = v.vram_gb || 96;
        var vVramPct = vVramTotal > 0 ? Math.round(vVramUsed * 100 / vVramTotal) : 0;
        var vGpuName = v.gpu || '4x RTX 4090';
        var vCpu = v.cpu_pct || 0;
        var vCpuCores = v.cpu_cores || 0;
        vastCpuItem.title = $('  CPU  ' + bar(vCpu, 100, bw) + '  ' + vCpu + '%' + (vCpuCores > 0 ? '  ' + vCpuCores + 'C' : ''));
        vastCpuItem.hidden = false;
        var vRamUsed = v.ram_used_gb || 0;
        var vRamTotal = v.ram_total_gb || 0;
        var vRamPct = vRamTotal > 0 ? Math.round(vRamUsed * 100 / vRamTotal) : 0;
        vastRamItem.title = $('  RAM  ' + bar(vRamPct, 100, bw) + '  ' + vRamUsed + '/' + vRamTotal + 'GB');
        vastRamItem.hidden = false;
        vastGpuItem.title = $('  GPU  ' + bar(vGpuUtil, 100, bw) + '  ' + vGpuUtil + '%  VRAM ' + vVramUsed + '/' + vVramTotal + 'GB  ' + vGpuName);
        vastGpuItem.hidden = false;
    } else {
        var vastTag = (infra && infra._stale) ? '\u25CB stale' : '\u25CB';
        vastHeader.title = $('\u2501\u2501\u2501 Vast.ai ' + vastTag + ' \u2501\u2501\u2501');
        vastCpuItem.title = $('  CPU  ' + bar(0, 100, bw) + '  0%');
        vastCpuItem.hidden = false;
        vastRamItem.title = $('  RAM  ' + bar(0, 100, bw) + '  0/0GB');
        vastRamItem.hidden = false;
        vastGpuItem.title = $('  GPU  ' + bar(0, 100, bw) + '  0%  VRAM 0/96GB  4x RTX 4090');
        vastGpuItem.hidden = false;
    }

    // ═══ Safety ═══
    var freeMB = j.free_mb || 0;
    if (lv === 'critical') {
        safetyItem.title = $('\u25CB CRITICAL \u2014 RAM ' + freeMB + 'MB free');
    } else if (lv === 'danger') {
        safetyItem.title = $('\u26A0 THROTTLE active');
    } else {
        safetyItem.title = $('\u2705 Safe \u2014 ' + Math.round(freeMB/1024*10)/10 + 'G free');
    }
});

app.run;
