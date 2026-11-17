pub mod downloads;
pub mod engine;
pub mod errors;
pub mod instances;
pub mod java;
pub mod manifest;
pub mod minecraft;
pub mod process;
pub mod profiles;
pub mod settings;
use engine::Engine;
use errors::{Context, Result};
use tauri::Manager;

#[tauri::command]
fn snapshot(engine: tauri::State<'_, Engine>) -> Result<engine::Snapshot> {
    engine.snapshot()
}
#[tauri::command]
fn save_state(
    engine: tauri::State<'_, Engine>,
    saved: settings::SavedState,
) -> Result<settings::SavedState> {
    engine.save(saved)
}
#[tauri::command]
async fn versions(
    engine: tauri::State<'_, Engine>,
    refresh: bool,
) -> Result<Vec<manifest::Version>> {
    manifest::list(&engine, refresh).await
}
#[tauri::command]
async fn pick_directory() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Escolha uma pasta para o Cryon")
        .pick_folder()
        .await
        .map(|p| p.path().to_string_lossy().into())
}
#[tauri::command]
async fn pick_java() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Escolha java.exe")
        .add_filter("Java", &["exe"])
        .pick_file()
        .await
        .map(|p| p.path().to_string_lossy().into())
}
#[tauri::command]
fn cancel_download(engine: tauri::State<'_, Engine>) {
    engine.cancel()
}
#[tauri::command]
fn stop_game(engine: tauri::State<'_, Engine>) {
    engine
        .0
        .stop_game
        .store(true, std::sync::atomic::Ordering::SeqCst)
}
#[tauri::command]
fn start_operation(
    engine: tauri::State<'_, Engine>,
    instance_id: String,
    launch: bool,
) -> Result<()> {
    let instance = engine.instance(&instance_id)?;
    engine.begin(if launch { "launch" } else { "install" })?;
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn(async move {
        let result = async {
            let installed = minecraft::install(&engine, &instance).await?;
            if launch {
                process::launch(&engine, instance, installed).await?;
            }
            Ok(())
        }
        .await;
        engine.finish(&result);
    });
    Ok(())
}
#[tauri::command]
fn export_logs(engine: tauri::State<'_, Engine>) -> Result<Option<String>> {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("cryon-diagnostico.txt")
        .save_file()
    else {
        return Ok(None);
    };
    let snapshot = engine.snapshot()?;
    let report = format!(
        "Cryon {}\nSO: {} {}\nDados: {}\nJava: {}\nOperação: {} / {}\n\n{}",
        snapshot.app_version,
        snapshot.runtime_os,
        snapshot.runtime_arch,
        snapshot.root,
        snapshot.saved.settings.java_path,
        snapshot.progress.status,
        snapshot.progress.stage,
        snapshot.logs.join("\n")
    );
    std::fs::write(&path, report).ctx("Exportar diagnóstico")?;
    Ok(Some(path.to_string_lossy().into()))
}
pub fn default_root() -> Result<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("CRYON_DATA_DIR") {
        return Ok(crate::instances::absolute_directory(
            &path.to_string_lossy(),
        )?);
    }
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA ausente")?;
    Ok(std::path::PathBuf::from(local).join("CryonData"))
}
pub fn run() {
    let result = tauri::Builder::default()
        .setup(
            |app| match Engine::new(default_root().map_err(std::io::Error::other)?) {
                Ok(engine) => {
                    app.manage(engine);
                    Ok(())
                }
                Err(error) => {
                    rfd::MessageDialog::new()
                        .set_title("Cryon — erro de inicialização")
                        .set_description(&error)
                        .set_level(rfd::MessageLevel::Error)
                        .show();
                    Err(std::io::Error::other(error).into())
                }
            },
        )
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_state,
            versions,
            pick_directory,
            pick_java,
            cancel_download,
            stop_game,
            start_operation,
            export_logs
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("Falha ao iniciar Cryon: {e}");
    }
}
