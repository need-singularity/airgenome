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
        "sysctl_set" => {
            let key = field(raw, "key").unwrap_or("");
            let value = field(raw, "value").unwrap_or("");
            if !WHITELIST.contains(&key) {
                return refused("sysctl key not in whitelist");
            }
            if value.is_empty() {
                return error_resp("missing value");
            }
            // Sanity: value must parse as integer (all whitelisted keys are int).
            if value.parse::<i64>().is_err() {
                return refused("value must be integer");
            }
            eprintln!("[airgenome-helper] sysctl -w {}={}", key, value);
            match Command::new("sysctl").args(["-w", &format!("{}={}", key, value)]).output() {
                Ok(o) if o.status.success() => {
                    let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    ok_resp(&format!("wrote {}={} (out={})", key, value, escape_json(&stdout)))
                }
                Ok(o) => error_resp(&format!("sysctl exit: {:?}, err={}",
                    o.status.code(),
                    escape_json(&String::from_utf8_lossy(&o.stderr)))),
                Err(e) => error_resp(&format!("sysctl spawn: {}", e)),
            }
        }
        "purge" => {
            eprintln!("[airgenome-helper] /usr/sbin/purge");
            match Command::new("/usr/sbin/purge").output() {
                Ok(o) if o.status.success() => ok_resp("purge completed"),
                Ok(o) => error_resp(&format!("purge exit: {:?}", o.status.code())),
                Err(e) => error_resp(&format!("purge spawn: {}", e)),
            }
        }
        "mdutil_off" => {
            eprintln!("[airgenome-helper] mdutil -i off /");
            match Command::new("/usr/bin/mdutil").args(["-i", "off", "/"]).output() {
                Ok(o) if o.status.success() => ok_resp("spotlight indexing disabled on /"),
                Ok(o) => error_resp(&format!("mdutil exit: {:?}, stderr={}",
                    o.status.code(),
                    escape_json(&String::from_utf8_lossy(&o.stderr)))),
                Err(e) => error_resp(&format!("mdutil spawn: {}", e)),
            }
        }
        "mdutil_on" => {
            eprintln!("[airgenome-helper] mdutil -i on /");
            match Command::new("/usr/bin/mdutil").args(["-i", "on", "/"]).output() {
                Ok(o) if o.status.success() => ok_resp("spotlight indexing re-enabled on /"),
                Ok(o) => error_resp(&format!("mdutil exit: {:?}", o.status.code())),
                Err(e) => error_resp(&format!("mdutil spawn: {}", e)),
            }
        }
        "tmutil_disable" => {
            eprintln!("[airgenome-helper] tmutil disable");
            match Command::new("/usr/bin/tmutil").arg("disable").output() {
                Ok(o) if o.status.success() => ok_resp("time machine disabled"),
                Ok(o) => error_resp(&format!("tmutil exit: {:?}", o.status.code())),
                Err(e) => error_resp(&format!("tmutil spawn: {}", e)),
            }
        }
        "tmutil_enable" => {
            eprintln!("[airgenome-helper] tmutil enable");
            match Command::new("/usr/bin/tmutil").arg("enable").output() {
                Ok(o) if o.status.success() => ok_resp("time machine enabled"),
                Ok(o) => error_resp(&format!("tmutil exit: {:?}", o.status.code())),
                Err(e) => error_resp(&format!("tmutil spawn: {}", e)),
            }
        }
        "mdutil_status" => {
            match Command::new("/usr/bin/mdutil").args(["-s", "/"]).output() {
                Ok(o) => {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    ok_resp(&escape_json(&s))
                }
                Err(e) => error_resp(&format!("mdutil: {}", e)),
            }
        }
        "tmutil_status" => {
            match Command::new("/usr/bin/tmutil").arg("status").output() {
                Ok(o) => {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    ok_resp(&escape_json(&s))
                }
                Err(e) => error_resp(&format!("tmutil: {}", e)),
            }
        }
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
    fn sysctl_set_refuses_non_whitelisted() {
        let r = handle_request(r#"{"op":"sysctl_set","key":"kern.hostname","value":"x"}"#);
        assert!(r.contains("refused"));
        assert!(r.contains("whitelist"));
    }

    #[test]
    fn sysctl_set_refuses_non_integer_value() {
        let r = handle_request(r#"{"op":"sysctl_set","key":"vm.compressor_mode","value":"not-a-number"}"#);
        assert!(r.contains("refused"));
        assert!(r.contains("integer"));
    }

    #[test]
    fn sysctl_set_errors_on_missing_value() {
        let r = handle_request(r#"{"op":"sysctl_set","key":"vm.compressor_mode"}"#);
        assert!(r.contains("error"));
        assert!(r.contains("missing value"));
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
