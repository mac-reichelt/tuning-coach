/**
 * lap-status.js — Lap-status badge.
 *
 * Reads `lap_status` from the telemetry WS payload and updates a badge
 * element to show one of: valid / dirty / pit / reset / out_lap / unknown.
 *
 * Usage:
 *   import { LapStatus } from './lap-status.js';
 *   const badge = new LapStatus(document.getElementById('lap-status'));
 *   badge.update('dirty');
 */

/** Human-readable labels for each status value. */
const STATUS_LABELS = {
  valid:   '✓ Valid',
  dirty:   '⚠ Dirty',
  pit:     'Pit',
  reset:   '↺ Reset',
  out_lap: 'Out lap',
  unknown: '—',
};

const VALID_STATUSES = new Set(Object.keys(STATUS_LABELS));

export class LapStatus {
  /** @type {HTMLElement} */
  #el;

  /** @type {string} */
  #current = 'unknown';

  /**
   * @param {HTMLElement} el  The badge element (e.g. #lap-status).
   */
  constructor(el) {
    this.#el = el;
    this.set('unknown');
  }

  /**
   * Update the badge from a telemetry data payload.
   * Reads `data.lap_status` if present.
   *
   * @param {object} data  Telemetry `data` object.
   */
  updateFromTelemetry(data) {
    if (data?.lap_status) {
      this.set(data.lap_status);
    }
  }

  /**
   * Directly set the status.
   * @param {string} status  One of the STATUS_LABELS keys.
   */
  set(status) {
    const normalized = VALID_STATUSES.has(status) ? status : 'unknown';
    if (normalized === this.#current) return;
    this.#current = normalized;
    this.#el.dataset.status = normalized;
    this.#el.textContent = STATUS_LABELS[normalized];
  }

  /** Current status string. */
  get current() {
    return this.#current;
  }
}
