// airgenome settings panel — 계정 관리 전용
// launched by: osascript -l JavaScript settings.js <configPath>
ObjC.import('Cocoa');
ObjC.import('Foundation');

var args = $.NSProcessInfo.processInfo.arguments;
var configPath = args.count > 4 ? args.objectAtIndex(4).js : $.NSHomeDirectory().js + '/.airgenome/config.json';

var app = $.NSApplication.sharedApplication;
app.setActivationPolicy($.NSApplicationActivationPolicyAccessory);

var accountsPath = $.NSHomeDirectory().js + '/.airgenome/accounts.json';
var usageCachePath = $.NSHomeDirectory().js + '/.airgenome/usage-cache.json';

function readConfig() {
    try {
        var str = $.NSString.stringWithContentsOfFileEncodingError($(configPath), $.NSUTF8StringEncoding, null);
        if (str.isNil()) return {};
        return JSON.parse(str.js);
    } catch(e) { return {}; }
}

var win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
    $.NSMakeRect(0, 0, 420, 500),
    $.NSWindowStyleMaskTitled | $.NSWindowStyleMaskClosable,
    $.NSBackingStoreBuffered, false
);
win.title = $('airgenome — 계정 관리');
win.level = $.NSFloatingWindowLevel;
win.center;

var cv = win.contentView;

// ═══════════════════════════════════════════════════════════════════════
//  계정 관리
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
var accTitle = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, 450, 360, 22));
accTitle.stringValue = $('Claude Code 계정 관리');
accTitle.bordered = false;
accTitle.editable = false;
accTitle.drawsBackground = false;
accTitle.font = $.NSFont.boldSystemFontOfSize(16);
cv.addSubview(accTitle);

// Scroll view with table
var scrollView = $.NSScrollView.alloc.initWithFrame($.NSMakeRect(20, 80, 380, 360));
scrollView.hasVerticalScroller = true;
scrollView.autohidesScrollers = true;
scrollView.borderType = $.NSBezelBorder;

var tableView = $.NSTableView.alloc.initWithFrame($.NSMakeRect(0, 0, 360, 360));
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
colStatus.width = 140;
tableView.addTableColumn(colStatus);

var accountData = [];
function refreshAccountData() {
    accountData = readAccounts();
    var usage = readUsageCache();
    for (var i = 0; i < accountData.length; i++) {
        var a = accountData[i];
        var u = usage[a.name] || {};
        a._session = u.session_pct != null ? u.session_pct : '-';
        a._week = u.week_all_pct != null ? u.week_all_pct : '-';
        a._status = '';
        if (a.removed) a._status = '폐기됨';
        else if (a._week !== '-' && parseFloat(a._week) >= 100) a._status = '\u2717 EXHAUSTED';
        else if (a._week !== '-' && parseFloat(a._week) >= 80) a._status = '\u26A0 HIGH';
        else if (a._week !== '-' && parseFloat(a._week) >= 50) a._status = '\u25B3 MID';
        else a._status = '\u2713 OK';
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
cv.addSubview(scrollView);

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
cv.addSubview(removeBtn);

var restoreBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(130, 40, 100, 28));
restoreBtn.title = $('복원');
restoreBtn.bezelStyle = $.NSBezelStyleRounded;
restoreBtn.target = accActions;
restoreBtn.action = 'restoreAccount:';
cv.addSubview(restoreBtn);

var refreshBtn = $.NSButton.alloc.initWithFrame($.NSMakeRect(240, 40, 100, 28));
refreshBtn.title = $('\u21BB 새로고침');
refreshBtn.bezelStyle = $.NSBezelStyleRounded;
refreshBtn.target = accActions;
refreshBtn.action = 'refreshTable:';
cv.addSubview(refreshBtn);

// Hint
var hintLabel = $.NSTextField.alloc.initWithFrame($.NSMakeRect(20, 10, 360, 22));
hintLabel.stringValue = $('계정 선택 후 폐기/복원. /login은 자동 감지됩니다.');
hintLabel.bordered = false;
hintLabel.editable = false;
hintLabel.drawsBackground = false;
hintLabel.font = $.NSFont.systemFontOfSize(11);
hintLabel.textColor = $.NSColor.secondaryLabelColor;
cv.addSubview(hintLabel);

win.makeKeyAndOrderFront(null);
app.activateIgnoringOtherApps(true);

// Auto-refresh table every 10s
$.NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(10.0, true, function() {
    refreshAccountData();
    tableView.reloadData();
});

app.run;
