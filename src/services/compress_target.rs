use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tar::Builder;
use tracing::{error, info};
use walkdir::WalkDir;
use zstd::stream::Encoder;
use tokio::io::{AsyncRead, ReadBuf};

use crate::error::AgentError;

pub struct CleanupFile {
    file: tokio::fs::File,
    path: PathBuf,
}

impl CleanupFile {
    pub fn new(file: tokio::fs::File, path: PathBuf) -> Self {
        Self { file, path }
    }
}

impl AsyncRead for CleanupFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.file).poll_read(cx, buf)
    }
}

impl Drop for CleanupFile {
    fn drop(&mut self) {
        let path = self.path.clone();
        if let Err(e) = std::fs::remove_file(&path) {
            error!("Error deleting temporary backup file {}: {:?}", path.display(), e);
        } else {
            info!("Deleted temporary backup file: {:?}", path.display());
        }
    }
}

fn execute_sync(dir_origem: &str, arquivo_saida: &str, data_ultimo_backup: SystemTime) -> Result<(), AgentError> {
    
    let file = File::create(arquivo_saida).map_err(|e| {
        error!("Error creating temporary backup file {}: {:?}", arquivo_saida, e);
        AgentError::Encode
    })?;
    let file_writer = BufWriter::new(file);
    
    let encoder = Encoder::new(file_writer, 3).map_err(|e| {
        error!("Error while initializing Zstd encoder: {:?}", e);
        AgentError::Encode
    })?;
    
    let mut archive = Builder::new(encoder);
    
    for entry in WalkDir::new(dir_origem).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        
        if path == Path::new(dir_origem) {
            continue;
        }
        
        if let Ok(modificado) = metadata.modified() {
            if modificado > data_ultimo_backup {
                info!("Adicionando: {:?}", path);
                
                let name_in_archive = path.strip_prefix(dir_origem).map_err(|e| {
                    error!("Error stripping prefix of path {:?}: {:?}", path, e);
                    AgentError::Encode
                })?;
                
                let name_in_archive_normalized = name_in_archive.to_string_lossy().replace('\\', "/");
                
                if metadata.is_dir() {
                    archive.append_dir(&name_in_archive_normalized, path).map_err(|e| {
                        error!("Error appending directory {:?} to tar: {:?}", path, e);
                        AgentError::Encode
                    })?;
                } else {
                    let mut f = File::open(path).map_err(|e| {
                        error!("Error opening file {:?}: {:?}", path, e);
                        AgentError::Encode
                    })?;
                    archive.append_file(&name_in_archive_normalized, &mut f).map_err(|e| {
                        error!("Error appending file {:?} to tar: {:?}", path, e);
                        AgentError::Encode
                    })?;
                }
            }
        }
    }
    
    let encoder = archive.into_inner().map_err(|e| {
        error!("Error finishing Tar archive: {:?}", e);
        AgentError::Encode
    })?;
    
    let mut writer = encoder.finish().map_err(|e| {
        error!("Error finishing Zstd compression: {:?}", e);
        AgentError::Encode
    })?;
    
    writer.flush().map_err(|e| {
        error!("Error flushing BufWriter to disk: {:?}", e);
        AgentError::Encode
    })?;

    Ok(())
}

pub async fn execute_stream(dir_origem: &str, data_ultimo_backup: SystemTime) -> Result<CleanupFile, AgentError> {
    
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let arquivo_saida = format!(
        "temp-backup-{}-{}.tar.zst", 
        dir_origem.replace("/", "_").replace("\\", "_").replace(":", "-Disk_"), 
        timestamp
    );

    let dir_origem_clone = dir_origem.to_string();
    let arquivo_saida_clone = arquivo_saida.clone();
    
    tokio::task::spawn_blocking(move || {
        execute_sync(&dir_origem_clone, &arquivo_saida_clone, data_ultimo_backup)
    })
    .await
    .map_err(|e| {
        error!("Blocking task panicked: {:?}", e);
        AgentError::Encode
    })??;
    
    let file = tokio::fs::File::open(&arquivo_saida).await.map_err(|e| {
        error!("Error opening temporary backup file for streaming {}: {:?}", arquivo_saida, e);
        AgentError::Encode
    })?;
    
    Ok(CleanupFile::new(file, PathBuf::from(arquivo_saida)))
}
