use crate::{
    errors::{Context, Result},
    instances::Instance,
    settings::{self, SavedState},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
#[derive(Clone, serde::Serialize, Default)]
pub struct Progress {
    pub status: String,
    pub kind: String,
    pub stage: String,
    pub current: String,
    pub total_bytes: u64,
    pub processed_bytes: u64,
    pub downloaded_bytes: u64,
    pub files_total: u64,
    pub files_done: u64,
    pub retries: u32,
    pub speed: f64,
    pub error: Option<String>,
}
#[derive(Clone, serde::Serialize, Default)]
pub struct GameStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub version: String,
    pub exit_code: Option<i32>,
}
#[derive(Clone, serde::Serialize)]
pub struct Snapshot {
    pub saved: SavedState,
    pub progress: Progress,
    pub game: GameStatus,
    pub logs: Vec<String>,
    pub installed: Vec<String>,
    pub root: String,
    pub runtime_os: String,
    pub runtime_arch: String,
    pub app_version: String,
}
pub struct Inner {
    pub _instance_lock: std::fs::File,
    pub root: PathBuf,
    pub saved: Mutex<SavedState>,
    pub progress: Mutex<Progress>,
    pub game: Mutex<GameStatus>,
    pub logs: Mutex<VecDeque<String>>,
    pub cancel: AtomicBool,
    pub stop_game: AtomicBool,
    pub busy: AtomicBool,
    pub started: Mutex<Instant>,
    pub http: reqwest::Client,
}
#[derive(Clone)]
pub struct Engine(pub Arc<Inner>);
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_writer_per_data_directory() {
        let root = std::env::temp_dir().join(format!("cryon-lock-{}", uuid::Uuid::new_v4()));
        let first = Engine::new(root.clone()).expect("first engine");
        assert!(Engine::new(root.clone()).is_err());
        drop(first);
        assert!(Engine::new(root).is_ok());
    }
}
impl Engine {
    pub fn new(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root).ctx("Criar pasta privada")?;
        let instance_lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("cryon.lock"))
            .ctx("Abrir trava do launcher")?;
        instance_lock
            .try_lock()
            .ctx("O Cryon já está usando esta pasta de dados; feche a outra instância")?;
        let saved = settings::load(&root)?;
        settings::save(&root, &saved)?;
        let http = reqwest::Client::builder()
            .user_agent("Cryon/0.2.0 (Windows Vanilla Launcher)")
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::custom(|a| {
                if a.previous().len() > 8 || !crate::downloads::trusted_url(a.url()) {
                    a.error("Redirecionamento não confiável")
                } else {
                    a.follow()
                }
            }))
            .build()
            .ctx("Criar cliente HTTPS")?;
        Ok(Self(Arc::new(Inner {
            _instance_lock: instance_lock,
            root,
            saved: Mutex::new(saved),
            progress: Mutex::new(Progress {
                status: "idle".into(),
                ..Default::default()
            }),
            game: Mutex::new(GameStatus::default()),
            logs: Mutex::new(VecDeque::new()),
            cancel: AtomicBool::new(false),
            stop_game: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            started: Mutex::new(Instant::now()),
            http,
        })))
    }
    pub fn config(&self) -> Result<SavedState> {
        Ok(self
            .0
            .saved
            .lock()
            .map_err(|_| "Configuração ocupada")?
            .clone())
    }
    pub fn save(&self, mut state: SavedState) -> Result<SavedState> {
        state.validate()?;
        let mut guard = self.0.saved.lock().map_err(|_| "Configuração ocupada")?;
        settings::save(&self.0.root, &state)?;
        *guard = state.clone();
        Ok(state)
    }
    pub fn instance(&self, id: &str) -> Result<Instance> {
        self.config()?
            .instances
            .into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| "Instância não encontrada.".into())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut progress = self
            .0
            .progress
            .lock()
            .map_err(|_| "Progresso ocupado")?
            .clone();
        let secs = self
            .0
            .started
            .lock()
            .map_err(|_| "Relógio ocupado")?
            .elapsed()
            .as_secs_f64();
        progress.speed = if secs > 0.0 && progress.status == "running" {
            progress.downloaded_bytes as f64 / secs
        } else {
            0.0
        };
        let dir = self.0.root.join("versions");
        let mut installed = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.join("installed.json").is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_owned())
                    }
                }
            }
        }
        installed.sort();
        Ok(Snapshot {
            saved: self.config()?,
            progress,
            game: self
                .0
                .game
                .lock()
                .map_err(|_| "Estado do jogo ocupado")?
                .clone(),
            logs: self
                .0
                .logs
                .lock()
                .map_err(|_| "Logs ocupados")?
                .iter()
                .cloned()
                .collect(),
            installed,
            root: self.0.root.to_string_lossy().into(),
            runtime_os: std::env::consts::OS.into(),
            runtime_arch: std::env::consts::ARCH.into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
        })
    }
    pub fn log(&self, message: impl AsRef<str>) {
        let message = message.as_ref();
        if let Ok(mut logs) = self.0.logs.lock() {
            logs.push_back(message.to_owned());
            while logs.len() > 500 {
                logs.pop_front();
            }
        }
        let dir = self.0.root.join("logs");
        let _ = std::fs::create_dir_all(&dir);
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("cryon.log"))
        {
            let _ = writeln!(file, "{message}");
        }
    }
    pub fn begin(&self, kind: &str) -> Result<()> {
        if self
            .0
            .game
            .lock()
            .map_err(|_| "Estado do jogo ocupado")?
            .running
        {
            return Err("Encerre o jogo antes de iniciar outra instalação ou execução.".into());
        }
        self.0
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Há uma operação em andamento.")?;
        self.0.cancel.store(false, Ordering::SeqCst);
        if let Ok(mut start) = self.0.started.lock() {
            *start = Instant::now()
        }
        self.update(|p| {
            *p = Progress {
                status: "running".into(),
                kind: kind.into(),
                stage: "Preparando".into(),
                ..Default::default()
            }
        });
        Ok(())
    }
    pub fn finish(&self, result: &Result<()>) {
        self.update(|p| match result {
            Ok(()) => {
                p.status = "completed".into();
                p.stage = "Concluído".into()
            }
            Err(e) => {
                p.status = if self.cancelled() {
                    "cancelled"
                } else {
                    "error"
                }
                .into();
                p.error = Some(e.clone());
                p.stage = if self.cancelled() {
                    "Cancelado"
                } else {
                    "Erro"
                }
                .into()
            }
        });
        if let Err(e) = result {
            self.log(format!("ERRO: {e}"));
        }
        self.0.busy.store(false, Ordering::SeqCst);
    }
    pub fn update(&self, f: impl FnOnce(&mut Progress)) {
        if let Ok(mut p) = self.0.progress.lock() {
            f(&mut p)
        }
    }
    pub fn stage(&self, s: &str) {
        self.update(|p| p.stage = s.into());
        self.log(s);
    }
    pub fn cancelled(&self) -> bool {
        self.0.cancel.load(Ordering::SeqCst)
    }
    pub fn check_cancel(&self) -> Result<()> {
        if self.cancelled() && self.0.busy.load(Ordering::SeqCst) {
            Err("Operação cancelada pelo usuário.".into())
        } else {
            Ok(())
        }
    }
    pub fn cancel(&self) {
        self.0.cancel.store(true, Ordering::SeqCst)
    }
}
