use std::fs::File;
use std::path::Path;
use std::time::SystemTime;
use tar::Builder;
use walkdir::WalkDir;
use zstd::stream::Encoder;

use crate::error::AgentError;

pub fn execute(dir_origem: &str, arquivo_saida: &str, data_ultimo_backup: SystemTime) -> Result<(), AgentError> {
    // 1. Cria o arquivo de saída (.tar.zst)
    let file = File::create(arquivo_saida).map_err(|_| AgentError::Encode)?;
    
    // 2. Cria o compressor Zstd (Sem o auto_finish na mesma linha)
    let encoder = Encoder::new(file, 3).map_err(|_| AgentError::Encode)?;
    
    // 3. Cria o arquivador Tar em cima do compressor Zstd diretamente
    let mut archive = Builder::new(encoder);

    // 4. Varre o diretório de origem
    for entry in WalkDir::new(dir_origem).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Evita travar o loop inteiro se um arquivo sumir no meio do caminho
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => continue, // Pula o arquivo se não puder ler os metadados
        };

        // Ignora o próprio diretório raiz na varredura direta
        if path == Path::new(dir_origem) {
            continue;
        }

        // 5. Verifica se o arquivo foi modificado após o último backup
        if let Ok(modificado) = metadata.modified() {
            if modificado > data_ultimo_backup {
                println!("Adicionando: {:?}", path);
                
                let name_in_archive = path.strip_prefix(dir_origem).map_err(|_| AgentError::Encode)?;
                
                if metadata.is_dir() {
                    archive.append_dir(name_in_archive, path).map_err(|_| AgentError::Encode)?;
                } else {
                    let mut f = File::open(path).map_err(|_| AgentError::Encode)?;
                    archive.append_file(name_in_archive, &mut f).map_err(|_| AgentError::Encode)?;
                }
            }
        }
    }

    // Finaliza o Tar primeiro e recupera o Encoder interno
    let encoder = archive.into_inner().map_err(|_| AgentError::Encode)?;
    
    // Finaliza o compressor Zstd manualmente garantindo que tudo foi gravado no disco
    encoder.finish().map_err(|_| AgentError::Encode)?;
    
    Ok(())
}
