ObjC.import('Cocoa');
ObjC.import('Foundation');

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

var statePath = '__STATE__';
var configPath = '__CONFIG__';
var settingsJsPath = '__DIR__/settings.js';

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

menu.addItem($.NSMenuItem.separatorItem);

var gateItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Gate ...'), null, $(''));
gateItem.enabled = false;
menu.addItem(gateItem);

var uCpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  CPU ...'), null, $(''));
uCpuItem.enabled = false;
menu.addItem(uCpuItem);

var uRamItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  RAM ...'), null, $(''));
uRamItem.enabled = false;
menu.addItem(uRamItem);

var uGpuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('  GPU ...'), null, $(''));
uGpuItem.enabled = false;
menu.addItem(uGpuItem);

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
    cpuItem.title  = $('CPU  ' + bar(j.cpu, cc, bw) + '  ' + j.cpu  + '/' + cc + '%');
    ramItem.title  = $('RAM  ' + bar(j.ram, rc, bw) + '  ' + j.ram  + '/' + rc + '%');
    var swapIcon = swapHigh ? '\u26D4 ' : (swapMid ? '\u26A0 ' : '');
    swapItem.title = $(swapIcon + 'Swap ' + bar(j.swap, sc, bw) + '  ' + j.swap + '/' + sc + '%');

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
        var uRamTotalG = Math.round((j.ubu_ram_total || 0) / 1024 * 10) / 10;
        var uCpu = j.ubu_cpu || 0;
        var uJobs = j.ubu_jobs || 0;
        var uGpuUtil = j.ubu_gpu_util || 0;
        var uGpuMem = j.ubu_gpu_mem || 0;
        var uGpuName = j.ubu_gpu_name || '';
        var jobsTag = uJobs > 0 ? '  \u2191' + uJobs + 'jobs' : '';
        gateItem.title = $('\u25CF Ubuntu  load=' + uLoad + jobsTag);
        uCpuItem.title = $('  CPU  ' + bar(uCpu, 100, bw) + '  ' + uCpu + '%');
        uCpuItem.hidden = false;
        uRamItem.title = $('  RAM  ' + bar(uRamPct, 100, bw) + '  ' + uRamPct + '%  (' + uRamAvailG + 'G/' + uRamTotalG + 'G)');
        uRamItem.hidden = false;
        if (uGpuName) {
            uGpuItem.title = $('  GPU  ' + bar(uGpuUtil, 100, bw) + '  ' + uGpuUtil + '%  VRAM ' + uGpuMem + '%  ' + uGpuName);
            uGpuItem.hidden = false;
        } else {
            uGpuItem.title = $('  GPU  \u2014 driver not installed');
            uGpuItem.hidden = false;
        }
    } else {
        gateItem.title = $('\u25CB Ubuntu \u2014 offline');
        uCpuItem.title = $('');
        uRamItem.title = $('');
        uGpuItem.title = $('');
        uCpuItem.hidden = true;
        uRamItem.hidden = true;
        uGpuItem.hidden = true;
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
