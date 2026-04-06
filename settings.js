// airgenome settings panel — standalone JXA
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : '/tmp/airgenome-config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyRegular);

var defaults = {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30, forge: false, guard: false};

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
    $.NSMakeRect(0, 0, 340, 470),
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

var forgeToggle = makeToggle(78, 'token-forge (10-account manager)', cfg.forge);
var guardToggle = makeToggle(48, 'resource-guard (CPU/RAM limiter)', cfg.guard);

win.makeKeyAndOrderFront(null);
app.activateIgnoringOtherApps(true);

var lastAllVal = allAvg;

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

    writeFullConfig({cpu_ceil: cc, ram_ceil: rc, swap_ceil: sc, forge: forgeOn, guard: guardOn});
});

app.run;
