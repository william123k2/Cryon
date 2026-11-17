use crate::{
    engine::Engine,
    errors::{Context, Result},
    instances::Instance,
    minecraft::{rules_allowed, Installation},
};
use serde_json::Value;
use std::{collections::HashMap, sync::atomic::Ordering};
fn expand(value: &str, vars: &HashMap<String, String>) -> Result<String> {
    let mut out = value.to_owned();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v)
    }
    if out.contains("${") {
        return Err(format!("Argumento não suportado pelos metadados: {out}"));
    }
    Ok(out)
}
pub fn resolve_arguments(
    values: &Value,
    features: &HashMap<String, bool>,
    vars: &HashMap<String, String>,
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    if let Some(values) = values.as_array() {
        for value in values {
            if let Some(s) = value.as_str() {
                out.push(expand(s, vars)?)
            } else if rules_allowed(&value["rules"], features) {
                match &value["value"] {
                    Value::String(s) => out.push(expand(s, vars)?),
                    Value::Array(a) => {
                        for s in a {
                            out.push(expand(
                                s.as_str().ok_or("Argumento de versão inválido")?,
                                vars,
                            )?)
                        }
                    }
                    _ => return Err("Formato de argumento inválido.".into()),
                }
            }
        }
    }
    Ok(out)
}
pub fn arguments(
    engine: &Engine,
    instance: &Instance,
    installed: &Installation,
) -> Result<Vec<String>> {
    let config = engine.config()?;
    let path_string = |p: &std::path::Path| p.to_string_lossy().to_string();
    let cp = std::env::join_paths(&installed.classpath)
        .ctx("Montar classpath")?
        .to_string_lossy()
        .to_string();
    let mut vars = HashMap::new();
    let mut put = |k: &str, v: String| {
        vars.insert(k.to_owned(), v);
    };
    put("auth_player_name", config.profile.name);
    put("auth_uuid", config.profile.uuid.clone());
    put("auth_access_token", "0".into());
    put("auth_session", format!("token:0:{}", config.profile.uuid));
    put("user_type", "legacy".into());
    put("user_properties", "{}".into());
    put("auth_xuid", "0".into());
    put("clientid", "0".into());
    put("version_name", installed.version.clone());
    put(
        "version_type",
        installed.metadata["type"]
            .as_str()
            .unwrap_or("release")
            .into(),
    );
    put("game_directory", instance.directory.clone());
    put("assets_root", path_string(&installed.assets));
    put("game_assets", path_string(&installed.game_assets));
    put("assets_index_name", installed.asset_index.clone());
    put("natives_directory", path_string(&installed.natives));
    put(
        "library_directory",
        path_string(&engine.0.root.join("libraries")),
    );
    put("classpath", cp.clone());
    put("classpath_separator", ";".into());
    put("launcher_name", "Cryon".into());
    put("launcher_version", env!("CARGO_PKG_VERSION").into());
    put("resolution_width", instance.width.to_string());
    put("resolution_height", instance.height.to_string());
    let features = HashMap::from([("has_custom_resolution".into(), true)]);
    let mut args = vec![
        format!("-Xms{}M", instance.min_ram),
        format!("-Xmx{}M", instance.max_ram),
        "-Dlog4j2.formatMsgNoLookups=true".into(),
    ];
    if installed.metadata["arguments"]["jvm"].is_array() {
        args.extend(resolve_arguments(
            &installed.metadata["arguments"]["jvm"],
            &features,
            &vars,
        )?)
    } else {
        args.extend([
            format!("-Djava.library.path={}", installed.natives.display()),
            "-cp".into(),
            cp,
        ]);
    }
    if let Some(logging) = installed.metadata["logging"]["client"]["argument"].as_str() {
        let id = installed.metadata["logging"]["client"]["file"]["id"]
            .as_str()
            .ok_or("Configuração de log sem ID")?;
        let p = crate::instances::safe_join(&installed.assets, &format!("log_configs/{id}"))?;
        args.push(logging.replace("${path}", &p.to_string_lossy()));
    }
    args.push(installed.main_class.clone());
    if installed.metadata["arguments"]["game"].is_array() {
        args.extend(resolve_arguments(
            &installed.metadata["arguments"]["game"],
            &features,
            &vars,
        )?)
    } else {
        for arg in installed.metadata["minecraftArguments"]
            .as_str()
            .ok_or("Versão não possui argumentos de inicialização")?
            .split_whitespace()
        {
            args.push(expand(arg, &vars)?)
        }
        args.extend([
            "--width".into(),
            instance.width.to_string(),
            "--height".into(),
            instance.height.to_string(),
        ]);
    }
    Ok(args)
}
pub async fn launch(engine: &Engine, instance: Instance, installed: Installation) -> Result<()> {
    engine.check_cancel()?;
    let major = crate::java::required(&installed.metadata)?;
    let java = crate::java::ensure(engine, major).await?;
    let args = arguments(engine, &instance, &installed)?;
    let log_dir = engine.0.root.join("logs");
    std::fs::create_dir_all(&log_dir).ctx("Criar logs")?;
    let path = log_dir.join(format!("minecraft-{}.log", uuid::Uuid::new_v4()));
    let log = std::sync::Arc::new(tokio::sync::Mutex::new(
        tokio::fs::File::create(&path)
            .await
            .ctx("Criar log do jogo")?,
    ));
    engine.stage("Iniciando Minecraft com perfil local");
    let mut command = tokio::process::Command::new(java);
    command
        .args(&args)
        .current_dir(&instance.directory)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.spawn().ctx("Iniciar processo Java")?;
    let pid = child.id();
    engine.0.stop_game.store(false, Ordering::SeqCst);
    {
        let mut status = engine.0.game.lock().map_err(|_| "Estado do jogo ocupado")?;
        status.running = true;
        status.pid = pid;
        status.version = installed.version.clone();
        status.exit_code = None;
    }
    engine.log(format!(
        "Minecraft {} iniciado; PID {:?}; Java {major}; RAM {}–{} MB; perfil offline.",
        installed.version, pid, instance.min_ram, instance.max_ram
    ));
    engine.log(format!("Log do jogo: {}", path.display()));
    async fn lines(
        engine: Engine,
        reader: impl tokio::io::AsyncRead + Unpin,
        log: std::sync::Arc<tokio::sync::Mutex<tokio::fs::File>>,
    ) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut lines = tokio::io::BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            engine.log(&line);
            let _ = log
                .lock()
                .await
                .write_all(format!("{line}\n").as_bytes())
                .await;
        }
    }
    let stdout = child.stdout.take().ok_or("Java não forneceu stdout")?;
    let stderr = child.stderr.take().ok_or("Java não forneceu stderr")?;
    let e = engine.clone();
    let out_log = log.clone();
    let out = tokio::spawn(lines(e, stdout, out_log));
    let e = engine.clone();
    let err = tokio::spawn(lines(e, stderr, log));
    let e = engine.clone();
    tokio::spawn(async move {
        let code = loop {
            if e.0.stop_game.load(Ordering::SeqCst) {
                let _ = child.start_kill();
            }
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(400)).await,
                Err(error) => {
                    e.log(format!("Falha ao acompanhar Java: {error}"));
                    break None;
                }
            }
        };
        let _ = out.await;
        let _ = err.await;
        if let Ok(mut status) = e.0.game.lock() {
            status.running = false;
            status.pid = None;
            status.exit_code = code;
        }
        e.log(format!("Minecraft encerrado. Código de saída: {code:?}"));
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn args_rules() {
        let vars = HashMap::from([("game_directory".into(), "C:\\Uma pasta\\Jogo".into())]);
        let v = serde_json::json!(["--gameDir","${game_directory}",{"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"}]);
        let r = resolve_arguments(&v, &HashMap::new(), &vars).expect("args");
        assert_eq!(r, vec!["--gameDir", "C:\\Uma pasta\\Jogo"]);
        assert!(expand("${unknown}", &vars).is_err());
    }
}
