/**
 * dyno-graph.js — Guided in-session dynamometer panel.
 *
 * State machine driven by `dyno_update` WS events from the sidecar.
 * Phases: waiting_for_ready → ready_to_go → collecting → complete
 */

import { makeDraggable } from './drag.js';

const W_TO_HP  = 1 / 745.7;
const NM_TO_FT = 0.7376;

const DRIVETRAIN_LABEL = { 0: 'FWD', 1: 'RWD', 2: 'AWD' };

const PAD_L = 54, PAD_R = 54, PAD_T = 20, PAD_B = 32;

export class DynoGraph {
  /** @type {HTMLElement} */
  #root;
  #visible = false;
  #imperial = false;
  #phase = 'waiting_for_ready';
  #targetGear = 1;
  #drivetrain = 2; // default AWD
  #stoppedProgress = 0; // 0–1 from telemetry data.dyno.stopped_progress

  /** @type {Array<{rpm:number,power_w:number,torque_nm:number}>} */
  #bins = [];
  #redlineRpm   = null;
  #powerBandRpm = null;
  #liveRpm  = 0;

  #els = {};

  constructor(rootEl) {
    this.#root = rootEl;
    this.#build();
  }

  show()   { this.#visible = true;  this.#root.hidden = false; }
  hide()   { this.#visible = false; this.#root.hidden = true;  }
  toggle() { this.#visible ? this.hide() : this.show(); }

  /** Call on every `dyno_update` WS event. */
  onDynoUpdate(payload) {
    this.#phase        = payload.phase ?? 'waiting_for_ready';
    this.#targetGear   = payload.target_gear ?? 1;
    this.#drivetrain   = payload.drivetrain ?? 2;
    this.#bins         = (payload.bins ?? []).map(b => ({
      rpm:       b.rpm,
      power_w:   b.power_w   ?? 0,
      torque_nm: b.torque_nm ?? 0,
    }));
    this.#redlineRpm   = payload.detected_redline_rpm ?? null;
    this.#powerBandRpm = payload.power_band_start_rpm ?? null;
    this.#render();
  }

  /** Call on every `telemetry` WS event. */
  onTelemetry(data) {
    if (data?.rpm != null) this.#liveRpm = data.rpm;

    // Read real-time dyno fields from the telemetry packet
    if (data?.dyno) {
      const d = data.dyno;
      if (d.phase)            this.#phase          = d.phase;
      if (d.target_gear)      this.#targetGear     = d.target_gear;
      if (d.drivetrain != null) this.#drivetrain   = d.drivetrain;
      if (d.stopped_progress != null) this.#stoppedProgress = d.stopped_progress;
    }

    if (!this.#visible) return;

    if (this.#phase === 'complete' && this.#bins.length > 0) {
      this.#drawGraph();
    } else if (this.#phase === 'waiting_for_ready') {
      // Update stop-progress bar in real time without a full re-render
      if (this.#els.stopProgress) {
        this.#els.stopProgress.style.width = `${(this.#stoppedProgress * 100).toFixed(1)}%`;
      }
    }
  }

  // ── Private ────────────────────────────────────────────────

  #build() {
    this.#root.innerHTML = `
      <div class="dyno-header">
        <span class="dyno-title" data-dyno="title">Dyno</span>
        <div class="dyno-header-btns">
          <button class="dyno-btn" data-dyno="unit-toggle">SI</button>
          <button class="dyno-btn" data-dyno="retry" hidden>Retry</button>
        </div>
      </div>
      <div class="dyno-instructions" data-dyno="instructions"></div>
      <div class="dyno-stop-progress-wrap" data-dyno="stop-wrap" hidden>
        <div class="dyno-stop-progress-track">
          <div class="dyno-stop-progress-fill" data-dyno="stop-progress" style="width:0%"></div>
        </div>
        <span class="dyno-stop-label" data-dyno="stop-label">Hold…</span>
      </div>
      <div class="dyno-graph-wrap" data-dyno="graph-wrap" hidden>
        <canvas class="dyno-canvas" data-dyno="canvas" width="360" height="180"></canvas>
        <div class="dyno-stats" data-dyno="stats"></div>
      </div>
    `;

    const q = (s) => this.#root.querySelector(`[data-dyno="${s}"]`);
    this.#els = {
      title:        q('title'),
      instructions: q('instructions'),
      stopWrap:     q('stop-wrap'),
      stopProgress: q('stop-progress'),
      stopLabel:    q('stop-label'),
      graphWrap:    q('graph-wrap'),
      canvas:       q('canvas'),
      stats:        q('stats'),
      unitToggle:   q('unit-toggle'),
      retry:        q('retry'),
    };

    this.#els.unitToggle.addEventListener('click', () => {
      this.#imperial = !this.#imperial;
      this.#els.unitToggle.textContent = this.#imperial ? 'Imperial' : 'SI';
      this.#render();
    });

    this.#els.retry.addEventListener('click', () => {
      fetch('/api/v1/dyno/reset', { method: 'POST' }).catch(() => {});
    });

    // Make the panel draggable by its header
    makeDraggable(this.#root, this.#root.querySelector('.dyno-header'));
  }

  /** Generate phase-specific instructions based on current drivetrain/gear. */
  #phaseText() {
    const dt    = DRIVETRAIN_LABEL[this.#drivetrain] ?? 'AWD';
    const gear  = this.#targetGear;
    const gearOrdinal = gear === 1 ? '1st' : gear === 2 ? '2nd' : `${gear}th`;

    switch (this.#phase) {
      case 'waiting_for_ready':
        return {
          title: 'Dyno — Setup',
          body: `Drive to a long straight and come to a complete stop. ` +
                `Select ${gearOrdinal} gear (detected: ${dt}). ` +
                `Turn Traction Control OFF — TC will cut throttle and corrupt the data. ` +
                `Hold stopped in ${gearOrdinal} for 3 seconds to arm the dyno.`,
        };
      case 'ready_to_go':
        return {
          title: 'Dyno — Ready! Go!',
          body: `Apply full throttle (100%) and hold it pinned to the rev limiter. ` +
                `Do not shift — stay in ${gearOrdinal} gear the entire pull.`,
        };
      case 'collecting':
        return {
          title: 'Dyno — Collecting…',
          body: `Full throttle, stay in ${gearOrdinal} gear. Do not lift or shift.`,
        };
      case 'complete':
        return { title: 'Dyno — Complete', body: 'Results shown below.' };
      default:
        return { title: 'Dyno', body: '' };
    }
  }

  #render() {
    const { title, body } = this.#phaseText();
    this.#els.title.textContent        = title;
    this.#els.instructions.textContent = body;

    const waiting = this.#phase === 'waiting_for_ready';
    const done    = this.#phase === 'complete';

    this.#els.stopWrap.hidden  = !waiting;
    this.#els.retry.hidden     = !done;
    this.#els.graphWrap.hidden = !done || this.#bins.length === 0;

    if (waiting) {
      this.#els.stopProgress.style.width = `${(this.#stoppedProgress * 100).toFixed(1)}%`;
      this.#els.stopLabel.textContent    = this.#stoppedProgress >= 1 ? 'Armed!' : 'Hold…';
    }

    if (done && this.#bins.length > 0) {
      this.#drawGraph();
      this.#renderStats();
    }
  }

  #drawGraph() {
    const canvas = this.#els.canvas;
    const ctx    = canvas.getContext('2d');
    const W = canvas.width;
    const H = canvas.height;
    const gW = W - PAD_L - PAD_R;
    const gH = H - PAD_T - PAD_B;

    ctx.clearRect(0, 0, W, H);
    if (this.#bins.length === 0) return;

    const rpmVals  = this.#bins.map(b => b.rpm);
    const rpmMin   = Math.min(...rpmVals);
    const rpmMax   = Math.max(...rpmVals);

    const powerVals = this.#bins.map(b => this.#imperial ? b.power_w * W_TO_HP   : b.power_w / 1000);
    const torqVals  = this.#bins.map(b => this.#imperial ? b.torque_nm * NM_TO_FT : b.torque_nm);
    const pMax = Math.max(...powerVals, 0.01);
    const tMax = Math.max(...torqVals,  0.01);

    const xS  = (rpm) => PAD_L + ((rpm - rpmMin) / (rpmMax - rpmMin || 1)) * gW;
    const yP  = (v)   => PAD_T + gH - (v / pMax) * gH;
    const yT  = (v)   => PAD_T + gH - (v / tMax) * gH;

    // Grid
    ctx.strokeStyle = 'rgba(255,255,255,0.07)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const y = PAD_T + (gH / 4) * i;
      ctx.beginPath(); ctx.moveTo(PAD_L, y); ctx.lineTo(PAD_L + gW, y); ctx.stroke();
    }

    // Power-band start (cyan dashed)
    if (this.#powerBandRpm != null && this.#powerBandRpm >= rpmMin) {
      const x = xS(this.#powerBandRpm);
      ctx.save();
      ctx.strokeStyle = 'rgba(0,220,255,0.55)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 3]);
      ctx.beginPath(); ctx.moveTo(x, PAD_T); ctx.lineTo(x, PAD_T + gH); ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(0,220,255,0.75)';
      ctx.font = '9px monospace';
      ctx.textAlign = 'left';
      ctx.fillText(`${Math.round(this.#powerBandRpm)}`, x + 3, PAD_T + 11);
      ctx.restore();
    }

    // Redline (red dashed)
    if (this.#redlineRpm != null) {
      const x = xS(this.#redlineRpm);
      ctx.save();
      ctx.strokeStyle = 'rgba(240,74,74,0.7)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 3]);
      ctx.beginPath(); ctx.moveTo(x, PAD_T); ctx.lineTo(x, PAD_T + gH); ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(240,74,74,0.9)';
      ctx.font = '9px monospace';
      ctx.textAlign = 'right';
      ctx.fillText(`RL ${Math.round(this.#redlineRpm)}`, x - 3, PAD_T + 11);
      ctx.restore();
    }

    // Torque line (orange)
    ctx.strokeStyle = '#f0844a';
    ctx.lineWidth = 2;
    ctx.beginPath();
    this.#bins.forEach((b, i) => {
      const v = this.#imperial ? b.torque_nm * NM_TO_FT : b.torque_nm;
      i === 0 ? ctx.moveTo(xS(b.rpm), yT(v)) : ctx.lineTo(xS(b.rpm), yT(v));
    });
    ctx.stroke();

    // Power line (blue)
    ctx.strokeStyle = '#4d9fff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    this.#bins.forEach((b, i) => {
      const v = this.#imperial ? b.power_w * W_TO_HP : b.power_w / 1000;
      i === 0 ? ctx.moveTo(xS(b.rpm), yP(v)) : ctx.lineTo(xS(b.rpm), yP(v));
    });
    ctx.stroke();

    // X-axis RPM labels
    ctx.fillStyle = 'rgba(200,200,220,0.6)';
    ctx.font = '9px monospace';
    ctx.textAlign = 'center';
    const midRpm = Math.round((rpmMin + rpmMax) / 2 / 100) * 100;
    [rpmMin, midRpm, rpmMax].forEach(rpm => {
      ctx.fillText(`${(rpm / 1000).toFixed(1)}k`, xS(rpm), PAD_T + gH + 16);
    });

    // Y-axis peak labels
    const pUnit = this.#imperial ? 'HP'    : 'kW';
    const tUnit = this.#imperial ? 'ft·lb' : 'Nm';
    ctx.fillStyle = '#4d9fff';
    ctx.textAlign = 'right';
    ctx.fillText(`${Math.round(pMax)} ${pUnit}`, PAD_L - 4, PAD_T + 10);
    ctx.fillStyle = '#f0844a';
    ctx.textAlign = 'left';
    ctx.fillText(`${Math.round(tMax)} ${tUnit}`, PAD_L + gW + 4, PAD_T + 10);

    // Live RPM tick (white vertical)
    if (this.#liveRpm > 0 && rpmMax > rpmMin) {
      const clamp = Math.min(Math.max(this.#liveRpm, rpmMin), rpmMax);
      const x = xS(clamp);
      ctx.strokeStyle = 'rgba(255,255,255,0.55)';
      ctx.lineWidth = 1.5;
      ctx.beginPath(); ctx.moveTo(x, PAD_T); ctx.lineTo(x, PAD_T + gH); ctx.stroke();
    }
  }

  #renderStats() {
    if (this.#bins.length === 0) { this.#els.stats.textContent = ''; return; }

    const pScale = this.#imperial ? W_TO_HP   : 1 / 1000;
    const tScale = this.#imperial ? NM_TO_FT  : 1;
    const pUnit  = this.#imperial ? 'HP'      : 'kW';
    const tUnit  = this.#imperial ? 'ft·lb'   : 'Nm';

    const maxP = this.#bins.reduce((a, b) => b.power_w   > a.power_w   ? b : a, this.#bins[0]);
    const maxT = this.#bins.reduce((a, b) => b.torque_nm > a.torque_nm ? b : a, this.#bins[0]);
    const rl  = this.#redlineRpm   ? `${Math.round(this.#redlineRpm)} RPM`   : '—';
    const pbs = this.#powerBandRpm ? `${Math.round(this.#powerBandRpm)} RPM` : '—';

    this.#els.stats.innerHTML = `
      <div class="dyno-stat">
        <span class="dyno-stat-label">Peak Power</span>
        <span class="dyno-stat-val">${Math.round(maxP.power_w * pScale)} ${pUnit} @ ${Math.round(maxP.rpm)} RPM</span>
      </div>
      <div class="dyno-stat">
        <span class="dyno-stat-label">Peak Torque</span>
        <span class="dyno-stat-val">${Math.round(maxT.torque_nm * tScale)} ${tUnit} @ ${Math.round(maxT.rpm)} RPM</span>
      </div>
      <div class="dyno-stat">
        <span class="dyno-stat-label">Redline</span>
        <span class="dyno-stat-val">${rl}</span>
      </div>
      <div class="dyno-stat">
        <span class="dyno-stat-label">Power Band Start</span>
        <span class="dyno-stat-val">${pbs}</span>
      </div>
    `;
  }
}
