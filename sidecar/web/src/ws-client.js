/**
 * ws-client.js — WebSocket client with exponential-backoff reconnection.
 *
 * Usage:
 *   import { WsClient } from './ws-client.js';
 *   const client = new WsClient('ws://127.0.0.1:7778/ws');
 *   client.addEventListener('telemetry', (ev) => { ... });
 *   client.connect();
 */

const SCHEMA_VERSION = 1;
const SUBPROTOCOL = 'tuning-coach.v1';

/** Reconnect backoff: 500 ms → 1 s → 2 s → 4 s → … capped at 30 s. */
const BACKOFF_BASE_MS = 500;
const BACKOFF_MAX_MS  = 30_000;
const BACKOFF_FACTOR  = 2;

/** Send an application-level ping every 25 s to keep the idle timer alive. */
const PING_INTERVAL_MS = 25_000;

/**
 * After this many consecutive failed connection attempts without ever having
 * connected successfully, transition to the 'down' state (still retrying).
 */
const DOWN_THRESHOLD = 5;

function resolveDefaultWsUrl() {
  const params = new URLSearchParams(location.search);
  return params.get('ws') ?? `ws://${location.host}/ws`;
}

export class WsClient extends EventTarget {
  /** @type {string} */
  #url;

  /** @type {WebSocket | null} */
  #ws = null;

  /** @type {'connected' | 'reconnecting' | 'down'} */
  #state = 'reconnecting';

  /** Current retry delay in ms. */
  #retryDelayMs = BACKOFF_BASE_MS;

  /** Timer handle for the next reconnect attempt. */
  #retryTimer = null;

  /** Timer handle for the heartbeat ping. */
  #pingTimer = null;

  /** Whether connect() has been called at all. */
  #started = false;

  /** Whether the client has been permanently stopped. */
  #stopped = false;

  /** Number of consecutive failed connection attempts. */
  #failCount = 0;

  /**
   * @param {string} url  WebSocket URL (e.g. 'ws://127.0.0.1:7778/ws')
   */
  constructor(url = resolveDefaultWsUrl()) {
    super();
    this.#url = url;
  }

  /** Current connection state. */
  get state() {
    return this.#state;
  }

  /** Open the WebSocket connection. Idempotent if already connected. */
  connect() {
    if (this.#stopped) return;
    if (this.#started) return;
    this.#started = true;
    this.#open();
  }

  /** Permanently close the connection and stop reconnecting. */
  destroy() {
    this.#stopped = true;
    this.#clearTimers();
    if (this.#ws) {
      this.#ws.close(1000, 'client destroyed');
      this.#ws = null;
    }
  }

  /**
   * Send a client → server message envelope.
   * @param {string} type
   * @param {object} data
   */
  send(type, data = {}) {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) return;
    const envelope = {
      type,
      schema_version: SCHEMA_VERSION,
      t_ms: Date.now(),
      data,
    };
    try {
      this.#ws.send(JSON.stringify(envelope));
    } catch {
      // ignore — socket may be closing
    }
  }

  // ── Private ────────────────────────────────────────────────

  #open() {
    if (this.#stopped) return;

    let ws;
    try {
      ws = new WebSocket(this.#url, SUBPROTOCOL);
    } catch (err) {
      // URL may be invalid in the SimHub preview pane before the sidecar
      // launches — treat like a failed connection attempt.
      console.warn('[tuning-coach] WebSocket construction failed:', err);
      this.#onFailedAttempt();
      this.#scheduleReconnect();
      return;
    }
    this.#ws = ws;

    ws.addEventListener('open', () => this.#onOpen(ws));
    ws.addEventListener('message', (ev) => this.#onMessage(ev));
    ws.addEventListener('close', (ev) => this.#onClose(ws, ev));
    ws.addEventListener('error', () => {
      // 'error' is always followed by 'close'; nothing to do here.
    });
  }

  #onOpen(ws) {
    if (ws !== this.#ws) return; // stale handle
    this.#retryDelayMs = BACKOFF_BASE_MS;
    this.#failCount = 0;
    this.#setState('connected');
    this.#startPing();
    this.dispatchEvent(new CustomEvent('open'));
  }

  #onMessage(ev) {
    let envelope;
    try {
      envelope = JSON.parse(ev.data);
    } catch {
      return; // malformed frame — ignore
    }

    const { type, data } = envelope;
    if (typeof type !== 'string') return;

    // Dispatch a typed event so listeners can subscribe to specific types.
    this.dispatchEvent(new CustomEvent(type, { detail: data ?? {} }));
    // Also dispatch a generic 'message' event for catch-all listeners.
    this.dispatchEvent(new CustomEvent('message', { detail: envelope }));
  }

  #onClose(ws, ev) {
    if (ws !== this.#ws) return; // stale handle
    this.#ws = null;
    this.#stopPing();

    if (this.#stopped) return;

    // Close code 4001/4002 = server-side schema/subprotocol rejection.
    // Still reconnect — the sidecar may be restarted with compatible config.
    this.#onFailedAttempt();
    this.dispatchEvent(new CustomEvent('close', {
      detail: { code: ev.code, reason: ev.reason },
    }));
    this.#scheduleReconnect();
  }

  /**
   * Track consecutive failures and transition to 'down' once the threshold
   * is reached (overlay still retries — 'down' is a UI hint, not a stop).
   */
  #onFailedAttempt() {
    this.#failCount += 1;
    if (this.#failCount >= DOWN_THRESHOLD) {
      this.#setState('down');
    } else {
      this.#setState('reconnecting');
    }
  }

  #scheduleReconnect() {
    if (this.#stopped) return;
    const delay = this.#retryDelayMs;
    this.#retryDelayMs = Math.min(this.#retryDelayMs * BACKOFF_FACTOR, BACKOFF_MAX_MS);

    this.#retryTimer = setTimeout(() => {
      this.#retryTimer = null;
      if (!this.#stopped) this.#open();
    }, delay);
  }

  #startPing() {
    this.#stopPing();
    this.#pingTimer = setInterval(() => {
      this.send('ping');
    }, PING_INTERVAL_MS);
  }

  #stopPing() {
    if (this.#pingTimer !== null) {
      clearInterval(this.#pingTimer);
      this.#pingTimer = null;
    }
  }

  #clearTimers() {
    this.#stopPing();
    if (this.#retryTimer !== null) {
      clearTimeout(this.#retryTimer);
      this.#retryTimer = null;
    }
  }

  #setState(state) {
    if (this.#state === state) return;
    this.#state = state;
    this.dispatchEvent(new CustomEvent('statechange', { detail: { state } }));
  }
}
