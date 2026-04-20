use std::time::{Duration, Instant};

use tokio::sync::{broadcast, watch};

use crate::{
    storage::Storage,
    telemetry::{DashPacket, SledPacket, TelemetryPacket},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionState {
    Loading,
    InRace,
    Paused,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionStateChanged {
    pub(crate) from: SessionState,
    pub(crate) to: SessionState,
    pub(crate) session_id: i64,
    pub(crate) at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionStateMachineConfig {
    pub(crate) pause_debounce: Duration,
    pub(crate) packet_timeout: Duration,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionInput {
    pub(crate) is_race_on: bool,
    pub(crate) current_race_time_s: Option<f32>,
    pub(crate) lap_number: Option<u16>,
    pub(crate) car_ordinal: Option<i32>,
    pub(crate) at_ms: u64,
}

impl From<&TelemetryPacket> for SessionInput {
    fn from(packet: &TelemetryPacket) -> Self {
        match packet {
            TelemetryPacket::Sled(sled) => Self::from_sled(sled),
            TelemetryPacket::Dash(dash) => Self::from_dash(dash),
        }
    }
}

impl SessionInput {
    fn from_sled(sled: &SledPacket) -> Self {
        Self {
            is_race_on: sled.is_race_on != 0,
            current_race_time_s: None,
            lap_number: None,
            car_ordinal: Some(sled.car_ordinal),
            at_ms: u64::from(sled.timestamp_ms),
        }
    }

    fn from_dash(dash: &DashPacket) -> Self {
        Self {
            is_race_on: dash.sled.is_race_on != 0,
            current_race_time_s: Some(dash.current_race_time),
            lap_number: Some(dash.lap_number),
            car_ordinal: Some(dash.sled.car_ordinal),
            at_ms: u64::from(dash.sled.timestamp_ms),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionTransition {
    pub(crate) from: SessionState,
    pub(crate) to: SessionState,
    pub(crate) at_ms: u64,
    pub(crate) car_ordinal: Option<i32>,
}

pub(crate) struct SessionStateMachine {
    state: SessionState,
    config: SessionStateMachineConfig,
    pause_candidate_since: Option<Instant>,
    last_packet_at: Option<Instant>,
    last_packet_at_ms: u64,
}

impl SessionStateMachine {
    pub(crate) fn new(config: SessionStateMachineConfig) -> Self {
        Self {
            state: SessionState::Loading,
            config,
            pause_candidate_since: None,
            last_packet_at: None,
            last_packet_at_ms: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> SessionState {
        self.state
    }

    pub(crate) fn on_packet(
        &mut self,
        input: SessionInput,
        received_at: Instant,
    ) -> Option<SessionTransition> {
        self.last_packet_at = Some(received_at);
        self.last_packet_at_ms = input.at_ms;

        if self.state == SessionState::InRace && is_session_boundary(input) {
            self.pause_candidate_since = None;
            return self.transition(SessionState::Loading, input.at_ms, input.car_ordinal);
        }

        match self.state {
            SessionState::Loading | SessionState::Finished => {
                self.pause_candidate_since = None;
                if is_race_start(input) {
                    return self.transition(SessionState::InRace, input.at_ms, input.car_ordinal);
                }
            }
            SessionState::InRace => {
                if input.is_race_on {
                    self.pause_candidate_since = None;
                    return None;
                }

                let pause_since = self.pause_candidate_since.get_or_insert(received_at);
                if received_at.duration_since(*pause_since) >= self.config.pause_debounce {
                    self.pause_candidate_since = None;
                    return self.transition(SessionState::Paused, input.at_ms, input.car_ordinal);
                }
            }
            SessionState::Paused => {
                if input.is_race_on {
                    self.pause_candidate_since = None;
                    return self.transition(SessionState::InRace, input.at_ms, input.car_ordinal);
                }
            }
        }

        None
    }

    pub(crate) fn on_tick(&mut self, now: Instant) -> Option<SessionTransition> {
        if !matches!(self.state, SessionState::InRace | SessionState::Paused) {
            return None;
        }
        let last_packet_at = self.last_packet_at?;
        if now.duration_since(last_packet_at) >= self.config.packet_timeout {
            self.pause_candidate_since = None;
            return self.transition(SessionState::Finished, self.last_packet_at_ms, None);
        }
        None
    }

    fn transition(
        &mut self,
        to: SessionState,
        at_ms: u64,
        car_ordinal: Option<i32>,
    ) -> Option<SessionTransition> {
        if self.state == to {
            return None;
        }
        let from = self.state;
        self.state = to;
        Some(SessionTransition {
            from,
            to,
            at_ms,
            car_ordinal,
        })
    }
}

fn is_race_start(input: SessionInput) -> bool {
    input.is_race_on && input.current_race_time_s.is_some_and(|time_s| time_s > 0.0)
}

fn is_session_boundary(input: SessionInput) -> bool {
    input.lap_number == Some(0) && input.current_race_time_s.is_some_and(|time_s| time_s < 2.0)
}

pub(crate) async fn session_state_loop(
    mut latest_packet_rx: watch::Receiver<Option<TelemetryPacket>>,
    session_state_tx: broadcast::Sender<SessionStateChanged>,
    storage: Storage,
    mut shutdown_rx: broadcast::Receiver<()>,
    machine_config: SessionStateMachineConfig,
    sidecar_version: &str,
) -> anyhow::Result<()> {
    let mut machine = SessionStateMachine::new(machine_config);
    let mut active_session_id: Option<i64> = None;
    let mut timeout_tick = tokio::time::interval(Duration::from_millis(100));
    timeout_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            _ = timeout_tick.tick() => {
                if let Some(transition) = machine.on_tick(Instant::now()) {
                    apply_transition(
                        transition,
                        &storage,
                        &mut active_session_id,
                        &session_state_tx,
                        sidecar_version,
                    )?;
                }
            }
            changed = latest_packet_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let Some(packet) = latest_packet_rx.borrow_and_update().clone() else {
                    continue;
                };
                if let Some(transition) = machine.on_packet((&packet).into(), Instant::now()) {
                    apply_transition(
                        transition,
                        &storage,
                        &mut active_session_id,
                        &session_state_tx,
                        sidecar_version,
                    )?;
                }
            }
        }
    }

    if let Some(session_id) = active_session_id.take() {
        storage.end_session(session_id)?;
    }

    Ok(())
}

fn apply_transition(
    transition: SessionTransition,
    storage: &Storage,
    active_session_id: &mut Option<i64>,
    session_state_tx: &broadcast::Sender<SessionStateChanged>,
    sidecar_version: &str,
) -> anyhow::Result<()> {
    let session_id = match transition.to {
        SessionState::InRace => {
            if active_session_id.is_none() {
                *active_session_id =
                    Some(storage.start_session(transition.car_ordinal, sidecar_version)?);
            }
            *active_session_id
        }
        SessionState::Loading | SessionState::Finished => {
            let session_id = *active_session_id;
            if let Some(id) = active_session_id.take() {
                storage.end_session(id)?;
            }
            session_id
        }
        SessionState::Paused => *active_session_id,
    };

    if let Some(session_id) = session_id {
        let _ = session_state_tx.send(SessionStateChanged {
            from: transition.from,
            to: transition.to,
            session_id,
            at_ms: transition.at_ms,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rusqlite::Connection;
    use tempfile::TempDir;
    use tokio::{
        sync::{broadcast, watch},
        time::{sleep, timeout},
    };

    use super::*;
    use crate::telemetry::{DashPacket, SledPacket, TelemetryPacket};

    fn test_config() -> SessionStateMachineConfig {
        SessionStateMachineConfig {
            pause_debounce: Duration::from_secs(2),
            packet_timeout: Duration::from_secs(10),
        }
    }

    fn packet(
        is_race_on: i32,
        current_race_time: f32,
        lap_number: u16,
        timestamp_ms: u32,
    ) -> SessionInput {
        SessionInput {
            is_race_on: is_race_on != 0,
            current_race_time_s: Some(current_race_time),
            lap_number: Some(lap_number),
            car_ordinal: Some(12),
            at_ms: u64::from(timestamp_ms),
        }
    }

    #[test]
    fn initializes_in_loading() {
        let machine = SessionStateMachine::new(test_config());
        assert_eq!(machine.state(), SessionState::Loading);
    }

    #[test]
    fn loading_to_in_race_on_valid_packet() {
        let mut machine = SessionStateMachine::new(test_config());
        let now = Instant::now();
        let transition = machine
            .on_packet(packet(1, 1.0, 0, 100), now)
            .expect("transition expected");
        assert_eq!(transition.from, SessionState::Loading);
        assert_eq!(transition.to, SessionState::InRace);
        assert_eq!(transition.at_ms, 100);
    }

    #[test]
    fn in_race_to_paused_after_debounce() {
        let mut machine = SessionStateMachine::new(test_config());
        let start = Instant::now();
        machine
            .on_packet(packet(1, 5.0, 1, 100), start)
            .expect("enter race");
        assert!(machine
            .on_packet(packet(0, 6.0, 1, 200), start + Duration::from_secs(1))
            .is_none());
        let transition = machine
            .on_packet(packet(0, 7.0, 1, 300), start + Duration::from_secs(3))
            .expect("debounced pause");
        assert_eq!(transition.from, SessionState::InRace);
        assert_eq!(transition.to, SessionState::Paused);
    }

    #[test]
    fn paused_to_in_race_when_race_resumes() {
        let mut machine = SessionStateMachine::new(test_config());
        let start = Instant::now();
        machine
            .on_packet(packet(1, 5.0, 1, 100), start)
            .expect("enter race");
        assert!(machine
            .on_packet(packet(0, 6.0, 1, 150), start + Duration::from_secs(1))
            .is_none());
        machine
            .on_packet(packet(0, 6.5, 1, 200), start + Duration::from_secs(3))
            .expect("pause");
        let transition = machine
            .on_packet(packet(1, 7.0, 1, 300), start + Duration::from_secs(4))
            .expect("resume");
        assert_eq!(transition.from, SessionState::Paused);
        assert_eq!(transition.to, SessionState::InRace);
    }

    #[test]
    fn in_race_to_loading_on_session_boundary() {
        let mut machine = SessionStateMachine::new(test_config());
        let start = Instant::now();
        machine
            .on_packet(packet(1, 8.0, 1, 100), start)
            .expect("enter race");
        let transition = machine
            .on_packet(packet(1, 1.5, 0, 200), start + Duration::from_secs(1))
            .expect("session boundary");
        assert_eq!(transition.from, SessionState::InRace);
        assert_eq!(transition.to, SessionState::Loading);
    }

    #[test]
    fn in_race_or_paused_to_finished_on_timeout() {
        let mut machine = SessionStateMachine::new(test_config());
        let start = Instant::now();
        machine
            .on_packet(packet(1, 8.0, 1, 100), start)
            .expect("enter race");
        let transition = machine
            .on_tick(start + Duration::from_secs(11))
            .expect("timeout to finished");
        assert_eq!(transition.from, SessionState::InRace);
        assert_eq!(transition.to, SessionState::Finished);
    }

    #[test]
    fn debounce_prevents_spurious_paused_transition() {
        let mut machine = SessionStateMachine::new(test_config());
        let start = Instant::now();
        machine
            .on_packet(packet(1, 5.0, 1, 100), start)
            .expect("enter race");

        assert!(machine
            .on_packet(packet(0, 6.0, 1, 200), start + Duration::from_millis(500))
            .is_none());
        assert!(machine
            .on_packet(packet(1, 6.5, 1, 300), start + Duration::from_millis(1200))
            .is_none());
        assert!(machine
            .on_packet(packet(0, 7.0, 1, 400), start + Duration::from_millis(1700))
            .is_none());

        let transition = machine
            .on_packet(packet(0, 7.5, 1, 500), start + Duration::from_millis(3800))
            .expect("pause after second sustained zero");
        assert_eq!(transition.to, SessionState::Paused);
    }

    #[tokio::test]
    async fn session_state_loop_persists_sessions_and_emits_events() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage should open");
        let (latest_packet_tx, latest_packet_rx) = watch::channel(None);
        let (session_state_tx, mut session_state_rx) = broadcast::channel(16);
        let (shutdown_tx, _) = broadcast::channel(4);
        let config = SessionStateMachineConfig {
            pause_debounce: Duration::from_millis(40),
            packet_timeout: Duration::from_millis(80),
        };

        let loop_task = tokio::spawn(session_state_loop(
            latest_packet_rx,
            session_state_tx.clone(),
            storage.clone(),
            shutdown_tx.subscribe(),
            config,
            "0.1.0",
        ));

        latest_packet_tx
            .send(Some(make_dash_packet(1, 10.0, 1, 100, 42)))
            .expect("send in-race packet");
        let started = recv_event(&mut session_state_rx).await;
        assert_eq!(started.from, SessionState::Loading);
        assert_eq!(started.to, SessionState::InRace);

        latest_packet_tx
            .send(Some(make_dash_packet(0, 10.5, 1, 150, 42)))
            .expect("send pause candidate");
        sleep(Duration::from_millis(45)).await;
        latest_packet_tx
            .send(Some(make_dash_packet(0, 11.0, 1, 200, 42)))
            .expect("send debounced pause packet");
        let paused = recv_event(&mut session_state_rx).await;
        assert_eq!(paused.from, SessionState::InRace);
        assert_eq!(paused.to, SessionState::Paused);
        assert_eq!(paused.session_id, started.session_id);

        latest_packet_tx
            .send(Some(make_dash_packet(1, 11.5, 1, 250, 42)))
            .expect("send resume packet");
        let resumed = recv_event(&mut session_state_rx).await;
        assert_eq!(resumed.from, SessionState::Paused);
        assert_eq!(resumed.to, SessionState::InRace);
        assert_eq!(resumed.session_id, started.session_id);

        let finished = timeout(Duration::from_secs(2), async {
            loop {
                let event = recv_event(&mut session_state_rx).await;
                if event.to == SessionState::Finished {
                    break event;
                }
            }
        })
        .await
        .expect("finished event should arrive");
        assert_eq!(finished.from, SessionState::InRace);
        assert_eq!(finished.session_id, started.session_id);

        let _ = shutdown_tx.send(());
        loop_task
            .await
            .expect("session loop task should join")
            .expect("session loop should exit cleanly");

        let conn = Connection::open(temp.path().join("tuning-coach.db")).expect("open sqlite");
        let (started_at, ended_at, car_ordinal): (String, Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT started_at, ended_at, car_ordinal FROM sessions WHERE id = ?1",
                rusqlite::params![started.session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query stored session");
        assert!(!started_at.is_empty());
        assert!(ended_at.is_some());
        assert_eq!(car_ordinal, Some(42));
    }

    #[tokio::test]
    async fn session_boundary_closes_previous_session_and_starts_new_one() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage should open");
        let (latest_packet_tx, latest_packet_rx) = watch::channel(None);
        let (session_state_tx, mut session_state_rx) = broadcast::channel(16);
        let (shutdown_tx, _) = broadcast::channel(4);

        let loop_task = tokio::spawn(session_state_loop(
            latest_packet_rx,
            session_state_tx,
            storage.clone(),
            shutdown_tx.subscribe(),
            SessionStateMachineConfig {
                pause_debounce: Duration::from_millis(20),
                packet_timeout: Duration::from_secs(1),
            },
            "0.1.0",
        ));

        latest_packet_tx
            .send(Some(make_dash_packet(1, 8.0, 1, 100, 9)))
            .expect("send race packet");
        let started = recv_event(&mut session_state_rx).await;
        assert_eq!(started.from, SessionState::Loading);
        assert_eq!(started.to, SessionState::InRace);

        latest_packet_tx
            .send(Some(make_dash_packet(1, 1.2, 0, 200, 9)))
            .expect("send boundary packet");
        let boundary = recv_event(&mut session_state_rx).await;
        assert_eq!(boundary.from, SessionState::InRace);
        assert_eq!(boundary.to, SessionState::Loading);
        assert_eq!(boundary.session_id, started.session_id);

        latest_packet_tx
            .send(Some(make_dash_packet(1, 3.0, 0, 300, 10)))
            .expect("send next session start packet");
        let restarted = recv_event(&mut session_state_rx).await;
        assert_eq!(restarted.from, SessionState::Loading);
        assert_eq!(restarted.to, SessionState::InRace);
        assert_ne!(restarted.session_id, started.session_id);

        let _ = shutdown_tx.send(());
        loop_task
            .await
            .expect("session loop task should join")
            .expect("session loop should exit cleanly");

        let conn = Connection::open(temp.path().join("tuning-coach.db")).expect("open sqlite");
        let ended_at: Option<String> = conn
            .query_row(
                "SELECT ended_at FROM sessions WHERE id = ?1",
                rusqlite::params![started.session_id],
                |row| row.get(0),
            )
            .expect("query ended_at");
        assert!(ended_at.is_some());
    }

    async fn recv_event(rx: &mut broadcast::Receiver<SessionStateChanged>) -> SessionStateChanged {
        timeout(Duration::from_secs(2), async {
            loop {
                match rx.recv().await {
                    Ok(event) => return event,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(err) => panic!("session event channel closed unexpectedly: {err}"),
                }
            }
        })
        .await
        .expect("session event should arrive")
    }

    fn make_dash_packet(
        is_race_on: i32,
        current_race_time: f32,
        lap_number: u16,
        timestamp_ms: u32,
        car_ordinal: i32,
    ) -> TelemetryPacket {
        let sled = SledPacket {
            is_race_on,
            timestamp_ms,
            car_ordinal,
            ..SledPacket::default()
        };
        TelemetryPacket::Dash(DashPacket {
            sled,
            current_race_time,
            lap_number,
            ..DashPacket::default()
        })
    }
}
