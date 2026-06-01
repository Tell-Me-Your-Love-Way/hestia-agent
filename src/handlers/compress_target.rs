use std::time::SystemTime;

use crate::{error::AgentError, services};
use salvo::prelude::*;
use serde::Deserialize;
use tracing::info;
use tokio_util::io::ReaderStream;

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
    info!("Trying to backup path: {:?}", path);
    
    let download_filename = format!("backup-{}.tar.zst", path.replace("/", "_"))
        .replace("\\", "_")
        .replace(":", "-Disk_");
    
    let cleanup_file = services::compress_target::execute_stream(&path, SystemTime::UNIX_EPOCH).await?;
    
    res.headers_mut().insert(
        "content-type",
        "application/octet-stream".parse().unwrap()
    );
    res.headers_mut().insert(
        "content-disposition",
        format!("attachment; filename=\"{}\"", download_filename).parse().unwrap()
    );
    
    let stream = ReaderStream::new(cleanup_file);
    res.stream(stream);

    Ok(())
}
