// airgenome settings panel — standalone JXA
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : '/tmp/airgenome-config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyRegular);

// read current config
function readConfig() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(configPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return {cpu_ceil: 90, ram_ceil: 80, swap_ceil: 50};
        return JSON.parse(str.js);
    } catch(e) { return {cpu_ceil: 90, ram_ceil: 80, swap_ceil: 50}; }
}

function writeConfig(c, r, s) {
    var json = '{"cpu_ceil":' + c + ',"ram_ceil":' + r + ',"swap_ceil":' + s + '}';
    $(json).writeToFileAtomicallyEncodingError($(configPath), true, $.NSUTF8StringEncoding, null);
}

var cfg = readConfig();

// window
var win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(0, 0, 340, 280),
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
    cv.addSubview(sl);

    return {label: lbl, slider: sl, name: name};
}

var cpuRow  = makeRow(190, 'CPU Ceiling',  10, 100, cfg.cpu_ceil);
var ramRow  = makeRow(120, 'RAM Ceiling',  10, 100, cfg.ram_ceil);
var swapRow = makeRow(50,  'Swap Ceiling',  0, 100, cfg.swap_ceil);

win.makeKeyAndOrderFront(null);
app.activateIgnoringOtherApps(true);

// poll sliders and save
$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(0.5, true, function() {
    var cc = Math.round(cpuRow.slider.doubleValue);
    var rc = Math.round(ramRow.slider.doubleValue);
    var sc = Math.round(swapRow.slider.doubleValue);

    cpuRow.label.stringValue  = $(cpuRow.name + ': ' + cc + '%');
    ramRow.label.stringValue  = $(ramRow.name + ': ' + rc + '%');
    swapRow.label.stringValue = $(swapRow.name + ': ' + sc + '%');

    writeConfig(cc, rc, sc);
});

app.run;
