use async_compression::Level;
use tokio_tar::Builder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::io::{AsyncRead, ReadBuf};
use tracing::{error, info};
use walkdir::WalkDir;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio::io::AsyncWriteExt;
use async_compression::tokio::write::ZstdEncoder as Encoder;

use crate::error::AgentError;


async fn execute_async(
    dir_origem: &str,
    arquivo_saida: &str,
    data_ultimo_backup: SystemTime,
) -> Result<(), AgentError> {
    info!(
        "Coletando arquivos modificados após {:?}",
        data_ultimo_backup
    );

    let arquivos_para_adicionar: Vec<(PathBuf, String, bool)> = WalkDir::new(dir_origem)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            let path = entry.path();
            if path == Path::new(dir_origem) {
                return false;
            }

            if let Ok(metadata) = entry.metadata() {
                if let Ok(modificado) = metadata.modified() {
                    return modificado > data_ultimo_backup;
                }
            }
            false
        })
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|entry| {
            let path = entry.path();
            let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);

            let name_in_archive = match path.strip_prefix(dir_origem) {
                Ok(name) => name.to_string_lossy().replace('\\', "/"),
                Err(e) => {
                    error!("Error stripping prefix of path {:?}: {:?}", path, e);
                    return None;
                }
            };

            Some((path.to_path_buf(), name_in_archive, is_dir))
        })
        .collect::<Vec<(PathBuf, String, bool)>>();

    if arquivos_para_adicionar.is_empty() {
        info!("Nenhum arquivo modificado encontrado para backup");
        return Ok(());
    }

    info!(
        "Encontrados {} arquivos/diretórios para adicionar",
        arquivos_para_adicionar.len()
    );
    let file = File::create(arquivo_saida)
        .await
        .map_err(|e| {
            error!("Error creating backup file {}: {:?}", arquivo_saida, e);
            AgentError::Encode
        })?;

    let file_writer = BufWriter::new(file);
    let encoder = Encoder::with_quality(file_writer, Level::Precise(3));
    let mut archive = Builder::new(encoder);
    for (path, name_in_archive, is_dir) in arquivos_para_adicionar {
        info!("Adicionando ao TAR: {}", name_in_archive);

        if is_dir {
            archive
                .append_dir(&name_in_archive, &path)
                .await
                .map_err(|e| {
                    error!("Error appending directory {:?} to tar: {:?}", path, e);
                    AgentError::Encode
                })?;
        } else {
            let mut file = File::open(&path)
                .await
                .map_err(|e| {
                    error!("Error opening file {:?}: {:?}", path, e);
                    AgentError::Encode
                })?;

            archive
                .append_file(&name_in_archive, &mut file)
                .await
                .map_err(|e| {
                    error!("Error appending file {:?} to tar: {:?}", path, e);
                    AgentError::Encode
                })?;
        }
    }
    let mut encoder = archive
        .into_inner()
        .await
        .map_err(|e| {
            error!("Error finishing Tar archive: {:?}", e);
            AgentError::Encode
        })?;
    encoder.shutdown().await.map_err(|e| {
        error!("Error finishing Zstd compression: {:?}", e);
        AgentError::Encode
    })?;
    let mut writer = encoder.into_inner();
    writer.flush().await.map_err(|e| {
        error!("Error flushing BufWriter to disk: {:?}", e);
        AgentError::Encode
    })?;
    info!("Backup concluído com sucesso: {}", arquivo_saida);
    Ok(())
}
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
            error!(
                "Error deleting temporary backup file {}: {:?}",
                path.display(),
                e
            );
        } else {
            info!("Deleted temporary backup file: {:?}", path.display());
        }
    }
}

pub async fn execute_stream(
    dir_origem: &str,
    data_ultimo_backup: SystemTime,
) -> Result<CleanupFile, AgentError> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let arquivo_saida = format!(
        "temp-backup-{}-{}.tar.zst",
        dir_origem
            .replace("/", "_")
            .replace("\\", "_")
            .replace(":", "-Disk_"),
        timestamp
    );

    let dir_origem_clone = dir_origem.to_string();
    let arquivo_saida_clone = arquivo_saida.clone();
    execute_async(&dir_origem_clone, &arquivo_saida_clone, data_ultimo_backup).await?;
    let file = tokio::fs::File::open(&arquivo_saida).await.map_err(|e| {
        error!(
            "Error opening temporary backup file for streaming {}: {:?}",
            arquivo_saida, e
        );
        AgentError::Encode
    })?;

    Ok(CleanupFile::new(file, PathBuf::from(arquivo_saida)))
}
