//! Per-process random admin password for the integration-test crate.
//!
//! Eliminates hardcoded password literals from test fixtures (task #1037,
//! CodeQL finding on PR #15). The plaintext is generated once per test
//! process and never recurs across runs, so it is not a static credential
//! a scanner can flag.
//!
//! `password()` and `password_hash()` are a guaranteed-consistent pair:
//! the bcrypt hash is computed once inside the `LazyLock` body (bcrypt is
//! non-deterministic, so re-hashing per call would break the coupling that
//! the login-flow POST tests depend on).

use std::sync::LazyLock;

/// Random plaintext admin password, fixed for the lifetime of the test
/// process. A fresh UUIDv4 is unique per process and carries no literal a
/// credential scanner can match.
static PASSWORD: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().to_string());

/// bcrypt hash of [`PASSWORD`], computed exactly once. Paired with the
/// plaintext so a login POST of `password()` authenticates against an
/// `AppState` seeded with `password_hash()`.
static PASSWORD_HASH: LazyLock<String> =
    LazyLock::new(|| site_core::auth::hash_password(&PASSWORD).expect("test password must hash"));

/// The per-process plaintext admin password. Use in login-flow POST bodies.
pub fn password() -> &'static str {
    &PASSWORD
}

/// bcrypt hash of [`password`]. Use to fill `AppState.admin_password_hash`.
pub fn password_hash() -> String {
    PASSWORD_HASH.clone()
}
