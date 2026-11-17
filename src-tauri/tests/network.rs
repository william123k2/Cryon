//! Integração HTTPS real; executar explicitamente com --ignored.
use cryon_lib::{
    downloads::{self, Artifact},
    engine::Engine,
    manifest,
};
#[tokio::test]
#[ignore = "usa a rede oficial Mojang e cria somente uma pasta temporária própria"]
async fn cancellation_and_integrity_recovery() {
    let root = std::env::temp_dir().join(format!("cryon-network-{}", uuid::Uuid::new_v4()));
    let engine = Engine::new(root.clone()).expect("engine");
    engine.begin("test").expect("begin");
    let versions = manifest::list(&engine, true)
        .await
        .expect("official manifest");
    let version = versions
        .into_iter()
        .find(|v| v.id == "1.16.5")
        .expect("release");
    let a = Artifact {
        url: version.url,
        path: root.join("metadata.json"),
        hash: version.sha1,
        algorithm: "sha1".into(),
        size: 0,
    };
    downloads::fetch(&engine, a.clone())
        .await
        .expect("metadata download");
    assert!(downloads::valid(&a));
    std::fs::write(&a.path, b"corruption inside owned test fixture").expect("corrupt fixture");
    assert!(!downloads::valid(&a));
    downloads::fetch(&engine, a.clone())
        .await
        .expect("integrity repair");
    assert!(downloads::valid(&a));
    let data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&a.path).expect("metadata")).expect("json");
    let client = &data["downloads"]["client"];
    let path = root.join("client.jar");
    let big = Artifact {
        url: client["url"].as_str().expect("url").into(),
        path: path.clone(),
        hash: client["sha1"].as_str().expect("hash").into(),
        algorithm: "sha1".into(),
        size: client["size"].as_u64().expect("size"),
    };
    let task_engine = engine.clone();
    let task = tokio::spawn(async move { downloads::fetch(&task_engine, big).await });
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    engine.cancel();
    let result = tokio::time::timeout(std::time::Duration::from_secs(25), task)
        .await
        .expect("cancel timeout")
        .expect("task");
    assert!(result.is_err());
    engine.finish(&result);
    assert_eq!(
        engine.snapshot().expect("status").progress.status,
        "cancelled"
    );
    assert!(!path.exists());
    assert!(!std::fs::read_dir(root)
        .expect("test files")
        .flatten()
        .any(|e| e.path().extension().map(|x| x == "part").unwrap_or(false)));
}
