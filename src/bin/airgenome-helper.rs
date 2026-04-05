//! airgenome-helper — privileged helper daemon (Tier 2 skeleton).
//!
//! Listens on a Unix domain socket for JSON Request messages from the
//! user-space airgenome client, then responds with JSON Response
//! messages.
//!
//! **This is the skeleton.** SysctlGet performs a real (read-only)
//! sysctl query. SysctlSet and Purge currently respond with a refusal
//! — actual privileged writes are deferred to a future stage once the
//! peer-authentication path is designed.
//!
//! # Install
//!
//! ```sh
//! sudo cp target/release/airgenome-helper /usr/local/libexec/
//! sudo launchctl bootstrap system /Library/LaunchDaemons/com.airgenome.helper.plist
//! ```
//!
//! The user-space airgenome client then connects to `/var/run/airgenome.sock`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::Command;

// FFI for macOS getpeereid(2).
extern "C" {
    fn getpeereid(s: i32, uid: *mut u32, gid: *mut u32) -> i32;
}

/// Get the connecting peer's UID from a Unix stream.
fn peer_uid(stream: &UnixStream) -> Option<u32> {
    let mut uid: u32 = 0;
    let mut gid: u32 = 0;
    let rc = unsafe { getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if rc == 0 { Some(uid) } else { None }
}

const SOCKET_PATH: &str = "/var/run/airgenome.sock";
const WHITELIST: &[&str] = &[
    "vm.compressor_mode",
    "vm.page_free_target",
    "kern.vm_swapsubdir",
    "kern.timer.longterm.qlen",
];

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| SOCKET_PATH.to_string());
    // Remove existing socket if present.
    if Path::new(&path).exists() {
        let _ = std::fs::remove_file(&path);
    }
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[airgenome-helper] cannot bind {}: {}", path, e);
            std::process::exit(1);
        }
    };
    eprintln!("[airgenome-helper] listening on {}", path);

    // Allowed UIDs: root (0) + AIRGENOME_ALLOW_UID env var (comma-separated).
    let mut allowed: Vec<u32> = vec![0];
    if let Ok(env) = std::env::var("AIRGENOME_ALLOW_UID") {
        for tok in env.split(',') {
            if let Ok(u) = tok.trim().parse::<u32>() { allowed.push(u); }
        }
    }
    eprintln!("[airgenome-helper] allowed UIDs: {:?}", allowed);

    for stream in listener.incoming() {
        let stream = match stream { Ok(s) => s, Err(_) => continue };
        let peer = peer_uid(&stream);
        let authed = peer.map(|u| allowed.contains(&u)).unwrap_or(false);
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() { continue; }
        let resp = if !authed {
            eprintln!("[airgenome-helper] refusing peer uid={:?}", peer);
            refused(&format!("peer not authenticated (uid={:?})", peer))
        } else {
            handle_request(&line)
        };
        let _ = writeln!(writer, "{}", resp);
        let _ = writer.flush();
    }
}

fn handle_request(raw: &str) -> String {
    // Tiny JSON-ish parser: we only care about top-level string fields.
    let raw = raw.trim();
    let op = field(raw, "op").unwrap_or("").to_string();
    match op.as_str() {
        "ping" => r#"{"status":"ok","detail":"pong"}"#.to_string(),
        "sysctl_get" => {
            let key = field(raw, "key").unwrap_or("");
            if !WHITELIST.contains(&key) {
                return refused("sysctl key not in whitelist");
            }
            match Command::new("sysctl").args(["-n", key]).output() {
                Ok(o) if o.status.success() => {
                    let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    ok_resp(&format!("{}={}", key, escape_json(&v)))
                }
                Ok(o) => error_resp(&format!("sysctl exit: {:?}", o.status.code())),
                Err(e) => error_resp(&format!("sysctl spawn: {}", e)),
            }
        }
        "sysctl_set" => refused("writes disabled in skeleton"),
        "purge" => refused("purge disabled in skeleton"),
        "" => error_resp("missing op"),
        other => refused(&format!("unknown op: {}", other)),
    }
}

/// Fish a `"key":"value"` or `"key":non-string` pair out of a flat JSON-ish
/// object. Crude but dependency-free.
fn field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let i = s.find(&needle)?;
    let rest = &s[i + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(&stripped[..end])
    } else {
        let end = after.find(|c: char| c == ',' || c == '}').unwrap_or(after.len());
        Some(after[..end].trim())
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn ok_resp(detail: &str) -> String {
    format!(r#"{{"status":"ok","detail":"{}"}}"#, escape_json(detail))
}
fn refused(reason: &str) -> String {
    format!(r#"{{"status":"refused","reason":"{}"}}"#, escape_json(reason))
}
fn error_resp(message: &str) -> String {
    format!(r#"{{"status":"error","message":"{}"}}"#, escape_json(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_ping() {
        let r = handle_request(r#"{"op":"ping"}"#);
        assert!(r.contains("\"ok\""));
        assert!(r.contains("pong"));
    }

    #[test]
    fn handles_missing_op() {
        let r = handle_request("{}");
        assert!(r.contains("\"error\""));
    }

    #[test]
    fn sysctl_get_refuses_non_whitelisted() {
        let r = handle_request(r#"{"op":"sysctl_get","key":"kern.hostname"}"#);
        assert!(r.contains("refused"));
        assert!(r.contains("whitelist"));
    }

    #[test]
    fn sysctl_set_always_refused() {
        let r = handle_request(r#"{"op":"sysctl_set","key":"vm.compressor_mode","value":"4"}"#);
        assert!(r.contains("refused"));
    }

    #[test]
    fn purge_always_refused() {
        let r = handle_request(r#"{"op":"purge"}"#);
        assert!(r.contains("refused"));
    }

    #[test]
    fn unknown_op_refused() {
        let r = handle_request(r#"{"op":"unknown_thing"}"#);
        assert!(r.contains("refused"));
        assert!(r.contains("unknown"));
    }

    #[test]
    fn field_parses_strings_and_bare_values() {
        let s = r#"{"op":"ping","key":"kern.hostname","count":42}"#;
        assert_eq!(field(s, "op"), Some("ping"));
        assert_eq!(field(s, "key"), Some("kern.hostname"));
        assert_eq!(field(s, "count"), Some("42"));
    }
}
