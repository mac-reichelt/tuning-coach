use std::{
    net::TcpListener,
    path::Path,
    process::Child,
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn sidecar_streams_telemetry_and_recommendations_to_multiple_clients() {
    let ws_port = find_free_port();
    let udp_port = find_free_port();
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .env("TUNING_COACH_UDP_LISTEN_PORT", udp_port.to_string())
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
    let udp_port = find_free_port();
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .env("TUNING_COACH_UDP_LISTEN_PORT", udp_port.to_string())
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

#[tokio::test]
async fn sidecar_emits_lap_dirty_detected_and_persists_invalid_lap() {
    let ws_port = find_free_port();
    let udp_port = find_free_port();
    let temp_data_dir = tempfile::tempdir().expect("temp dir");
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .env("TUNING_COACH_UDP_LISTEN_PORT", udp_port.to_string())
            .env("TUNING_COACH_DATA_DIR", temp_data_dir.path())
            .spawn()
            .expect("sidecar should start"),
    };

    wait_for_health(ws_port).await;

    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");
    let (mut ws_client, _) = connect_async(&ws_url).await.expect("client connects");
    let hello = next_json_text_frame(&mut ws_client).await;
    assert_eq!(hello["type"], "hello");

    let packet = wall_contact_packet_bytes();
    let udp_socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("udp socket should bind");
    udp_socket
        .send_to(&packet, format!("127.0.0.1:{udp_port}"))
        .await
        .expect("send packet");

    let dirty = wait_for_event(&mut ws_client, "lap_dirty_detected").await;
    assert_eq!(dirty["data"]["reason"]["code"], "WallContact");
    assert_eq!(dirty["data"]["reason"]["best_effort"], false);

    let db_path = temp_data_dir.path().join("tuning-coach.db");
    let row = timeout(Duration::from_secs(5), async {
        loop {
            match read_first_lap_row(&db_path) {
                Some(row) => break row,
                None => sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("lap row should be persisted");

    assert_eq!(row.0, 0);
    assert_eq!(row.1.as_deref(), Some("WallContact"));
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

fn wall_contact_packet_bytes() -> Vec<u8> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dash_packet_01.bin");
    let mut bytes = std::fs::read(fixture_path).expect("fixture should load");

    write_i32_le(&mut bytes, 0, 1);
    write_u32_le(&mut bytes, 4, 1_000);
    write_f32_le(&mut bytes, 20, 11.0 * 9.81);
    write_u16_le(&mut bytes, 300, 1);

    bytes
}

fn write_f32_le(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32_le(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u16_le(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn read_first_lap_row(db_path: &Path) -> Option<(i64, Option<String>)> {
    let conn = rusqlite::Connection::open(db_path).ok()?;
    conn.query_row(
        "SELECT valid, dirty_reason FROM laps ORDER BY id ASC LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
    )
    .ok()
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

#[tokio::test]
async fn admin_stub_recommendation_arrives_within_200ms_and_matches_schema() {
    let ws_port = find_free_port();
    let udp_port = find_free_port();
    let _sidecar = SidecarProcessGuard {
        child: std::process::Command::new(assert_cmd::cargo::cargo_bin("tuning-coach-sidecar"))
            .env("TUNING_COACH_WS_LISTEN_PORT", ws_port.to_string())
            .env("TUNING_COACH_UDP_LISTEN_PORT", udp_port.to_string())
            .spawn()
            .expect("sidecar should start"),
    };

    wait_for_health(ws_port).await;

    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");
    let (mut ws_client, _) = connect_async(&ws_url).await.expect("client connects");

    // consume hello frame
    let hello = next_json_text_frame(&mut ws_client).await;
    assert_eq!(hello["type"], "hello");

    let http_client = reqwest::Client::new();

    // record time before trigger
    let trigger_at = std::time::Instant::now();

    let response = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/admin/test/recommendation"
        ))
        .send()
        .await
        .expect("trigger stub");
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.expect("parse response");
    assert_eq!(body["emitted"], "recommendation");

    // receive the recommendation event
    let event = wait_for_event(&mut ws_client, "recommendation").await;
    let elapsed = trigger_at.elapsed();

    // timing requirement: arrives within 200ms
    assert!(
        elapsed <= Duration::from_millis(200),
        "stub recommendation took {elapsed:?} to arrive (limit: 200ms)"
    );

    // WS envelope shape (ADR-0002 + ADR-0003)
    assert_eq!(event["type"], "recommendation");
    assert_eq!(event["schema_version"], 1);
    assert!(event["t_ms"].is_number(), "envelope must include t_ms");

    let data = &event["data"];

    // core ADR-0002 fields
    assert!(data["id"].is_string());
    assert!(data["session_id"].is_string());
    assert!(data["lap_number"].is_number());
    assert_eq!(data["category"], "springs");
    assert!(data["title"].is_string());
    assert!(data["detected"].is_string());
    assert!(data["cause"].is_string());

    let adj = &data["adjustment"];
    assert!(adj.is_object());
    assert!(adj["parameter"].is_string());
    assert!(adj["from"].is_number());
    assert!(adj["to"].is_number());
    assert!(adj["step"].is_number());
    assert!(adj["unit"].is_string());

    assert_eq!(data["confidence"], "high");
    assert!(data["caveats"].is_array());
    assert!(data["alternatives"].is_array());
    assert!(data["locked_fallback_used"].is_boolean());

    // ADR-0003 additive fields
    assert!(data["corners"].is_array(), "corners[] must be present");
    assert!(
        !data["corners"].as_array().unwrap().is_empty(),
        "stub corners must be non-empty"
    );
    assert!(
        data["needs_setup_form"].is_boolean(),
        "needs_setup_form must be bool"
    );
    assert!(
        data["tire_wear_max_at_emit"].is_number(),
        "tire_wear_max_at_emit must be number"
    );
}
