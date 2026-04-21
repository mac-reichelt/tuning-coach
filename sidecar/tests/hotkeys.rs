use std::{
    net::TcpListener,
    path::Path,
    process::Child,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn sidecar_hotkey_endpoints_emit_events_and_persist_hotkey_rows() {
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

    let http_client = reqwest::Client::new();
    let ws_url = format!("ws://127.0.0.1:{ws_port}/ws");
    let (mut ws_client, _) = connect_async(&ws_url).await.expect("client connects");
    let hello = next_json_text_frame(&mut ws_client).await;
    assert_eq!(hello["type"], "hello");

    let no_session = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/mark-lap-dirty"
        ))
        .send()
        .await
        .expect("request should complete");
    assert_eq!(
        no_session.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );

    send_udp_packet(udp_port, race_packet_bytes(1_000, 1.0, 1)).await;
    sleep(Duration::from_millis(200)).await;

    let dirty_response: serde_json::Value = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/mark-lap-dirty"
        ))
        .send()
        .await
        .expect("mark dirty")
        .error_for_status()
        .expect("mark dirty should succeed")
        .json()
        .await
        .expect("mark dirty response json");
    assert_eq!(dirty_response["lap_number"], 1);
    assert!(dirty_response["marked_dirty_at"].is_string());
    let dirty_event = wait_for_event(&mut ws_client, "lap_dirty_detected").await;
    assert_eq!(dirty_event["data"]["reason"]["code"], "ManualOverride");

    let clean_response: serde_json::Value = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/mark-lap-clean"
        ))
        .send()
        .await
        .expect("mark clean")
        .error_for_status()
        .expect("mark clean should succeed")
        .json()
        .await
        .expect("mark clean response json");
    assert_eq!(clean_response, serde_json::json!({ "lap_number": 1 }));
    let clean_event = wait_for_event(&mut ws_client, "lap_clean_marked").await;
    assert_eq!(clean_event["data"]["lap_number"], 1);

    let not_in_pit = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/force-pit-end"
        ))
        .send()
        .await
        .expect("force pit end request");
    assert_eq!(not_in_pit.status(), reqwest::StatusCode::CONFLICT);

    let pit_start: serde_json::Value = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/force-pit-start"
        ))
        .send()
        .await
        .expect("force pit start")
        .error_for_status()
        .expect("pit start should succeed")
        .json()
        .await
        .expect("pit start response json");
    assert_eq!(pit_start["lap_number"], 1);
    assert!(pit_start["session_id"].is_number());
    let pit_start_event = wait_for_event(&mut ws_client, "pit_stop_started").await;
    assert_eq!(pit_start_event["data"]["lap_number"], 1);

    let already_in_pit = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/force-pit-start"
        ))
        .send()
        .await
        .expect("force pit start while in pit");
    assert_eq!(already_in_pit.status(), reqwest::StatusCode::CONFLICT);

    send_udp_packet(udp_port, race_packet_bytes(4_000, 4.0, 1)).await;
    sleep(Duration::from_millis(100)).await;
    let pit_end: serde_json::Value = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/force-pit-end"
        ))
        .send()
        .await
        .expect("force pit end")
        .error_for_status()
        .expect("pit end should succeed")
        .json()
        .await
        .expect("pit end response json");
    let duration_s = pit_end["duration_s"]
        .as_f64()
        .expect("duration_s should be a float");
    assert!(duration_s >= 3.0);
    let pit_end_event = wait_for_event(&mut ws_client, "pit_stop_ended").await;
    assert!(
        pit_end_event["data"]["duration_s"]
            .as_f64()
            .unwrap_or_default()
            >= 3.0
    );

    let session_boundary: serde_json::Value = http_client
        .post(format!(
            "http://127.0.0.1:{ws_port}/api/v1/hotkeys/force-session-boundary"
        ))
        .send()
        .await
        .expect("force session boundary")
        .error_for_status()
        .expect("session boundary should succeed")
        .json()
        .await
        .expect("session boundary response json");
    assert_ne!(
        session_boundary["prior_session_id"],
        session_boundary["new_session_id"]
    );
    let reset_event = wait_for_event(&mut ws_client, "session_reset_detected").await;
    assert_eq!(
        reset_event["data"]["prior_session_id"],
        session_boundary["prior_session_id"]
    );
    assert_eq!(
        reset_event["data"]["new_session_id"],
        session_boundary["new_session_id"]
    );

    let rows = timeout(Duration::from_secs(5), async {
        loop {
            let rows = read_hotkey_rows(&temp_data_dir.path().join("tuning-coach.db"));
            if rows.len() >= 5 {
                break rows;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("hotkey rows should be persisted");

    let actions: Vec<String> = rows.iter().map(|row| row.1.clone()).collect();
    assert_eq!(
        actions,
        vec![
            "mark_lap_dirty",
            "mark_lap_clean",
            "force_pit_start",
            "force_pit_end",
            "force_session_boundary"
        ]
    );
    for (_session_id, _action, payload_json, t_ms) in rows {
        assert!(!payload_json.is_empty());
        assert!(t_ms.is_some());
    }
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

async fn send_udp_packet(udp_port: u16, packet: Vec<u8>) {
    let udp_socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("udp socket should bind");
    udp_socket
        .send_to(&packet, format!("127.0.0.1:{udp_port}"))
        .await
        .expect("send packet");
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

fn read_hotkey_rows(db_path: &Path) -> Vec<(i64, String, String, Option<i64>)> {
    let Ok(conn) = rusqlite::Connection::open(db_path) else {
        return Vec::new();
    };
    let mut statement = conn
        .prepare(
            "SELECT session_id, action, payload_json, t_ms
               FROM hotkey_events
              ORDER BY id ASC",
        )
        .expect("prepare query");
    statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })
        .expect("query rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect rows")
}

fn race_packet_bytes(timestamp_ms: u32, current_race_time_s: f32, lap_number: u16) -> Vec<u8> {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dash_packet_01.bin");
    let mut bytes = std::fs::read(fixture_path).expect("fixture should load");

    write_i32_le(&mut bytes, 0, 1);
    write_u32_le(&mut bytes, 4, timestamp_ms);
    write_f32_le(&mut bytes, 20, 0.0);
    write_f32_le(&mut bytes, 24, 0.0);
    write_f32_le(&mut bytes, 28, 0.0);
    write_i32_le(&mut bytes, 256, 42);
    write_f32_le(&mut bytes, 244, 30.0);
    write_f32_le(&mut bytes, 296, current_race_time_s);
    write_u16_le(&mut bytes, 300, lap_number);

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
