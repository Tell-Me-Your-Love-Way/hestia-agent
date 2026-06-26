use std::time::SystemTime;

use crate::{error::AgentError, services};
use chrono::{DateTime, Utc};
use salvo::prelude::*;
use serde::Deserialize;
use tracing::info;
use tokio_util::io::ReaderStream;

#[derive(Deserialize)]
struct HandledRequest {
    path: String,
    time: String,
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

    info!("{:?}", DateTime::<Utc>::from(SystemTime::UNIX_EPOCH));
    let dt: DateTime<Utc> = DateTime::parse_from_rfc3339(&req_json.time)
        .map_err(|_| AgentError::Validation)?
        .with_timezone(&Utc);
    let system_time: SystemTime = dt.into();
    
    let cleanup_file = services::compress_target::execute_stream(&path, system_time).await?;
    
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
