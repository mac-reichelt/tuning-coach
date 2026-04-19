use std::{
    net::TcpListener,
    time::{Duration, Instant},
};

use tokio::time::sleep;

#[tokio::test]
async fn sidecar_health_endpoint_returns_ok() {
    let ws_port = find_free_port();
    let mut child =
        std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .spawn()
            .expect("sidecar should start");

    let health_url = format!("http://127.0.0.1:{ws_port}/health");
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    let response = loop {
        match client.get(&health_url).send().await {
            Ok(response) => break response,
            Err(err) if Instant::now() < deadline => {
                eprintln!("health endpoint not ready yet: {err}");
                sleep(Duration::from_millis(100)).await;
            }
            Err(err) => panic!("health endpoint never became available: {err}"),
        }
    };

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = response.json().await.expect("health JSON body");
    assert_eq!(body, serde_json::json!({ "status": "ok" }));

    if let Err(err) = child.kill() {
        eprintln!("failed to kill sidecar process: {err}");
    }
    child.wait().expect("sidecar process should exit");
}

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free local port");
    listener.local_addr().expect("local addr").port()
}
