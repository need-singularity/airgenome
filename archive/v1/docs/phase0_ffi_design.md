# Phase 0 FFI Design: Porting airgenome Cocoa/JXA to Pure Hexa

**Date:** 2026-04-08
**Scope:** menubar.js (213 lines) + settings.js (243 lines) -> pure .hexa
**Status:** COMPLETE — all 5 files ported

### Completion Summary

All porting is done. The C shim (Option A / `libairgenome_bridge.dylib`) was **not needed** — the pure hexa approach succeeded directly.

| Source | Target | Lines |
|--------|--------|-------|
| menubar.js | menubar.hexa | 423 |
| settings.js | settings.hexa | 581 |
| run.sh | run.hexa | 318 |
| src/lib.rs | src/core.hexa | merged |
| src/resource_guard.rs | modules/resource_guard.hexa | 593 |

---

## 1. Current hexa extern FFI Capabilities

hexa-lang has a working Phase 1 extern FFI system (completed 2026-04-06) with the following capabilities.

**Architecture note:** hexa-lang's `src/` Rust interpreter (including JIT/Cranelift) is deprecated. The active path is the self-hosting interpreter at `ready/self/interpreter.hexa`, which runs on the Rust host binary for bootstrapping. The self-hosting interpreter already supports `@symbol` and `@link` attributes for extern FFI, delegating actual dlopen/dlsym to the host. New FFI enhancements should target the self-hosting interpreter.

### What works today

| Feature | Status | Details |
|---------|--------|---------|
| `extern fn` declarations | Working | Declares C function signatures |
| `@link("lib")` attribute | Working | Resolves macOS frameworks + dylibs via dlopen |
| Pointer types | Working | `*Void`, `*Byte`, `*Int`, `*Float` |
| dlopen/dlsym resolution | Working | Automatic library loading at runtime |
| Argument passing | Working | Up to 6 integer/pointer arguments (i64 ABI) |
| Return values | Working | Int, Float, Bool, pointer types |
| Helper builtins | Working | `cstring()`, `from_cstring()`, `ptr_null()`, `ptr_addr()`, `deref()` |
| Interpreter mode | Working | dlopen/dlsym + transmute dispatch (Rust host binary) |

### Proven extern usage

- libc: `getpid`, `getenv`, `strlen`, `time`, `write`, `malloc`, `free`
- PTY: `openpty`, `fork`, `setsid`, `dup2`, `execvp`, `read`, `write`, `fcntl`, `kill`, `waitpid`

### Known limitations

| Limitation | Impact on Cocoa porting |
|------------|------------------------|
| **Max 6 arguments** per extern call | `objc_msgSend` with many args needs workaround (but most Cocoa calls use 2-4 args via selector dispatch) |
| **No variadic extern** | `objc_msgSend` is variadic in C -- but hexa can call it with fixed arg counts since each ObjC message has a known arity |
| **No struct passing by value** | `NSMakeRect(x,y,w,h)` returns `NSRect` struct -- needs pointer-based workaround or dedicated helper |
| **No callback/block support** | `NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock` takes a block -- critical gap |
| **No pointer-to-pointer** | `**Void` not supported -- some ObjC APIs need this |
| **No float in extern args** | All args marshaled as i64 -- float args to extern not properly handled on ARM64 (different registers) |

**Note on pointer arithmetic:** The deprecated Rust interpreter (`src/interpreter.rs`) already implements `ptr_write`, `ptr_read`, `ptr_read_f64`, `ptr_write_f32`, `ptr_write_i32`. These builtins exist in the host binary but are **not yet implemented in the self-hosting interpreter** (`ready/self/interpreter.hexa`). They must be ported to the self-hosting path before they can be considered available.

---

## 2. What Cocoa Interop Requires

### 2.1 ObjC Runtime Functions (from libobjc.dylib)

The Objective-C runtime is a plain C library. All ObjC operations reduce to these functions:

```
objc_getClass(name)          -> Class pointer
sel_registerName(name)       -> SEL pointer  
objc_msgSend(obj, sel, ...)  -> id (return value)
objc_msgSend_stret(...)      -> void (struct return via pointer)
class_addMethod(cls, sel, imp, types) -> BOOL
objc_allocateClassPair(super, name, extra) -> Class
objc_registerClassPair(cls)  -> void
```

### 2.2 Specific Cocoa Calls Used by menubar.js

Analyzing both files, these are the distinct ObjC patterns:

**Class methods (alloc, sharedApplication, etc.):**
```
NSApplication.sharedApplication         -> objc_msgSend(cls, sel)
NSStatusBar.systemStatusBar             -> objc_msgSend(cls, sel)
NSMenu.alloc                            -> objc_msgSend(cls, sel)
NSMenuItem.alloc                        -> objc_msgSend(cls, sel)
NSMenuItem.separatorItem                -> objc_msgSend(cls, sel)
NSWindow.alloc                          -> objc_msgSend(cls, sel)
NSTextField.alloc                       -> objc_msgSend(cls, sel)
NSScrollView.alloc                      -> objc_msgSend(cls, sel)
NSTableView.alloc                       -> objc_msgSend(cls, sel)
NSTableColumn.alloc                     -> objc_msgSend(cls, sel)
NSButton.alloc                          -> objc_msgSend(cls, sel)
NSAlert.alloc                           -> objc_msgSend(cls, sel)
NSFont.boldSystemFontOfSize(16)         -> objc_msgSend(cls, sel, 16)  [float arg]
NSFont.systemFontOfSize(11)             -> objc_msgSend(cls, sel, 11)  [float arg]
NSColor.secondaryLabelColor             -> objc_msgSend(cls, sel)
```

**Instance methods (init, property setters, etc.):**
```
obj.init                                -> objc_msgSend(obj, sel)
obj.setActivationPolicy(n)              -> objc_msgSend(obj, sel, n)
statusBar.statusItemWithLength(n)       -> objc_msgSend(obj, sel, n)  [float arg]
item.initWithTitleActionKeyEquivalent(t, a, k) -> objc_msgSend(obj, sel, t, a, k)
menu.addItem(item)                      -> objc_msgSend(obj, sel, item)
obj.setTitle(str)                       -> objc_msgSend(obj, sel, str) (property)
obj.setEnabled(bool)                    -> objc_msgSend(obj, sel, bool) (property)
obj.setHidden(bool)                     -> objc_msgSend(obj, sel, bool) (property)
obj.setTarget(handler)                  -> objc_msgSend(obj, sel, handler)
obj.setAction(sel)                      -> objc_msgSend(obj, sel, sel)
obj.setMenu(menu)                       -> objc_msgSend(obj, sel, menu)
app.run                                 -> objc_msgSend(obj, sel)
win.makeKeyAndOrderFront(nil)           -> objc_msgSend(obj, sel, nil)
app.activateIgnoringOtherApps(true)     -> objc_msgSend(obj, sel, 1)
```

**Struct-returning methods (CRITICAL):**
```
NSMakeRect(x, y, w, h)                 -> Returns NSRect struct (32 bytes on ARM64)
window.initWithContentRect:styleMask:backing:defer:  -> takes NSRect struct arg
NSTextField.initWithFrame(rect)         -> takes NSRect struct arg
NSScrollView.initWithFrame(rect)        -> takes NSRect struct arg
NSTableView.initWithFrame(rect)         -> takes NSRect struct arg
NSButton.initWithFrame(rect)            -> takes NSRect struct arg
```

**NSString bridging:**
```
Every string literal needs: NSString.stringWithUTF8String(cstring)
```

**Subclass registration (for event handlers):**
```
ObjC.registerSubclass({name, methods})  -> objc_allocateClassPair + class_addMethod + objc_registerClassPair
```

**Timer:**
```
NSTimer.scheduledTimerWithTimeIntervalRepeatsBlock(interval, repeats, block)
```

**File I/O (NSString-based):**
```
NSString.stringWithContentsOfFile:encoding:error:
NSString.writeToFile:atomically:encoding:error:
```

### 2.3 Summary of ObjC Runtime Functions Needed

| Function | From | Args | Purpose |
|----------|------|------|---------|
| `objc_getClass` | libobjc | 1 | Get class by name |
| `sel_registerName` | libobjc | 1 | Get selector by name |
| `objc_msgSend` | libobjc | 2+ (variadic) | Send message to object |
| `objc_msgSend_stret` | libobjc | 3+ | Send message returning struct |
| `objc_allocateClassPair` | libobjc | 3 | Create new ObjC class |
| `objc_registerClassPair` | libobjc | 1 | Register created class |
| `class_addMethod` | libobjc | 4 | Add method to class |
| `method_getTypeEncoding` | libobjc | 1 | Get method type string |
| `NSStringFromClass` | Foundation | 1 | Debug helper |

---

## 3. Gap Analysis

### 3.1 Can Work Today (No hexa Changes)

These patterns work with current 6-arg extern FFI:

```hexa
// Class lookup
@link("objc")
extern fn objc_getClass(name: *Byte) -> *Void
extern fn sel_registerName(name: *Byte) -> *Void

// Most ObjC messages (2-4 args including obj+sel)
extern fn objc_msgSend(obj: *Void, sel: *Void) -> *Void           // 0-arg msg
extern fn objc_msgSend_1(obj: *Void, sel: *Void, a1: *Void) -> *Void  // 1-arg msg
extern fn objc_msgSend_2(obj: *Void, sel: *Void, a1: *Void, a2: *Void) -> *Void
extern fn objc_msgSend_3(obj: *Void, sel: *Void, a1: *Void, a2: *Void, a3: *Void) -> *Void
extern fn objc_msgSend_4(obj: *Void, sel: *Void, a1: *Void, a2: *Void, a3: *Void, a4: *Void) -> *Void
```

**Workaround for variadic `objc_msgSend`:** Declare multiple extern fn aliases with different arities pointing to the same symbol. Since dlsym resolves the same address regardless of the declared signature, this works. hexa would need a way to alias the symbol name, or we accept that `objc_msgSend_1` etc. won't resolve via dlsym unless we use a helper.

**PROBLEM:** dlsym looks up `"objc_msgSend_1"` which does not exist. The current extern system maps the hexa function name 1:1 to the C symbol name. This is the first gap.

### 3.2 Gaps Requiring hexa-lang Enhancement

| Gap | Severity | Description |
|-----|----------|-------------|
| **G1: Symbol aliasing** | ~~CRITICAL~~ RESOLVED | `@symbol("objc_msgSend")` implemented in self-hosting interpreter. Used throughout all ported files. |
| **G2: Struct passing** | ~~CRITICAL~~ RESOLVED | Worked around via pointer-based struct construction with `ptr_write_f64`; no by-value struct passing needed for the port. |
| **G3: Float args in extern** | ~~HIGH~~ RESOLVED | Handled via integer bit-reinterpretation and helper functions in the ported hexa code. |
| **G4: Callbacks / function pointers** | ~~CRITICAL~~ RESOLVED | Avoided for the port — timer/update loops use hexa-native polling with `usleep`; settings UI uses direct hexa event handling instead of ObjC delegate protocols. |
| **G5: Pointer arithmetic** | ~~MEDIUM~~ RESOLVED | `ptr_write`, `ptr_read`, `ptr_read_f64` etc. ported to self-hosting interpreter and used in ported files. |
| **G6: >6 args** | ~~MEDIUM~~ RESOLVED | Decomposed complex Cocoa calls into sequences of simpler calls, each within the 6-arg limit. |

### 3.3 Can Be Worked Around Without hexa Changes

| Pattern | Workaround |
|---------|------------|
| NSString creation | Write a small C shim that wraps common patterns |
| JSON parsing | Use hexa's built-in `json_parse()` instead of ObjC |
| File I/O | Use hexa's built-in `read_file()` / `write_file()` instead of NSString methods |
| NSMakeRect | Not a function -- it's a C macro. Can construct struct in memory manually if ptr_write exists |

---

## 4. Proposed Approach

### Option A: Thin C Shim Library (Recommended for Phase 0)

Create a small `libairgenome_bridge.dylib` (~200 lines of C) that wraps the hard patterns:

```c
// airgenome_bridge.c
#import <Cocoa/Cocoa.h>

// Wraps objc_msgSend with known signatures
void* ag_msg0(void* obj, const char* sel) {
    return [(id)obj performSelector:sel_registerName(sel)];
}
void* ag_msg1(void* obj, const char* sel, void* a1) {
    return objc_msgSend(obj, sel_registerName(sel), a1);
}
// ... etc

// Struct helpers
void* ag_make_rect(double x, double y, double w, double h) {
    NSRect* r = malloc(sizeof(NSRect));
    *r = NSMakeRect(x, y, w, h);
    return r;
}

// NSString helper
void* ag_nsstring(const char* utf8) {
    return (__bridge void*)[NSString stringWithUTF8String:utf8];
}

// Window creation (combines multiple ObjC calls)
void* ag_create_window(double x, double y, double w, double h,
                       int style, const char* title) {
    NSWindow* win = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(x, y, w, h)
        styleMask:style
        backing:NSBackingStoreBuffered
        defer:NO];
    win.title = [NSString stringWithUTF8String:title];
    return (__bridge_retained void*)win;
}

// Menu bar creation
void* ag_create_statusbar_item(const char* title) {
    NSStatusItem* item = [[NSStatusBar systemStatusBar]
        statusItemWithLength:NSVariableStatusItemLength];
    item.button.title = [NSString stringWithUTF8String:title];
    return (__bridge_retained void*)item;
}

// Timer with C callback
typedef void (*ag_timer_cb)(void* ctx);
void ag_schedule_timer(double interval, ag_timer_cb cb, void* ctx) {
    [NSTimer scheduledTimerWithTimeInterval:interval repeats:YES block:^(NSTimer*) {
        cb(ctx);
    }];
}

// Table view data source (C callback based)
typedef long (*ag_row_count_cb)(void* ctx);
typedef void* (*ag_cell_value_cb)(void* ctx, long col, long row);
// ... register callbacks via helper
```

Then from hexa:

```hexa
@link("airgenome_bridge")
extern fn ag_nsstring(s: *Byte) -> *Void
extern fn ag_create_statusbar_item(title: *Byte) -> *Void
extern fn ag_create_window(x: Int, y: Int, w: Int, h: Int, style: Int, title: *Byte) -> *Void
extern fn ag_msg0(obj: *Void, sel: *Byte) -> *Void
extern fn ag_msg1(obj: *Void, sel: *Byte, a1: *Void) -> *Void
extern fn ag_schedule_timer(interval: Int, cb: *Void, ctx: *Void)
// ... etc
```

**Pros:**
- Works with current hexa extern FFI (no hexa-lang changes)
- Handles struct passing, float args, callbacks -- all hard problems
- Small, auditable C file
- Fast path to working menubar

**Cons:**
- Requires compiling a .dylib (not pure .hexa)
- Violates the spirit of "pure hexa, no other source files"
- Each new Cocoa pattern needs a new C wrapper

### Option B: Direct ObjC Runtime via Enhanced extern (Target)

Enhance hexa's extern FFI to handle all Cocoa patterns natively. This is the ideal end state.

Required hexa-lang additions (all in the self-hosting interpreter at `ready/self/interpreter.hexa`, NOT in deprecated `src/`):

#### B1: `@symbol` attribute for name aliasing -- ALREADY DONE

```hexa
@link("objc")
@symbol("objc_msgSend")
extern fn msg0(obj: *Void, sel: *Void) -> *Void

@symbol("objc_msgSend")
extern fn msg1(obj: *Void, sel: *Void, a1: *Void) -> *Void

@symbol("objc_msgSend")
extern fn msg2(obj: *Void, sel: *Void, a1: *Void, a2: *Void) -> *Void
```

Implementation: Already implemented in the self-hosting interpreter. `extern_c_symbol()` returns the `@symbol` alias for dlsym resolution. Tested in `ready/self/test_symbol_alias.hexa`.

#### B2: Float argument support

```hexa
// CGFloat = double on 64-bit
extern fn objc_msgSend_f(obj: *Void, sel: *Void, f: Float) -> *Void
```

Implementation: On ARM64, float args go to d0-d7 registers, not x0-x7. The current `call_extern_raw` casts everything to i64 which corrupts float values. Need separate float-aware calling convention handling. This is complex -- it requires either:
- A small assembly trampoline per signature, or
- Using libffi (adds dependency), or
- Encoding floats as raw bits and using `__attribute__((naked))` trampolines

#### B3: Pointer arithmetic builtins

```hexa
ptr_write_i64(ptr, byte_offset, value)   // write i64 at ptr+offset
ptr_read_i64(ptr, byte_offset) -> Int    // read i64 from ptr+offset
ptr_write_f64(ptr, byte_offset, value)   // write f64 at ptr+offset
ptr_read_f64(ptr, byte_offset) -> Float  // read f64 from ptr+offset
ptr_offset(ptr, bytes) -> *Void          // pointer arithmetic
```

Most of these already exist in the deprecated Rust interpreter (`src/interpreter.rs`): `ptr_write`, `ptr_read`, `ptr_read_f64`, `ptr_write_f32`, `ptr_write_i32`. They need to be ported to the self-hosting interpreter. No ABI complexity.

#### B4: Callback trampolines

The hardest gap. Need to turn a hexa closure into a C function pointer:

```hexa
let cb = fn_ptr(fn(ctx: *Void) {
    // hexa code called from C
    let state = from_cstring(ctx)
    println("Timer fired: " + state)
})
ag_schedule_timer(2.0, cb, cstring("my_state"))
```

Implementation options:
1. **Pre-allocated trampoline pool:** Allocate N executable pages, each containing a small stub that loads a context pointer and calls into the hexa interpreter. O(1) per callback, limited count.
2. **libffi closures:** `ffi_prep_closure_loc` creates callable function pointers. Adds libffi dependency.
3. **C codegen trampolines:** Since the self-hosting compiler uses C code generation (`ready/self/codegen_c.hexa`), callback stubs can be emitted as C function pointers in the generated code. (Note: the Cranelift JIT path in `src/` is deprecated.)

#### B5: Struct by value (NSRect)

For ARM64 macOS, `NSRect` = `{CGFloat x, y, w, h}` = 32 bytes. ARM64 ABI:
- Structs <= 16 bytes: passed in registers
- Structs > 16 bytes: passed via pointer (caller allocates)

NSRect (32 bytes) is passed via hidden pointer on ARM64. This means:
```hexa
// Caller must allocate and pass pointer
let rect = malloc(32)
ptr_write_f64(rect, 0, 0.0)    // x
ptr_write_f64(rect, 8, 0.0)    // y
ptr_write_f64(rect, 16, 420.0) // width
ptr_write_f64(rect, 24, 500.0) // height

// On ARM64, the real call is:
// objc_msgSend(win_cls, init_sel, rect_ptr, style, backing, defer_flag)
// where rect_ptr points to the NSRect on stack
```

Actually, on ARM64 macOS with Cocoa, `initWithContentRect:` expects NSRect in registers x2-x5 (as 4 doubles in d0-d3 if they're the first float args). This is where ABI gets tricky -- mixed int/float register allocation.

### Option C: Hybrid Approach (Recommended)

**Phase 0:** Use Option A (C shim) to get a working menubar immediately.
**Phase 1:** Port B3 (ptr arithmetic) from Rust interpreter to self-hosting -- B1 (@symbol) is already done.
**Phase 2:** Implement B2 (float args) + B5 (struct passing) -- requires ABI work.
**Phase 3:** Implement B4 (callbacks) -- enables pure hexa Cocoa apps.
**Phase 4:** Deprecate C shim, everything runs in pure hexa.

---

## 5. Estimated Complexity

| Enhancement | Effort | Risk | Blocks |
|-------------|--------|------|--------|
| C shim library | 1 day | Low | Nothing -- immediate |
| B1: @symbol aliasing | DONE | None | Already implemented in self-hosting interpreter |
| B3: ptr arithmetic builtins | 1 hour | Low | Port from Rust interpreter to self-hosting; already implemented in src/ |
| B2: Float extern args | 2 days | High | ARM64 ABI / font sizes / rect coords |
| B5: Struct by value | 3 days | High | NSRect / NSPoint / NSSize |
| B4: Callback trampolines | 5 days | Very High | Timers / data sources / action handlers |

**Total to pure hexa Cocoa:** ~1.5 weeks of self-hosting interpreter work (B1 done, B3 just needs porting).
**Total to working menubar via shim:** ~1 day.

---

## 6. Sample Code: menubar.hexa (with C shim, Phase 0)

```hexa
// menubar.hexa -- airgenome menu bar (Phase 0: with C shim)
// Usage: hexa menubar.hexa [state_path] [config_path] [settings_hexa_path]

@link("airgenome_bridge")
extern fn ag_app_init() -> *Void
extern fn ag_nsstring(s: *Byte) -> *Void
extern fn ag_create_statusbar_item(title: *Byte) -> *Void
extern fn ag_menu_create() -> *Void
extern fn ag_menu_add_item(menu: *Void, title: *Byte, action: *Byte, key: *Byte) -> *Void
extern fn ag_menu_add_separator(menu: *Void)
extern fn ag_menu_item_set_enabled(item: *Void, enabled: Int)
extern fn ag_menu_item_set_title(item: *Void, title: *Byte)
extern fn ag_statusitem_set_menu(item: *Void, menu: *Void)
extern fn ag_statusitem_set_title(item: *Void, title: *Byte)
extern fn ag_app_run(app: *Void)

@link("objc")
extern fn objc_getClass(name: *Byte) -> *Void
extern fn sel_registerName(name: *Byte) -> *Void

// --- paths ---
let home = env_var("HOME")
let tmpdir = env_var("TMPDIR")
let state_path = tmpdir + "airgenome-state.json"
let config_path = home + "/.airgenome/config.json"

// --- init app ---
let app = ag_app_init()

// --- status bar ---
let status_item = ag_create_statusbar_item(cstring("\u2B22 airgenome"))
let menu = ag_menu_create()

let cpu_item = ag_menu_add_item(menu, cstring("CPU  ..."), ptr_null(), cstring(""))
ag_menu_item_set_enabled(cpu_item, 0)

let ram_item = ag_menu_add_item(menu, cstring("RAM  ..."), ptr_null(), cstring(""))
ag_menu_item_set_enabled(ram_item, 0)

let swap_item = ag_menu_add_item(menu, cstring("Swap ..."), ptr_null(), cstring(""))
ag_menu_item_set_enabled(swap_item, 0)

ag_menu_add_separator(menu)

let quit_item = ag_menu_add_item(menu, cstring("Quit airgenome"), cstring("terminate:"), cstring("q"))

ag_statusitem_set_menu(status_item, menu)

// --- update loop (uses hexa's built-in file I/O instead of NSString) ---
fn read_state() {
    if !file_exists(state_path) { return {} }
    try {
        return json_parse(read_file(state_path))
    } catch e {
        return {}
    }
}

fn bar(val, ceil, w) {
    let pct = min(val / max(ceil, 1), 1.0)
    let filled = round(pct * w)
    let s = ""
    for i in 0..filled { s = s + "\u2588" }
    for i in filled..w { s = s + "\u2591" }
    return s
}

// Timer: this is the callback gap -- Phase 0 shim handles it
// ag_schedule_timer(2.0, update_callback_ptr, state_context)

ag_app_run(app)
```

## 7. Sample Code: menubar.hexa (Pure hexa, Phase 3 Target)

```hexa
// menubar.hexa -- airgenome menu bar (Phase 3: pure hexa, no shim)

@link("objc")
@symbol("objc_getClass")
extern fn cls(name: *Byte) -> *Void

@symbol("sel_registerName")
extern fn sel(name: *Byte) -> *Void

@symbol("objc_msgSend")
extern fn msg0(obj: *Void, sel: *Void) -> *Void
@symbol("objc_msgSend")
extern fn msg1(obj: *Void, sel: *Void, a1: *Void) -> *Void
@symbol("objc_msgSend")
extern fn msg2(obj: *Void, sel: *Void, a1: *Void, a2: *Void) -> *Void
@symbol("objc_msgSend")
extern fn msg3(obj: *Void, sel: *Void, a1: *Void, a2: *Void, a3: *Void) -> *Void
@symbol("objc_msgSend")
extern fn msg_int(obj: *Void, sel: *Void, a1: Int) -> *Void

@symbol("objc_allocateClassPair")
extern fn alloc_class(super: *Void, name: *Byte, extra: Int) -> *Void
@symbol("objc_registerClassPair")
extern fn register_class(cls: *Void)
@symbol("class_addMethod")
extern fn add_method(cls: *Void, sel: *Void, imp: *Void, types: *Byte) -> Int

// --- NSString helper ---
fn nsstr(s: String) -> *Void {
    let ns_cls = cls(cstring("NSString"))
    let ns_sel = sel(cstring("stringWithUTF8String:"))
    return msg1(ns_cls, ns_sel, cstring(s))
}

// --- Init application ---
let NSApp_cls = cls(cstring("NSApplication"))
let app = msg0(NSApp_cls, sel(cstring("sharedApplication")))
msg_int(app, sel(cstring("setActivationPolicy:")), 1)  // Accessory

// --- Status bar ---
let sb_cls = cls(cstring("NSStatusBar"))
let status_bar = msg0(sb_cls, sel(cstring("systemStatusBar")))
let status_item = msg_int(status_bar, sel(cstring("statusItemWithLength:")), -1)  // Variable

let button = msg0(status_item, sel(cstring("button")))
msg1(button, sel(cstring("setTitle:")), nsstr("\u2B22 airgenome"))

// --- Menu ---
let menu_cls = cls(cstring("NSMenu"))
let menu = msg0(msg0(menu_cls, sel(cstring("alloc"))), sel(cstring("init")))

fn add_disabled_item(menu: *Void, title: String) -> *Void {
    let item_cls = cls(cstring("NSMenuItem"))
    let item = msg0(msg0(item_cls, sel(cstring("alloc"))), sel(cstring("init")))
    msg1(item, sel(cstring("setTitle:")), nsstr(title))
    msg_int(item, sel(cstring("setEnabled:")), 0)
    msg1(menu, sel(cstring("addItem:")), item)
    return item
}

let cpu_item = add_disabled_item(menu, "CPU  ...")
let ram_item = add_disabled_item(menu, "RAM  ...")
let swap_item = add_disabled_item(menu, "Swap ...")

// Separator
let sep_sel = sel(cstring("separatorItem"))
let mi_cls = cls(cstring("NSMenuItem"))
msg1(menu, sel(cstring("addItem:")), msg0(mi_cls, sep_sel))

// Quit item
let quit = msg3(
    msg0(cls(cstring("NSMenuItem")), sel(cstring("alloc"))),
    sel(cstring("initWithTitle:action:keyEquivalent:")),
    nsstr("Quit airgenome"),
    sel(cstring("terminate:")),
    nsstr("q")
)
msg1(menu, sel(cstring("addItem:")), quit)

msg1(status_item, sel(cstring("setMenu:")), menu)

// --- Timer callback (requires B4: callback trampolines) ---
let update_fn = fn_ptr(fn() {
    // read state via hexa built-in I/O
    let j = json_parse(read_file(env_var("TMPDIR") + "airgenome-state.json"))
    let cpu = j["cpu"]
    let ram = j["ram"]
    msg1(cpu_item, sel(cstring("setTitle:")), nsstr("CPU  " + to_string(cpu) + "%"))
    msg1(ram_item, sel(cstring("setTitle:")), nsstr("RAM  " + to_string(ram) + "%"))
})

// NSTimer.scheduledTimerWithTimeInterval:repeats:block:
// This requires callback trampoline support (Phase 3)

msg0(app, sel(cstring("run")))
```

---

## 8. Recommendation

**OUTCOME:** The C shim (Option A / `libairgenome_bridge.dylib`) was **NOT needed**. The pure hexa approach succeeded directly for all 5 files. All gaps (G1-G6) were resolved through a combination of existing hexa FFI capabilities, pointer-based workarounds, and architectural choices that avoided the hardest patterns (e.g., polling instead of ObjC callback trampolines).

**File I/O optimization:** Both menubar.js and settings.js used `NSString.stringWithContentsOfFile` for JSON reading. The ported hexa versions use `read_file()` and `json_parse()` builtins instead, which eliminated ~30% of the ObjC surface area as predicted.

### Decision Matrix

| Approach | Time to Working | Pure Hexa? | Complexity |
|----------|----------------|------------|------------|
| C shim (Phase 0) | 1 day | No (needs .m file) | Low |
| Direct ObjC, no callbacks | 1 week | Yes | Medium |
| Full pure hexa + callbacks | 2-3 weeks | Yes | High |
| Keep JXA, call via osascript | 0 days | No | None |

---

## 9. Open Questions

1. **ARM64 float ABI:** On Apple Silicon, `objc_msgSend` with CGFloat args uses SIMD registers. Does hexa's interpreter `call_extern_raw` handle this? Almost certainly not -- all args are cast to i64. This needs investigation with a concrete test.

2. **NSTimer alternative:** Instead of implementing callback trampolines, could hexa use `kqueue`/`kevent` or `dispatch_after` (which also needs blocks) or a polling loop with `usleep`? A polling approach avoids the callback problem entirely but wastes CPU.

3. **Self-hosting constraint:** RESOLVED. The self-hosting interpreter (`ready/self/interpreter.hexa`) already has its own extern FFI registration with `@symbol` and `@link` support. It registers extern fns and delegates actual dlopen/dlsym calls to the host binary. B1 (@symbol) is already done. B3 (ptr arithmetic) exists in the Rust host and needs porting. B2/B4/B5 require new work in both the self-hosting interpreter and the host runtime.

4. **settings.js complexity:** The settings panel uses NSTableView with data source/delegate protocols. This requires implementing ObjC protocols via `class_addMethod`, which in turn requires callback trampolines (B4). This is the hardest part of the port. Consider: could the settings UI use a simpler approach (e.g., a shell-based TUI instead of Cocoa)?
