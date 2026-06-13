/**
 * telemetry-status.js — Game telemetry activity indicator.
 *
 * Shows whether the game is actively streaming data:
 *   live    — is_race_on=true, packets arriving
 *   replay  — same as live, but telemetry is sourced from a packet-capture
 *             replay rather than a live UDP feed (set via setReplay(true))
 *   paused  — is_race_on=false (game paused / in menu), packets still arriving
 *   none    — no telemetry received in STALE_MS; shows "No data" indicator
 *
 * Usage:
 *   import { TelemetryStatus } from './telemetry-status.js';
 *   const ts = new TelemetryStatus(document.getElementById('telemetry-status'));
 *   ts.setReplay(true);          // optional — flag replay source
 *   ts.update(data.is_race_on);
 */

/** Hide the indicator after this many ms without a telemetry packet. */
const STALE_MS = 3_000;

export class TelemetryStatus {
  /** @type {HTMLElement} */
  #root;

  /** @type {HTMLElement} */
  #text;

  /** @type {ReturnType<typeof setTimeout> | null} */
  #staleTimer = null;

  /** @type {boolean} Whether telemetry is sourced from a capture replay. */
  #replay = false;

  /**
   * @param {HTMLElement} el  The #telemetry-status container.
   */
  constructor(el) {
    this.#root = el;
    this.#root.innerHTML = `
      <span class="ts-dot"></span>
      <span class="ts-text"></span>
    `;
    this.#text = this.#root.querySelector('.ts-text');
    this.#apply('none');
  }

  /**
   * Flag whether telemetry comes from a packet-capture replay. When true, an
   * active stream is labelled "Replay" instead of "Live".
   * @param {boolean} isReplay
   */
  setReplay(isReplay) {
    this.#replay = Boolean(isReplay);
    // Re-apply current active state so the label updates immediately.
    if (this.#root.dataset.state === 'live' || this.#root.dataset.state === 'replay') {
      this.#apply(this.#replay ? 'replay' : 'live');
    }
  }

  /**
   * Call on each incoming telemetry event.
   * @param {boolean} isRaceOn  Value of data.is_race_on.
   */
  update(isRaceOn) {
    const active = this.#replay ? 'replay' : 'live';
    this.#apply(isRaceOn ? active : 'paused');
    this.#armStale();
  }

  // ── Private ────────────────────────────────────────────────

  #apply(state) {
    this.#root.dataset.state = state;
    const labels = { live: 'Live', replay: 'Replay', paused: 'Paused', none: 'No data' };
    this.#text.textContent = labels[state] ?? '';
  }

  #armStale() {
    if (this.#staleTimer !== null) clearTimeout(this.#staleTimer);
    this.#staleTimer = setTimeout(() => {
      this.#staleTimer = null;
      this.#apply('none');
    }, STALE_MS);
  }
}
