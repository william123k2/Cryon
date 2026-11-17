use crate::{
    errors::{Context, Result},
    instances::{absolute_directory, Instance},
    profiles::Profile,
};
use std::path::Path;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub game_directory: String,
    pub java_path: String,
    pub auto_java: bool,
    pub min_ram: u32,
    pub max_ram: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedState {
    pub schema: u32,
    pub profile: Profile,
    pub settings: Settings,
    pub instances: Vec<Instance>,
    pub selected: String,
}
impl SavedState {
    pub fn fresh(root: &Path) -> Result<Self> {
        let games = root.join("games");
        let dir = games.join("primeiro-mundo");
        Ok(Self {
            schema: 1,
            profile: Profile::offline("Explorador")?,
            settings: Settings {
                game_directory: games.to_string_lossy().into(),
                java_path: String::new(),
                auto_java: true,
                min_ram: 512,
                max_ram: 2048,
                width: 1280,
                height: 720,
            },
            instances: vec![Instance {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Meu primeiro mundo".into(),
                version: "1.21.4".into(),
                directory: dir.to_string_lossy().into(),
                min_ram: 512,
                max_ram: 2048,
                width: 1280,
                height: 720,
            }],
            selected: String::new(),
        })
    }
    pub fn validate(&mut self) -> Result<()> {
        if self.schema != 1 {
            return Err("Versão do arquivo de configurações não suportada.".into());
        }
        self.profile = Profile::offline(&self.profile.name)?;
        absolute_directory(&self.settings.game_directory)?;
        let s = &self.settings;
        if s.min_ram < 512
            || s.max_ram > 65536
            || s.max_ram < s.min_ram
            || !(640..=7680).contains(&s.width)
            || !(480..=4320).contains(&s.height)
        {
            return Err("Configurações de memória/resolução inválidas.".into());
        }
        if !s.java_path.is_empty()
            && (!Path::new(&s.java_path).is_absolute() || !Path::new(&s.java_path).is_file())
        {
            return Err("O executável Java informado não existe ou não é absoluto.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for i in &self.instances {
            i.validate()?;
            if !ids.insert(&i.id) {
                return Err("Instância duplicada.".into());
            }
        }
        if !self.instances.iter().any(|i| i.id == self.selected) {
            self.selected = self
                .instances
                .first()
                .map(|i| i.id.clone())
                .unwrap_or_default();
        }
        Ok(())
    }
}
pub fn load(root: &Path) -> Result<SavedState> {
    std::fs::create_dir_all(root).ctx("Criar pasta privada")?;
    let file = root.join("state.json");
    let mut state = if file.exists() {
        serde_json::from_slice(&std::fs::read(&file).ctx("Ler configurações")?).ctx("JSON de configurações inválido; preserve o arquivo e restaure state.backup.json se necessário")?
    } else {
        SavedState::fresh(root)?
    };
    state.validate()?;
    Ok(state)
}
pub fn atomic_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ctx("Criar pasta")?;
    }
    let bytes = serde_json::to_vec_pretty(value).ctx("Serializar JSON")?;
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    use std::io::Write;
    let mut f = std::fs::File::create(&temp).ctx("Criar arquivo temporário")?;
    f.write_all(&bytes).ctx("Gravar JSON")?;
    f.sync_all().ctx("Sincronizar JSON")?;
    drop(f);
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(a: *const u16, b: *const u16, flags: u32) -> i32;
        }
        let a: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 1 | 8) } == 0 {
            return Err(format!(
                "Troca atômica falhou: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(&temp, path).ctx("Trocar arquivo JSON")?;
    Ok(())
}
pub fn save(root: &Path, state: &SavedState) -> Result<()> {
    let path = root.join("state.json");
    if path.exists() {
        std::fs::copy(&path, root.join("state.backup.json")).ctx("Backup das configurações")?;
    }
    atomic_json(&path, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_and_corruption() {
        let root = std::env::temp_dir().join(format!("cryon-state-{}", uuid::Uuid::new_v4()));
        let mut s = load(&root).expect("fresh");
        s.profile = Profile::offline("TesteCryon").expect("profile");
        s.instances[0].name = "Aventura persistente".into();
        s.validate().expect("validate");
        save(&root, &s).expect("save");
        let actual = load(&root).expect("reopen");
        assert_eq!(actual.profile.name, "TesteCryon");
        assert_eq!(actual.instances[0].name, "Aventura persistente");
        assert!(!actual.selected.is_empty());
        save(&root, &s).expect("save backup");
        assert!(root.join("state.backup.json").is_file());
        std::fs::write(root.join("state.json"), b"invalid JSON").expect("corrupt owned fixture");
        assert!(load(&root).is_err());
        assert_eq!(
            std::fs::read(root.join("state.json")).expect("preserve"),
            b"invalid JSON"
        );
    }
}
