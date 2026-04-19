use std::{
    net::TcpListener,
    process::Child,
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn sidecar_streams_telemetry_and_recommendations_to_multiple_clients() {
    let ws_port = find_free_port();
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .spawn()
            .expect("sidecar should start"),
    };

    wait_for_health(ws_port).await;

    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");
    let (mut ws_client_a, _) = connect_async(&ws_url).await.expect("client A connects");
    let (mut ws_client_b, _) = connect_async(&ws_url).await.expect("client B connects");

    let hello_a = next_json_text_frame(&mut ws_client_a).await;
    let hello_b = next_json_text_frame(&mut ws_client_b).await;
    assert_eq!(hello_a["type"], "hello");
    assert_eq!(hello_b["type"], "hello");
    assert_eq!(hello_a["schema_version"], 1);
    assert_eq!(hello_b["schema_version"], 1);
    assert!(hello_a["sidecar_version"].is_string());

    let http_client = reqwest::Client::new();

    let telemetry_response = http_client
        .post(format!("http://127.0.0.1:{ws_port}/test/telemetry"))
        .json(&serde_json::json!({ "data": { "speed_kph": 187.4, "gear": 4 } }))
        .send()
        .await
        .expect("inject telemetry");
    assert!(telemetry_response.status().is_success());

    let telemetry_a = wait_for_event(&mut ws_client_a, "telemetry").await;
    let telemetry_b = wait_for_event(&mut ws_client_b, "telemetry").await;
    assert_eq!(telemetry_a["data"]["speed_kph"], 187.4);
    assert_eq!(telemetry_b["data"]["gear"], 4);

    let recommendation_response = http_client
        .post(format!("http://127.0.0.1:{ws_port}/test/recommendation"))
        .json(&serde_json::json!({ "data": { "id": "rec-1", "title": "Raise front springs" } }))
        .send()
        .await
        .expect("inject recommendation");
    assert!(recommendation_response.status().is_success());

    let recommendation_a = wait_for_event(&mut ws_client_a, "recommendation").await;
    let recommendation_b = wait_for_event(&mut ws_client_b, "recommendation").await;
    assert_eq!(recommendation_a["data"]["id"], "rec-1");
    assert_eq!(recommendation_b["data"]["title"], "Raise front springs");
}

#[tokio::test]
async fn sidecar_rejects_client_schema_version_mismatch() {
    let ws_port = find_free_port();
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .spawn()
            .expect("sidecar should start"),
    };

    wait_for_health(ws_port).await;

    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");
    let (mut ws_client, _) = connect_async(&ws_url).await.expect("client connects");

    let hello = next_json_text_frame(&mut ws_client).await;
    assert_eq!(hello["type"], "hello");

    ws_client
        .send(Message::Text(
            serde_json::json!({
                "type": "ping",
                "schema_version": 999,
                "data": {}
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("send mismatched version");

    let close_frame = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(message) = ws_client.next().await {
                if let Message::Close(frame) = message.expect("ws message") {
                    break frame.expect("close frame");
                }
            }
        }
    })
    .await
    .expect("receive close frame");

    assert_eq!(u16::from(close_frame.code), 4001);
    assert!(close_frame.reason.contains("schema_version mismatch"));
}

async fn wait_for_health(ws_port: u16) {
    let health_url = format!("http://127.0.0.1:{ws_port}/health");
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => return,
            Ok(_) | Err(_) if Instant::now() < deadline => sleep(Duration::from_millis(100)).await,
            Ok(response) => panic!("health endpoint not ready: status {}", response.status()),
            Err(err) => panic!("health endpoint never became available: {err}"),
        }
    }
}

async fn next_json_text_frame(
    ws_client: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    timeout(Duration::from_secs(5), async {
        loop {
            if let Some(message) = ws_client.next().await {
                if let Message::Text(text) = message.expect("ws message") {
                    return serde_json::from_str::<serde_json::Value>(&text)
                        .expect("valid JSON message");
                }
            }
        }
    })
    .await
    .expect("receive text frame")
}

async fn wait_for_event(
    ws_client: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    event_type: &str,
) -> serde_json::Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let message = ws_client
                .next()
                .await
                .expect("websocket open")
                .expect("valid websocket message");
            if let Message::Text(text) = message {
                let parsed = serde_json::from_str::<serde_json::Value>(&text).expect("valid JSON");
                if parsed.get("type") == Some(&serde_json::Value::String(event_type.to_string())) {
                    return parsed;
                }
            }
        }
    })
    .await
    .expect("receive expected event")
}

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free local port");
    listener.local_addr().expect("local addr").port()
}

struct SidecarProcessGuard {
    child: Child,
}

impl Drop for SidecarProcessGuard {
    fn drop(&mut self) {
        if let Err(err) = self.child.kill() {
            eprintln!("failed to kill sidecar process: {err}");
        }
        if let Err(err) = self.child.wait() {
            eprintln!("failed to wait for sidecar process: {err}");
        }
    }
}
