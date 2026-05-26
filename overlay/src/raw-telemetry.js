/**
 * raw-telemetry.js — Collapsible panel for raw Forza telemetry data.
 *
 * Usage:
 *   import { RawTelemetryPanel } from './raw-telemetry.js';
 *   const raw = new RawTelemetryPanel(
 *     document.getElementById('raw-telemetry-panel'),
 *     document.getElementById('raw-toggle'),
 *   );
 *   // On telemetry events:
 *   raw.update(data.raw);
 */

import { makeDraggable } from './drag.js';

/** Label overrides for well-known fields. */
const LABELS = {
  speed_mps: 'Speed (m/s)',
  tire_temp_fl_f: 'Tire Temp FL (°F)',
  tire_temp_fr_f: 'Tire Temp FR (°F)',
  tire_temp_rl_f: 'Tire Temp RL (°F)',
  tire_temp_rr_f: 'Tire Temp RR (°F)',
  tire_wear_fl: 'Tire Wear FL',
  tire_wear_fr: 'Tire Wear FR',
  tire_wear_rl: 'Tire Wear RL',
  tire_wear_rr: 'Tire Wear RR',
  current_engine_rpm: 'Engine RPM',
  engine_max_rpm: 'Max RPM',
  engine_idle_rpm: 'Idle RPM',
  power_w: 'Power (W)',
  torque_nm: 'Torque (Nm)',
  boost_bar: 'Boost (bar)',
  fuel: 'Fuel',
  dist_m: 'Distance (m)',
  best_lap_s: 'Best Lap (s)',
  last_lap_s: 'Last Lap (s)',
  current_lap_s: 'Lap Time (s)',
  race_time_s: 'Race Time (s)',
  lap_number: 'Lap #',
  race_pos: 'Race Pos',
  timestamp_ms: 'Timestamp (ms)',
  car_ordinal: 'Car Ordinal',
  car_class: 'Car Class',
  car_pi: 'Car PI',
  drivetrain: 'Drivetrain',
  num_cylinders: 'Cylinders',
  gear_raw: 'Gear (raw)',
  steer_raw: 'Steer (raw)',
  accel_raw: 'Accel (raw)',
  brake_raw: 'Brake (raw)',
  clutch_raw: 'Clutch (raw)',
  hand_brake_raw: 'Hand Brake (raw)',
  driving_line: 'Driving Line',
  ai_brake_diff: 'AI Brake Diff',
  track_ordinal: 'Track Ordinal',
};

/** Format a numeric value to 3 decimal places when it isn't an integer. */
function fmt(val) {
  if (val === null || val === undefined) return 'N/A';
  if (typeof val === 'boolean') return String(val);
  if (Number.isInteger(val)) return String(val);
  if (typeof val === 'number') return val.toFixed(3);
  return String(val);
}

export class RawTelemetryPanel {
  /** @type {HTMLElement} */
  #panel;

  /** @type {HTMLButtonElement} */
  #btn;

  /** @type {HTMLTableSectionElement | null} */
  #tbody = null;

  /** @type {boolean} */
  #visible = false;

  /** @type {Record<string, number | null> | null} */
  #lastRaw = null;

  /**
   * @param {HTMLElement}       panelEl  The #raw-telemetry-panel container.
   * @param {HTMLButtonElement} btnEl    The #raw-toggle button.
   */
  constructor(panelEl, btnEl) {
    this.#panel = panelEl;
    this.#btn = btnEl;

    this.#panel.innerHTML = `
      <div class="raw-header">Raw Telemetry</div>
      <div class="raw-scroll">
        <table class="raw-table">
          <tbody></tbody>
        </table>
      </div>
    `;
    this.#tbody = this.#panel.querySelector('tbody');

    this.#btn.addEventListener('click', () => this.toggle());

    // Make the panel draggable by its header
    makeDraggable(this.#panel, this.#panel.querySelector('.raw-header'));

    this.#setVisible(false);
  }

  /**
   * Refresh the panel with the latest raw data.
   * @param {Record<string, number | null>} raw  The data.raw object.
   */
  update(raw) {
    this.#lastRaw = raw;
    if (!raw || !this.#visible) return;
    this.#render(raw);
  }

  toggle() {
    this.#setVisible(!this.#visible);
  }

  // ── Private ────────────────────────────────────────────────

  #setVisible(vis) {
    this.#visible = vis;
    this.#panel.hidden = !vis;
    this.#btn.textContent = vis ? 'Hide Raw' : 'Raw Data';
    this.#btn.classList.toggle('active', vis);
    if (vis && this.#lastRaw) this.#render(this.#lastRaw);
  }

  #render(raw) {
    if (!this.#tbody) return;
    const rows = Object.entries(raw)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([key, val]) => {
        const label = LABELS[key] ?? key.replace(/_/g, ' ');
        const isNull = val === null || val === undefined;
        return `<tr>
          <td class="raw-key">${label}</td>
          <td class="raw-val${isNull ? ' raw-null' : ''}">${fmt(val)}</td>
        </tr>`;
      });
    this.#tbody.innerHTML = rows.join('');
  }
}
