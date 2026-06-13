/**
 * track-map.js — Track view: traced map, inferred edge envelope, racing-line gauge.
 *
 * Built entirely from telemetry already emitted by the sidecar (no extra Rust):
 *   data.raw.pos_x / pos_z   — car position in track-space metres (ground plane)
 *   data.raw.dist_m          — cumulative distance travelled
 *   data.raw.lap_number      — lap counter (used to derive lap-relative distance)
 *   data.raw.driving_line    — NormalizedDrivingLine [-127..127], 0 = on the AI line
 *   data.raw.track_ordinal   — track id (reset accumulation when it changes)
 *   data.is_race_on          — only accumulate while racing
 *
 * Forza gives no real track boundaries, so the edges are *inferred*: we bin the
 * driven path by lap-relative distance into fixed stations, average each
 * station to a centreline, then project the spread of driven points either side
 * of the centreline to estimate left/right edges. The estimate sharpens as the
 * driver uses more of the track across laps.
 */

import { makeDraggable } from './drag.js';

/** Metres of track per centreline station (≈ the live-telemetry sample spacing). */
export const STATION_M = 10;
/** Minimum spacing between stored envelope points (metres). */
const DECIMATE_M = 2.0;
/** Cap on the envelope point ring buffer. */
const MAX_POINTS = 12000;
/** A drop in dist_m larger than this (metres) signals a new lap/session/track. */
const DIST_RESET_M = 50;
/** Don't connect centreline/edge nodes across a station-index gap wider than this. */
const MAX_GAP_STATIONS = 3;

const PAD = 14;

/**
 * Create an empty accumulation model.
 * @returns {object}
 */
export function createModel() {
  return {
    trackOrdinal: null,
    anchored: false,
    lapStartDist: null,
    lastDist: null,
    lapNumber: null,
    /** @type {Map<number, {n:number,sumX:number,sumZ:number}>} */
    stations: new Map(),
    /** @type {Array<{x:number,z:number,station:number}>} decimated ring buffer */
    points: [],
    lastStoredX: null,
    lastStoredZ: null,
  };
}

/**
 * Fold one telemetry sample into the model.
 *
 * Station 0 is anchored to the track's start/finish line so the same physical
 * spot maps to the same station on every lap (letting the edge envelope build
 * across laps). The start/finish line is detected as a lap boundary: either a
 * lap-number increment or a backwards jump in distance (replay loop / lap
 * rollover). The very first boundary also discards the initial partial lap,
 * whose stations were numbered from an arbitrary mid-lap connection point.
 *
 * @param {object} model   model from createModel()
 * @param {object} s       { x, z, dist, lap, track }
 * @returns {object} the same model (mutated)
 */
export function accumulate(model, s) {
  const { x, z, dist, lap, track } = s;
  if (!Number.isFinite(x) || !Number.isFinite(z) || !Number.isFinite(dist)) {
    return model;
  }

  // Reset when the track changes.
  if (track != null && track !== model.trackOrdinal) {
    resetModel(model);
    model.trackOrdinal = track;
  }

  const backwardJump = model.lastDist != null && dist < model.lastDist - DIST_RESET_M;
  const lapChange = lap != null && model.lapNumber != null && lap !== model.lapNumber;

  if (backwardJump || lapChange) {
    if (!model.anchored) {
      // First real start/finish crossing: throw away the partial-lap stations
      // (numbered from wherever we connected) and anchor cleanly from here.
      resetModel(model);
      model.anchored = true;
    }
    model.lapStartDist = dist;
  }

  if (model.lapStartDist == null) model.lapStartDist = dist;
  model.lapNumber = lap ?? model.lapNumber;

  let lapDist = dist - model.lapStartDist;
  if (lapDist < 0) lapDist = 0;
  const station = Math.floor(lapDist / STATION_M);

  // Online centreline mean per station.
  let st = model.stations.get(station);
  if (!st) {
    st = { n: 0, sumX: 0, sumZ: 0 };
    model.stations.set(station, st);
  }
  st.n += 1;
  st.sumX += x;
  st.sumZ += z;

  // Decimated point buffer for edge spread.
  if (
    model.lastStoredX == null ||
    Math.hypot(x - model.lastStoredX, z - model.lastStoredZ) >= DECIMATE_M
  ) {
    model.points.push({ x, z, station });
    model.lastStoredX = x;
    model.lastStoredZ = z;
    if (model.points.length > MAX_POINTS) model.points.shift();
  }

  model.lastDist = dist;
  return model;
}

/** Clear accumulated geometry. */
export function resetModel(model) {
  model.anchored = false;
  model.lapStartDist = null;
  model.lastDist = null;
  model.lapNumber = null;
  model.stations.clear();
  model.points = [];
  model.lastStoredX = null;
  model.lastStoredZ = null;
}

/**
 * Build the ordered centreline (station means) as an array sorted by station.
 * @returns {Array<{station:number,x:number,z:number}>}
 */
export function centerline(model) {
  const out = [];
  for (const [station, st] of model.stations) {
    if (st.n > 0) out.push({ station, x: st.sumX / st.n, z: st.sumZ / st.n });
  }
  out.sort((a, b) => a.station - b.station);
  return out;
}

/**
 * Estimate left/right track edges by projecting the driven-point spread onto
 * the per-station normal of the centreline.
 *
 * @param {object} model
 * @returns {{ center: Array, left: Array, right: Array }}
 *   Each array is a list of {x,z}. left/right may be shorter (only stations
 *   with enough spread data are included).
 */
export function edges(model) {
  const center = centerline(model);
  if (center.length < 3) return { center, left: [], right: [] };

  // Index centreline by station for O(1) lookup and neighbour tangents.
  const byStation = new Map();
  center.forEach((c, i) => byStation.set(c.station, i));

  // Per-station unit normal (perpendicular to the local tangent).
  const normals = center.map((c, i) => {
    const prev = center[Math.max(0, i - 1)];
    const next = center[Math.min(center.length - 1, i + 1)];
    let tx = next.x - prev.x;
    let tz = next.z - prev.z;
    const len = Math.hypot(tx, tz) || 1;
    tx /= len;
    tz /= len;
    // Normal = rotate tangent by +90°.
    return { nx: -tz, nz: tx };
  });

  // Per-station lateral min/max from buffered points.
  const lat = center.map(() => ({ min: 0, max: 0, has: false }));
  for (const p of model.points) {
    const idx = byStation.get(p.station);
    if (idx == null) continue;
    const c = center[idx];
    const n = normals[idx];
    const d = (p.x - c.x) * n.nx + (p.z - c.z) * n.nz;
    const l = lat[idx];
    if (!l.has) {
      l.min = d;
      l.max = d;
      l.has = true;
    } else {
      if (d < l.min) l.min = d;
      if (d > l.max) l.max = d;
    }
  }

  // Add a small margin so the band sits just outside the widest driven line.
  const MARGIN = 1.0;
  const left = [];
  const right = [];
  center.forEach((c, i) => {
    const l = lat[i];
    if (!l.has) return;
    const n = normals[i];
    left.push({ station: c.station, x: c.x + n.nx * (l.max + MARGIN), z: c.z + n.nz * (l.max + MARGIN) });
    right.push({ station: c.station, x: c.x + n.nx * (l.min - MARGIN), z: c.z + n.nz * (l.min - MARGIN) });
  });

  return { center, left, right };
}

/**
 * Compute a world→screen transform fitting all points into a w×h canvas.
 *
 * @param {Array<{x:number,z:number}>} pts
 * @param {number} w
 * @param {number} h
 * @param {number} [pad]
 * @returns {?{scale:number,minX:number,minZ:number,offX:number,offY:number,project:Function}}
 */
export function fitTransform(pts, w, h, pad = PAD) {
  if (!pts.length) return null;
  let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
  for (const p of pts) {
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.z < minZ) minZ = p.z;
    if (p.z > maxZ) maxZ = p.z;
  }
  const spanX = maxX - minX || 1;
  const spanZ = maxZ - minZ || 1;
  const scale = Math.min((w - 2 * pad) / spanX, (h - 2 * pad) / spanZ);
  // Centre the drawing in the canvas.
  const offX = pad + ((w - 2 * pad) - spanX * scale) / 2;
  const offY = pad + ((h - 2 * pad) - spanZ * scale) / 2;
  const project = (x, z) => ({
    px: offX + (x - minX) * scale,
    // Flip Z so the map reads like a top-down view (north-up-ish).
    py: h - (offY + (z - minZ) * scale),
  });
  return { scale, minX, minZ, offX, offY, project };
}

/**
 * Map a NormalizedDrivingLine value [-127..127] to a [-1, 1] gauge fraction
 * (negative = left of the line, positive = right). Clamped.
 * @param {number} drivingLine
 * @returns {number}
 */
export function gaugeFraction(drivingLine) {
  if (!Number.isFinite(drivingLine)) return 0;
  return Math.max(-1, Math.min(1, drivingLine / 127));
}

export class TrackMap {
  /** @type {HTMLElement} */
  #root;
  #visible = false;
  #model = createModel();
  #els = {};
  #car = { x: null, z: null, drivingLine: 0, raceOn: false };
  #renderScheduled = false;

  constructor(rootEl) {
    this.#root = rootEl;
    this.#build();
  }

  show()   { this.#visible = true;  this.#root.hidden = false; this.#scheduleRender(); }
  hide()   { this.#visible = false; this.#root.hidden = true;  }
  toggle() { this.#visible ? this.hide() : this.show(); }
  isVisible() { return this.#visible; }

  /** Call on every `telemetry` WS event with the full frame. */
  onTelemetry(data) {
    const raw = data?.raw;
    if (!raw) return;
    this.#car.raceOn = data.is_race_on === true;
    this.#car.x = Number.isFinite(raw.pos_x) ? raw.pos_x : this.#car.x;
    this.#car.z = Number.isFinite(raw.pos_z) ? raw.pos_z : this.#car.z;
    this.#car.drivingLine = Number.isFinite(raw.driving_line) ? raw.driving_line : 0;

    if (this.#car.raceOn) {
      accumulate(this.#model, {
        x: raw.pos_x,
        z: raw.pos_z,
        dist: raw.dist_m,
        lap: raw.lap_number,
        track: raw.track_ordinal,
      });
    }

    if (this.#visible) this.#scheduleRender();
  }

  /** Throw away the accumulated map (e.g. user pressed Clear). */
  clear() {
    resetModel(this.#model);
    this.#model.trackOrdinal = null;
    if (this.#visible) this.#scheduleRender();
  }

  // ── Private ────────────────────────────────────────────────

  #build() {
    this.#root.innerHTML = `
      <div class="track-header">
        <span class="track-title" data-track="title">Track</span>
        <div class="track-header-btns">
          <button class="track-btn" data-track="clear">Clear</button>
        </div>
      </div>
      <div class="track-body">
        <canvas class="track-canvas" data-track="canvas" width="260" height="180"></canvas>
        <div class="track-line-gauge" data-track="gauge">
          <div class="track-line-gauge-label">Racing line</div>
          <div class="track-line-gauge-track">
            <div class="track-line-gauge-center"></div>
            <div class="track-line-gauge-dot" data-track="gauge-dot"></div>
          </div>
          <div class="track-line-gauge-val" data-track="gauge-val">on line</div>
        </div>
        <div class="track-empty" data-track="empty">Waiting for track data…</div>
      </div>
    `;

    const q = (s) => this.#root.querySelector(`[data-track="${s}"]`);
    this.#els = {
      title:   q('title'),
      canvas:  q('canvas'),
      gauge:   q('gauge'),
      gaugeDot: q('gauge-dot'),
      gaugeVal: q('gauge-val'),
      empty:   q('empty'),
      clear:   q('clear'),
    };

    this.#els.clear.addEventListener('click', () => this.clear());

    makeDraggable(this.#root, this.#root.querySelector('.track-header'));
  }

  #scheduleRender() {
    if (this.#renderScheduled) return;
    this.#renderScheduled = true;
    const raf = (typeof requestAnimationFrame === 'function')
      ? requestAnimationFrame
      : (cb) => setTimeout(cb, 16);
    raf(() => {
      this.#renderScheduled = false;
      this.#render();
    });
  }

  #render() {
    this.#renderGauge();
    this.#renderMap();
  }

  #renderGauge() {
    const f = gaugeFraction(this.#car.drivingLine);
    if (this.#els.gaugeDot) {
      this.#els.gaugeDot.style.left = `${((f + 1) / 2) * 100}%`;
    }
    if (this.#els.gaugeVal) {
      const mag = Math.abs(this.#car.drivingLine);
      if (mag < 4) {
        this.#els.gaugeVal.textContent = 'on line';
      } else {
        this.#els.gaugeVal.textContent = `${f < 0 ? 'left' : 'right'} ${mag}`;
      }
    }
  }

  #renderMap() {
    const canvas = this.#els.canvas;
    const ctx = canvas.getContext && canvas.getContext('2d');
    if (!ctx) return;
    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);

    const { center, left, right } = edges(this.#model);
    const haveMap = center.length >= 3;
    if (this.#els.empty) this.#els.empty.hidden = haveMap;
    if (!haveMap) {
      // Still show the live car as a lone dot if we have a position.
      return;
    }

    const allPts = center.concat(left, right);
    const t = fitTransform(allPts, W, H);
    if (!t) return;

    // Edge band: fill + outline per contiguous run of stations so we never
    // bridge a gap (missing section) with a chord across the map.
    if (left.length >= 2 && right.length >= 2) {
      const runs = contiguousRuns(left, right);
      ctx.fillStyle = 'rgba(120,160,220,0.12)';
      for (const run of runs) {
        if (run.left.length < 2) continue;
        ctx.beginPath();
        run.left.forEach((p, i) => {
          const s = t.project(p.x, p.z);
          i === 0 ? ctx.moveTo(s.px, s.py) : ctx.lineTo(s.px, s.py);
        });
        for (let i = run.right.length - 1; i >= 0; i--) {
          const s = t.project(run.right[i].x, run.right[i].z);
          ctx.lineTo(s.px, s.py);
        }
        ctx.closePath();
        ctx.fill();
      }

      ctx.strokeStyle = 'rgba(150,180,230,0.45)';
      ctx.lineWidth = 1;
      this.#strokePolyline(ctx, t, left);
      this.#strokePolyline(ctx, t, right);
    }

    // Centreline (dashed accent).
    ctx.strokeStyle = 'rgba(120,200,255,0.7)';
    ctx.lineWidth = 1.5;
    ctx.setLineDash([4, 3]);
    this.#strokePolyline(ctx, t, center);
    ctx.setLineDash([]);

    // Live car marker.
    if (Number.isFinite(this.#car.x) && Number.isFinite(this.#car.z)) {
      const s = t.project(this.#car.x, this.#car.z);
      ctx.beginPath();
      ctx.arc(s.px, s.py, 3.5, 0, Math.PI * 2);
      ctx.fillStyle = this.#car.raceOn ? '#f0844a' : 'rgba(240,132,74,0.5)';
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = 'rgba(255,255,255,0.85)';
      ctx.stroke();
    }
  }

  /**
   * Stroke a polyline, lifting the pen across station-index gaps wider than
   * MAX_GAP_STATIONS so sparse sampling never draws a chord across a corner.
   */
  #strokePolyline(ctx, t, pts) {
    if (pts.length < 2) return;
    ctx.beginPath();
    let penDown = false;
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      const s = t.project(p.x, p.z);
      const gap = i > 0 && p.station != null && pts[i - 1].station != null &&
        (p.station - pts[i - 1].station) > MAX_GAP_STATIONS;
      if (!penDown || gap) {
        ctx.moveTo(s.px, s.py);
        penDown = true;
      } else {
        ctx.lineTo(s.px, s.py);
      }
    }
    ctx.stroke();
  }
}

/**
 * Split aligned left/right edge arrays into runs of consecutive stations
 * (no gap wider than MAX_GAP_STATIONS), for gap-safe band filling.
 *
 * @param {Array<{station:number,x:number,z:number}>} left
 * @param {Array<{station:number,x:number,z:number}>} right
 * @returns {Array<{left:Array, right:Array}>}
 */
export function contiguousRuns(left, right) {
  const runs = [];
  let cur = null;
  for (let i = 0; i < left.length; i++) {
    const gap = i > 0 && (left[i].station - left[i - 1].station) > MAX_GAP_STATIONS;
    if (!cur || gap) {
      cur = { left: [], right: [] };
      runs.push(cur);
    }
    cur.left.push(left[i]);
    cur.right.push(right[i]);
  }
  return runs;
}
