/**
 * telemetry-status.js — Game telemetry activity indicator.
 *
 * Shows whether the game is actively streaming data:
 *   live    — is_race_on=true, packets arriving
 *   paused  — is_race_on=false (game paused / in menu), packets still arriving
 *   none    — no telemetry received in STALE_MS; shows "No data" indicator
 *
 * Usage:
 *   import { TelemetryStatus } from './telemetry-status.js';
 *   const ts = new TelemetryStatus(document.getElementById('telemetry-status'));
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
   * Call on each incoming telemetry event.
   * @param {boolean} isRaceOn  Value of data.is_race_on.
   */
  update(isRaceOn) {
    this.#apply(isRaceOn ? 'live' : 'paused');
    this.#armStale();
  }

  // ── Private ────────────────────────────────────────────────

  #apply(state) {
    this.#root.dataset.state = state;
    const labels = { live: 'Live', paused: 'Paused', none: 'No data' };
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
