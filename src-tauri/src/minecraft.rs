use crate::{
    downloads::{self, Artifact},
    engine::Engine,
    errors::{Context, Result},
    instances::{safe_join, valid_id, Instance},
    manifest,
    settings::atomic_json,
};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Installation {
    pub version: String,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub natives: PathBuf,
    pub assets: PathBuf,
    pub asset_index: String,
    pub game_assets: PathBuf,
    pub metadata: Value,
}
pub fn rules_allowed(value: &Value, features: &HashMap<String, bool>) -> bool {
    let Some(rules) = value.as_array() else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let mut applies = true;
        if let Some(os) = rule.get("os") {
            if let Some(name) = os["name"].as_str() {
                applies &= name == "windows";
            }
            if let Some(arch) = os["arch"].as_str() {
                applies &= matches!(arch, "x86_64" | "amd64" | "x64");
            }
            if let Some(version) = os["version"].as_str() {
                applies &= regex::Regex::new(version)
                    .map(|r| r.is_match(&windows_version()))
                    .unwrap_or(false);
            }
        }
        if let Some(f) = rule["features"].as_object() {
            for (k, v) in f {
                applies &=
                    features.get(k).copied().unwrap_or(false) == v.as_bool().unwrap_or(false);
            }
        }
        if applies {
            allowed = rule["action"].as_str() == Some("allow");
        }
    }
    allowed
}
fn windows_version() -> String {
    std::env::var("CRYON_WINDOWS_VERSION").unwrap_or_else(|_| "10.0".into())
}
pub fn artifact(value: &Value, base: &std::path::Path, relative: &str) -> Result<Artifact> {
    let url = value["url"].as_str().ok_or("Artefato sem URL")?;
    let hash = value["sha1"].as_str().unwrap_or_default();
    if !hash.is_empty() && (hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit())) {
        return Err("SHA-1 de artefato inválido.".into());
    }
    Ok(Artifact {
        url: url.into(),
        path: safe_join(base, relative)?,
        hash: hash.into(),
        algorithm: "sha1".into(),
        size: value["size"].as_u64().unwrap_or(0),
    })
}
pub fn library_plan(
    metadata: &Value,
    root: &std::path::Path,
) -> Result<(Vec<Artifact>, Vec<PathBuf>, Vec<(PathBuf, Vec<String>)>)> {
    let features = HashMap::new();
    let base = root.join("libraries");
    let mut files = Vec::new();
    let mut cp = Vec::new();
    let mut natives = Vec::new();
    for lib in metadata["libraries"]
        .as_array()
        .ok_or("Metadados sem bibliotecas")?
    {
        if !rules_allowed(&lib["rules"], &features) {
            continue;
        }
        if let Some(a) = lib["downloads"].get("artifact") {
            let p = a["path"].as_str().ok_or("Biblioteca sem caminho")?;
            let a = artifact(a, &base, p)?;
            cp.push(a.path.clone());
            files.push(a)
        } else if lib.get("downloads").is_none() {
            let coord = lib["name"]
                .as_str()
                .ok_or("Coordenada de biblioteca ausente")?;
            let parts: Vec<_> = coord.split(':').collect();
            if parts.len() < 3 || parts.len() > 4 {
                return Err("Coordenada Maven inválida.".into());
            }
            let suffix = if parts.len() == 4 {
                format!("-{}", parts[3])
            } else {
                String::new()
            };
            let p = format!(
                "{}/{}/{}/{}-{}{}.jar",
                parts[0].replace('.', "/"),
                parts[1],
                parts[2],
                parts[1],
                parts[2],
                suffix
            );
            let url = format!(
                "{}{}",
                lib["url"]
                    .as_str()
                    .unwrap_or("https://libraries.minecraft.net/"),
                p
            );
            let path = safe_join(&base, &p)?;
            cp.push(path.clone());
            files.push(Artifact {
                url,
                path,
                hash: String::new(),
                algorithm: "sha1".into(),
                size: 0,
            });
        }
        if let Some(native) = lib["natives"]["windows"].as_str() {
            let key = native.replace("${arch}", "64");
            let a = lib["downloads"]["classifiers"]
                .get(&key)
                .ok_or("Classificador nativo Windows ausente")?;
            let p = a["path"].as_str().ok_or("Nativa sem caminho")?;
            let a = artifact(a, &base, p)?;
            let excludes = lib["extract"]["exclude"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_else(|| vec!["META-INF/".into()]);
            natives.push((a.path.clone(), excludes));
            files.push(a);
        }
    }
    Ok((files, cp, natives))
}
pub async fn install(engine: &Engine, instance: &Instance) -> Result<Installation> {
    instance.validate()?;
    if instance.version.is_empty() {
        return Err("Selecione uma versão para a instância.".into());
    }
    engine.stage("Consultando metadados oficiais do Minecraft");
    let metadata = manifest::metadata(engine, &instance.version).await?;
    if metadata.get("inheritsFrom").is_some() {
        return Err(
            "Metadados herdados/modificados não são suportados nesta edição Vanilla.".into(),
        );
    }
    let major = crate::java::required(&metadata)?;
    crate::java::ensure(engine, major).await?;
    let root = &engine.0.root;
    let version_dir = root.join("versions").join(&instance.version);
    let jar = artifact(
        &metadata["downloads"]["client"],
        &version_dir,
        &format!("{}.jar", instance.version),
    )?;
    let (mut files, mut cp, native_archives) = library_plan(&metadata, root)?;
    cp.push(jar.path.clone());
    files.push(jar);
    let assets = root.join("assets");
    let index = &metadata["assetIndex"];
    let asset_id = index["id"]
        .as_str()
        .ok_or("Índice de assets ausente")?
        .to_owned();
    valid_id(&asset_id)?;
    let index_file = artifact(index, &assets, &format!("indexes/{asset_id}.json"))?;
    engine.stage("Validando índice de assets");
    downloads::batch(engine, vec![index_file.clone()]).await?;
    let index_data: Value =
        serde_json::from_slice(&std::fs::read(&index_file.path).ctx("Ler assets")?)
            .ctx("Índice de assets inválido")?;
    for obj in index_data["objects"]
        .as_object()
        .ok_or("Assets sem objetos")?
        .values()
    {
        let hash = obj["hash"].as_str().ok_or("Asset sem hash")?;
        if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Hash de asset inválido.".into());
        }
        files.push(Artifact {
            url: format!(
                "https://resources.download.minecraft.net/{}/{hash}",
                &hash[..2]
            ),
            path: safe_join(&assets, &format!("objects/{}/{hash}", &hash[..2]))?,
            hash: hash.into(),
            algorithm: "sha1".into(),
            size: obj["size"].as_u64().ok_or("Asset sem tamanho")?,
        });
    }
    if let Some(log) = metadata["logging"]["client"]["file"].as_object() {
        let v = Value::Object(log.clone());
        let id = v["id"].as_str().ok_or("Log sem ID")?;
        files.push(artifact(&v, &assets, &format!("log_configs/{id}"))?);
    }
    engine.stage("Baixando e verificando bibliotecas, cliente e assets");
    downloads::batch(engine, files).await?;
    engine.check_cancel()?;
    engine.stage("Preparando bibliotecas nativas Windows x64");
    let natives = version_dir.join("natives");
    std::fs::create_dir_all(&natives).ctx("Criar pasta de nativas")?;
    for (archive, exclude) in native_archives {
        engine.check_cancel()?;
        let dest = natives.clone();
        tokio::task::spawn_blocking(move || downloads::extract_zip(&archive, &dest, &exclude))
            .await
            .ctx("Extrair nativa")??;
    }
    let game = PathBuf::from(&instance.directory);
    std::fs::create_dir_all(&game).ctx("Criar diretório da instância")?;
    let virtual_assets = index_data["virtual"].as_bool().unwrap_or(false);
    let map_resources = index_data["map_to_resources"].as_bool().unwrap_or(false);
    let game_assets = if virtual_assets {
        assets.join("virtual").join(&asset_id)
    } else {
        assets.clone()
    };
    if virtual_assets || map_resources {
        for (name, obj) in index_data["objects"]
            .as_object()
            .ok_or("Assets inválidos")?
        {
            engine.check_cancel()?;
            let hash = obj["hash"].as_str().ok_or("Asset inválido")?;
            let source = safe_join(&assets, &format!("objects/{}/{hash}", &hash[..2]))?;
            for dest in [
                if virtual_assets {
                    Some(safe_join(&game_assets, name)?)
                } else {
                    None
                },
                if map_resources {
                    Some(safe_join(&game.join("resources"), name)?)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                if let Some(p) = dest.parent() {
                    std::fs::create_dir_all(p).ctx("Preparar assets legados")?;
                }
                std::fs::copy(&source, &dest).ctx("Copiar asset legado")?;
            }
        }
    }
    let main_class = metadata["mainClass"]
        .as_str()
        .ok_or("MainClass ausente")?
        .to_owned();
    let installed = Installation {
        version: instance.version.clone(),
        main_class,
        classpath: cp,
        natives,
        assets,
        asset_index: asset_id,
        game_assets,
        metadata,
    };
    atomic_json(&version_dir.join("installed.json"), &installed)?;
    engine.log(format!(
        "Minecraft {} instalado e verificado.",
        instance.version
    ));
    Ok(installed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rules() {
        let f = HashMap::new();
        assert!(rules_allowed(&Value::Null, &f));
        assert!(!rules_allowed(
            &serde_json::json!([{"action":"allow","os":{"name":"linux"}}]),
            &f
        ));
        assert!(rules_allowed(
            &serde_json::json!([{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]),
            &f
        ));
        assert!(!rules_allowed(
            &serde_json::json!([{"action":"allow","features":{"is_demo_user":true}}]),
            &f
        ));
    }
}
