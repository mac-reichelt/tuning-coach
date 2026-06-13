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
/** Minimum spacing between stored visible-trail points (metres). */
const TRAIL_DECIMATE_M = 2.0;
/** Cap on the envelope point ring buffer. */
const MAX_POINTS = 12000;
/** Cap on the visible trail ring buffer. */
const MAX_TRAIL = 8000;
/** A backward dist_m jump this large (metres) is a replay-loop / session restart. */
const DIST_RESET_M = 50;
/** Any backward dist_m step larger than this (metres) is treated as a rewind. */
const REWIND_EPS_M = 0.3;
/** Movement/dist below this (metres) between frames is treated as paused. */
const PAUSE_EPS_M = 0.05;
/** Don't connect centreline/edge nodes across a station-index gap wider than this. */
const MAX_GAP_STATIONS = 3;
/** Minimum spacing between distinct off-track / pit markers (metres). */
const MARK_MIN_SPACING_M = 8;
/** Half-length of the drawn start/finish line (metres). */
const SF_HALF_WIDTH_M = 9;

/** Off-track when NormalizedDrivingLine saturates at its i8 cap (±127). */
export const OFFTRACK_DRIVING_LINE_CAP = 127;

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
    /**
     * Pending (current-lap) envelope. Binned online here, then either committed
     * to the persistent envelope when a full lap completes or discarded if the
     * lap was a connect-in / out-lap (anchored === false). Kept as `stations` /
     * `points` so the live in-progress lap renders immediately.
     * @type {Map<number, {n:number,sumX:number,sumZ:number}>}
     */
    stations: new Map(),
    /** @type {Array<{x:number,z:number,station:number}>} decimated pending buffer */
    points: [],
    lastStoredX: null,
    lastStoredZ: null,
    /**
     * Persistent envelope, accumulated from completed full laps across every run
     * in the session. Survives run restarts so the inferred track sharpens over
     * the whole recording instead of only the current run.
     * @type {Map<number, {n:number,sumX:number,sumZ:number}>}
     */
    committedStations: new Map(),
    /** @type {Array<{x:number,z:number,station:number}>} */
    committedPoints: [],
    /** @type {Array<{x:number,z:number,dist:number,pit:boolean}>} ordered drawn path */
    trail: [],
    lastTrailX: null,
    lastTrailZ: null,
    /** @type {?{x:number,z:number,hx:number,hz:number}} start/finish crossing */
    startFinish: null,
    /** @type {?{x:number,z:number}} resume point awaiting a heading for S/F. */
    pendingSF: null,
    /** @type {Array<{x:number,z:number}>} */
    pitMarks: [],
    lastPitX: null,
    lastPitZ: null,
    /** @type {Array<{x:number,z:number}>} */
    offMarks: [],
    lastOffX: null,
    lastOffZ: null,
    offActive: false,
  };
}

/**
 * Compute the start/finish crossing geometry (position + unit heading) from the
 * trail's final segment, before the trail is reset for the new lap.
 * @returns {{x:number,z:number,hx:number,hz:number}}
 */
function computeStartFinish(model, x, z) {
  let hx = 1, hz = 0;
  const t = model.trail;
  if (t.length >= 1) {
    const p = t[t.length - 1];
    const dx = x - p.x, dz = z - p.z;
    const len = Math.hypot(dx, dz);
    if (len > 0.01) { hx = dx / len; hz = dz / len; }
  }
  return { x, z, hx, hz };
}

/** Drop trail points that are ahead of the current (rewound) distance. */
function rewindTrail(model, dist) {
  const t = model.trail;
  while (t.length && t[t.length - 1].dist > dist + 0.01) t.pop();
  const last = t[t.length - 1];
  model.lastTrailX = last ? last.x : null;
  model.lastTrailZ = last ? last.z : null;
}

/** Reset the pending (current-lap) envelope buffer. */
function clearPending(model) {
  model.stations.clear();
  model.points = [];
  model.lastStoredX = null;
  model.lastStoredZ = null;
}

/**
 * Commit the pending lap's envelope into the persistent envelope (a completed
 * full lap). Station sums merge so the per-station means tighten lap over lap.
 */
function commitPending(model) {
  for (const [station, st] of model.stations) {
    let c = model.committedStations.get(station);
    if (!c) {
      c = { n: 0, sumX: 0, sumZ: 0 };
      model.committedStations.set(station, c);
    }
    c.n += st.n;
    c.sumX += st.sumX;
    c.sumZ += st.sumZ;
  }
  for (const p of model.points) {
    model.committedPoints.push(p);
    if (model.committedPoints.length > MAX_POINTS) model.committedPoints.shift();
  }
}

/**
 * Handle a genuine start/finish crossing (a lap-number increment). Commit the
 * just-completed lap to the persistent envelope if it was a full lap (already
 * anchored), or discard it if it was the connect-in / out-lap. Then capture the
 * crossing geometry, anchor a fresh lap at station 0, and start a fresh visible
 * trail. The persistent envelope is never wiped, so it sharpens across laps and
 * runs.
 */
function onLapBoundary(model, x, z, dist, lap) {
  const sf = computeStartFinish(model, x, z);
  if (model.anchored) {
    commitPending(model);
  }
  clearPending(model);
  model.anchored = true;
  model.startFinish = sf;
  model.pendingSF = null;
  model.trail = [];
  model.lastTrailX = null;
  model.lastTrailZ = null;
  model.lapStartDist = dist;
  model.lapNumber = lap;
  model.lastDist = dist;
}

/**
 * Distance (metres) the car must teleport for a race resume to count as a new
 * session/run rather than an in-place pause/resume.
 */
const NEWRUN_TELEPORT_M = 30;

/**
 * Handle a session/run restart. Forza emits all-zero packets while out of the
 * race and re-enters behind the start/finish line, so the cumulative distance
 * jumps and the car teleports far from where the previous run ended. Discard the
 * interrupted pending lap and start a fresh out-lap, but KEEP the persistent
 * (committed) envelope so the inferred track accumulated from earlier runs is
 * preserved. `anchored` goes false so this run's connect-in lap is discarded at
 * its first S/F crossing, exactly as on session start.
 */
function startRun(model, x, z, dist, lap) {
  clearPending(model);
  model.anchored = false;
  model.trail = [];
  model.lastTrailX = null;
  model.lastTrailZ = null;
  model.lapStartDist = dist;
  model.lapNumber = lap;
  model.lastDist = dist;
  model.pendingSF = { x, z };
  model.offActive = false;
}

/**
 * Fold one telemetry sample into the model.
 *
 * Station 0 is anchored to the track's start/finish line so the same physical
 * spot maps to the same station on every lap (letting the edge envelope build
 * across laps). The start/finish line is detected as a lap boundary: either a
 * lap-number increment or a large backwards jump in distance (replay loop / lap
 * rollover). The very first boundary also discards the initial partial lap.
 *
 * In addition to the statistical envelope, an ordered `trail` records the
 * actual driven path for the current lap. The trail retracts on a rewind (a
 * small backwards dist step), holds on a pause (no movement), and tags points
 * driven during a pit stop so the renderer can colour them differently.
 *
 * @param {object} model   model from createModel()
 * @param {object} s       { x, z, dist, lap, track, offTrack?, pit? }
 * @returns {object} the same model (mutated)
 */
export function accumulate(model, s) {
  const { x, z, dist, lap, track } = s;
  const offTrack = s.offTrack === true;
  const pit = s.pit === true;
  const newRun = s.newRun === true;
  if (!Number.isFinite(x) || !Number.isFinite(z) || !Number.isFinite(dist)) {
    return model;
  }

  // Reset when the track changes.
  if (track != null && track !== model.trackOrdinal) {
    resetModel(model);
    model.trackOrdinal = track;
  }

  // Race resumed after leaving the session. If the car teleported (or distance
  // jumped) it's a genuine new run/session — discard the interrupted pending lap
  // and start a fresh out-lap while keeping the persistent envelope from earlier
  // runs. An in-place resume (a pause) teleports nowhere, so it falls through to
  // the normal pause/forward logic and the map is preserved.
  if (newRun && model.lastTrailX != null) {
    const teleported =
      Math.hypot(x - model.lastTrailX, z - model.lastTrailZ) > NEWRUN_TELEPORT_M ||
      (model.lastDist != null && Math.abs(dist - model.lastDist) > DIST_RESET_M);
    if (teleported) {
      startRun(model, x, z, dist, lap);
      // Fall through to record this sample as the new run's first point.
    }
  }

  {
    // Resolve a deferred start/finish heading from the first forward movement of
    // a freshly started run.
    if (model.pendingSF) {
      const dx = x - model.pendingSF.x;
      const dz = z - model.pendingSF.z;
      const len = Math.hypot(dx, dz);
      if (len > 0.5) {
        model.startFinish = { x: model.pendingSF.x, z: model.pendingSF.z, hx: dx / len, hz: dz / len };
        model.pendingSF = null;
      }
    }

    const backward = model.lastDist != null && dist < model.lastDist - REWIND_EPS_M;
    const lapIncreased =
      lap != null && model.lapNumber != null && lap > model.lapNumber;

    if (lapIncreased && !backward) {
      // Genuine start/finish crossing. DistanceTraveled is cumulative across laps,
      // so a real new lap moves forward; only rewinds/restarts jump backward.
      onLapBoundary(model, x, z, dist, lap);
      // Fall through to record the first sample of the new lap.
    } else if (backward) {
      // Rewind, replay-loop restart, or session restart. Retract the visible trail
      // to the rewound distance while keeping the cross-lap envelope and its
      // station origin (lapStartDist) stable — re-anchoring the origin mid-session
      // is what made the inferred edges "spider-web". A large jump back to the lap
      // start (loop / restart) additionally refreshes the start/finish crossing.
      if (dist < model.lastDist - DIST_RESET_M) {
        model.startFinish = computeStartFinish(model, x, z);
      }
      rewindTrail(model, dist);
      model.lastDist = dist;
      return model;
    } else if (
      model.lastDist != null &&
      Math.abs(dist - model.lastDist) <= PAUSE_EPS_M &&
      model.lastTrailX != null &&
      Math.hypot(x - model.lastTrailX, z - model.lastTrailZ) <= PAUSE_EPS_M
    ) {
      // Paused: hold the trace.
      model.offActive = offTrack ? model.offActive : false;
      return model;
    }
  }

  if (model.lapStartDist == null) model.lapStartDist = dist;
  model.lapNumber = lap ?? model.lapNumber;

  let lapDist = dist - model.lapStartDist;
  if (lapDist < 0) lapDist = 0;
  const station = Math.floor(lapDist / STATION_M);

  // Online centreline mean per station (envelope).
  let st = model.stations.get(station);
  if (!st) {
    st = { n: 0, sumX: 0, sumZ: 0 };
    model.stations.set(station, st);
  }
  st.n += 1;
  st.sumX += x;
  st.sumZ += z;

  // Decimated point buffer for edge spread (envelope).
  if (
    model.lastStoredX == null ||
    Math.hypot(x - model.lastStoredX, z - model.lastStoredZ) >= DECIMATE_M
  ) {
    model.points.push({ x, z, station });
    model.lastStoredX = x;
    model.lastStoredZ = z;
    if (model.points.length > MAX_POINTS) model.points.shift();
  }

  // Visible ordered trail (the bright drawn path). Keep it strictly forward in
  // distance so a sub-threshold backward drift can never fold the line back on
  // itself.
  const lastTrail = model.trail[model.trail.length - 1];
  if (
    (lastTrail == null || dist >= lastTrail.dist) &&
    (model.lastTrailX == null ||
      Math.hypot(x - model.lastTrailX, z - model.lastTrailZ) >= TRAIL_DECIMATE_M)
  ) {
    model.trail.push({ x, z, dist, pit });
    model.lastTrailX = x;
    model.lastTrailZ = z;
    if (model.trail.length > MAX_TRAIL) model.trail.shift();
  }

  // Off-track excursion marker (rising edge, spaced).
  if (offTrack) {
    if (!model.offActive) {
      if (
        model.lastOffX == null ||
        Math.hypot(x - model.lastOffX, z - model.lastOffZ) >= MARK_MIN_SPACING_M
      ) {
        model.offMarks.push({ x, z });
        model.lastOffX = x;
        model.lastOffZ = z;
      }
      model.offActive = true;
    }
  } else {
    model.offActive = false;
  }

  model.lastDist = dist;
  return model;
}

/** Record a pit-stop start marker at the given position (spaced from the last). */
export function markPitStart(model, x, z) {
  if (!Number.isFinite(x) || !Number.isFinite(z)) return;
  if (
    model.lastPitX == null ||
    Math.hypot(x - model.lastPitX, z - model.lastPitZ) >= MARK_MIN_SPACING_M
  ) {
    model.pitMarks.push({ x, z });
    model.lastPitX = x;
    model.lastPitZ = z;
  }
}

/**
 * Decide whether the car is off track from raw telemetry. NormalizedDrivingLine
 * is an i8 [-127..127] lateral offset from the AI line; it saturates at ±127
 * when the car runs wide of the track, which we treat as an off-track marker.
 *
 * @param {object} raw   telemetry `data.raw`
 * @returns {boolean}
 */
export function offTrackFromRaw(raw) {
  if (!raw) return false;
  const dl = Number(raw.driving_line);
  return Number.isFinite(dl) && Math.abs(dl) >= OFFTRACK_DRIVING_LINE_CAP;
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
  model.committedStations.clear();
  model.committedPoints = [];
  model.trail = [];
  model.lastTrailX = null;
  model.lastTrailZ = null;
  model.startFinish = null;
  model.pendingSF = null;
  model.pitMarks = [];
  model.lastPitX = null;
  model.lastPitZ = null;
  model.offMarks = [];
  model.lastOffX = null;
  model.lastOffZ = null;
  model.offActive = false;
}

/**
 * Merge a station-sum entry into an accumulator map.
 */
function mergeStation(acc, station, st) {
  let a = acc.get(station);
  if (!a) {
    a = { n: 0, sumX: 0, sumZ: 0 };
    acc.set(station, a);
  }
  a.n += st.n;
  a.sumX += st.sumX;
  a.sumZ += st.sumZ;
}

/**
 * Build the ordered centreline (station means) as an array sorted by station.
 * Combines the persistent (committed) envelope with the pending current lap so
 * the live lap renders before its first crossing commits it.
 * @returns {Array<{station:number,x:number,z:number}>}
 */
export function centerline(model) {
  const acc = new Map();
  for (const [station, st] of model.committedStations) mergeStation(acc, station, st);
  for (const [station, st] of model.stations) mergeStation(acc, station, st);
  const out = [];
  for (const [station, st] of acc) {
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

  // Per-station lateral min/max from buffered points (committed + pending).
  const lat = center.map(() => ({ min: 0, max: 0, has: false }));
  const allPoints = model.committedPoints.concat(model.points);
  for (const p of allPoints) {
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
  #pitActive = false;
  #offVisible = true;
  #prevRaceOn = false;

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
      const newRun = !this.#prevRaceOn;
      accumulate(this.#model, {
        x: raw.pos_x,
        z: raw.pos_z,
        dist: raw.dist_m,
        lap: raw.lap_number,
        track: raw.track_ordinal,
        offTrack: offTrackFromRaw(raw),
        pit: this.#pitActive,
        newRun,
      });
    }
    this.#prevRaceOn = this.#car.raceOn;

    if (this.#visible) this.#scheduleRender();
  }

  /** Mark the start of a pit stop (from the `pit_stop_started` WS event). */
  onPitStart() {
    this.#pitActive = true;
    markPitStart(this.#model, this.#car.x, this.#car.z);
    if (this.#visible) this.#scheduleRender();
  }

  /** Mark the end of a pit stop (from the `pit_stop_ended` WS event). */
  onPitEnd() {
    this.#pitActive = false;
  }

  /** Throw away the accumulated map (e.g. user pressed Clear). */
  clear() {
    resetModel(this.#model);
    this.#model.trackOrdinal = null;
    this.#pitActive = false;
    this.#prevRaceOn = false;
    if (this.#visible) this.#scheduleRender();
  }

  // ── Private ────────────────────────────────────────────────

  #build() {
    this.#root.innerHTML = `
      <div class="track-header">
        <span class="track-title" data-track="title">Track</span>
        <div class="track-header-btns">
          <button class="track-btn track-btn-active" data-track="off-toggle" title="Show/hide off-track markers">Off ✓</button>
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
      offToggle: q('off-toggle'),
    };

    this.#els.clear.addEventListener('click', () => this.clear());
    this.#els.offToggle.addEventListener('click', () => this.#toggleOffMarkers());

    makeDraggable(this.#root, this.#root.querySelector('.track-header'));
  }

  #toggleOffMarkers() {
    this.#offVisible = !this.#offVisible;
    const btn = this.#els.offToggle;
    if (btn) {
      btn.textContent = this.#offVisible ? 'Off ✓' : 'Off ✗';
      btn.classList.toggle('track-btn-active', this.#offVisible);
    }
    if (this.#visible) this.#scheduleRender();
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

    const model = this.#model;
    const { center, left, right } = edges(model);
    const trail = model.trail;
    const haveMap = trail.length >= 2 || center.length >= 3;
    if (this.#els.empty) this.#els.empty.hidden = haveMap;
    if (!haveMap) return;

    // Fit over everything we draw so nothing clips.
    const allPts = center.concat(left, right, trail, model.pitMarks, model.offMarks);
    if (model.startFinish) allPts.push(model.startFinish);
    const t = fitTransform(allPts, W, H);
    if (!t) return;

    // Faint inferred edge band (envelope across laps), gap-safe per run.
    if (left.length >= 2 && right.length >= 2) {
      const runs = contiguousRuns(left, right);
      ctx.fillStyle = 'rgba(120,160,220,0.10)';
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
      ctx.strokeStyle = 'rgba(150,180,230,0.30)';
      ctx.lineWidth = 1;
      this.#strokePolyline(ctx, t, left);
      this.#strokePolyline(ctx, t, right);
    }

    // Driven trail — the bright path, coloured by pit state, contiguous in time.
    this.#strokeTrail(ctx, t, trail);

    // Start/finish line (perpendicular to the crossing heading).
    if (model.startFinish) this.#strokeStartFinish(ctx, t, model.startFinish);

    // Pit-stop start markers.
    for (const m of model.pitMarks) {
      const s = t.project(m.x, m.z);
      ctx.fillStyle = 'rgba(224,179,65,0.95)';
      ctx.fillRect(s.px - 3.5, s.py - 3.5, 7, 7);
      ctx.lineWidth = 1;
      ctx.strokeStyle = 'rgba(0,0,0,0.55)';
      ctx.strokeRect(s.px - 3.5, s.py - 3.5, 7, 7);
    }

    // Off-track markers (toggleable).
    if (this.#offVisible) {
      ctx.strokeStyle = 'rgba(255,70,70,0.95)';
      ctx.lineWidth = 2;
      for (const m of model.offMarks) {
        const s = t.project(m.x, m.z);
        const r = 3;
        ctx.beginPath();
        ctx.moveTo(s.px - r, s.py - r);
        ctx.lineTo(s.px + r, s.py + r);
        ctx.moveTo(s.px + r, s.py - r);
        ctx.lineTo(s.px - r, s.py + r);
        ctx.stroke();
      }
    }

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
   * Stroke the ordered trail, switching colour between normal driving and pit
   * segments. Points are contiguous in time, so no gap-breaking is needed.
   */
  #strokeTrail(ctx, t, trail) {
    if (trail.length < 2) return;
    ctx.lineWidth = 2;
    let i = 0;
    while (i < trail.length - 1) {
      const segPit = trail[i + 1].pit === true;
      ctx.strokeStyle = segPit ? 'rgba(224,179,65,0.95)' : 'rgba(70,200,255,0.95)';
      ctx.beginPath();
      const s0 = t.project(trail[i].x, trail[i].z);
      ctx.moveTo(s0.px, s0.py);
      let j = i + 1;
      while (j < trail.length && (trail[j].pit === true) === segPit) {
        const s = t.project(trail[j].x, trail[j].z);
        ctx.lineTo(s.px, s.py);
        j++;
      }
      ctx.stroke();
      i = j - 1;
    }
  }

  /** Draw the start/finish line as a short bar across the track at the crossing. */
  #strokeStartFinish(ctx, t, sf) {
    const px = -sf.hz, pz = sf.hx; // perpendicular to heading
    const a = t.project(sf.x + px * SF_HALF_WIDTH_M, sf.z + pz * SF_HALF_WIDTH_M);
    const b = t.project(sf.x - px * SF_HALF_WIDTH_M, sf.z - pz * SF_HALF_WIDTH_M);
    ctx.strokeStyle = 'rgba(255,255,255,0.9)';
    ctx.lineWidth = 2;
    ctx.setLineDash([3, 2]);
    ctx.beginPath();
    ctx.moveTo(a.px, a.py);
    ctx.lineTo(b.px, b.py);
    ctx.stroke();
    ctx.setLineDash([]);
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
