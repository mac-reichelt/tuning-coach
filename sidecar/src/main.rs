use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::Context;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::Parser;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{broadcast, Notify},
    time::{sleep, Duration},
};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info, info_span};
use tracing_subscriber::EnvFilter;

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
    data_dir: PathBuf,
    log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            udp_listen_port: 7777,
            ws_listen_port: 7778,
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
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    active_ws_connections: Arc<AtomicUsize>,
    ws_drain_notify: Arc<Notify>,
    shutdown_tx: broadcast::Sender<()>,
}

struct WsConnectionGuard {
    active_ws_connections: Arc<AtomicUsize>,
    ws_drain_notify: Arc<Notify>,
}

impl WsConnectionGuard {
    fn new(state: &AppState) -> Self {
        state.active_ws_connections.fetch_add(1, Ordering::SeqCst);
        Self {
            active_ws_connections: Arc::clone(&state.active_ws_connections),
            ws_drain_notify: Arc::clone(&state.ws_drain_notify),
        }
    }
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        if self.active_ws_connections.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.ws_drain_notify.notify_waiters();
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct HelloMessage<'a> {
    r#type: &'static str,
    version: &'a str,
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
    let env_filter = EnvFilter::try_new(&config.log_level)
        .or_else(|_| EnvFilter::try_new("info"))
        .context("invalid log level")?;

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
    let request_counter = Arc::new(AtomicU64::new(1));
    let (shutdown_tx, _) = broadcast::channel(16);
    let state = AppState {
        active_ws_connections: Arc::new(AtomicUsize::new(0)),
        ws_drain_notify: Arc::new(Notify::new()),
        shutdown_tx,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state.clone())
        .layer(PropagateRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
        ))
        .layer(SetRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(TraceLayer::new_for_http().make_span_with(
            move |request: &axum::http::Request<_>| {
                let request_span_id = request_counter.fetch_add(1, Ordering::Relaxed);
                info_span!(
                    "http_request",
                    request_span_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            },
        ));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.ws_listen_port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind ws server on {addr}"))?;
    info!(
        module = module_path!(),
        ws_listen_port = config.ws_listen_port,
        "http/ws server listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .context("http/ws server exited with error")?;

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
    let hello = HelloMessage {
        r#type: "hello",
        version: env!("CARGO_PKG_VERSION"),
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

    loop {
        tokio::select! {
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        error!(module = module_path!(), %err, "websocket receive error");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
    let _ = socket.send(Message::Close(None)).await;
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
                    sleep(Duration::from_secs(3600)).await;
                }
            }
        } => {},
    }

    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;

    info!(module = module_path!(), "shutdown signal received");
    let _ = state.shutdown_tx.send(());

    if state.active_ws_connections.load(Ordering::SeqCst) > 0 {
        state.ws_drain_notify.notified().await;
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use serial_test::serial;
    use temp_env::with_var;

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
}
