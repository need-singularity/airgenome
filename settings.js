// airgenome settings panel — standalone JXA
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : $.NSHomeDirectory().js + '/.airgenome/config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

var defaults = {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30, bridge_max: 4, forge: false, guard: false, autostart: true};
var plistPath = $.NSHomeDirectory().js + '/Library/LaunchAgents/com.airgenome.menubar.plist';
var accountsPath = $.NSHomeDirectory().js + '/.airgenome/accounts.json';
var usageCachePath = $.NSHomeDirectory().js + '/.airgenome/usage-cache.json';

function isAutoStartEnabled() {
    return $.NSFileManager.defaultManager.fileExistsAtPath($(plistPath));
}

function setAutoStart(on) {
    if (on) {
        // copy plist template
        var plist = '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict><key>Label</key><string>com.airgenome.menubar</string><key>ProgramArguments</key><array><string>' + $.NSHomeDirectory().js + '/.hx/bin/airgenome</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><false/><key>EnvironmentVariables</key><dict><key>PATH</key><string>/usr/local/bin:/usr/bin:/bin:' + $.NSHomeDirectory().js + '/.cargo/bin:' + $.NSHomeDirectory().js + '/.hx/bin</string></dict><key>StandardOutPath</key><string>/tmp/airgenome.log</string><key>StandardErrorPath</key><string>/tmp/airgenome.err</string></dict></plist>';
        $(plist).writeToFileAtomicallyEncodingError($(plistPath), true, $.NSUTF8StringEncoding, null);
    } else {
        $.NSFileManager.defaultManager.removeItemAtPathError($(plistPath), null);
    }
}

function readConfig() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(configPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return defaults;
        var cfg = JSON.parse(str.js);
        for (var k in defaults) { if (cfg[k] === undefined) cfg[k] = defaults[k]; }
        return cfg;
    } catch(e) { return defaults; }
}

function writeFullConfig(obj) {
    var json = JSON.stringify(obj);
    $(json).writeToFileAtomicallyEncodingError($(configPath), true, $.NSUTF8StringEncoding, null);
}

var cfg = readConfig();

var win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(0, 0, 420, 610),
    $.NSWindowStyleMaskTitled | $.NSWindowStyleMaskClosable,
    $.NSBackingStoreBuffered, false
);
win.title = $('airgenome Settings');
win.level = $.NSFloatingWindowLevel;
win.center;

// ─── Tab View (기본 설정 | 계정 관리) ───
var tabView = $.NSTabView.alloc.initWithFrame($.NSMakeRect(0, 0, 420, 610));

var settingsTab = $.NSTabViewItem.alloc.initWithIdentifier($('settings'));
settingsTab.label = $('기본 설정');
var settingsView = $.NSView.alloc.initWithFrame($.NSMakeRect(0, 0, 400, 560));
settingsTab.view = settingsView;

var accountsTab = $.NSTabViewItem.alloc.initWithIdentifier($('accounts'));
accountsTab.label = $('계정 관리');
var accountsView = $.NSView.alloc.initWithFrame($.NSMakeRect(0, 0, 400, 560));
accountsTab.view = accountsView;

tabView.addTabViewItem(settingsTab);
tabView.addTabViewItem(accountsTab);
win.contentView.addSubview(tabView);

var cv = settingsView;

function makeRow(y, name, min, max, val) {
    var lbl = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, y + 28, 300, 18));
    lbl.stringValue = $(name + ': ' + val + '%');
    lbl.bordered = false;
    lbl.editable = false;
    lbl.drawsBackground = false;
    lbl.font = $.NSFont.systemFontOfSize(13);
    cv.addSubview(lbl);

    var sl = $.NSSlider.alloc.initWithFrame($.NSMakeRect(20, y, 300, 26));
    sl.minValue = min;
    sl.maxValue = max;
    sl.doubleValue = val;
    sl.continuous = true;
    sl.numberOfTickMarks = Math.floor((max - min) / 5) + 1;
    sl.allowsTickMarkValuesOnly = true;
    cv.addSubview(sl);

    return {label: lbl, slider: sl, name: name};
}

// --- All (master) slider at top ---
var sep1 = $.NSBox.alloc.initWithFrame($.NSMakeRect(20, 438, 300, 1));
sep1.boxType = $.NSBoxSeparator;
cv.addSubview(sep1);

var allAvg = Math.round((cfg.cpu_ceil + cfg.ram_ceil + cfg.swap_ceil) / 3);
var allRow = makeRow(450, 'All', 0, 100, allAvg);
allRow.label.font = $.NSFont.boldSystemFontOfSize(14);

// --- Individual sliders ---
var cpuRow  = makeRow(370, 'CPU Ceiling',  10, 100, cfg.cpu_ceil);
var ramRow  = makeRow(300, 'RAM Ceiling',  10, 100, cfg.ram_ceil);
var swapRow = makeRow(230, 'Swap Ceiling',  0, 100, cfg.swap_ceil);

// --- Bridge limiter ---
var bridgeRow = makeRow(160, 'Bridge Max (hexa)', 0, 20, cfg.bridge_max || 4);

// --- Modules section ---
var sep2 = $.NSBox.alloc.initWithFrame($.NSMakeRect(20, 140, 300, 1));
sep2.boxType = $.NSBoxSeparator;
cv.addSubview(sep2);

var modTitle = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, 108, 200, 20));
modTitle.stringValue = $('Modules');
modTitle.bordered = false;
modTitle.editable = false;
modTitle.drawsBackground = false;
modTitle.font = $.NSFont.boldSystemFontOfSize(14);
cv.addSubview(modTitle);

function makeToggle(y, name, isOn) {
    var lbl = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, y, 200, 20));
    lbl.stringValue = $(name);
    lbl.bordered = false;
    lbl.editable = false;
    lbl.drawsBackground = false;
    lbl.font = $.NSFont.systemFontOfSize(13);
    cv.addSubview(lbl);

    var btn = $.NSButton.alloc.initWithFrame($.NSMakeRect(260, y, 60, 24));
    btn.setButtonType($.NSSwitchButton);
    btn.title = $('');
    btn.state = isOn ? $.NSControlStateValueOn : $.NSControlStateValueOff;
    cv.addSubview(btn);

    return {label: lbl, button: btn, name: name};
}

var forgeToggle = makeToggle(88, 'token-forge (10-account manager)', cfg.forge);
var guardToggle = makeToggle(58, 'resource-guard (CPU/RAM limiter)', cfg.guard);

var sep3 = $.NSBox.alloc.initWithFrame($.NSMakeRect(20, 48, 300, 1));
sep3.boxType = $.NSBoxSeparator;
cv.addSubview(sep3);

var autostartToggle = makeToggle(20, 'Start at login', isAutoStartEnabled());

// --- Reset button ---
var resetBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(20, -18, 300, 28));
resetBtn.title = $('\u21BA Reset to Profile Defaults');
resetBtn.bezelStyle = $.NSBezelStyleRounded;
cv.addSubview(resetBtn);

// Profile defaults (read from profiles.json via args or hardcoded detection)
var profileDefaults = null;
function loadProfileDefaults() {
    try {
        var scriptDir = args.count > 3 ? args.objectAtIndex(3).js : '';
        var profPath = scriptDir.replace(/\/[^\/]*$/, '') + '/profiles.json';
        var str = $.NSString.stringWithContentsOfFileEncodingError($(profPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return null;
        var data = JSON.parse(str.js);

        var pipe = $.NSPipe.pipe;
        var task = $.NSTask.alloc.init;
        task.executableURL = $.NSURL.fileURLWithPath($('/usr/sbin/sysctl'));
        task.arguments = $(['-n', 'machdep.cpu.brand_string']);
        task.standardOutput = pipe;
        task.launchAndReturnError(null);
        task.waitUntilExit();
        var chipStr = $.NSString.alloc.initWithDataEncoding(
            pipe.fileHandleForReading.readDataToEndOfFile, $.NSUTF8StringEncoding).js.trim();
        var chipMatch = chipStr.match(/M\d+/);
        var chip = chipMatch ? chipMatch[0] : '';

        var pipe2 = $.NSPipe.pipe;
        var task2 = $.NSTask.alloc.init;
        task2.executableURL = $.NSURL.fileURLWithPath($('/usr/sbin/sysctl'));
        task2.arguments = $(['-n', 'hw.memsize']);
        task2.standardOutput = pipe2;
        task2.launchAndReturnError(null);
        task2.waitUntilExit();
        var memStr = $.NSString.alloc.initWithDataEncoding(
            pipe2.fileHandleForReading.readDataToEndOfFile, $.NSUTF8StringEncoding).js.trim();
        var ramGB = Math.round(parseInt(memStr) / 1073741824);

        var profiles = data.profiles;
        for (var k in profiles) {
            if (k === 'default') continue;
            var m = profiles[k].match || {};
            if (m.chip && chip.indexOf(m.chip) >= 0 && m.ram_gb === ramGB) {
                return profiles[k];
            }
        }
        return profiles['default'];
    } catch(e) { return {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30, note: 'default'}; }
}
profileDefaults = loadProfileDefaults();
var resetPressed = false;

win.makeKeyAndOrderFront(null);
app.activateIgnoringOtherApps(true);

var lastAllVal = allAvg;

var resetCooldown = false;

ObjC.registerSubclass({
    name: 'ResetHandler',
    methods: {
        'doReset:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                if (!profileDefaults || resetCooldown) return;
                resetCooldown = true;
                var d = profileDefaults;
                cpuRow.slider.doubleValue = d.cpu_ceil;
                ramRow.slider.doubleValue = d.ram_ceil;
                swapRow.slider.doubleValue = d.swap_ceil;
                bridgeRow.slider.doubleValue = d.bridge_max || 4;
                var avg = Math.round((d.cpu_ceil + d.ram_ceil + d.swap_ceil) / 3);
                allRow.slider.doubleValue = avg;
                forgeToggle.button.state = $.NSControlStateValueOff;
                guardToggle.button.state = $.NSControlStateValueOff;
                // cooldown 1s
                $.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(1.0, false, function() {
                    resetCooldown = false;
                });
            }
        }
    }
});
var resetHandler = $.ResetHandler.alloc.init;
resetBtn.target = resetHandler;
resetBtn.action = 'doReset:';

$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(0.3, true, function() {
    function snap5(v) { return Math.round(v / 5) * 5; }
    var av = snap5(allRow.slider.doubleValue);
    var cc = snap5(cpuRow.slider.doubleValue);
    var rc = snap5(ramRow.slider.doubleValue);
    var sc = snap5(swapRow.slider.doubleValue);

    // If All slider moved, sync all three
    if (av !== lastAllVal) {
        cc = av;
        rc = av;
        sc = Math.max(0, av);
        cpuRow.slider.doubleValue = cc;
        ramRow.slider.doubleValue = rc;
        swapRow.slider.doubleValue = sc;
        lastAllVal = av;
    } else {
        // individual moved — update All to average
        var newAvg = Math.round((cc + rc + sc) / 3);
        allRow.slider.doubleValue = newAvg;
        lastAllVal = newAvg;
    }

    var forgeOn = forgeToggle.button.state === $.NSControlStateValueOn;
    var guardOn = guardToggle.button.state === $.NSControlStateValueOn;

    var bm = Math.round(bridgeRow.slider.doubleValue);

    allRow.label.stringValue   = $('All: ' + av + '%');
    cpuRow.label.stringValue   = $(cpuRow.name + ': ' + cc + '%');
    ramRow.label.stringValue   = $(ramRow.name + ': ' + rc + '%');
    swapRow.label.stringValue  = $(swapRow.name + ': ' + sc + '%');
    bridgeRow.label.stringValue = $('Bridge Max (hexa): ' + bm + (bm === 0 ? ' (unlimited)' : ''));

    var autoOn = autostartToggle.button.state === $.NSControlStateValueOn;
    setAutoStart(autoOn);

    // forge toggle → proxy on/off
    var prevCfg = readConfig();
    var prevForge = prevCfg.forge || false;
    if (forgeOn && !prevForge) {
        // start proxy
        var t = $.NSTask.alloc.init;
        t.executableURL = $.NSURL.fileURLWithPath($('/bin/launchctl'));
        t.arguments = $(['load', $.NSHomeDirectory().js + '/Library/LaunchAgents/com.token-forge.proxy.plist']);
        t.launchAndReturnError(null);
    } else if (!forgeOn && prevForge) {
        // stop proxy
        var t2 = $.NSTask.alloc.init;
        t2.executableURL = $.NSURL.fileURLWithPath($('/bin/launchctl'));
        t2.arguments = $(['unload', $.NSHomeDirectory().js + '/Library/LaunchAgents/com.token-forge.proxy.plist']);
        t2.launchAndReturnError(null);
    }

    writeFullConfig({cpu_ceil: cc, ram_ceil: rc, swap_ceil: sc, bridge_max: bm, forge: forgeOn, guard: guardOn, autostart: autoOn});
});

// ═══════════════════════════════════════════════════════════════════════
//  계정 관리 탭
// ═══════════════════════════════════════════════════════════════════════

function readAccounts() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(accountsPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return [];
        return JSON.parse(str.js).accounts || [];
    } catch(e) { return []; }
}

function readUsageCache() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(usageCachePath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return {};
        return JSON.parse(str.js);
    } catch(e) { return {}; }
}

function writeAccounts(accounts) {
    var json = JSON.stringify({accounts: accounts}, null, 2);
    $(json).writeToFileAtomicallyEncodingError($(accountsPath), true, $.NSUTF8StringEncoding, null);
}

// Title
var accTitle = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, 430, 360, 22));
accTitle.stringValue = $('Claude Code 계정 관리');
accTitle.bordered = false;
accTitle.editable = false;
accTitle.drawsBackground = false;
accTitle.font = $.NSFont.boldSystemFontOfSize(16);
accountsView.addSubview(accTitle);

// Scroll view with table
var scrollView = $.NSScrollView.alloc.initWithFrame($.NSMakeRect(20, 80, 360, 340));
scrollView.hasVerticalScroller = true;
scrollView.autohidesScrollers = true;
scrollView.borderType = $.NSBezelBorder;

var tableView = $.NSTableView.alloc.initWithFrame($.NSMakeRect(0, 0, 340, 340));
tableView.usesAlternatingRowBackgroundColors = true;
tableView.rowHeight = 28;

var colName = $.NSTableColumn.alloc.initWithIdentifier($('name'));
colName.headerCell.stringValue = $('계정');
colName.width = 80;
tableView.addTableColumn(colName);

var colSession = $.NSTableColumn.alloc.initWithIdentifier($('session'));
colSession.headerCell.stringValue = $('세션%');
colSession.width = 60;
tableView.addTableColumn(colSession);

var colWeek = $.NSTableColumn.alloc.initWithIdentifier($('week'));
colWeek.headerCell.stringValue = $('주간%');
colWeek.width = 60;
tableView.addTableColumn(colWeek);

var colStatus = $.NSTableColumn.alloc.initWithIdentifier($('status'));
colStatus.headerCell.stringValue = $('상태');
colStatus.width = 120;
tableView.addTableColumn(colStatus);

var accountData = [];
function refreshAccountData() {
    accountData = readAccounts();
    var usage = readUsageCache();
    for (var i = 0; i < accountData.length; i++) {
        var a = accountData[i];
        var u = usage[a.name] || {};
        a._session = u.session_pct != null ? u.session_pct : '?';
        a._week = u.week_all_pct != null ? u.week_all_pct : '?';
        a._status = '';
        if (a.removed) a._status = '폐기됨';
        else if (a._week !== '?' && parseFloat(a._week) >= 100) a._status = '✗ EXHAUSTED';
        else if (a._week !== '?' && parseFloat(a._week) >= 80) a._status = '⚠ HIGH';
        else a._status = '✓ OK';
    }
}
refreshAccountData();

ObjC.registerSubclass({
    name: 'AccountTableDS',
    protocols: ['NSTableViewDataSource', 'NSTableViewDelegate'],
    methods: {
        'numberOfRowsInTableView:': {
            types: ['long', ['id']],
            implementation: function(tv) {
                return accountData.length;
            }
        },
        'tableView:objectValueForTableColumn:row:': {
            types: ['id', ['id', 'id', 'long']],
            implementation: function(tv, col, row) {
                if (row >= accountData.length) return $('');
                var a = accountData[row];
                var cid = col.identifier.js;
                if (cid === 'name') return $(a.name);
                if (cid === 'session') return $(String(a._session));
                if (cid === 'week') return $(String(a._week));
                if (cid === 'status') return $(a._status);
                return $('');
            }
        }
    }
});

var tableDS = $.AccountTableDS.alloc.init;
tableView.dataSource = tableDS;
tableView.delegate = tableDS;

scrollView.documentView = tableView;
accountsView.addSubview(scrollView);

// Buttons
ObjC.registerSubclass({
    name: 'AccountActions',
    methods: {
        'removeAccount:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                var row = tableView.selectedRow;
                if (row < 0 || row >= accountData.length) return;
                var a = accountData[row];
                if (a.removed) return;

                // 확인 다이얼로그
                var alert = $.NSAlert.alloc.init;
                alert.messageText = $('계정 폐기');
                alert.informativeText = $('\"' + a.name + '\" 계정을 폐기하시겠습니까?\n(복원 가능: accounts.json에서 removed를 false로 변경)');
                alert.addButtonWithTitle($('폐기'));
                alert.addButtonWithTitle($('취소'));
                alert.alertStyle = $.NSAlertStyleWarning;

                if (alert.runModal() === $.NSAlertFirstButtonReturn) {
                    accountData[row].removed = true;
                    writeAccounts(accountData);
                    refreshAccountData();
                    tableView.reloadData();
                }
            }
        },
        'restoreAccount:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                var row = tableView.selectedRow;
                if (row < 0 || row >= accountData.length) return;
                var a = accountData[row];
                if (!a.removed) return;

                accountData[row].removed = false;
                writeAccounts(accountData);
                refreshAccountData();
                tableView.reloadData();
            }
        },
        'refreshTable:': {
            types: ['void', ['id']],
            implementation: function(sender) {
                refreshAccountData();
                tableView.reloadData();
            }
        }
    }
});

var accActions = $.AccountActions.alloc.init;

var removeBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(20, 40, 100, 28));
removeBtn.title = $('폐기');
removeBtn.bezelStyle = $.NSBezelStyleRounded;
removeBtn.target = accActions;
removeBtn.action = 'removeAccount:';
accountsView.addSubview(removeBtn);

var restoreBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(130, 40, 100, 28));
restoreBtn.title = $('복원');
restoreBtn.bezelStyle = $.NSBezelStyleRounded;
restoreBtn.target = accActions;
restoreBtn.action = 'restoreAccount:';
accountsView.addSubview(restoreBtn);

var refreshBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(240, 40, 100, 28));
refreshBtn.title = $('↻ 새로고침');
refreshBtn.bezelStyle = $.NSBezelStyleRounded;
refreshBtn.target = accActions;
refreshBtn.action = 'refreshTable:';
accountsView.addSubview(refreshBtn);

// Hint
var hintLabel = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, 10, 360, 22));
hintLabel.stringValue = $('계정 선택 후 폐기/복원. /login은 자동 감지됩니다.');
hintLabel.bordered = false;
hintLabel.editable = false;
hintLabel.drawsBackground = false;
hintLabel.font = $.NSFont.systemFontOfSize(11);
hintLabel.textColor = $.NSColor.secondaryLabelColor;
accountsView.addSubview(hintLabel);

// Auto-refresh table every 10s
$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(10.0, true, function() {
    refreshAccountData();
    tableView.reloadData();
});

app.run;
