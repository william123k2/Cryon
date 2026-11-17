use crate::{
    engine::Engine,
    errors::{Context, Result},
    instances::safe_join,
};
use futures_util::StreamExt;
use sha1::Digest;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;
pub fn trusted_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "piston-meta.mojang.com"
                    | "piston-data.mojang.com"
                    | "libraries.minecraft.net"
                    | "resources.download.minecraft.net"
                    | "launchermeta.mojang.com"
                    | "launcher.mojang.com"
                    | "api.adoptium.net"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
pub fn check_url(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value).ctx("URL inválida")?;
    if !trusted_url(&url) {
        return Err(format!(
            "Fonte de download bloqueada: {}",
            url.host_str().unwrap_or("sem host")
        ));
    }
    Ok(url)
}
pub async fn json(engine: &Engine, url: &str) -> Result<serde_json::Value> {
    engine.check_cancel()?;
    let r = engine
        .0
        .http
        .get(check_url(url)?)
        .send()
        .await
        .ctx("Consultar metadados")?
        .error_for_status()
        .ctx("Resposta HTTP dos metadados")?;
    let mut bytes = Vec::new();
    let mut chunks = r.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        engine.check_cancel()?;
        let chunk = chunk.ctx("Ler metadados")?;
        if bytes.len() + chunk.len() > 16 * 1024 * 1024 {
            return Err("Metadados excedem limite de 16 MiB.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ctx("Metadados JSON inválidos")
}
#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct Artifact {
    pub url: String,
    pub path: PathBuf,
    pub hash: String,
    pub algorithm: String,
    pub size: u64,
}
pub fn checksum(path: &Path, algorithm: &str) -> Result<String> {
    let mut file = std::fs::File::open(path).ctx("Abrir arquivo para verificar integridade")?;
    let mut buf = [0u8; 65536];
    let mut s1 = sha1::Sha1::new();
    let mut s2 = sha2::Sha256::new();
    loop {
        let n = file.read(&mut buf).ctx("Verificar arquivo")?;
        if n == 0 {
            break;
        }
        if algorithm == "sha256" {
            s2.update(&buf[..n])
        } else {
            s1.update(&buf[..n])
        }
    }
    Ok(if algorithm == "sha256" {
        format!("{:x}", s2.finalize())
    } else {
        format!("{:x}", s1.finalize())
    })
}
pub fn valid(a: &Artifact) -> bool {
    a.path.is_file()
        && (a.size == 0
            || a.path
                .metadata()
                .map(|m| m.len() == a.size)
                .unwrap_or(false))
        && (a.hash.is_empty()
            || checksum(&a.path, &a.algorithm)
                .map(|s| s.eq_ignore_ascii_case(&a.hash))
                .unwrap_or(false))
}
struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
pub async fn fetch(engine: &Engine, a: Artifact) -> Result<()> {
    engine.check_cancel()?;
    check_url(&a.url)?;
    if valid(&a) {
        engine.update(|p| {
            p.processed_bytes += a.size;
            p.files_done += 1
        });
        return Ok(());
    }
    if let Some(parent) = a.path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .ctx("Criar pasta de download")?;
    }
    let part = a
        .path
        .with_extension(format!("{}.part", uuid::Uuid::new_v4()));
    let _cleanup = PartialFile(part.clone());
    let mut last = String::new();
    for attempt in 0..3 {
        engine.check_cancel()?;
        if attempt > 0 {
            engine.update(|p| p.retries += 1);
            tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt as u64))).await;
        }
        engine.update(|p| {
            p.current = a
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        });
        let mut transferred = 0u64;
        let result:Result<()>=async{
   let mut r=engine.0.http.get(check_url(&a.url)?).send().await.ctx("Iniciar download")?.error_for_status().ctx("Download HTTP")?;
   let mut file=tokio::fs::File::create(&part).await.ctx("Criar download temporário")?;
   loop{engine.check_cancel()?;let next=tokio::select!{r=r.chunk()=>r.ctx("Receber download")?,_ = tokio::time::sleep(std::time::Duration::from_millis(250))=>{engine.check_cancel()?;continue}};let Some(bytes)=next else{break};transferred+=bytes.len() as u64;if transferred>2*1024*1024*1024 ||(a.size>0&&transferred>a.size){return Err("Download excedeu tamanho declarado.".into())}
    file.write_all(&bytes).await.ctx("Gravar download")?;engine.update(|p|{p.downloaded_bytes+=bytes.len() as u64;p.processed_bytes+=bytes.len() as u64;if a.size==0{p.total_bytes+=bytes.len() as u64}});}
   file.sync_all().await.ctx("Sincronizar download")?;drop(file);
   let temp=Artifact{path:part.clone(),..a.clone()};if !valid(&temp){return Err(format!("Falha de integridade: {}",a.path.display()))}
   engine.check_cancel()?;tokio::fs::rename(&part,&a.path).await.ctx("Finalizar download verificado")?;Ok(())
  }.await;
        match result {
            Ok(()) => {
                engine.update(|p| p.files_done += 1);
                return Ok(());
            }
            Err(e) => {
                last = e;
                engine.update(|p| {
                    p.processed_bytes = p.processed_bytes.saturating_sub(transferred);
                    if a.size == 0 {
                        p.total_bytes = p.total_bytes.saturating_sub(transferred)
                    }
                });
                let _ = tokio::fs::remove_file(&part).await;
                if engine.cancelled() {
                    return Err(last);
                }
                engine.log(format!("Tentativa {}/3: {}", attempt + 1, last));
            }
        }
    }
    Err(last)
}
pub async fn batch(engine: &Engine, artifacts: Vec<Artifact>) -> Result<()> {
    let mut unique = std::collections::HashSet::new();
    let artifacts: Vec<_> = artifacts
        .into_iter()
        .filter(|a| unique.insert(a.path.clone()))
        .collect();
    engine.update(|p| {
        p.total_bytes += artifacts.iter().map(|a| a.size).sum::<u64>();
        p.files_total += artifacts.len() as u64
    });
    let results = futures_util::stream::iter(artifacts.into_iter().map(|a| fetch(engine, a)))
        .buffer_unordered(6);
    tokio::pin!(results);
    while let Some(result) = results.next().await {
        result?;
    }
    Ok(())
}
pub fn extract_zip(file: &Path, dest: &Path, excluded: &[String]) -> Result<()> {
    std::fs::create_dir_all(dest).ctx("Criar pasta de extração")?;
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(file).ctx("Abrir ZIP")?).ctx("ZIP inválido")?;
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).ctx("Ler ZIP")?;
        let name = entry.name().to_owned();
        let path = safe_join(dest, name.trim_end_matches('/'))?;
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err("Link simbólico bloqueado no ZIP.".into());
            }
        }
        if excluded.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        total = total.checked_add(entry.size()).ok_or("ZIP muito grande")?;
        if total > 3 * 1024 * 1024 * 1024 || entry.size() > 1024 * 1024 * 1024 {
            return Err("Limite seguro de extração excedido.".into());
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&path).ctx("Criar pasta do ZIP")?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ctx("Criar pasta do ZIP")?;
            }
            let mut out = std::fs::File::create(&path).ctx("Extrair arquivo")?;
            std::io::copy(&mut entry, &mut out).ctx("Extrair ZIP")?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sources() {
        assert!(check_url("https://resources.download.minecraft.net/ab/abc").is_ok());
        assert!(check_url("http://libraries.minecraft.net/x").is_err());
        assert!(check_url("https://github.com.evil.test/x").is_err());
    }
    #[test]
    fn corrupt_hash() {
        let d = std::env::temp_dir().join(format!("cryon-hash-{}", uuid::Uuid::new_v4()));
        std::fs::write(&d, b"abc").expect("temp");
        let a = Artifact {
            url: String::new(),
            path: d.clone(),
            hash: "a9993e364706816aba3e25717850c26c9cd0d89d".into(),
            algorithm: "sha1".into(),
            size: 3,
        };
        assert!(valid(&a));
        std::fs::write(&d, b"xyz").expect("write");
        assert!(!valid(&a));
        let _ = std::fs::remove_file(d);
    }
    #[test]
    fn zip_traversal_is_blocked() {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("cryon-zip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp");
        let archive = root.join("malicious.zip");
        let f = std::fs::File::create(&archive).expect("zip");
        let mut zip = zip::ZipWriter::new(f);
        zip.start_file("../escaped.txt", zip::write::SimpleFileOptions::default())
            .expect("entry");
        zip.write_all(b"must not escape").expect("entry");
        zip.finish().expect("finish");
        assert!(extract_zip(&archive, &root.join("dest"), &[]).is_err());
        assert!(!root.join("escaped.txt").exists());
    }
}
