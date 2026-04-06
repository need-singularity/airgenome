// airgenome settings panel — standalone JXA
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : '/tmp/airgenome-config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyRegular);

function readConfig() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(configPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30};
        return JSON.parse(str.js);
    } catch(e) { return {cpu_ceil: 75, ram_ceil: 70, swap_ceil: 30}; }
}

function writeConfig(c, r, s) {
    var json = '{"cpu_ceil":' + c + ',"ram_ceil":' + r + ',"swap_ceil":' + s + '}';
    $(json).writeToFileAtomicallyEncodingError($(configPath), true, $.NSUTF8StringEncoding, null);
}

var cfg = readConfig();

var win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(0, 0, 340, 370),
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
var sep1 = $.NSBox.alloc.initWithFrame($.NSMakeRect(20, 268, 300, 1));
sep1.boxType = $.NSBoxSeparator;
cv.addSubview(sep1);

var allAvg = Math.round((cfg.cpu_ceil + cfg.ram_ceil + cfg.swap_ceil) / 3);
var allRow = makeRow(280, 'All', 0, 100, allAvg);
allRow.label.font = $.NSFont.boldSystemFontOfSize(14);

// --- Individual sliders ---
var cpuRow  = makeRow(200, 'CPU Ceiling',  10, 100, cfg.cpu_ceil);
var ramRow  = makeRow(130, 'RAM Ceiling',  10, 100, cfg.ram_ceil);
var swapRow = makeRow(60,  'Swap Ceiling',  0, 100, cfg.swap_ceil);

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

    allRow.label.stringValue   = $('All: ' + av + '%');
    cpuRow.label.stringValue   = $(cpuRow.name + ': ' + cc + '%');
    ramRow.label.stringValue   = $(ramRow.name + ': ' + rc + '%');
    swapRow.label.stringValue  = $(swapRow.name + ': ' + sc + '%');

    writeConfig(cc, rc, sc);
});

app.run;
