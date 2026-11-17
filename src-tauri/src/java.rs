use crate::{
    downloads::{self, Artifact},
    engine::Engine,
    errors::{Context, Result},
    settings::atomic_json,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
pub fn required(metadata: &Value) -> Result<u32> {
    if let Some(major) = metadata["javaVersion"]["majorVersion"].as_u64() {
        if !(8..=40).contains(&major) {
            return Err("Versão Java exigida não suportada.".into());
        }
        return Ok(major as u32);
    }
    // Versões antigas não declaram javaVersion e utilizam Java 8.
    Ok(8)
}
pub async fn inspect(path: &Path) -> Result<u32> {
    if !path.is_absolute() || !path.is_file() {
        return Err(format!(
            "Executável Java não encontrado: {}",
            path.display()
        ));
    }
    let mut c = tokio::process::Command::new(path);
    c.arg("-version").kill_on_drop(true);
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    let output = tokio::time::timeout(std::time::Duration::from_secs(20), c.output())
        .await
        .ctx("Java não respondeu")?
        .ctx("Executar java -version")?;
    if !output.status.success() {
        return Err("java -version retornou erro.".into());
    }
    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let re = regex::Regex::new(r#"version\s+"(?:1\.)?(\d+)"#).ctx("Regex Java")?;
    let major = re
        .captures(&text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .ok_or("Não foi possível identificar a versão do Java")?;
    Ok(major)
}
pub async fn ensure(engine: &Engine, major: u32) -> Result<PathBuf> {
    if std::env::consts::OS != "windows" || std::env::consts::ARCH != "x86_64" {
        return Err("O gerenciamento automático desta edição requer Windows x64.".into());
    }
    let settings = engine.config()?.settings;
    if !settings.java_path.is_empty() {
        let path = PathBuf::from(&settings.java_path);
        let found = inspect(&path).await?;
        if found != major {
            return Err(format!("Java configurado é {found}; esta versão do Minecraft exige Java {major}. Ative o runtime gerenciado ou escolha Java compatível."));
        }
        return Ok(path);
    }
    let root = engine.0.root.join("java").join(major.to_string());
    let receipt = root.join("runtime.json");
    if let Ok(bytes) = std::fs::read(&receipt) {
        if let Ok(saved) = serde_json::from_slice::<Value>(&bytes) {
            if let Some(relative) = saved["executable"].as_str() {
                let exe = crate::instances::safe_join(&root, relative)?;
                if inspect(&exe).await == Ok(major) {
                    return Ok(exe);
                }
            }
        }
    }
    if !settings.auto_java {
        return Err(format!("Java {major} não encontrado. Habilite o download automático ou informe um executável compatível."));
    }
    engine.stage(&format!("Preparando Eclipse Temurin Java {major}"));
    let mut package = None;
    for image in ["jre", "jdk"] {
        let url=format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture=x64&heap_size=normal&image_type={image}&jvm_impl=hotspot&os=windows&vendor=eclipse");
        match downloads::json(engine, &url).await {
            Ok(data) => {
                if let Some(v) = data.as_array().and_then(|a| a.first()) {
                    package = Some(v.clone());
                    break;
                }
            }
            Err(e) => engine.log(format!("Consulta Temurin {image}: {e}")),
        }
    }
    let data = package.ok_or_else(|| {
        format!("Nenhum pacote Temurin Windows x64 encontrado para Java {major}.")
    })?;
    let p = &data["binary"]["package"];
    let url = p["link"].as_str().ok_or("API Java sem URL")?;
    if !url.starts_with(&format!(
        "https://github.com/adoptium/temurin{major}-binaries/"
    )) {
        return Err("Pacote Java fora do repositório oficial Temurin.".into());
    }
    let hash = p["checksum"].as_str().ok_or("Pacote Java sem SHA-256")?;
    if hash.len() != 64 {
        return Err("Checksum Java inválido.".into());
    }
    let zip = engine
        .0
        .root
        .join("java")
        .join(format!("temurin-{major}-{hash}.zip"));
    let a = Artifact {
        url: url.into(),
        path: zip.clone(),
        hash: hash.into(),
        algorithm: "sha256".into(),
        size: p["size"].as_u64().ok_or("Pacote Java sem tamanho")?,
    };
    downloads::batch(engine, vec![a]).await?;
    engine.check_cancel()?;
    engine.stage("Extraindo Java com validação de caminhos");
    let dest = root.clone();
    let archive = zip.clone();
    tokio::task::spawn_blocking(move || downloads::extract_zip(&archive, &dest, &[]))
        .await
        .ctx("Extrair runtime Java")??;
    engine.check_cancel()?;
    let exe = find_java(&root, 0)?.ok_or("Pacote Java não contém bin/java.exe")?;
    if inspect(&exe).await? != major {
        return Err("O runtime baixado não corresponde ao Java exigido.".into());
    }
    let relative = exe
        .strip_prefix(&root)
        .ctx("Validar runtime privado")?
        .to_string_lossy()
        .replace('\\', "/");
    atomic_json(
        &receipt,
        &serde_json::json!({"executable":relative,"major":major,"sha256":hash,"source":url,"vendor":"Eclipse Adoptium Temurin","release":data["release_name"],"license":"GPLv2 with Classpath Exception; legal/ e arquivos de licença preservados do pacote"}),
    )?;
    engine.log(format!("Java {major} verificado: {}", exe.display()));
    Ok(exe)
}
fn find_java(root: &Path, depth: usize) -> Result<Option<PathBuf>> {
    if depth > 4 {
        return Ok(None);
    }
    for entry in std::fs::read_dir(root).ctx("Procurar Java")? {
        let entry = entry.ctx("Ler pasta Java")?;
        let p = entry.path();
        if entry.file_type().ctx("Ler runtime")?.is_dir() {
            if let Some(v) = find_java(&p, depth + 1)? {
                return Ok(Some(v));
            }
        } else if entry.file_name() == "java.exe"
            && p.parent()
                .and_then(|p| p.file_name())
                .map(|n| n == "bin")
                .unwrap_or(false)
        {
            return Ok(Some(p));
        }
    }
    Ok(None)
}
