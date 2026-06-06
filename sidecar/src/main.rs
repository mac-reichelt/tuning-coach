#![recursion_limit = "256"]

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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

mod hotkeys;
mod lap_validity;
mod overlay;
mod recommendation;
mod session_state;
mod storage;
mod telemetry;

const SCHEMA_VERSION: u16 = 1;
const DEFAULT_TELEMETRY_HZ: u16 = 10;
const MAX_TELEMETRY_HZ: u16 = 60;
const DEFAULT_PAUSE_DEBOUNCE_MS: u64 = 2_000;
const DEFAULT_PACKET_TIMEOUT_MS: u64 = 10_000;
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct AppConfig {
    udp_listen_port: u16,
    ws_listen_port: u16,
    telemetry_hz: u16,
    rewind_backward_jump_m: f32,
    session_reset_race_time_window_s: f32,
    pit_entry_speed_threshold_kph: f32,
    pit_entry_dwell_s: f32,
    pit_exit_speed_threshold_kph: f32,
    pit_exit_dwell_s: f32,
    off_track_window_ms: u32,
    off_track_min_wheels: u8,
    surface_rumble_threshold: f32,
    surface_rumble_window_packets: usize,
    wall_contact_g_threshold: f32,
    corner_cut_speed_kph_min: f32,
    corner_cut_combined_slip_threshold: f32,
    corner_cut_max_abs_steer_norm: f32,
    pause_debounce_ms: u64,
    packet_timeout_ms: u64,
    bind_address: IpAddr,
    data_dir: PathBuf,
    log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let defaults = lap_validity::LapValidityConfig::default();
        Self {
            udp_listen_port: 7777,
            ws_listen_port: 7778,
            telemetry_hz: DEFAULT_TELEMETRY_HZ,
            rewind_backward_jump_m: defaults.rewind_backward_jump_m,
            session_reset_race_time_window_s: defaults.session_reset_race_time_window_s,
            pit_entry_speed_threshold_kph: defaults.pit_entry_speed_threshold_kph,
            pit_entry_dwell_s: defaults.pit_entry_dwell_s,
            pit_exit_speed_threshold_kph: defaults.pit_exit_speed_threshold_kph,
            pit_exit_dwell_s: defaults.pit_exit_dwell_s,
            off_track_window_ms: defaults.off_track_window_ms,
            off_track_min_wheels: defaults.off_track_min_wheels,
            surface_rumble_threshold: defaults.surface_rumble_threshold,
            surface_rumble_window_packets: defaults.surface_rumble_window_packets,
            wall_contact_g_threshold: defaults.wall_contact_g_threshold,
            corner_cut_speed_kph_min: defaults.corner_cut_speed_kph_min,
            corner_cut_combined_slip_threshold: defaults.corner_cut_combined_slip_threshold,
            corner_cut_max_abs_steer_norm: defaults.corner_cut_max_abs_steer_norm,
            pause_debounce_ms: DEFAULT_PAUSE_DEBOUNCE_MS,
            packet_timeout_ms: DEFAULT_PACKET_TIMEOUT_MS,
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
        lap_validity::LapValidityConfig {
            rewind_backward_jump_m: self.rewind_backward_jump_m,
            session_reset_race_time_window_s: self.session_reset_race_time_window_s,
            pit_entry_speed_threshold_kph: self.pit_entry_speed_threshold_kph,
            pit_entry_dwell_s: self.pit_entry_dwell_s,
            pit_exit_speed_threshold_kph: self.pit_exit_speed_threshold_kph,
            pit_exit_dwell_s: self.pit_exit_dwell_s,
            off_track_window_ms: self.off_track_window_ms,
            off_track_min_wheels: self.off_track_min_wheels,
            surface_rumble_threshold: self.surface_rumble_threshold,
            surface_rumble_window_packets: self.surface_rumble_window_packets,
            wall_contact_g_threshold: self.wall_contact_g_threshold,
            corner_cut_speed_kph_min: self.corner_cut_speed_kph_min,
            corner_cut_combined_slip_threshold: self.corner_cut_combined_slip_threshold,
            corner_cut_max_abs_steer_norm: self.corner_cut_max_abs_steer_norm,
        }
        .validate()
        .map_err(|err| Box::new(figment::Error::from(err)))?;
        if self.pause_debounce_ms == 0 {
            return Err(Box::new(figment::Error::from(
                "pause_debounce_ms must be greater than 0",
            )));
        }
        if self.packet_timeout_ms == 0 {
            return Err(Box::new(figment::Error::from(
                "packet_timeout_ms must be greater than 0",
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PitStopRuntime {
    pub(crate) session_id: i64,
    pub(crate) lap_number: u16,
    pub(crate) started_at_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LapContext {
    pub(crate) lap_number: u16,
    pub(crate) at_ms: u32,
    pub(crate) car_ordinal: i32,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) storage: storage::Storage,
    active_ws_connections: Arc<AtomicUsize>,
    ws_connections_tx: watch::Sender<usize>,
    latest_telemetry_tx: watch::Sender<Option<telemetry::TelemetryPacket>>,
    shutdown_tx: broadcast::Sender<()>,
    telemetry_tx: watch::Sender<Value>,
    recommendation_tx: broadcast::Sender<Value>,
    lap_validity_tx: broadcast::Sender<lap_validity::LapValidityEvent>,
    pit_runtime: Arc<Mutex<Option<PitStopRuntime>>>,
    suppress_heuristics_tx: watch::Sender<bool>,
    session_state_tx: broadcast::Sender<session_state::SessionStateChanged>,
    dyno_tx: broadcast::Sender<Value>,
    dyno_reset_tx: broadcast::Sender<()>,
    telemetry_hz: u16,
}

impl AppState {
    fn emit_telemetry(&self, payload: Value) {
        let _ = self.telemetry_tx.send(payload);
    }

    fn emit_recommendation(&self, payload: Value) {
        if *self.suppress_heuristics_tx.borrow() {
            return;
        }
        let _ = self.recommendation_tx.send(payload);
    }

    fn emit_dyno_update(&self, payload: Value) {
        let _ = self.dyno_tx.send(payload);
    }

    pub(crate) fn current_lap_context(&self) -> Option<LapContext> {
        match self.latest_telemetry_tx.borrow().clone() {
            Some(telemetry::TelemetryPacket::Dash(packet)) => Some(LapContext {
                lap_number: packet.lap_number,
                at_ms: packet.sled.timestamp_ms,
                car_ordinal: packet.sled.car_ordinal,
            }),
            _ => None,
        }
    }

    pub(crate) fn pit_runtime(&self) -> Option<PitStopRuntime> {
        *self.pit_runtime.lock().expect("pit runtime lock")
    }

    pub(crate) fn clear_pit_runtime(&self) {
        *self.pit_runtime.lock().expect("pit runtime lock") = None;
    }

    pub(crate) fn emit_lap_validity_event(&self, event: lap_validity::LapValidityEvent) {
        {
            let mut pit_runtime = self.pit_runtime.lock().expect("pit runtime lock");
            match &event {
                lap_validity::LapValidityEvent::PitStopStarted {
                    session_id,
                    lap_number,
                    at_ms,
                } => {
                    *pit_runtime = Some(PitStopRuntime {
                        session_id: *session_id,
                        lap_number: *lap_number,
                        started_at_ms: *at_ms,
                    });
                }
                lap_validity::LapValidityEvent::PitStopEnded { .. }
                | lap_validity::LapValidityEvent::SessionResetDetected { .. } => {
                    *pit_runtime = None;
                }
                _ => {}
            }
        }
        let _ = self.lap_validity_tx.send(event);
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
    t_ms: u64,
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
        storage,
        active_ws_connections: Arc::new(AtomicUsize::new(0)),
        ws_connections_tx,
        latest_telemetry_tx,
        shutdown_tx,
        telemetry_tx: watch::channel(Value::Null).0,
        recommendation_tx: broadcast::channel(256).0,
        lap_validity_tx: broadcast::channel(256).0,
        pit_runtime: Arc::new(Mutex::new(None)),
        suppress_heuristics_tx: watch::channel(false).0,
        session_state_tx: broadcast::channel(256).0,
        dyno_tx: broadcast::channel(64).0,
        dyno_reset_tx: broadcast::channel(16).0,
        telemetry_hz: config.telemetry_hz,
    };

    let app = Router::new()
        .route("/", get(overlay::index))
        .route("/src/{*path}", get(overlay::src_asset))
        .route("/styles/{*path}", get(overlay::styles_asset))
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .route("/test/telemetry", post(test_emit_telemetry))
        .route("/test/recommendation", post(test_emit_recommendation))
        .route(
            "/admin/test/recommendation",
            post(admin_test_emit_recommendation),
        )
        .nest("/api/v1/hotkeys", hotkeys::router())
        .route("/api/v1/dyno/reset", post(dyno_reset))
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
    let lap_validity_task = tokio::spawn(lap_validity_loop(
        state.clone(),
        lap_validity::LapValidityConfig {
            rewind_backward_jump_m: config.rewind_backward_jump_m,
            session_reset_race_time_window_s: config.session_reset_race_time_window_s,
            pit_entry_speed_threshold_kph: config.pit_entry_speed_threshold_kph,
            pit_entry_dwell_s: config.pit_entry_dwell_s,
            pit_exit_speed_threshold_kph: config.pit_exit_speed_threshold_kph,
            pit_exit_dwell_s: config.pit_exit_dwell_s,
            off_track_window_ms: config.off_track_window_ms,
            off_track_min_wheels: config.off_track_min_wheels,
            surface_rumble_threshold: config.surface_rumble_threshold,
            surface_rumble_window_packets: config.surface_rumble_window_packets,
            wall_contact_g_threshold: config.wall_contact_g_threshold,
            corner_cut_speed_kph_min: config.corner_cut_speed_kph_min,
            corner_cut_combined_slip_threshold: config.corner_cut_combined_slip_threshold,
            corner_cut_max_abs_steer_norm: config.corner_cut_max_abs_steer_norm,
        },
    ));
    let session_state_task = tokio::spawn(session_state::session_state_loop(
        state.latest_telemetry_tx.subscribe(),
        state.session_state_tx.clone(),
        state.storage.clone(),
        state.shutdown_tx.subscribe(),
        session_state::SessionStateMachineConfig {
            pause_debounce: Duration::from_millis(config.pause_debounce_ms),
            packet_timeout: Duration::from_millis(config.packet_timeout_ms),
        },
        env!("CARGO_PKG_VERSION"),
    ));
    let telemetry_bridge_task = tokio::spawn(telemetry_bridge_loop(state.clone()));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .context("http/ws server exited with error")?;
    telemetry_task
        .await
        .context("udp telemetry task panicked")??;
    lap_validity_task
        .await
        .context("lap validity task panicked")??;
    session_state_task
        .await
        .context("session state task panicked")??;
    telemetry_bridge_task
        .await
        .context("telemetry bridge task panicked")??;

    Ok(())
}

async fn lap_validity_loop(
    state: AppState,
    config: lap_validity::LapValidityConfig,
) -> anyhow::Result<()> {
    let mut detector = lap_validity::LapValidityDetector::new(config);
    let mut packet_rx = state.latest_telemetry_tx.subscribe();
    let mut shutdown_rx = state.shutdown_tx.subscribe();
    let mut session_state_rx = state.session_state_tx.subscribe();
    let sidecar_version = env!("CARGO_PKG_VERSION");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            session_change = session_state_rx.recv() => {
                match session_change {
                    Ok(change) if change.to == session_state::SessionState::Finished => {
                        let events = detector.finalize_at_ms(change.at_ms as u32)?;
                        for event in events {
                            state.emit_lap_validity_event(event);
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            changed = packet_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let packet = packet_rx.borrow_and_update().clone();
                let Some(telemetry::TelemetryPacket::Dash(dash_packet)) = packet else {
                    continue;
                };
                let events = detector.process_packet(&dash_packet, &state.storage, sidecar_version)?;
                let _ = state.suppress_heuristics_tx.send(detector.suppress_current_lap_analysis());
                for event in events {
                    state.emit_lap_validity_event(event);
                }
            }
        }
    }
    let events = detector.finalize()?;
    for event in events {
        state.emit_lap_validity_event(event);
    }

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

async fn dyno_reset(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.dyno_reset_tx.send(());
    axum::http::StatusCode::OK
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.protocols(["tuning-coach.v1"])
        .on_upgrade(move |socket| ws_connection_loop(socket, state))
}

async fn ws_connection_loop(mut socket: WebSocket, state: AppState) {
    let _guard = WsConnectionGuard::new(&state);
    let mut shutdown_rx = state.shutdown_tx.subscribe();
    let mut recommendation_rx = state.recommendation_tx.subscribe();
    let mut lap_validity_rx = state.lap_validity_tx.subscribe();
    let mut session_state_rx = state.session_state_tx.subscribe();
    let mut dyno_rx = state.dyno_tx.subscribe();
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
            lap_event = lap_validity_rx.recv() => {
                match lap_event {
                    Ok(event) => {
                        let event_type = match event {
                            lap_validity::LapValidityEvent::LapRewindDetected {..} => "lap_rewind_detected",
                            lap_validity::LapValidityEvent::LapDirtyDetected {..} => "lap_dirty_detected",
                            lap_validity::LapValidityEvent::LapCleanMarked {..} => "lap_clean_marked",
                            lap_validity::LapValidityEvent::SessionResetDetected {..} => "session_reset_detected",
                            lap_validity::LapValidityEvent::PitStopStarted {..} => "pit_stop_started",
                            lap_validity::LapValidityEvent::PitStopEnded {..} => "pit_stop_ended",
                        };
                        if let Ok(payload) = serde_json::to_value(event) {
                            if send_event(&mut socket, event_type, &payload).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            session_ev = session_state_rx.recv() => {
                match session_ev {
                    Ok(change) => {
                        let maybe_type = match change.to {
                            session_state::SessionState::InRace => Some("session_started"),
                            session_state::SessionState::Finished => Some("session_ended"),
                            _ => None,
                        };
                        if let Some(event_type) = maybe_type {
                            let payload = serde_json::json!({
                                "session_id": change.session_id,
                                "at_ms": change.at_ms,
                            });
                            if send_event(&mut socket, event_type, &payload).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            dyno_ev = dyno_rx.recv() => {
                match dyno_ev {
                    Ok(payload) => {
                        if send_event(&mut socket, "dyno_update", &payload).await.is_err() {
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
    let t_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|err| {
            error!(
                module = module_path!(),
                %err,
                "system clock is before Unix epoch; t_ms will be zero"
            );
            Duration::ZERO
        })
        .as_millis() as u64;
    let message = EventMessage {
        r#type: message_type,
        schema_version: SCHEMA_VERSION,
        t_ms,
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
    // Drain messages until the peer echoes the close frame, completing the
    // WebSocket close handshake before we drop the socket.  Without this,
    // Windows drops the underlying TCP connection with a RST
    // (WSAECONNRESET / error 10054) before the client can read the close
    // frame, causing the test to panic instead of receiving CloseFrame(4001).
    let drain = async {
        loop {
            match socket.recv().await {
                Some(Ok(Message::Close(_))) | None => break,
                Some(_) => {}
            }
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(2), drain).await;
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

/// Emits a canonical stub [`recommendation::RecommendationPayload`] into the WS
/// fan-out channel.  Intended for overlay renderer development and integration
/// tests.  See `docs/adr/0003-phase3-recommendation-payload-extensions.md`.
async fn admin_test_emit_recommendation(State(state): State<AppState>) -> impl IntoResponse {
    let stub = recommendation::stub_recommendation();
    let payload = serde_json::to_value(&stub)
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    state.emit_recommendation(payload);
    Json(InjectEventResponse {
        emitted: "recommendation",
    })
}

// ── Telemetry bridge ────────────────────────────────────────

/// Convert a non-finite f32 to 0.0 for HUD display fields.
#[inline]
fn jf(v: f32) -> f64 {
    if v.is_finite() {
        f64::from(v)
    } else {
        0.0
    }
}

/// Convert an f32 to a JSON number, or null if non-finite.
#[inline]
fn raw_f(v: f32) -> Value {
    serde_json::Number::from_f64(f64::from(v))
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

/// Convert an Option<f32> to a JSON number or null.
#[inline]
fn raw_fo(v: Option<f32>) -> Value {
    v.and_then(|f| serde_json::Number::from_f64(f64::from(f)))
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

/// Map a lap validity event to the overlay lap_status string.
fn lap_event_to_status(event: &lap_validity::LapValidityEvent) -> &'static str {
    match event {
        lap_validity::LapValidityEvent::LapDirtyDetected { .. } => "dirty",
        lap_validity::LapValidityEvent::LapRewindDetected { .. } => "dirty",
        lap_validity::LapValidityEvent::LapCleanMarked { .. } => "valid",
        lap_validity::LapValidityEvent::PitStopStarted { .. } => "pit",
        lap_validity::LapValidityEvent::PitStopEnded { .. } => "valid",
        lap_validity::LapValidityEvent::SessionResetDetected { .. } => "reset",
    }
}

// ── Dyno collector ───────────────────────────────────────────

const DYNO_BIN_RPM: u32 = 50;
const DYNO_STOP_SECS: f32 = 3.0;
const DYNO_THROTTLE_MIN: u8 = 242;
const DYNO_REDLINE_DROP_RATIO: f32 = 0.03;
const DYNO_POWER_BAND_FRAC: f32 = 0.80;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DynoPhase {
    WaitingForReady,
    ReadyToGo,
    Collecting,
    Complete,
}

struct DynoCollector {
    phase: DynoPhase,
    car_ordinal: Option<i32>,
    drivetrain: i32,
    target_gear: u8,
    stopped_secs: f32,
    bins: BTreeMap<u32, (f32, f32)>,
    prev_rpm: f32,
    peak_rpm: f32,
    collecting_gear: u8,
    pub detected_redline: Option<f32>,
    pub power_band_start: Option<f32>,
}

impl DynoCollector {
    fn new() -> Self {
        Self {
            phase: DynoPhase::WaitingForReady,
            car_ordinal: None,
            drivetrain: 2,
            target_gear: 1,
            stopped_secs: 0.0,
            bins: BTreeMap::new(),
            prev_rpm: 0.0,
            peak_rpm: 0.0,
            collecting_gear: 1,
            detected_redline: None,
            power_band_start: None,
        }
    }

    fn target_gear_for_drivetrain(dt: i32) -> u8 {
        if dt == 1 {
            2
        } else {
            1
        }
    }

    fn reset_to_waiting(&mut self) {
        self.phase = DynoPhase::WaitingForReady;
        self.bins.clear();
        self.stopped_secs = 0.0;
        self.peak_rpm = 0.0;
        self.prev_rpm = 0.0;
        self.detected_redline = None;
        self.power_band_start = None;
    }

    fn compute_derived(&mut self) {
        if self.bins.is_empty() {
            return;
        }
        let peak_power = self.bins.values().map(|(p, _)| *p).fold(0.0f32, f32::max);
        if peak_power <= 0.0 {
            return;
        }
        let threshold = peak_power * DYNO_POWER_BAND_FRAC;
        self.power_band_start = self
            .bins
            .iter()
            .find(|(_, (p, _))| *p >= threshold)
            .map(|(rpm, _)| *rpm as f32);
    }

    fn phase_name(&self) -> &'static str {
        match self.phase {
            DynoPhase::WaitingForReady => "waiting_for_ready",
            DynoPhase::ReadyToGo => "ready_to_go",
            DynoPhase::Collecting => "collecting",
            DynoPhase::Complete => "complete",
        }
    }

    fn dyno_status_fields(&self) -> Value {
        serde_json::json!({
            "phase": self.phase_name(),
            "target_gear": self.target_gear,
            "drivetrain": self.drivetrain,
            "detected_redline_rpm": self.detected_redline.map(|r| r as u32),
            "power_band_start_rpm": self.power_band_start.map(|r| r as u32),
            "stopped_progress": (self.stopped_secs / DYNO_STOP_SECS).min(1.0),
        })
    }

    fn to_update_payload(&self) -> Value {
        let bins_arr: Vec<Value> = self
            .bins
            .iter()
            .map(|(rpm, (pw, tq))| {
                serde_json::json!({
                    "rpm": rpm,
                    "power_w": raw_f(*pw),
                    "torque_nm": raw_f(*tq),
                })
            })
            .collect();
        serde_json::json!({
            "phase": self.phase_name(),
            "target_gear": self.target_gear,
            "drivetrain": self.drivetrain,
            "detected_redline_rpm": self.detected_redline.map(|r| r as u32),
            "power_band_start_rpm": self.power_band_start.map(|r| r as u32),
            "bins": bins_arr,
        })
    }

    /// Returns true if a dyno_update WS message should be emitted.
    fn update(&mut self, dash: &telemetry::DashPacket, dt_secs: f32) -> bool {
        let sled = &dash.sled;
        let rpm = sled.current_engine_rpm;
        let speed = dash.speed;
        let gear = dash.gear;
        let accel = dash.accel;
        let car_ordinal = sled.car_ordinal;
        let drivetrain = sled.drivetrain_type;

        // New car — reset everything
        if Some(car_ordinal) != self.car_ordinal {
            self.car_ordinal = Some(car_ordinal);
            self.drivetrain = drivetrain;
            self.target_gear = Self::target_gear_for_drivetrain(drivetrain);
            self.reset_to_waiting();
            return true;
        }

        let prev_phase = self.phase.clone();
        let mut bins_changed = false;

        match self.phase {
            DynoPhase::WaitingForReady => {
                // < 0.05 m/s ≈ 0.18 kph — treats near-zero as stopped
                if speed.abs() < 0.05 && gear == self.target_gear {
                    self.stopped_secs += dt_secs;
                    if self.stopped_secs >= DYNO_STOP_SECS {
                        self.phase = DynoPhase::ReadyToGo;
                    }
                } else {
                    self.stopped_secs = 0.0;
                }
            }
            DynoPhase::ReadyToGo => {
                if speed.abs() >= 0.05 || gear != self.target_gear {
                    self.stopped_secs = 0.0;
                    self.phase = DynoPhase::WaitingForReady;
                } else if accel >= DYNO_THROTTLE_MIN {
                    self.phase = DynoPhase::Collecting;
                    self.collecting_gear = gear;
                    self.peak_rpm = rpm;
                    self.prev_rpm = rpm;
                }
            }
            DynoPhase::Collecting => {
                if gear != self.collecting_gear {
                    // Gear change aborts the pull
                    self.stopped_secs = 0.0;
                    self.phase = DynoPhase::WaitingForReady;
                } else if accel >= DYNO_THROTTLE_MIN {
                    if rpm >= self.prev_rpm * (1.0 - DYNO_REDLINE_DROP_RATIO) {
                        // RPM rising or stable — valid sample
                        let bucket = ((rpm / DYNO_BIN_RPM as f32).round() as u32) * DYNO_BIN_RPM;
                        let entry = self.bins.entry(bucket).or_insert((0.0, 0.0));
                        if dash.power > entry.0 || dash.torque > entry.1 {
                            if dash.power > entry.0 {
                                entry.0 = dash.power;
                            }
                            if dash.torque > entry.1 {
                                entry.1 = dash.torque;
                            }
                            bins_changed = true;
                        }
                        if rpm > self.peak_rpm {
                            self.peak_rpm = rpm;
                        }
                    } else {
                        // RPM dropped ≥3% at full throttle = limiter bounce → complete
                        self.detected_redline = Some(self.peak_rpm);
                        self.compute_derived();
                        self.phase = DynoPhase::Complete;
                        bins_changed = true;
                    }
                }
                self.prev_rpm = rpm;
            }
            DynoPhase::Complete => {}
        }

        bins_changed || self.phase != prev_phase
    }
}

/// Convert a [`DashPacket`] to the overlay WS telemetry JSON schema.
///
/// All f32 fields are sanitized: non-finite values become 0.0 in HUD fields
/// and `null` in raw fields to avoid JSON serialization panics.
fn dash_to_overlay_json(dash: &telemetry::DashPacket, lap_status: &str, dyno: &Value) -> Value {
    let s = &dash.sled;

    // Build the raw block imperatively to avoid serde_json::json! recursion limits.
    let mut raw = serde_json::Map::with_capacity(80);
    macro_rules! ri {
        ($k:expr, $v:expr) => {
            raw.insert($k.to_string(), Value::from($v));
        };
    }
    macro_rules! rf {
        ($k:expr, $v:expr) => {
            raw.insert($k.to_string(), raw_f($v));
        };
    }
    macro_rules! rfo {
        ($k:expr, $v:expr) => {
            raw.insert($k.to_string(), raw_fo($v));
        };
    }

    ri!("timestamp_ms", s.timestamp_ms);
    rf!("engine_max_rpm", s.engine_max_rpm);
    rf!("engine_idle_rpm", s.engine_idle_rpm);
    rf!("current_engine_rpm", s.current_engine_rpm);
    rf!("accel_x", s.acceleration_x);
    rf!("accel_y", s.acceleration_y);
    rf!("accel_z", s.acceleration_z);
    rf!("vel_x", s.velocity_x);
    rf!("vel_y", s.velocity_y);
    rf!("vel_z", s.velocity_z);
    rf!("ang_vel_x", s.angular_velocity_x);
    rf!("ang_vel_y", s.angular_velocity_y);
    rf!("ang_vel_z", s.angular_velocity_z);
    rf!("yaw", s.yaw);
    rf!("pitch", s.pitch);
    rf!("roll", s.roll);
    rf!("susp_norm_fl", s.normalized_suspension_travel_front_left);
    rf!("susp_norm_fr", s.normalized_suspension_travel_front_right);
    rf!("susp_norm_rl", s.normalized_suspension_travel_rear_left);
    rf!("susp_norm_rr", s.normalized_suspension_travel_rear_right);
    rf!("tire_slip_ratio_fl", s.tire_slip_ratio_front_left);
    rf!("tire_slip_ratio_fr", s.tire_slip_ratio_front_right);
    rf!("tire_slip_ratio_rl", s.tire_slip_ratio_rear_left);
    rf!("tire_slip_ratio_rr", s.tire_slip_ratio_rear_right);
    rf!("wheel_rot_speed_fl", s.wheel_rotation_speed_front_left);
    rf!("wheel_rot_speed_fr", s.wheel_rotation_speed_front_right);
    rf!("wheel_rot_speed_rl", s.wheel_rotation_speed_rear_left);
    rf!("wheel_rot_speed_rr", s.wheel_rotation_speed_rear_right);
    ri!("on_rumble_fl", s.wheel_on_rumble_strip_front_left);
    ri!("on_rumble_fr", s.wheel_on_rumble_strip_front_right);
    ri!("on_rumble_rl", s.wheel_on_rumble_strip_rear_left);
    ri!("on_rumble_rr", s.wheel_on_rumble_strip_rear_right);
    rf!("puddle_fl", s.wheel_in_puddle_depth_front_left);
    rf!("puddle_fr", s.wheel_in_puddle_depth_front_right);
    rf!("puddle_rl", s.wheel_in_puddle_depth_rear_left);
    rf!("puddle_rr", s.wheel_in_puddle_depth_rear_right);
    rf!("surface_rumble_fl", s.surface_rumble_front_left);
    rf!("surface_rumble_fr", s.surface_rumble_front_right);
    rf!("surface_rumble_rl", s.surface_rumble_rear_left);
    rf!("surface_rumble_rr", s.surface_rumble_rear_right);
    rf!("slip_angle_fl", s.tire_slip_angle_front_left);
    rf!("slip_angle_fr", s.tire_slip_angle_front_right);
    rf!("slip_angle_rl", s.tire_slip_angle_rear_left);
    rf!("slip_angle_rr", s.tire_slip_angle_rear_right);
    rf!("combined_slip_fl", s.tire_combined_slip_front_left);
    rf!("combined_slip_fr", s.tire_combined_slip_front_right);
    rf!("combined_slip_rl", s.tire_combined_slip_rear_left);
    rf!("combined_slip_rr", s.tire_combined_slip_rear_right);
    rf!("susp_travel_m_fl", s.suspension_travel_meters_front_left);
    rf!("susp_travel_m_fr", s.suspension_travel_meters_front_right);
    rf!("susp_travel_m_rl", s.suspension_travel_meters_rear_left);
    rf!("susp_travel_m_rr", s.suspension_travel_meters_rear_right);
    ri!("car_ordinal", s.car_ordinal);
    ri!("car_class", s.car_class);
    ri!("car_pi", s.car_performance_index);
    ri!("drivetrain", s.drivetrain_type);
    ri!("num_cylinders", s.num_cylinders);
    rf!("pos_x", dash.position_x);
    rf!("pos_y", dash.position_y);
    rf!("pos_z", dash.position_z);
    rf!("speed_mps", dash.speed);
    rf!("power_w", dash.power);
    rf!("torque_nm", dash.torque);
    rf!("tire_temp_fl_f", dash.tire_temp_front_left);
    rf!("tire_temp_fr_f", dash.tire_temp_front_right);
    rf!("tire_temp_rl_f", dash.tire_temp_rear_left);
    rf!("tire_temp_rr_f", dash.tire_temp_rear_right);
    rf!("boost_bar", dash.boost);
    rf!("fuel", dash.fuel);
    rf!("dist_m", dash.distance_traveled);
    rf!("best_lap_s", dash.best_lap);
    rf!("last_lap_s", dash.last_lap);
    rf!("current_lap_s", dash.current_lap);
    rf!("race_time_s", dash.current_race_time);
    ri!("lap_number", dash.lap_number);
    ri!("race_pos", dash.race_position);
    ri!("accel_raw", dash.accel);
    ri!("brake_raw", dash.brake);
    ri!("clutch_raw", dash.clutch);
    ri!("hand_brake_raw", dash.hand_brake);
    ri!("gear_raw", dash.gear);
    ri!("steer_raw", dash.steer);
    ri!("driving_line", dash.normalized_driving_line);
    ri!("ai_brake_diff", dash.normalized_ai_brake_difference);
    rfo!("tire_wear_fl", dash.tire_wear_front_left);
    rfo!("tire_wear_fr", dash.tire_wear_front_right);
    rfo!("tire_wear_rl", dash.tire_wear_rear_left);
    rfo!("tire_wear_rr", dash.tire_wear_rear_right);
    raw.insert(
        "track_ordinal".to_string(),
        dash.track_ordinal.map_or(Value::Null, Value::from),
    );

    serde_json::json!({
        "speed_kph":  jf(dash.speed * 3.6),
        "gear":       dash.gear,
        "rpm":        jf(s.current_engine_rpm),
        "rpm_max":    jf(s.engine_max_rpm),
        "rpm_idle":   jf(s.engine_idle_rpm),
        "throttle":   jf(f32::from(dash.accel) / 255.0),
        "brake":      jf(f32::from(dash.brake) / 255.0),
        "clutch":     jf(f32::from(dash.clutch) / 255.0),
        "hand_brake": jf(f32::from(dash.hand_brake) / 255.0),
        "steer":      jf(f32::from(dash.steer) / 127.0),
        "is_race_on": s.is_race_on != 0,
        "lap_status": lap_status,
        "lap": {
            "current_s": jf(dash.current_lap),
            "best_s":    jf(dash.best_lap),
            "last_s":    jf(dash.last_lap),
            "number":    dash.lap_number,
        },
        "dyno": dyno,
        "raw": Value::Object(raw),
    })
}

/// Watches UDP-sourced telemetry packets, converts them to the overlay JSON
/// schema, and publishes each to `telemetry_tx` so connected WS clients
/// receive live game data at `telemetry_hz`.
///
/// Also tracks lap status from `lap_validity_tx` events so the telemetry
/// payload carries an up-to-date `lap_status` field.
async fn telemetry_bridge_loop(state: AppState) -> anyhow::Result<()> {
    let mut packet_rx = state.latest_telemetry_tx.subscribe();
    let mut lap_validity_rx = state.lap_validity_tx.subscribe();
    let mut shutdown_rx = state.shutdown_tx.subscribe();
    let mut dyno_reset_rx = state.dyno_reset_tx.subscribe();
    let mut current_lap_status = "unknown".to_string();
    let mut last_lap_number: Option<u16> = None;
    let mut dyno = DynoCollector::new();
    let mut last_packet_at: Option<std::time::Instant> = None;

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            _ = dyno_reset_rx.recv() => {
                dyno.reset_to_waiting();
                state.emit_dyno_update(dyno.to_update_payload());
            }
            event = lap_validity_rx.recv() => {
                match event {
                    Ok(ev) => {
                        current_lap_status = lap_event_to_status(&ev).to_string();
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            changed = packet_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let packet = packet_rx.borrow_and_update().clone();
                if let Some(telemetry::TelemetryPacket::Dash(dash)) = packet {
                    // Compute dt for dyno timing
                    let now = std::time::Instant::now();
                    let dt_secs = last_packet_at
                        .map(|t| now.duration_since(t).as_secs_f32().min(0.5))
                        .unwrap_or(0.0);
                    last_packet_at = Some(now);

                    // Lap number change → reset lap status
                    if Some(dash.lap_number) != last_lap_number {
                        if last_lap_number.is_some() {
                            current_lap_status = "unknown".to_string();
                        }
                        last_lap_number = Some(dash.lap_number);
                    }

                    // Update dyno collector
                    let should_emit_dyno = dyno.update(&dash, dt_secs);
                    if should_emit_dyno {
                        state.emit_dyno_update(dyno.to_update_payload());
                    }

                    let dyno_status = dyno.dyno_status_fields();
                    let payload = dash_to_overlay_json(&dash, &current_lap_status, &dyno_status);
                    state.emit_telemetry(payload);
                }
            }
        }
    }

    Ok(())
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
    use serde_json::json;
    use serial_test::serial;
    use temp_env::with_var;

    use super::{
        AppConfig, EventMessage, HelloMessage, DEFAULT_PACKET_TIMEOUT_MS,
        DEFAULT_PAUSE_DEBOUNCE_MS, SCHEMA_VERSION,
    };

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
    fn config_invalid_rewind_threshold_fails() {
        with_var("TUNING_COACH_REWIND_BACKWARD_JUMP_M", Some("0"), || {
            let result = AppConfig::from_sources(None, true);
            assert!(result.is_err());
        });
    }

    #[test]
    #[serial]
    fn config_invalid_pit_hysteresis_fails() {
        with_var(
            "TUNING_COACH_PIT_ENTRY_SPEED_THRESHOLD_KPH",
            Some("45.0"),
            || {
                with_var(
                    "TUNING_COACH_PIT_EXIT_SPEED_THRESHOLD_KPH",
                    Some("40.0"),
                    || {
                        let result = AppConfig::from_sources(None, true);
                        assert!(result.is_err());
                    },
                );
            },
        );
    }

    #[test]
    fn config_default_session_timeouts_are_set() {
        let config = AppConfig::default();
        assert_eq!(config.pause_debounce_ms, DEFAULT_PAUSE_DEBOUNCE_MS);
        assert_eq!(config.packet_timeout_ms, DEFAULT_PACKET_TIMEOUT_MS);
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
            t_ms: 1738012345678,
            data: &payload,
        };
        let serialized = serde_json::to_value(event).expect("serialize event");
        assert_eq!(
            serialized,
            json!({
                "type": "telemetry",
                "schema_version": 1,
                "t_ms": 1738012345678_u64,
                "data": { "speed_kph": 123.4 }
            })
        );
    }
}
