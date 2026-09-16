//! Phase 0 observability (task #3558): log the raw TCP peer, `fly-client-ip`
//! and `cf-connecting-ip` as three separately labelled fields on every
//! request, before `origin_lock`'s accept/reject decision. This is a new
//! instrument, not a repair — no request-level IP field existed anywhere
//! in the crate before this.
//!
//! Each field is rendered by parsing the raw value as an IP address and
//! then re-serialising the truncated network prefix (`/24` for IPv4,
//! `/48` for IPv6) — never the raw string. Every one of these header
//! values is caller-supplied and unverified, so the log line never
//! carries the raw bytes: a value that fails to parse renders as the
//! literal `"invalid"` rather than being passed through. Parsing and
//! truncating a value *is* the sanitisation step here, so there is no
//! separate raw-text sanitizer to run first.
//!
//! Time-boxed on purpose: this is a short-lived instrument for answering
//! one question (does a direct caller's `Fly-Client-IP` survive to the
//! app, or does Fly's proxy overwrite it?), not a permanent addition.
//! `phase_0_logging_is_time_boxed` fails after 2026-10-15 so the decision
//! to keep or remove this logging can't be silently skipped.

use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

use crate::net::truncate_to_prefix;

const IPV4_LOG_PREFIX: u32 = 24;
const IPV6_LOG_PREFIX: u32 = 48;

/// Parse `value` as an IP address and render it truncated to the
/// data-minimising prefix width for its family, in CIDR notation
/// (e.g. `"192.0.2.0/24"`). Anything that doesn't parse — including an
/// absent header — renders as the literal `"invalid"`.
fn truncate_for_log(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "invalid".to_string();
    };
    match value.trim().parse::<IpAddr>() {
        Ok(addr @ IpAddr::V4(_)) => {
            format!(
                "{}/{IPV4_LOG_PREFIX}",
                truncate_to_prefix(addr, IPV4_LOG_PREFIX)
            )
        }
        Ok(addr @ IpAddr::V6(_)) => {
            format!(
                "{}/{IPV6_LOG_PREFIX}",
                truncate_to_prefix(addr, IPV6_LOG_PREFIX)
            )
        }
        Err(_) => "invalid".to_string(),
    }
}

/// Log the three untrusted/semi-trusted IP surfaces for this request,
/// each truncated and separately labelled. Called from `origin_lock`
/// before the accept/reject decision, so it observes traffic — including
/// a rejected request — that never reaches any other middleware.
pub fn log_client_ip_fields(headers: &HeaderMap, peer_addr: Option<SocketAddr>) {
    let peer = truncate_for_log(peer_addr.map(|addr| addr.ip().to_string()).as_deref());
    let fly_client_ip =
        truncate_for_log(headers.get("fly-client-ip").and_then(|v| v.to_str().ok()));
    let cf_connecting_ip = truncate_for_log(
        headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok()),
    );

    tracing::info!(
        peer = %peer,
        fly_client_ip = %fly_client_ip,
        cf_connecting_ip = %cf_connecting_ip,
        "client ip fields (task #3558 phase 0, truncated)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;

    // Capture harness mirrors `db::config`'s `LogBuf`/`capture_logs`: kept
    // as its own small copy in the UNIT test binary rather than shared or
    // moved to an integration test, because `db::config`'s own comment
    // records a measured ~40% flake capturing tracing output inside the
    // larger, higher-parallelism integration-test binary versus zero
    // flake here.
    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);

    impl LogBuf {
        fn captured(&self) -> String {
            let bytes = self.0.lock().expect("log buffer mutex poisoned").clone();
            String::from_utf8(bytes).expect("log output must be valid UTF-8")
        }
    }

    impl io::Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut inner = self
                .0
                .lock()
                .map_err(|_| io::Error::other("log buffer mutex poisoned"))?;
            inner.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogBuf {
        type Writer = LogBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture_logs(f: impl FnOnce()) -> String {
        let buf = LogBuf::default();
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_target(false)
            .without_time();
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        tracing::subscriber::with_default(subscriber, f);
        buf.captured()
    }

    /// C10 / ruling 3-4: the runbook verifies the sentinel probe by
    /// `grep`-ing the log line for a field NAME (`fly_client_ip=...`).
    /// That instruction is only trustworthy if the field names are
    /// actually what this test locks them to, and if the peer-address
    /// branch renders correctly rather than always falling through to
    /// `"invalid"`. Exercises all three fields with header values AND a
    /// real `SocketAddr` peer, not `None`.
    #[test]
    fn log_line_carries_all_three_field_names_with_truncated_values() {
        let mut headers = HeaderMap::new();
        headers.insert("fly-client-ip", "203.0.113.42".parse().unwrap());
        headers.insert("cf-connecting-ip", "2001:db8:1234:5678::9".parse().unwrap());
        let peer: SocketAddr = "198.51.100.7:443".parse().unwrap();

        let captured = capture_logs(|| log_client_ip_fields(&headers, Some(peer)));

        assert!(
            captured.contains("peer=198.51.100.0/24"),
            "expected a truncated peer field, captured: {captured}"
        );
        assert!(
            captured.contains("fly_client_ip=203.0.113.0/24"),
            "expected a truncated fly_client_ip field, captured: {captured}"
        );
        assert!(
            captured.contains("cf_connecting_ip=2001:db8:1234::/48"),
            "expected a truncated cf_connecting_ip field, captured: {captured}"
        );
    }

    /// The no-peer, no-headers case: all three fields render `"invalid"`,
    /// distinguishable from a value that failed to reach the log line at
    /// all (which would show no field name).
    #[test]
    fn log_line_carries_invalid_for_absent_peer_and_headers() {
        let headers = HeaderMap::new();

        let captured = capture_logs(|| log_client_ip_fields(&headers, None));

        assert!(captured.contains("peer=invalid"), "captured: {captured}");
        assert!(
            captured.contains("fly_client_ip=invalid"),
            "captured: {captured}"
        );
        assert!(
            captured.contains("cf_connecting_ip=invalid"),
            "captured: {captured}"
        );
    }

    #[test]
    fn ipv4_header_truncates_to_24() {
        assert_eq!(truncate_for_log(Some("192.0.2.55")), "192.0.2.0/24");
    }

    #[test]
    fn ipv6_header_truncates_to_48() {
        assert_eq!(
            truncate_for_log(Some("2001:db8:1234:5678::1")),
            "2001:db8:1234::/48"
        );
    }

    #[test]
    fn absent_value_is_invalid() {
        assert_eq!(truncate_for_log(None), "invalid");
    }

    #[test]
    fn unparseable_value_is_invalid_not_echoed() {
        // A value that doesn't parse as an IP — including one containing
        // control characters such as a newline — renders as the literal
        // "invalid" only, never passed through to the log line verbatim.
        assert_eq!(
            truncate_for_log(Some("not-an-ip\nsecond line of the header value")),
            "invalid"
        );
    }

    #[test]
    fn whitespace_padded_value_still_parses() {
        assert_eq!(truncate_for_log(Some("  192.0.2.55  ")), "192.0.2.0/24");
    }

    /// Time-box (task #3558, Peter's ruling): Phase 0 logging is a
    /// short-lived instrument, not a permanent addition. This fails after
    /// 2026-10-15T00:00:00Z (unix 1792022400) so the decision to keep or
    /// remove it can't be silently skipped — either delete
    /// `log_client_ip_fields` and its call site, or replace this test
    /// with a recorded decision to keep it.
    #[test]
    fn phase_0_logging_is_time_boxed() {
        const DEADLINE_UNIX_SECS: u64 = 1_792_022_400; // 2026-10-15T00:00:00Z
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must not be before the unix epoch")
            .as_secs();
        assert!(
            now < DEADLINE_UNIX_SECS,
            "task #3558 Phase 0 client-IP logging is time-boxed and must be \
             revisited now: either remove `log_client_ip_fields` and its \
             call site in origin_lock, or record a decision to keep it and \
             replace this test."
        );
    }
}
