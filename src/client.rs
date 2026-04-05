//! Helper socket client — dials the privileged daemon and exchanges JSON.
//!
//! Protocol is one request, one response, both terminated by newline.
//! The client parses the flat-JSON response into [`HelperResponse`].

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// Default socket path (matches airgenome-helper).
pub const DEFAULT_SOCKET_PATH: &str = "/var/run/airgenome.sock";

/// Parsed response from the helper daemon.
#[derive(Debug, Clone, PartialEq)]
pub enum HelperResponse {
    Ok { detail: String },
    Refused { reason: String },
    Error { message: String },
}

/// Why a helper dial failed at the transport layer.
#[derive(Debug)]
pub enum DialError {
    SocketNotFound(String),
    ConnectFailed(String),
    WriteFailed(String),
    ReadFailed(String),
    MalformedResponse(String),
}

/// Send a single request line to `path` and parse the response.
pub fn dial(path: &str, request_json: &str) -> Result<HelperResponse, DialError> {
    if !Path::new(path).exists() {
        return Err(DialError::SocketNotFound(path.to_string()));
    }
    let mut stream = UnixStream::connect(path)
        .map_err(|e| DialError::ConnectFailed(e.to_string()))?;
    writeln!(stream, "{}", request_json)
        .map_err(|e| DialError::WriteFailed(e.to_string()))?;
    stream.flush()
        .map_err(|e| DialError::WriteFailed(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)
        .map_err(|e| DialError::ReadFailed(e.to_string()))?;
    parse_response(&line)
}

fn parse_response(raw: &str) -> Result<HelperResponse, DialError> {
    let raw = raw.trim();
    let status = field(raw, "status")
        .ok_or_else(|| DialError::MalformedResponse(raw.to_string()))?;
    match status {
        "ok" => {
            let detail = field(raw, "detail").unwrap_or("").to_string();
            Ok(HelperResponse::Ok { detail })
        }
        "refused" => {
            let reason = field(raw, "reason").unwrap_or("").to_string();
            Ok(HelperResponse::Refused { reason })
        }
        "error" => {
            let message = field(raw, "message").unwrap_or("").to_string();
            Ok(HelperResponse::Error { message })
        }
        other => Err(DialError::MalformedResponse(
            format!("unknown status '{}': {}", other, raw))),
    }
}

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

/// Build a JSON request for common ops (caller does own escaping).
pub fn req_ping() -> String {
    r#"{"op":"ping"}"#.to_string()
}
pub fn req_sysctl_get(key: &str) -> String {
    format!(r#"{{"op":"sysctl_get","key":"{}"}}"#, escape(key))
}
pub fn req_sysctl_set(key: &str, value: &str) -> String {
    format!(r#"{{"op":"sysctl_set","key":"{}","value":"{}"}}"#,
        escape(key), escape(value))
}
pub fn req_purge() -> String {
    r#"{"op":"purge"}"#.to_string()
}
pub fn req_mdutil_off() -> String { r#"{"op":"mdutil_off"}"#.to_string() }
pub fn req_mdutil_on() -> String { r#"{"op":"mdutil_on"}"#.to_string() }
pub fn req_tmutil_disable() -> String { r#"{"op":"tmutil_disable"}"#.to_string() }
pub fn req_tmutil_enable() -> String { r#"{"op":"tmutil_enable"}"#.to_string() }

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ok_response() {
        let r = parse_response(r#"{"status":"ok","detail":"pong"}"#).unwrap();
        match r {
            HelperResponse::Ok { detail } => assert_eq!(detail, "pong"),
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn parse_refused_response() {
        let r = parse_response(r#"{"status":"refused","reason":"whitelist"}"#).unwrap();
        match r {
            HelperResponse::Refused { reason } => assert_eq!(reason, "whitelist"),
            _ => panic!("expected Refused"),
        }
    }

    #[test]
    fn parse_error_response() {
        let r = parse_response(r#"{"status":"error","message":"boom"}"#).unwrap();
        match r {
            HelperResponse::Error { message } => assert_eq!(message, "boom"),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn parse_malformed_rejects() {
        assert!(parse_response("").is_err());
        assert!(parse_response("{}").is_err());
        assert!(parse_response(r#"{"status":"???"}"#).is_err());
    }

    #[test]
    fn dial_reports_missing_socket() {
        let r = dial("/nonexistent/socket", r#"{"op":"ping"}"#);
        assert!(matches!(r, Err(DialError::SocketNotFound(_))));
    }

    #[test]
    fn req_builders_emit_valid_json() {
        assert!(req_ping().contains("\"ping\""));
        assert!(req_sysctl_get("vm.compressor_mode").contains("vm.compressor_mode"));
        let s = req_sysctl_set("vm.compressor_mode", "4");
        assert!(s.contains("vm.compressor_mode"));
        assert!(s.contains("\"4\""));
        assert!(req_purge().contains("\"purge\""));
    }

    #[test]
    fn escape_handles_quotes_and_backslashes() {
        assert_eq!(escape("a\"b"), "a\\\"b");
        assert_eq!(escape("a\\b"), "a\\\\b");
    }
}
