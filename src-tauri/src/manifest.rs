use crate::{
    downloads::{self, Artifact},
    engine::Engine,
    errors::{Context, Result},
    instances::valid_id,
    settings::atomic_json,
};
use serde_json::Value;
pub const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub sha1: String,
    #[serde(default, rename = "releaseTime")]
    pub release_time: String,
}
pub async fn list(engine: &Engine, refresh: bool) -> Result<Vec<Version>> {
    let path = engine.0.root.join("manifest.json");
    let data: Value = if !refresh && path.is_file() {
        serde_json::from_slice(&std::fs::read(&path).ctx("Ler manifesto salvo")?)
            .ctx("Manifesto salvo inválido")?
    } else {
        match downloads::json(engine, MANIFEST).await {
            Ok(v) => {
                atomic_json(&path, &v)?;
                v
            }
            Err(e) => {
                if path.is_file() {
                    engine.log(format!("Usando catálogo salvo: {e}"));
                    serde_json::from_slice(&std::fs::read(&path).ctx("Ler catálogo")?)
                        .ctx("Catálogo inválido")?
                } else {
                    return Err(e);
                }
            }
        }
    };
    serde_json::from_value(
        data.get("versions")
            .cloned()
            .ok_or("Manifesto sem versões")?,
    )
    .ctx("Formato do manifesto inválido")
}
pub async fn metadata(engine: &Engine, id: &str) -> Result<Value> {
    valid_id(id)?;
    let versions = list(engine, false).await?;
    let version = versions
        .into_iter()
        .find(|v| v.id == id)
        .ok_or("Versão ausente do manifesto oficial")?;
    let dir = engine.0.root.join("versions").join(id);
    let path = dir.join(format!("{id}.json"));
    let a = Artifact {
        url: version.url,
        path: path.clone(),
        hash: version.sha1,
        algorithm: "sha1".into(),
        size: 0,
    };
    engine.update(|p| p.files_total += 1);
    downloads::fetch(engine, a).await?;
    let data: Value = serde_json::from_slice(&std::fs::read(&path).ctx("Ler metadados")?)
        .ctx("Metadados inválidos")?;
    if data["id"].as_str() != Some(id) {
        return Err("Metadados não correspondem à versão solicitada.".into());
    }
    Ok(data)
}
