use std::time::SystemTime;

use crate::{error::AgentError, services};
use salvo::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct HandledRequest {
    path: String,
}
#[handler]
pub async fn handle(req: &mut Request, res: &mut Response) -> Result<(), AgentError> {
    let req_json: HandledRequest = req.parse_json::<HandledRequest>()
        .await
        .map_err(|_| AgentError::Validation)?;
    let path = req_json.path;
    let output_path = format!("backup-{}.tar.zst", path.replace("/", "_"));
    tokio::task::spawn_blocking(move || {
        services::compress_target::execute(&path, &output_path, SystemTime::now())
    })
    .await
    .map_err(|_| AgentError::Encode)??;
    res.render(StatusCode::OK);
    Ok(())
}
