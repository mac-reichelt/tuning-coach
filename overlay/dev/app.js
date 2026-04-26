/**
 * tuning-coach diagnostic dev overlay — app.js
 *
 * Connects to the sidecar WebSocket at ws://127.0.0.1:<port>/ws and renders:
 *   1. Connection status
 *   2. Telemetry feed indicator (rolling 1-second Hz)
 *   3. Lap state
 *   4. Scrolling event log with collapsible JSON payloads
 *   5. Hotkey REST tester
 *
 * No build step required. ES module with top-level await.
 */

// ── Config (read from URL params, e.g. ?port=7778) ───────────────────────────

const params   = new URLSearchParams(location.search);
const WS_PORT  = params.get("port")  ?? "7778";
const WS_HOST  = params.get("host")  ?? "127.0.0.1";
const WS_URL   = `ws://${WS_HOST}:${WS_PORT}/ws`;
const API_BASE = `http://${WS_HOST}:${WS_PORT}`;

// ── Constants ─────────────────────────────────────────────────────────────────

const SCHEMA_VERSION  = 1;
const RECONNECT_DELAY = 3_000;   // ms
const MAX_LOG_ENTRIES = 500;
const HZ_WINDOW_MS    = 1_000;   // rolling window for packet-rate calculation

// Suppress noisy telemetry from the log by default
const HIDE_TELEMETRY_DEFAULT = true;

// ── State ─────────────────────────────────────────────────────────────────────

let ws            = null;
let reconnectTimer = null;
let lastMessageAt = null;

/** Timestamps (ms) of recent telemetry frames for Hz calculation */
const telemTimes = [];

/** Telemetry feed state */
const telem = {
  hz:       0,
  variant:  "—",
  lapNum:   "—",
  speed:    "—",
};

/** Lap state */
const lapState = {
  number:   "—",
  validity: "unknown",
  reason:   "",
};

/** Log entries */
const logEntries = [];
let logPaused    = false;
let hideTelemetry = HIDE_TELEMETRY_DEFAULT;

// ── DOM refs ──────────────────────────────────────────────────────────────────

const $ = (id) => document.getElementById(id);

const dom = {
  connDot:       $("conn-dot"),
  connStatus:    $("conn-status"),
  connUrl:       $("conn-url"),
  connLastMsg:   $("conn-last-msg"),
  connSchema:    $("conn-schema"),

  telemHz:       $("telem-hz"),
  telemVariant:  $("telem-variant"),
  telemLap:      $("telem-lap"),
  telemSpeed:    $("telem-speed"),

  lapNumber:     $("lap-number"),
  lapValidity:   $("lap-validity"),
  lapReason:     $("lap-reason"),

  hotkeyResp:    $("hotkey-response"),
  eventLog:      $("event-log"),
  logCount:      $("log-count"),
  autoScroll:    $("auto-scroll"),
  hideTelemChk:  $("hide-telem"),
};

// ── WebSocket lifecycle ───────────────────────────────────────────────────────

function connect() {
  setConnState("connecting");
  ws = new WebSocket(WS_URL, "tuning-coach.v1");

  ws.onopen = () => {
    setConnState("connected");
    // Request all events at max diagnostic rate
    sendMsg({ type: "set_rate", data: { hz: 10 } });
  };

  ws.onmessage = (ev) => {
    lastMessageAt = Date.now();
    let msg;
    try {
      msg = JSON.parse(ev.data);
    } catch {
      appendLog("parse_error", { raw: ev.data });
      return;
    }
    handleMessage(msg);
  };

  ws.onclose = (ev) => {
    setConnState("disconnected");
    appendLog("ws_closed", { code: ev.code, reason: ev.reason || "(none)" });
    scheduleReconnect();
  };

  ws.onerror = () => {
    // onerror is always followed by onclose; just log
    setConnState("disconnected");
  };
}

function scheduleReconnect() {
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => {
    appendLog("ws_reconnecting", { url: WS_URL });
    connect();
  }, RECONNECT_DELAY);
}

function sendMsg(obj) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ schema_version: SCHEMA_VERSION, t_ms: Date.now(), ...obj }));
  }
}

// ── Message dispatch ──────────────────────────────────────────────────────────

function handleMessage(msg) {
  const { type, data = {} } = msg;

  switch (type) {
    case "hello":
      dom.connSchema.textContent = `schema v${msg.schema_version ?? "?"} · sidecar ${msg.sidecar_version ?? "?"}`;
      break;

    case "telemetry":
      recordTelemTime();
      updateTelemPanel(data);
      updateLapFromTelem(data);
      break;

    case "lap_completed":
      updateLapNumber(data.lap_number ?? "?");
      break;

    case "lap_dirty_detected":
      setLapValidity("dirty", data.reason?.code ?? "dirty");
      break;

    case "session_started":
      resetLapState();
      break;

    case "session_ended":
      resetLapState();
      break;

    default:
      break;
  }

  appendLog(type, data, msg);
}

// ── Telemetry panel ───────────────────────────────────────────────────────────

function recordTelemTime() {
  const now = Date.now();
  telemTimes.push(now);
  // purge timestamps outside the rolling window
  const cutoff = now - HZ_WINDOW_MS;
  while (telemTimes.length > 0 && telemTimes[0] < cutoff) {
    telemTimes.shift();
  }
  telem.hz = telemTimes.length;
}

function inferVariant(data) {
  // Packet-variant heuristic based on fields present in the telemetry data
  if (data.boost_bar !== undefined) return "FM2023Dash";
  if (data.gear      !== undefined) return "Dash";
  return "Sled";
}

function updateTelemPanel(data) {
  telem.variant = inferVariant(data);
  telem.lapNum  = data.lap?.number ?? telem.lapNum;
  telem.speed   = data.speed_kph != null ? `${data.speed_kph.toFixed(1)} kph` : telem.speed;

  dom.telemHz.textContent      = `${telem.hz} Hz`;
  dom.telemVariant.textContent = telem.variant;
  dom.telemLap.textContent     = telem.lapNum;
  dom.telemSpeed.textContent   = telem.speed;
}

// ── Lap state panel ───────────────────────────────────────────────────────────

function updateLapFromTelem(data) {
  const num = data.lap?.number;
  if (num != null) updateLapNumber(num);

  const status = data.lap_status;
  if (status) {
    let reason = "";
    if (status === "dirty" && data.lap?.dirty_reason) {
      reason = data.lap.dirty_reason;
    }
    setLapValidity(status, reason);
  }
}

function updateLapNumber(num) {
  lapState.number = num;
  dom.lapNumber.textContent = num;
}

function setLapValidity(validity, reason = "") {
  lapState.validity = validity;
  lapState.reason   = reason;

  dom.lapValidity.textContent = reason ? `${validity}(${reason})` : validity;
  dom.lapValidity.className   = `lap-value ${validity}`;
  dom.lapReason.textContent   = reason;
}

function resetLapState() {
  updateLapNumber("—");
  setLapValidity("unknown", "");
}

// ── Connection state display ──────────────────────────────────────────────────

function setConnState(state) {
  dom.connDot.className    = `${state}`;
  dom.connStatus.textContent = state.charAt(0).toUpperCase() + state.slice(1);
}

// ── Event log ─────────────────────────────────────────────────────────────────

/** Build a short one-line summary string for the log header */
function summarise(type, data) {
  switch (type) {
    case "telemetry":
      return `lap=${data.lap?.number ?? "?"} speed=${data.speed_kph?.toFixed(1) ?? "?"} kph status=${data.lap_status ?? "?"}`;
    case "lap_completed":
      return `lap=${data.lap_number} time=${data.lap_time_s?.toFixed(3) ?? "?"}s validity=${data.validity ?? "?"}`;
    case "lap_dirty_detected":
      return `lap=${data.lap_number} reason=${data.reason?.code ?? "?"}`;
    case "session_started":
      return `session=${data.session_id ?? "?"} car=${data.car_ordinal ?? "?"}`;
    case "session_ended":
      return `session=${data.session_id ?? "?"} laps=${data.lap_count ?? "?"}`;
    case "hello":
      return `schema=${data.schema_version ?? "?"} sidecar=${data.sidecar_version ?? "?"}`;
    case "recommendation":
      return `category=${data.category ?? "?"} title=${data.title ?? "?"}`;
    case "ws_closed":
      return `code=${data.code} reason=${data.reason}`;
    default:
      return JSON.stringify(data).slice(0, 80);
  }
}

function appendLog(type, data, fullMsg) {
  if (hideTelemetry && type === "telemetry") return;

  const ts = new Date().toISOString().slice(11, 23); // HH:MM:SS.mmm

  // Prune oldest entry from DOM if over limit
  if (logEntries.length >= MAX_LOG_ENTRIES) {
    logEntries.shift();
    const firstChild = dom.eventLog.firstElementChild;
    if (firstChild) firstChild.remove();
  }

  const payload = fullMsg ?? data;
  const summary = summarise(type, data);

  // Build DOM element
  const entry = document.createElement("div");
  entry.className = "log-entry";

  const header = document.createElement("div");
  header.className = "log-header";
  header.setAttribute("role", "button");
  header.setAttribute("tabindex", "0");
  header.setAttribute("aria-expanded", "false");

  const tsEl = document.createElement("span");
  tsEl.className = "log-ts";
  tsEl.textContent = ts;

  const typeEl = document.createElement("span");
  typeEl.className = `log-type log-type-${type}`;
  typeEl.textContent = type;

  const sumEl = document.createElement("span");
  sumEl.className = "log-summary";
  sumEl.textContent = summary;

  header.appendChild(tsEl);
  header.appendChild(typeEl);
  header.appendChild(sumEl);

  const payloadEl = document.createElement("pre");
  payloadEl.className = "log-payload";
  payloadEl.textContent = JSON.stringify(payload, null, 2);

  entry.appendChild(header);
  entry.appendChild(payloadEl);

  // Toggle expand/collapse
  const toggle = () => {
    const expanded = entry.classList.toggle("expanded");
    header.setAttribute("aria-expanded", expanded);
  };
  header.addEventListener("click", toggle);
  header.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); }
  });

  logEntries.push(entry);
  dom.eventLog.appendChild(entry);

  dom.logCount.textContent = `${logEntries.length} events`;

  if (!logPaused && dom.autoScroll.checked) {
    dom.eventLog.scrollTop = dom.eventLog.scrollHeight;
  }
}

// ── Hotkey REST tester ────────────────────────────────────────────────────────

async function callHotkey(path) {
  dom.hotkeyResp.className = "";
  dom.hotkeyResp.textContent = "⏳ …";

  try {
    const res = await fetch(`${API_BASE}/api/v1/hotkeys/${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    const text = await res.text();
    let pretty;
    try {
      pretty = JSON.stringify(JSON.parse(text), null, 2);
    } catch {
      pretty = text;
    }
    dom.hotkeyResp.className   = res.ok ? "ok" : "err";
    dom.hotkeyResp.textContent = `${res.status} ${res.statusText}\n${pretty}`;
  } catch (err) {
    dom.hotkeyResp.className   = "err";
    dom.hotkeyResp.textContent = `Network error: ${err.message}`;
  }
}

// ── Last-message ticker ───────────────────────────────────────────────────────

function updateLastMsgDisplay() {
  if (lastMessageAt === null) {
    dom.connLastMsg.textContent = "—";
  } else {
    const ago = Math.floor((Date.now() - lastMessageAt) / 1000);
    dom.connLastMsg.textContent = ago < 2 ? "just now" : `${ago}s ago`;
  }
}

// ── Init ──────────────────────────────────────────────────────────────────────

function init() {
  // Populate static elements
  dom.connUrl.textContent = WS_URL;

  // Hotkey buttons
  document.querySelectorAll("[data-hotkey]").forEach((btn) => {
    btn.addEventListener("click", () => callHotkey(btn.dataset.hotkey));
  });

  // Log controls
  $("btn-clear").addEventListener("click", () => {
    logEntries.length = 0;
    dom.eventLog.innerHTML = "";
    dom.logCount.textContent = "0 events";
  });

  $("btn-pause").addEventListener("click", (e) => {
    logPaused = !logPaused;
    e.target.textContent = logPaused ? "▶ Resume" : "⏸ Pause";
  });

  dom.hideTelemChk.checked = HIDE_TELEMETRY_DEFAULT;
  dom.hideTelemChk.addEventListener("change", () => {
    hideTelemetry = dom.hideTelemChk.checked;
  });

  // Tick display
  setInterval(updateLastMsgDisplay, 1_000);

  // Connect
  connect();
}

document.addEventListener("DOMContentLoaded", init);
