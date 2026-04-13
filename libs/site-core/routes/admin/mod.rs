pub mod agents;
pub mod articles;
pub mod dashboard;
pub mod education;
pub mod experience;
pub mod faq;
pub mod gaps;
pub mod instructions;
pub mod links;
pub mod profile;
pub mod projects;
pub mod skills;
pub mod values;

use crate::auth;
use crate::state::DbState;
use axum::{Router, middleware, routing::post};

pub fn admin_router(state: DbState) -> Router<DbState> {
    let protected = Router::new()
        .merge(dashboard::routes())
        .merge(profile::routes())
        .merge(articles::routes())
        .merge(education::routes())
        .merge(experience::routes())
        .merge(faq::routes())
        .merge(gaps::routes())
        .merge(instructions::routes())
        .merge(links::routes())
        .merge(projects::routes())
        .merge(skills::routes())
        .merge(values::routes())
        .merge(agents::routes())
        .layer(middleware::from_fn_with_state(state, auth::require_auth));

    Router::new()
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .merge(protected)
}
