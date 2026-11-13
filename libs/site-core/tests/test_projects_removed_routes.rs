//! R-15 backend route tests (#4726): no verb on a removed path returns
//! `application/json`. Plan T6, anchor D8.
//!
//! held: G1 (level 3, held for Peter). The assertion on the response type
//! cannot be written to the criterion as locked, so no R-15 route assertion
//! is written here. The paths the tests will cover once G1 is amended:
//! `/api/projects`, `/api/projects/{slug}` and every `/api/admin/projects`
//! path, each for GET, POST, PUT and DELETE, the admin paths under an
//! authenticated session (`common::admin_login`), through the production
//! router (`common::build_app_server`). Nothing under `migrations/` changes.
//!
//! This file is deliberately empty of tests until then.
