use cryon_lib::{engine::Engine, errors::Result, minecraft, process};
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("ERRO: {e}");
        std::process::exit(1)
    }
}
async fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let action = args.get(1).map(String::as_str).unwrap_or("status");
    let engine = Engine::new(cryon_lib::default_root()?)?;
    if action == "status" {
        println!(
            "{}",
            serde_json::to_string_pretty(&engine.snapshot()?).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if action == "versions" {
        let versions = cryon_lib::manifest::list(&engine, true).await?;
        println!(
            "{}",
            serde_json::to_string(&versions).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if !matches!(action, "install" | "launch") {
        return Err("Use status, versions, install <versão> ou launch <versão>.".into());
    }
    let version = args.get(2).ok_or("Informe a versão")?.clone();
    cryon_lib::instances::valid_id(&version)?;
    let mut config = engine.config()?;
    let instance = if let Some(existing) = config.instances.iter().find(|i| i.version == version) {
        existing.clone()
    } else {
        let s = &config.settings;
        let id = uuid::Uuid::new_v4().to_string();
        let instance = cryon_lib::instances::Instance {
            id: id.clone(),
            name: format!("Vanilla {version}"),
            version: version.clone(),
            directory: std::path::Path::new(&s.game_directory)
                .join(&id)
                .to_string_lossy()
                .into(),
            min_ram: s.min_ram,
            max_ram: s.max_ram,
            width: s.width,
            height: s.height,
        };
        config.instances.push(instance.clone());
        instance
    };
    config.selected = instance.id.clone();
    engine.save(config)?;
    engine.begin(action)?;
    let monitor = engine.clone();
    let printer = tokio::spawn(async move {
        loop {
            if let Ok(s) = monitor.snapshot() {
                eprintln!(
                    "{}: {}/{} arquivos | {} / {} bytes | {}",
                    s.progress.stage,
                    s.progress.files_done,
                    s.progress.files_total,
                    s.progress.processed_bytes,
                    s.progress.total_bytes,
                    s.progress.current
                );
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    let result = async {
        let installed = minecraft::install(&engine, &instance).await?;
        if action == "launch" {
            process::launch(&engine, instance, installed, None).await?;
            let mut seconds = 0;
            while engine.snapshot()?.game.running {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                seconds += 1;
                if seconds >= 300 {
                    engine
                        .0
                        .stop_game
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            let s = engine.snapshot()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&s.game).map_err(|e| e.to_string())?
            );
            if s.game.exit_code != Some(0) {
                return Err(format!("Jogo encerrou com código {:?}", s.game.exit_code));
            }
        }
        Ok(())
    }
    .await;
    engine.finish(&result);
    printer.abort();
    result
}
