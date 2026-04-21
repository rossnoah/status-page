use axum::routing::get;
use axum::Router;

use crate::AppState;
use crate::views::public;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(public::index))
}
