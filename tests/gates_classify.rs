use airgenome::gates::{classify, GateId};

#[test]
fn finder_classified_as_finder() {
    assert_eq!(classify("/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder"),
               GateId::Finder);
}

#[test]
fn telegram_classified() {
    assert_eq!(classify("/Applications/Telegram.app/Contents/MacOS/Telegram"),
               GateId::Telegram);
}

#[test]
fn chrome_and_helpers_classified() {
    assert_eq!(classify("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
               GateId::Chrome);
    assert_eq!(classify("Google Chrome Helper (Renderer)"), GateId::Chrome);
}

#[test]
fn safari_classified() {
    assert_eq!(classify("/Applications/Safari.app/Contents/MacOS/Safari"),
               GateId::Safari);
}

#[test]
fn system_daemons_classified_as_macos() {
    assert_eq!(classify("/sbin/launchd"), GateId::Macos);
    assert_eq!(classify("/System/Library/.../WindowServer"), GateId::Macos);
    assert_eq!(classify("kernel_task"), GateId::Macos);
    assert_eq!(classify("mdworker_shared"), GateId::Macos);
}

#[test]
fn unrelated_process_returns_none() {
    assert_eq!(classify("/Applications/Notion.app/.../Notion"), GateId::None);
    assert_eq!(classify("python3.11"), GateId::None);
}

#[test]
fn gate_id_roundtrip() {
    for g in GateId::ALL {
        assert_eq!(GateId::from_name(g.name()), Some(g));
    }
    assert_eq!(GateId::from_name("bogus"), None);
}
