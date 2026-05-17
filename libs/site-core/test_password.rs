//! Per-process random admin password for in-crate `#[cfg(test)]` modules.
//!
//! The integration-test crate has its own copy under
//! `tests/common/test_password.rs`. This sibling exists because in-crate
//! `#[cfg(test)]` modules (e.g. `routes/ai.rs`'s `m1_no_key_path_tests`)
//! compile in a separate unit from `tests/**` and cannot import from
//! `tests/common/`. The duplication is across a compilation boundary, not
//! a DRY violation worth bridging (task #1037).

use std::sync::LazyLock;

static PASSWORD: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().to_string());

static PASSWORD_HASH: LazyLock<String> =
    LazyLock::new(|| crate::auth::hash_password(&PASSWORD).expect("test password must hash"));

/// bcrypt hash of the per-process random password. In-crate test modules
/// only need *a* non-literal hash to fill `AppState.admin_password_hash`;
/// none of them POST a login, so the plaintext is not re-exported.
pub(crate) fn password_hash() -> String {
    PASSWORD_HASH.clone()
}
