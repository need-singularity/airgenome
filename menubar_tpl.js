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

// ─── AG3 Ubuntu-First ───
var ag3HeaderItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('\u2501\u2501\u2501 AG3 Ubuntu-First \u2501\u2501\u2501'), null, $(''));
ag3HeaderItem.enabled = false;
menu.addItem(ag3HeaderItem);

var ag3UbuItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('ubu ...'), null, $(''));
ag3UbuItem.enabled = false;
menu.addItem(ag3UbuItem);

var ag3VramItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('VRAM ...'), null, $(''));
ag3VramItem.enabled = false;
menu.addItem(ag3VramItem);

var ag3RingItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Ring ...'), null, $(''));
ag3RingItem.enabled = false;
menu.addItem(ag3RingItem);

var ag3GpuDetailItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('GPU detail ...'), null, $(''));
ag3GpuDetailItem.enabled = false;
menu.addItem(ag3GpuDetailItem);

var ag3SvcItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Svc ...'), null, $(''));
ag3SvcItem.enabled = false;
menu.addItem(ag3SvcItem);

var ag3FallbackItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('Fallback ...'), null, $(''));
ag3FallbackItem.enabled = false;
menu.addItem(ag3FallbackItem);

var ag3ApiItem = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent($('API ...'), null, $(''));
ag3ApiItem.enabled = false;
menu.addItem(ag3ApiItem);

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

function readAg3() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(ag3StatusPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        return JSON.parse(str.js);
    } catch(e) { return null; }
}

function readApi() {
    try {
        var task = $.NSTask.alloc.init;
        task.executableURL = $.NSURL.fileURLWithPath($('/usr/bin/curl'));
        task.arguments = $(['-s', '--connect-timeout', '1', 'http://127.0.0.1:17777/state']);
        var pipe = $.NSPipe.pipe;
        task.standardOutput = pipe;
        task.standardError = $.NSPipe.pipe;
        task.launchAndReturnError(null);
        task.waitUntilExit;
        if (task.terminationStatus !== 0) return null;
        var data = pipe.fileHandleForReading.readDataToEndOfFile;
        var str = $.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding);
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
        var uRamUsedG = Math.round((uRamTotalG - uRamAvailG) * 10) / 10;
        uRamItem.title = $('  RAM  ' + bar(uRamPct, 100, bw) + '  ' + uRamPct + '%  (' + uRamUsedG + 'G/' + uRamTotalG + 'G)');
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

    // ─── AG3 section ───
    var ag3 = readAg3();
    if (ag3) {
        var upIcon = ag3.ubu_up ? '\uD83D\uDFE2' : '\uD83D\uDD34';
        var upTxt  = ag3.ubu_up ? 'ubu UP' : 'ubu DOWN';
        var gname = ag3.gpu_name || '';
        ag3UbuItem.title = $(upIcon + '  ' + upTxt + (gname ? '  \u25CF ' + gname : ''));
        var vu = ag3.gpu_vram_used_mb || 0;
        var vt = ag3.gpu_vram_total_mb || 0;
        var vpct = vt > 0 ? Math.round(vu * 100 / vt) : 0;
        ag3VramItem.title = $('\uD83C\uDFAE VRAM ' + bar(vpct, 100, 10) + '  ' + vu + '/' + vt + ' MB (' + vpct + '%)  util ' + (ag3.gpu_util || 0) + '%');
        // GPU temp / power detail line
        var gTemp = ag3.gpu_temp_c || 0;
        var gPow  = ag3.gpu_power_w || 0;
        var tempIcon = gTemp >= 80 ? '\u26A0 ' : '';
        ag3GpuDetailItem.title = $(tempIcon + '\uD83C\uDF21 ' + gTemp + '\u00B0C  \u26A1' + gPow + 'W');
        ag3GpuDetailItem.hidden = false;
        // Ring stats with fill bar
        var rwi = ag3.ring_write_idx || 0;
        var rsc = ag3.ring_slot_count || 0;
        var ringPct = rsc > 0 ? Math.round(rwi * 100 / rsc) : 0;
        ag3RingItem.title = $('\uD83D\uDCBE Ring ' + bar(ringPct, 100, 10) + '  ' + rwi + '/' + rsc + ' (' + ringPct + '%)');
        // Service status
        var gateUp = ag3.svc_gate ? '\u25CF' : '\u25CB';
        var fillUp = ag3.svc_fill ? '\u25CF' : '\u25CB';
        ag3SvcItem.title = $('\u2699 svc  gate ' + gateUp + '  fill ' + fillUp);
        ag3SvcItem.hidden = false;
        // Fallback
        var fb = ag3.fallback_count_10min || 0;
        var fbIcon = fb > 0 ? '\u26A0\uFE0F ' : '\u2500 ';
        ag3FallbackItem.title = $(fbIcon + 'Fallback(10m): ' + fb);
    } else {
        ag3UbuItem.title = $('\u26A0 AG3 feed unavailable');
        ag3VramItem.title = $('');
        ag3GpuDetailItem.title = $(''); ag3GpuDetailItem.hidden = true;
        ag3RingItem.title = $('');
        ag3SvcItem.title = $(''); ag3SvcItem.hidden = true;
        ag3FallbackItem.title = $('');
    }

    // ─── API poll ───
    var apiState = readApi();
    if (apiState) {
        var genomeCount = apiState.genome_count || 0;
        var forgeActive = apiState.forge_active ? '\u25CF' : '\u25CB';
        ag3ApiItem.title = $('\uD83C\uDF10 API :17777  forge ' + forgeActive + '  genomes=' + genomeCount);
        ag3ApiItem.hidden = false;
    } else {
        ag3ApiItem.title = $('\u25CB API :17777 \u2014 offline');
        ag3ApiItem.hidden = false;
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
