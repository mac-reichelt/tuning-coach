use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{broadcast, watch},
    time::{interval, MissedTickBehavior},
};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info, info_span};
use tracing_subscriber::EnvFilter;

mod storage;
mod telemetry;

const SCHEMA_VERSION: u16 = 1;
const DEFAULT_TELEMETRY_HZ: u16 = 10;
const MAX_TELEMETRY_HZ: u16 = 60;
const PING_INTERVAL: Duration = Duration::from_secs(30);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Parser)]
#[command(name = "tuning-coach-sidecar", version)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    print_config: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct AppConfig {
    udp_listen_port: u16,
    ws_listen_port: u16,
    telemetry_hz: u16,
    bind_address: IpAddr,
    data_dir: PathBuf,
    log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            udp_listen_port: 7777,
            ws_listen_port: 7778,
            telemetry_hz: DEFAULT_TELEMETRY_HZ,
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            data_dir: PathBuf::from("./data"),
            log_level: "info".to_string(),
        }
    }
}

impl AppConfig {
    fn load(config_path: Option<&Path>) -> anyhow::Result<Self> {
        Self::from_sources(config_path, true).context("failed to load sidecar config")
    }

    fn from_sources(
        config_path: Option<&Path>,
        include_defaults: bool,
    ) -> Result<Self, Box<figment::Error>> {
        let mut figment = Figment::new();
        if include_defaults {
            figment = figment.merge(Serialized::defaults(Self::default()));
        }
        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        }
        figment = figment.merge(
            Env::prefixed("TUNING_COACH_").map(|key| key.as_str().to_ascii_lowercase().into()),
        );

        let config: Self = figment.extract().map_err(Box::new)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), Box<figment::Error>> {
        if self.data_dir.as_os_str().is_empty() {
            return Err(Box::new(figment::Error::from("data_dir cannot be empty")));
        }
        if !(1..=MAX_TELEMETRY_HZ).contains(&self.telemetry_hz) {
            return Err(Box::new(figment::Error::from(format!(
                "telemetry_hz must be in the range [1, {MAX_TELEMETRY_HZ}]"
            ))));
        }
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    _storage: storage::Storage,
    active_ws_connections: Arc<AtomicUsize>,
    ws_connections_tx: watch::Sender<usize>,
    latest_telemetry_tx: watch::Sender<Option<telemetry::TelemetryPacket>>,
    shutdown_tx: broadcast::Sender<()>,
    telemetry_tx: watch::Sender<Value>,
    recommendation_tx: broadcast::Sender<Value>,
    telemetry_hz: u16,
}

impl AppState {
    fn emit_telemetry(&self, payload: Value) {
        let _ = self.telemetry_tx.send(payload);
    }

    fn emit_recommendation(&self, payload: Value) {
        let _ = self.recommendation_tx.send(payload);
    }
}

struct WsConnectionGuard {
    active_ws_connections: Arc<AtomicUsize>,
    ws_connections_tx: watch::Sender<usize>,
}

impl WsConnectionGuard {
    fn new(state: &AppState) -> Self {
        let active = state.active_ws_connections.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = state.ws_connections_tx.send(active);
        Self {
            active_ws_connections: Arc::clone(&state.active_ws_connections),
            ws_connections_tx: state.ws_connections_tx.clone(),
        }
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        let active = self.active_ws_connections.fetch_sub(1, Ordering::SeqCst) - 1;
        let _ = self.ws_connections_tx.send(active);
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct HelloMessage<'a> {
    r#type: &'static str,
    schema_version: u16,
    sidecar_version: &'a str,
}

#[derive(Serialize)]
struct EventMessage<'a> {
    r#type: &'a str,
    schema_version: u16,
    data: &'a Value,
}

#[derive(Deserialize)]
struct ClientEnvelope {
    schema_version: Option<u16>,
}

#[derive(Deserialize)]
struct InjectEventRequest {
    data: Value,
}

#[derive(Serialize)]
struct InjectEventResponse {
    emitted: &'static str,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = AppConfig::load(cli.config.as_deref())?;
    init_tracing(&config)?;

    info!(
        module = module_path!(),
        crate_version = env!("CARGO_PKG_VERSION"),
        "sidecar starting"
    );

    if cli.print_config {
        println!("{}", serde_json::to_string_pretty(&config)?);
        return Ok(());
    }

    run_server(config).await
}

fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_new(&config.log_level).context("invalid log level")?;

    let fmt_layer = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true);

    if cfg!(debug_assertions) {
        fmt_layer.pretty().init();
    } else {
        fmt_layer.json().init();
    }
    Ok(())
}

async fn run_server(config: AppConfig) -> anyhow::Result<()> {
    let storage = storage::Storage::open(&config.data_dir).with_context(|| {
        format!(
            "failed to initialize sqlite storage at {:?}",
            config.data_dir
        )
    })?;
    let request_counter = Arc::new(AtomicU64::new(1));
    let (shutdown_tx, _) = broadcast::channel(16);
    let (ws_connections_tx, _) = watch::channel(0usize);
    let (latest_telemetry_tx, _) = watch::channel(None);
    let state = AppState {
        _storage: storage,
        active_ws_connections: Arc::new(AtomicUsize::new(0)),
        ws_connections_tx,
        latest_telemetry_tx,
        shutdown_tx,
        telemetry_tx: watch::channel(Value::Null).0,
        recommendation_tx: broadcast::channel(256).0,
        telemetry_hz: config.telemetry_hz,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .route("/test/telemetry", post(test_emit_telemetry))
        .route("/test/recommendation", post(test_emit_recommendation))
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http().make_span_with(
            move |request: &axum::http::Request<_>| {
                let request_span_id = request_counter.fetch_add(1, Ordering::Relaxed);
                let request_id = request
                    .headers()
                    .get(axum::http::header::HeaderName::from_static("x-request-id"))
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("missing");
                info_span!(
                    "http_request",
                    request_span_id,
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            },
        ))
        .layer(PropagateRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
        ))
        .layer(SetRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ));

    let addr = SocketAddr::from((config.bind_address, config.ws_listen_port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind ws server on {addr}"))?;
    let udp_addr = SocketAddr::from((config.bind_address, config.udp_listen_port));
    let udp_socket = UdpSocket::bind(udp_addr)
        .await
        .with_context(|| format!("failed to bind udp listener on {udp_addr}"))?;
    info!(
        module = module_path!(),
        ws_listen_port = config.ws_listen_port,
        "http/ws server listening"
    );
    info!(
        module = module_path!(),
        udp_listen_port = config.udp_listen_port,
        "udp telemetry listener bound"
    );

    let telemetry_task = tokio::spawn(telemetry::udp_listener_loop(
        udp_socket,
        state.latest_telemetry_tx.clone(),
        state.shutdown_tx.subscribe(),
    ));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .context("http/ws server exited with error")?;
    telemetry_task
        .await
        .context("udp telemetry task panicked")??;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_connection_loop(socket, state))
}

async fn ws_connection_loop(mut socket: WebSocket, state: AppState) {
    let _guard = WsConnectionGuard::new(&state);
    let mut shutdown_rx = state.shutdown_tx.subscribe();
    let mut recommendation_rx = state.recommendation_tx.subscribe();
    let hello = HelloMessage {
        r#type: "hello",
        schema_version: SCHEMA_VERSION,
        sidecar_version: env!("CARGO_PKG_VERSION"),
    };
    match serde_json::to_string(&hello) {
        Ok(payload) => {
            if socket.send(Message::Text(payload.into())).await.is_err() {
                return;
            }
        }
        Err(err) => {
            error!(module = module_path!(), %err, "failed to serialize websocket hello payload");
            return;
        }
    }

    let mut telemetry_interval = interval(Duration::from_millis(
        (1000_u64 / u64::from(state.telemetry_hz.max(1))).max(1),
    ));
    telemetry_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ping_interval = interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut idle_check_interval = interval(Duration::from_secs(5));
    idle_check_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut telemetry_rx = state.telemetry_tx.subscribe();
    let mut last_client_activity = Instant::now();

    loop {
        tokio::select! {
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Text(payload))) => {
                        last_client_activity = Instant::now();
                        if let Some(version) = extract_schema_version(&payload) {
                            if version != SCHEMA_VERSION {
                                send_close(&mut socket, 4001, format!("schema_version mismatch: server={SCHEMA_VERSION} client={version}")).await;
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        last_client_activity = Instant::now();
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_client_activity = Instant::now();
                    }
                    Some(Ok(_)) => {
                        last_client_activity = Instant::now();
                    }
                    Some(Err(err)) => {
                        error!(module = module_path!(), %err, "websocket receive error");
                        break;
                    }
                }
            }
            _ = telemetry_interval.tick() => {
                if telemetry_rx.has_changed().unwrap_or(false) {
                    let telemetry_payload = telemetry_rx.borrow_and_update().clone();
                    if !telemetry_payload.is_null() && send_event(&mut socket, "telemetry", &telemetry_payload).await.is_err() {
                        break;
                    }
                }
            }
            recommendation = recommendation_rx.recv() => {
                match recommendation {
                    Ok(payload) => {
                        if send_event(&mut socket, "recommendation", &payload).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
            _ = idle_check_interval.tick() => {
                if last_client_activity.elapsed() >= IDLE_TIMEOUT {
                    send_close(&mut socket, 1011, "idle timeout".to_string()).await;
                    break;
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

fn extract_schema_version(payload: &str) -> Option<u16> {
    serde_json::from_str::<ClientEnvelope>(payload)
        .ok()
        .and_then(|message| message.schema_version)
}

async fn send_event(socket: &mut WebSocket, message_type: &str, payload: &Value) -> Result<(), ()> {
    let message = EventMessage {
        r#type: message_type,
        schema_version: SCHEMA_VERSION,
        data: payload,
    };
    let serialized = serde_json::to_string(&message).map_err(|_| ())?;
    socket
        .send(Message::Text(serialized.into()))
        .await
        .map_err(|_| ())
}

async fn send_close(socket: &mut WebSocket, code: u16, reason: String) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

async fn test_emit_telemetry(
    State(state): State<AppState>,
    Json(request): Json<InjectEventRequest>,
) -> impl IntoResponse {
    state.emit_telemetry(request.data);
    Json(InjectEventResponse {
        emitted: "telemetry",
    })
}

async fn test_emit_recommendation(
    State(state): State<AppState>,
    Json(request): Json<InjectEventRequest>,
) -> impl IntoResponse {
    state.emit_recommendation(request.data);
    Json(InjectEventResponse {
        emitted: "recommendation",
    })
}

async fn shutdown_signal(state: AppState) {
    #[cfg(unix)]
    let mut sigterm = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        Ok(sigterm) => Some(sigterm),
        Err(err) => {
            error!(module = module_path!(), %err, "failed to register SIGTERM handler");
            None
        }
    };

    #[cfg(unix)]
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = async {
            if let Some(sigterm) = &mut sigterm {
                sigterm.recv().await;
            } else {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            }
        } => {},
    }

    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;

    info!(module = module_path!(), "shutdown signal received");
    let _ = state.shutdown_tx.send(());
    let mut ws_connections_rx = state.ws_connections_tx.subscribe();
    while *ws_connections_rx.borrow() > 0 {
        if ws_connections_rx.changed().await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use serde_json::json;
    use serial_test::serial;
    use temp_env::with_var;

    use super::{EventMessage, HelloMessage, SCHEMA_VERSION};

    #[test]
    #[serial]
    fn config_env_override_ws_port() {
        with_var("TUNING_COACH_WS_LISTEN_PORT", Some("8787"), || {
            let config = AppConfig::from_sources(None, true).expect("config should load");
            assert_eq!(config.ws_listen_port, 8787);
        });
    }

    #[test]
    fn config_missing_required_without_defaults_fails() {
        let result = AppConfig::from_sources(None, false);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn config_invalid_port_fails() {
        with_var("TUNING_COACH_WS_LISTEN_PORT", Some("70000"), || {
            let result = AppConfig::from_sources(None, true);
            assert!(result.is_err());
        });
    }

    #[test]
    fn hello_message_serialization_contains_schema_version() {
        let hello = HelloMessage {
            r#type: "hello",
            schema_version: SCHEMA_VERSION,
            sidecar_version: "0.1.0",
        };
        let serialized = serde_json::to_value(hello).expect("serialize hello");
        assert_eq!(
            serialized,
            json!({
                "type": "hello",
                "schema_version": 1,
                "sidecar_version": "0.1.0"
            })
        );
    }

    #[test]
    fn event_message_serialization_contains_envelope() {
        let payload = json!({ "speed_kph": 123.4 });
        let event = EventMessage {
            r#type: "telemetry",
            schema_version: SCHEMA_VERSION,
            data: &payload,
        };
        let serialized = serde_json::to_value(event).expect("serialize event");
        assert_eq!(
            serialized,
            json!({
                "type": "telemetry",
                "schema_version": 1,
                "data": { "speed_kph": 123.4 }
            })
        );
    }
}
