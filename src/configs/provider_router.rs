use salvo::prelude::*;

use crate::handlers;

pub fn build() -> Router {
    Router::new().push(
        Router::with_path("/api")
        //.hoop(affix_state::inject(config))
        .hoop(max_concurrency(1))
        .get(handlers::health_check::handle)
        .post(handlers::compress_target::handle)
    )
}