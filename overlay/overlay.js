/**
 * tuning-coach overlay — WebSocket client
 *
 * Connects to the sidecar at the URL specified in config.json
 * (default: ws://127.0.0.1:7778/ws).  Renders recommendations in a
 * slide-in panel and tracks lap validity in the status bar.
 */

const DEFAULT_WS_URL = 'ws://127.0.0.1:7778/ws';
const RECONNECT_BASE_MS = 1_000;
const RECONNECT_MAX_MS = 30_000;
const AUTO_DISMISS_MS = 15_000;

// ── DOM refs ────────────────────────────────────────────────────────────────

const statusDot = document.getElementById('status-dot');
const statusText = document.getElementById('status-text');
const lapStateEl = document.getElementById('lap-state');
const panel = document.getElementById('recommendation-panel');
const dismissBtn = document.getElementById('dismiss-btn');

// ── State ────────────────────────────────────────────────────────────────────

let wsUrl = DEFAULT_WS_URL;
let socket = null;
let reconnectDelay = RECONNECT_BASE_MS;
let reconnectTimer = null;
let autoDismissTimer = null;

// ── Config loading ───────────────────────────────────────────────────────────

async function loadConfig() {
  try {
    const resp = await fetch('./config.json', { cache: 'no-store' });
    if (resp.ok) {
      const cfg = await resp.json();
      if (typeof cfg.wsUrl === 'string' && cfg.wsUrl.startsWith('ws')) {
        wsUrl = cfg.wsUrl;
      }
    }
  } catch {
    // fall back to default
  }
}

// ── Connection ───────────────────────────────────────────────────────────────

function setStatus(state, text) {
  statusDot.className = state; // '', 'connecting', 'connected'
  statusText.textContent = text;
}

function connect() {
  setStatus('connecting', `Connecting to ${wsUrl}…`);

  socket = new WebSocket(wsUrl, ['tuning-coach.v1']);

  socket.addEventListener('open', () => {
    reconnectDelay = RECONNECT_BASE_MS;
    setStatus('connected', 'Connected');
  });

  socket.addEventListener('message', (ev) => {
    try {
      handleMessage(JSON.parse(ev.data));
    } catch {
      // ignore malformed frames
    }
  });

  socket.addEventListener('close', () => {
    setStatus('', 'Disconnected — reconnecting…');
    scheduleReconnect();
  });

  socket.addEventListener('error', () => {
    socket.close();
  });
}

function scheduleReconnect() {
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => {
    reconnectDelay = Math.min(reconnectDelay * 2, RECONNECT_MAX_MS);
    connect();
  }, reconnectDelay);
}

// ── Message handling ─────────────────────────────────────────────────────────

const CATEGORY_ICONS = {
  tires: '🔴',
  aero: '🌬️',
  suspension: '🔩',
  differential: '⚙️',
  brakes: '🛑',
  gearing: '🔧',
  alignment: '📐',
  ballast: '⚖️',
};

function handleMessage(msg) {
  switch (msg.type) {
    case 'hello':
      setStatus('connected', `Connected · sidecar ${msg.sidecar_version ?? '?'}`);
      break;

    case 'lap_validity':
      updateLapState(msg.payload);
      break;

    case 'recommendation':
      showRecommendation(msg.payload);
      break;

    default:
      break;
  }
}

function updateLapState(payload) {
  const state = payload?.state ?? '';
  lapStateEl.className = '';
  if (state === 'valid') {
    lapStateEl.textContent = '✓ Valid';
    lapStateEl.classList.add('valid');
  } else if (state === 'dirty') {
    const reason = payload?.reason ? ` · ${payload.reason}` : '';
    lapStateEl.textContent = `✗ Dirty${reason}`;
    lapStateEl.classList.add('dirty');
  } else if (state === 'pit') {
    lapStateEl.textContent = '🏁 Pit';
  } else {
    lapStateEl.textContent = '';
  }
}

function createAdjustmentRow(adj) {
  const row = document.createElement('div');
  row.className = 'rec-adjustment';

  const paramEl = document.createElement('span');
  paramEl.className = 'rec-param';
  paramEl.textContent = adj.parameter ?? '';

  const valueEl = document.createElement('span');
  valueEl.className = 'rec-value';
  valueEl.textContent = adj.recommended_value != null ? String(adj.recommended_value) : '';

  const unitEl = document.createElement('span');
  unitEl.className = 'rec-unit';
  unitEl.textContent = adj.unit ?? '';

  row.appendChild(paramEl);
  row.appendChild(valueEl);
  row.appendChild(unitEl);
  return row;
}

function showRecommendation(payload) {
  if (!payload) return;

  const category = payload.category ?? 'unknown';
  const icon = CATEGORY_ICONS[category] ?? '💡';
  const confidence = payload.confidence ?? '';
  const title = payload.title ?? '';
  const detail = payload.detail ?? '';
  const adjustments = Array.isArray(payload.adjustments) ? payload.adjustments : [];

  document.getElementById('rec-icon').textContent = icon;
  document.getElementById('rec-category').textContent = category.replace(/_/g, ' ');
  document.getElementById('rec-confidence').textContent = confidence;
  document.getElementById('rec-title').textContent = title;
  document.getElementById('rec-detail').textContent = detail;

  const adjContainer = document.getElementById('rec-adjustments');
  adjContainer.replaceChildren(...adjustments.map(createAdjustmentRow));

  panel.classList.add('visible');

  clearTimeout(autoDismissTimer);
  autoDismissTimer = setTimeout(dismissPanel, AUTO_DISMISS_MS);
}

function dismissPanel() {
  panel.classList.remove('visible');
  clearTimeout(autoDismissTimer);
}

// ── Init ─────────────────────────────────────────────────────────────────────

dismissBtn.addEventListener('click', dismissPanel);

loadConfig().then(connect);
