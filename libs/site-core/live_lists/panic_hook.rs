//! A process panic hook that never prints a panic payload.
//!
//! std's default hook writes the payload to stderr, outside `tracing`. A
//! refresher that panics inside the source, the validator or the store could
//! carry text from a response body in that payload, and R-30 keeps response
//! bytes out of every log. This hook logs the location and the thread at
//! `error` and nothing else.

use std::panic::PanicHookInfo;

/// Replace the process panic hook with [`log_panic`]. `start` calls this in a
/// non-test build; a test build leaves libtest's hook so failures stay
/// readable, and the hook is tested directly.
pub fn install() {
    std::panic::set_hook(Box::new(log_panic));
}

/// The hook body: where and on which thread, never what.
pub fn log_panic(info: &PanicHookInfo<'_>) {
    let _ = info;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::live_lists::harness::LogCapture;

    /// The hook is process-global; tests that swap it take turns.
    static HOOK: Mutex<()> = Mutex::new(());

    /// Run `f` (which panics) with `log_panic` installed, restoring the
    /// previous hook after.
    fn panicking_with_the_hook_installed(f: impl FnOnce() + std::panic::UnwindSafe) {
        let _turn = HOOK.lock().unwrap_or_else(|p| p.into_inner());
        let previous = std::panic::take_hook();
        install();
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(previous);
        assert!(result.is_err(), "setup: the closure must have panicked");
    }

    #[test]
    fn a_panic_logs_its_location_and_thread_at_error() {
        let (log, _guard) = LogCapture::install();
        panicking_with_the_hook_installed(|| panic!("an ordinary payload"));
        let line = log.one_line("panic");
        assert!(line.contains("ERROR"), "{line}");
        assert!(line.contains("panic_hook.rs:"), "{line}");
        assert!(line.contains("thread="), "{line}");
    }

    #[test]
    fn a_panic_payload_never_reaches_the_log_whatever_its_type() {
        let sentinel = format!("sentinel-{}", uuid::Uuid::new_v4());
        let (log, _guard) = LogCapture::install();
        let owned = sentinel.clone();
        panicking_with_the_hook_installed(move || panic!("{owned}"));
        let owned = sentinel.clone();
        panicking_with_the_hook_installed(move || std::panic::panic_any(owned));
        assert!(!log.text().contains(&sentinel), "{}", log.text());
        // and the panics were logged, so the absence is not an empty capture
        assert_eq!(log.lines_containing("panic").len(), 2, "{}", log.text());
    }
}
