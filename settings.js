// airgenome settings panel — standalone JXA
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : '/tmp/airgenome-config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

var defaults = {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30, forge: false, guard: false, autostart: true};
var plistPath = $.NSHomeDirectory().js + '/Library/LaunchAgents/com.airgenome.menubar.plist';

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
    $.NSMakeRect(0, 0, 340, 500),
    $.NSWindowStyleMaskTitled | $.NSWindowStyleMaskClosable,
    $.NSBackingStoreBuffered, false
);
win.title = $('airgenome Settings');
win.level = $.NSFloatingWindowLevel;
win.center;

var cv = win.contentView;

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
var sep1 = $.NSBox.alloc.initWithFrame($.NSMakeRect(20, 368, 300, 1));
sep1.boxType = $.NSBoxSeparator;
cv.addSubview(sep1);

var allAvg = Math.round((cfg.cpu_ceil + cfg.ram_ceil + cfg.swap_ceil) / 3);
var allRow = makeRow(380, 'All', 0, 100, allAvg);
allRow.label.font = $.NSFont.boldSystemFontOfSize(14);

// --- Individual sliders ---
var cpuRow  = makeRow(300, 'CPU Ceiling',  10, 100, cfg.cpu_ceil);
var ramRow  = makeRow(230, 'RAM Ceiling',  10, 100, cfg.ram_ceil);
var swapRow = makeRow(160, 'Swap Ceiling',  0, 100, cfg.swap_ceil);

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

    allRow.label.stringValue   = $('All: ' + av + '%');
    cpuRow.label.stringValue   = $(cpuRow.name + ': ' + cc + '%');
    ramRow.label.stringValue   = $(ramRow.name + ': ' + rc + '%');
    swapRow.label.stringValue  = $(swapRow.name + ': ' + sc + '%');

    var autoOn = autostartToggle.button.state === $.NSControlStateValueOn;
    setAutoStart(autoOn);

    writeFullConfig({cpu_ceil: cc, ram_ceil: rc, swap_ceil: sc, forge: forgeOn, guard: guardOn, autostart: autoOn});
});

app.run;
