use salvo::prelude::*;
use tracing::info;
#[handler]
pub async fn handle(res: &mut Response) {
    info!("Healthy");
    res.render(StatusCode::OK);
}