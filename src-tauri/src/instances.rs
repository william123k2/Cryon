use crate::errors::Result;
use std::path::{Component, Path, PathBuf};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub directory: String,
    pub min_ram: u32,
    pub max_ram: u32,
    pub width: u32,
    pub height: u32,
}
pub fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 100
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        || id == "."
        || id == ".."
    {
        return Err("Identificador inválido.".into());
    }
    Ok(())
}
pub fn absolute_directory(value: &str) -> Result<PathBuf> {
    let p = PathBuf::from(value);
    if !p.is_absolute()
        || value.contains(['\0', ';', '"'])
        || p.components().any(|c| matches!(c, Component::ParentDir))
        || p.parent().is_none()
    {
        return Err("Escolha uma pasta absoluta válida; não use a raiz do disco, .., aspas ou ponto e vírgula.".into());
    }
    Ok(p)
}
pub fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.contains(['\\', ':', '\0']) || relative.is_empty() {
        return Err("Caminho relativo inseguro.".into());
    }
    let p = Path::new(relative);
    if p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("Path traversal bloqueado.".into());
    }
    let joined = root.join(p);
    let mut current = root.to_path_buf();
    for c in p.components() {
        let name = c.as_os_str().to_string_lossy();
        let stem = name
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && stem.as_bytes()[3].is_ascii_digit());
        if reserved
            || name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
        {
            return Err("Nome de arquivo Windows inseguro.".into());
        }
        current.push(c);
        if let Ok(m) = std::fs::symlink_metadata(&current) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err("Reparse point bloqueado no destino.".into());
                }
            }
            if m.file_type().is_symlink() {
                return Err("Link simbólico bloqueado no destino.".into());
            }
        }
    }
    Ok(joined)
}
impl Instance {
    pub fn validate(&self) -> Result<()> {
        valid_id(&self.id)?;
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self
                .name
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        {
            return Err("Nome da instância inválido.".into());
        }
        if !self.version.is_empty() {
            valid_id(&self.version)?;
        }
        absolute_directory(&self.directory)?;
        if self.min_ram < 512 || self.max_ram > 65536 || self.max_ram < self.min_ram {
            return Err(
                "RAM inválida: mínimo de 512 MB, máxima até 65536 MB e mínima ≤ máxima.".into(),
            );
        }
        if !(640..=7680).contains(&self.width) || !(480..=4320).contains(&self.height) {
            return Err("Resolução inválida.".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal() {
        let r = std::env::temp_dir();
        for p in [
            "../x",
            "/x",
            "a/../../x",
            "C:/x",
            "a\\b",
            "a/./../x",
            "NUL.txt",
            "a/COM1",
            "a/b. ",
        ] {
            assert!(safe_join(&r, p).is_err(), "{p}");
        }
        assert!(safe_join(&r, "org/lwjgl/native.jar").is_ok());
    }
}
