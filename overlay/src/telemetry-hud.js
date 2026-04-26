/**
 * telemetry-hud.js — Live telemetry HUD primitives.
 *
 * Renders speed, gear, RPM bar, throttle/brake bars, steering indicator,
 * and lap clock from a `telemetry` WS event payload.
 *
 * Usage:
 *   import { TelemetryHud } from './telemetry-hud.js';
 *   const hud = new TelemetryHud(document.getElementById('telemetry-hud'));
 *   hud.update(telemetryData);
 *   hud.setUnit('mph'); // or 'kph' (default)
 */

const KPH_TO_MPH = 0.621371;

export class TelemetryHud {
  /** @type {HTMLElement} */
  #root;

  /** @type {'kph' | 'mph'} */
  #unit = 'kph';

  /** @type {boolean} */
  #visible = false;

  // DOM refs populated in #build()
  #els = {};

  /**
   * @param {HTMLElement} rootEl  The #telemetry-hud container.
   */
  constructor(rootEl) {
    this.#root = rootEl;
    this.#build();
    this.hide(); // hidden by default per spec
  }

  /** Show the HUD. */
  show() {
    this.#visible = true;
    this.#root.classList.remove('hidden');
  }

  /** Hide the HUD. */
  hide() {
    this.#visible = false;
    this.#root.classList.add('hidden');
  }

  /** Toggle visibility. */
  toggle() {
    this.#visible ? this.hide() : this.show();
  }

  /** @param {'kph'|'mph'} unit */
  setUnit(unit) {
    this.#unit = unit === 'mph' ? 'mph' : 'kph';
    this.#els.speedUnit.textContent = this.#unit;
  }

  /**
   * Update the HUD with a fresh telemetry data object.
   * All fields are optional; missing fields are left unchanged.
   *
   * @param {object} data  Telemetry `data` payload from the WS envelope.
   */
  update(data) {
    if (!data) return;

    // Speed
    if (data.speed_kph != null) {
      const speed = this.#unit === 'mph'
        ? Math.round(data.speed_kph * KPH_TO_MPH)
        : Math.round(data.speed_kph);
      this.#els.speed.textContent = speed;
    }

    // Gear
    if (data.gear != null) {
      this.#els.gear.textContent = data.gear === 0 ? 'R' : String(data.gear);
    }

    // RPM bar
    if (data.rpm != null && data.rpm_max != null && data.rpm_max > 0) {
      const frac = Math.min(data.rpm / data.rpm_max, 1);
      this.#setBar(this.#els.rpmFill, frac);
      // Color cue: green → amber → red near rev-limit
      let color;
      if (frac < 0.75) {
        color = 'var(--color-rpm-safe)';
      } else if (frac < 0.92) {
        color = 'var(--color-rpm-warn)';
      } else {
        color = 'var(--color-rpm-limit)';
      }
      this.#els.rpmFill.style.background = color;
      this.#els.rpmLabel.textContent = `RPM ${Math.round(data.rpm).toLocaleString()}`;
    }

    // Throttle bar (0–1 → 0–100%)
    if (data.throttle != null) {
      this.#setBar(this.#els.throttleFill, Math.min(Math.max(data.throttle, 0), 1));
    }

    // Brake bar
    if (data.brake != null) {
      this.#setBar(this.#els.brakeFill, Math.min(Math.max(data.brake, 0), 1));
    }

    // Steering: normalized [-1, 1] → thumb position [0%, 100%]
    if (data.steer != null) {
      const pct = ((Math.min(Math.max(data.steer, -1), 1) + 1) / 2) * 100;
      this.#els.steerThumb.style.left = `${pct}%`;
    }

    // Lap clock
    if (data.lap) {
      const lap = data.lap;
      if (lap.current_s != null) {
        this.#els.lapTime.textContent = this.#formatTime(lap.current_s);
      }
      if (lap.best_s != null && lap.current_s != null && lap.best_s > 0) {
        const delta = lap.current_s - lap.best_s;
        this.#renderDelta(delta, lap.current_s, lap.best_s);
      } else {
        this.#els.lapDelta.textContent = '';
        this.#els.lapDelta.className = 'hud-lap-delta hud-lap-delta--neutral';
      }
    }
  }

  // ── Private helpers ────────────────────────────────────────

  #build() {
    this.#root.innerHTML = `
      <div class="hud-gear" data-hud="gear">N</div>

      <div class="hud-speed-wrap">
        <span class="hud-speed" data-hud="speed">0</span>
        <span class="hud-speed-unit" data-hud="speed-unit">${this.#unit}</span>
      </div>

      <div class="hud-rpm-wrap">
        <span class="hud-rpm-label" data-hud="rpm-label">RPM 0</span>
        <div class="hud-rpm-bar-track">
          <div class="hud-rpm-bar-fill" data-hud="rpm-fill" style="width:0%"></div>
        </div>
      </div>

      <div class="hud-pedals">
        <div class="hud-pedal hud-pedal--throttle">
          <span class="hud-pedal-label">THR</span>
          <div class="hud-pedal-track">
            <div class="hud-pedal-fill" data-hud="throttle-fill" style="width:0%"></div>
          </div>
        </div>
        <div class="hud-pedal hud-pedal--brake">
          <span class="hud-pedal-label">BRK</span>
          <div class="hud-pedal-track">
            <div class="hud-pedal-fill" data-hud="brake-fill" style="width:0%"></div>
          </div>
        </div>
      </div>

      <div class="hud-steer-wrap">
        <span class="hud-steer-label">Steer</span>
        <div class="hud-steer-track">
          <div class="hud-steer-center"></div>
          <div class="hud-steer-thumb" data-hud="steer-thumb" style="left:50%"></div>
        </div>
      </div>

      <div class="hud-lap-clock">
        <span class="hud-lap-time" data-hud="lap-time">--:--.---</span>
        <span class="hud-lap-delta hud-lap-delta--neutral" data-hud="lap-delta"></span>
      </div>
    `;

    const q = (sel) => this.#root.querySelector(`[data-hud="${sel}"]`);
    this.#els = {
      gear:         q('gear'),
      speed:        q('speed'),
      speedUnit:    q('speed-unit'),
      rpmFill:      q('rpm-fill'),
      rpmLabel:     q('rpm-label'),
      throttleFill: q('throttle-fill'),
      brakeFill:    q('brake-fill'),
      steerThumb:   q('steer-thumb'),
      lapTime:      q('lap-time'),
      lapDelta:     q('lap-delta'),
    };
  }

  /**
   * Set a progress-bar fill to a fraction [0, 1].
   * @param {HTMLElement} el
   * @param {number} frac
   */
  #setBar(el, frac) {
    el.style.width = `${(frac * 100).toFixed(1)}%`;
  }

  /**
   * Format seconds as `m:ss.SSS`.
   * @param {number} totalSeconds
   * @returns {string}
   */
  #formatTime(totalSeconds) {
    if (totalSeconds < 0) return '--:--.---';
    const m = Math.floor(totalSeconds / 60);
    const s = totalSeconds % 60;
    return `${m}:${s.toFixed(3).padStart(6, '0')}`;
  }

  /**
   * Render the lap delta vs best.
   * @param {number} delta   current_s - best_s
   * @param {number} currentS
   * @param {number} bestS
   */
  #renderDelta(delta, currentS, bestS) {
    // Don't show a delta on the out-lap (best not set yet) or the very
    // first few seconds when the current time is well under the best.
    if (bestS <= 0 || currentS < 5) {
      this.#els.lapDelta.textContent = '';
      return;
    }
    const sign = delta >= 0 ? '+' : '';
    this.#els.lapDelta.textContent = `${sign}${delta.toFixed(3)}`;
    const cls = delta > 0
      ? 'hud-lap-delta--positive'
      : delta < 0
        ? 'hud-lap-delta--negative'
        : 'hud-lap-delta--neutral';
    this.#els.lapDelta.className = `hud-lap-delta ${cls}`;
  }
}
