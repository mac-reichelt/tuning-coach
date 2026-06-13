/**
 * Tests for track-map.js.
 *
 * Coverage:
 *  - pure model: accumulate stations, lap-relative reset, track-change reset,
 *    point decimation
 *  - centerline ordering + means
 *  - edge extraction yields left/right offsets from driven spread
 *  - fitTransform keeps projected points within the canvas
 *  - gaugeFraction clamping + sign
 *  - TrackMap show/hide/toggle/clear + racing-line gauge DOM updates
 *
 * jsdom does not implement HTMLCanvasElement.getContext, so a ctx stub is
 * installed, and requestAnimationFrame is made synchronous so #render runs
 * inline during the test.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// makeDraggable touches pointer-capture APIs jsdom stubs incompletely.
vi.mock('./src/drag.js', () => ({ makeDraggable: vi.fn() }));

const ctxStub = {
  clearRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(),
  closePath: vi.fn(), stroke: vi.fn(), fill: vi.fn(), arc: vi.fn(),
  fillRect: vi.fn(), strokeRect: vi.fn(),
  save: vi.fn(), restore: vi.fn(), setLineDash: vi.fn(),
  strokeStyle: '', fillStyle: '', lineWidth: 1,
};
HTMLCanvasElement.prototype.getContext = () => ctxStub;

import {
  createModel, accumulate, resetModel, centerline, edges,
  fitTransform, gaugeFraction, contiguousRuns, offTrackFromRaw, markPitStart,
  OFFTRACK_DRIVING_LINE_CAP, STATION_M, TrackMap,
} from './src/track-map.js';

/** Feed a straight +X track with a lateral spread in Z into a model. */
function feedStraight(model, { lap = 1, track = 7, baseDist = 0 } = {}) {
  for (let x = 0; x <= 60; x++) {
    const z = x % 2 === 0 ? -3 : 3; // ±3 m spread per station
    accumulate(model, { x, z, dist: baseDist + x, lap, track });
  }
}

describe('track-map model', () => {
  it('bins samples into stations by lap-relative distance', () => {
    const m = createModel();
    feedStraight(m);
    // x 0..60 over 10 m stations → stations 0..6 = 7 stations.
    expect(centerline(m).length).toBe(7);
    expect(STATION_M).toBe(10);
  });

  it('aligns stations across laps after the first start/finish crossing', () => {
    const m = createModel();
    // Lap 1 is the connect-in-progress lap: discarded at the first boundary.
    feedStraight(m, { lap: 1, baseDist: 0 });
    // Laps 2 and 3 are full laps that commit to the persistent envelope; lap 4
    // drives a boundary so lap 3 also commits.
    feedStraight(m, { lap: 2, baseDist: 70 });
    feedStraight(m, { lap: 3, baseDist: 140 });
    feedStraight(m, { lap: 4, baseDist: 210 });
    // Laps 2 and 3 retrace the same geometry → stations align, not double.
    expect(centerline(m).length).toBeLessThanOrEqual(8);
    // Each aligned station was visited on both committed laps (2 and 3).
    const visited = [...m.committedStations.values()].every(st => st.n >= 2);
    expect(visited).toBe(true);
  });

  it('resets accumulation when the track ordinal changes', () => {
    const m = createModel();
    feedStraight(m, { track: 1 });
    expect(centerline(m).length).toBeGreaterThan(0);
    accumulate(m, { x: 0, z: 0, dist: 0, lap: 1, track: 2 });
    // Old geometry cleared; only the single new sample remains.
    expect(centerline(m).length).toBe(1);
  });

  it('ignores non-finite samples', () => {
    const m = createModel();
    accumulate(m, { x: NaN, z: 0, dist: 0, lap: 1, track: 1 });
    accumulate(m, { x: 0, z: Infinity, dist: 0, lap: 1, track: 1 });
    expect(centerline(m).length).toBe(0);
  });

  it('decimates the point buffer to >= ~1 m spacing', () => {
    const m = createModel();
    // 10 samples all within 0.1 m → only the first is stored.
    for (let i = 0; i < 10; i++) {
      accumulate(m, { x: 0.01 * i, z: 0, dist: i, lap: 1, track: 1 });
    }
    expect(m.points.length).toBe(1);
  });

  it('resetModel clears geometry', () => {
    const m = createModel();
    feedStraight(m);
    resetModel(m);
    expect(centerline(m).length).toBe(0);
    expect(m.points.length).toBe(0);
  });
});

describe('centerline + edges', () => {
  it('orders centerline by station', () => {
    const m = createModel();
    feedStraight(m);
    const c = centerline(m);
    for (let i = 1; i < c.length; i++) {
      expect(c[i].station).toBeGreaterThan(c[i - 1].station);
    }
  });

  it('extracts left/right edges straddling the centerline', () => {
    const m = createModel();
    feedStraight(m);
    const { left, right } = edges(m);
    expect(left.length).toBeGreaterThanOrEqual(2);
    expect(right.length).toBeGreaterThanOrEqual(2);
    // Tangent is +X so the normal is +Z: left edge sits at higher Z than right.
    for (let i = 0; i < Math.min(left.length, right.length); i++) {
      expect(left[i].z).toBeGreaterThan(right[i].z);
    }
    // Edges sit outside the ±3 m driven spread (plus 1 m margin).
    expect(Math.max(...left.map(p => p.z))).toBeGreaterThan(3);
    expect(Math.min(...right.map(p => p.z))).toBeLessThan(-3);
  });

  it('returns empty edges when too few stations', () => {
    const m = createModel();
    accumulate(m, { x: 0, z: 0, dist: 0, lap: 1, track: 1 });
    const { left, right } = edges(m);
    expect(left).toEqual([]);
    expect(right).toEqual([]);
  });
});

describe('contiguousRuns', () => {
  it('keeps consecutive stations in one run', () => {
    const left = [0, 1, 2, 3].map(s => ({ station: s, x: s, z: 0 }));
    const right = left.map(p => ({ ...p, z: -1 }));
    const runs = contiguousRuns(left, right);
    expect(runs.length).toBe(1);
    expect(runs[0].left.length).toBe(4);
  });

  it('splits across a wide station gap', () => {
    // gap of 5 between station 2 and 7 (> MAX_GAP_STATIONS) → two runs.
    const left = [0, 1, 2, 7, 8].map(s => ({ station: s, x: s, z: 0 }));
    const right = left.map(p => ({ ...p, z: -1 }));
    const runs = contiguousRuns(left, right);
    expect(runs.length).toBe(2);
    expect(runs[0].left.map(p => p.station)).toEqual([0, 1, 2]);
    expect(runs[1].left.map(p => p.station)).toEqual([7, 8]);
  });
});

describe('fitTransform', () => {
  it('keeps projected points within the canvas bounds', () => {
    const pts = [
      { x: -100, z: -50 }, { x: 100, z: 50 }, { x: 0, z: 0 },
    ];
    const t = fitTransform(pts, 200, 120, 10);
    expect(t).not.toBeNull();
    for (const p of pts) {
      const s = t.project(p.x, p.z);
      expect(s.px).toBeGreaterThanOrEqual(0);
      expect(s.px).toBeLessThanOrEqual(200);
      expect(s.py).toBeGreaterThanOrEqual(0);
      expect(s.py).toBeLessThanOrEqual(120);
    }
  });

  it('flips Z so larger Z maps to a smaller screen Y (top-down)', () => {
    const pts = [{ x: 0, z: 0 }, { x: 0, z: 10 }];
    const t = fitTransform(pts, 100, 100, 10);
    const low = t.project(0, 0);
    const high = t.project(0, 10);
    expect(high.py).toBeLessThan(low.py);
  });

  it('returns null for an empty point list', () => {
    expect(fitTransform([], 100, 100)).toBeNull();
  });
});

describe('gaugeFraction', () => {
  it('maps 0 to centre', () => { expect(gaugeFraction(0)).toBe(0); });
  it('clamps to [-1, 1]', () => {
    expect(gaugeFraction(200)).toBe(1);
    expect(gaugeFraction(-200)).toBe(-1);
  });
  it('is signed by driving-line direction', () => {
    expect(gaugeFraction(63)).toBeGreaterThan(0);
    expect(gaugeFraction(-63)).toBeLessThan(0);
  });
  it('treats non-finite as centre', () => {
    expect(gaugeFraction(NaN)).toBe(0);
  });
});

describe('trail, rewind, pause, pit', () => {
  /** Drive forward, appending one trail point roughly every metre. */
  function drive(model, { from = 0, to = 30, lap = 1, track = 1, pit = false } = {}) {
    for (let d = from; d <= to; d += 3) {
      accumulate(model, { x: d, z: 0, dist: d, lap, track, pit });
    }
  }

  it('builds an ordered trail as the car moves forward', () => {
    const m = createModel();
    drive(m, { to: 30 });
    expect(m.trail.length).toBeGreaterThan(2);
    // Trail is ordered by distance.
    for (let i = 1; i < m.trail.length; i++) {
      expect(m.trail[i].dist).toBeGreaterThanOrEqual(m.trail[i - 1].dist);
    }
  });

  it('retracts the trail on a rewind (backwards distance)', () => {
    const m = createModel();
    drive(m, { to: 30 });
    const peak = m.trail.length;
    // Rewind from 30 back to 12.
    for (let d = 30; d >= 12; d -= 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    expect(m.trail.length).toBeLessThan(peak);
    expect(m.trail[m.trail.length - 1].dist).toBeLessThanOrEqual(15);
  });

  it('keeps the envelope origin stable across a large rewind (no spider-web)', () => {
    const m = createModel();
    // Drive a straight lap; the inferred centreline should be monotonic in x.
    for (let d = 0; d <= 120; d += 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    const startDist = m.lapStartDist;
    const stationCount = centerline(m).length;
    const peak = m.trail.length;
    // A >50 m backwards jump (Forza rewind / replay loop): retract, don't re-anchor.
    accumulate(m, { x: 60, z: 0, dist: 60, lap: 1, track: 1 });
    expect(m.trail.length).toBeLessThan(peak); // trail retracted
    expect(m.lapStartDist).toBe(startDist); // station origin unchanged
    expect(m.lapNumber).toBe(1); // not mistaken for a new lap
    // Re-drive the same line; stations must not gain a wrong-origin twin.
    for (let d = 63; d <= 120; d += 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    const cl = centerline(m);
    expect(cl.length).toBe(stationCount); // no spurious mixed-origin stations
    for (let i = 1; i < cl.length; i++) {
      // A corrupted origin would pull a station's mean far off, breaking order.
      expect(cl[i].x).toBeGreaterThan(cl[i - 1].x);
    }
  });

  it('refreshes the start/finish line on a large backwards jump (loop restart)', () => {
    const m = createModel();
    for (let d = 0; d <= 120; d += 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    expect(m.startFinish).toBeNull();
    // Replay loops back to the capture start: >50 m backwards jump.
    accumulate(m, { x: 0, z: 0, dist: 0, lap: 1, track: 1 });
    expect(m.startFinish).not.toBeNull();
    expect(Number.isFinite(m.startFinish.hx)).toBe(true);
  });

  it('starts a fresh run on a race-resume teleport, discarding prior geometry', () => {
    const m = createModel();
    // Run 1: drive a straight stretch far from the origin.
    for (let d = 0; d <= 90; d += 3) {
      accumulate(m, { x: 500 + d, z: 200, dist: 1000 + d, lap: 1, track: 1 });
    }
    expect(m.trail.length).toBeGreaterThan(2);
    // Race resumes after a session break: teleport to a distant start, big dist
    // jump, newRun flagged. Prior run's geometry must be wiped (no smear).
    accumulate(m, { x: -300, z: -250, dist: 5000, lap: 1, track: 1, newRun: true });
    expect(m.trail.length).toBe(1); // only the new run's first point
    expect(m.lapStartDist).toBe(5000); // origin re-anchored to this run
    // The new run's bright path stays near its own start, not bridged to run 1.
    accumulate(m, { x: -297, z: -250, dist: 5003, lap: 1, track: 1 });
    for (const p of m.trail) {
      expect(p.x).toBeLessThan(0);
    }
  });

  it('preserves the committed envelope across a race-resume teleport', () => {
    const m = createModel();
    // Run 1: drive two full laps so lap 2 commits to the persistent envelope.
    feedStraight(m, { lap: 1, baseDist: 0 });
    feedStraight(m, { lap: 2, baseDist: 70 });
    feedStraight(m, { lap: 3, baseDist: 140 });
    const committed = m.committedStations.size;
    expect(committed).toBeGreaterThan(0);
    // Race resumes after a session break: teleport to a distant start.
    accumulate(m, { x: -300, z: -250, dist: 9000, lap: 1, track: 7, newRun: true });
    // Pending lap is discarded, but the persistent envelope from run 1 survives.
    expect(m.committedStations.size).toBe(committed);
    expect(centerline(m).length).toBeGreaterThan(0);
  });

  it('treats an in-place race resume (pause) as continuous, preserving the map', () => {
    const m = createModel();
    for (let d = 0; d <= 90; d += 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    const peak = m.trail.length;
    const stations = centerline(m).length;
    // Resume at essentially the same spot/distance: not a teleport → no wipe.
    accumulate(m, { x: 90.02, z: 0, dist: 90.02, lap: 1, track: 1, newRun: true });
    expect(m.trail.length).toBeGreaterThanOrEqual(peak);
    expect(centerline(m).length).toBe(stations);
  });

  it('defers the start/finish heading to the first forward step after a new run', () => {
    const m = createModel();
    for (let d = 0; d <= 60; d += 3) {
      accumulate(m, { x: d, z: 0, dist: d, lap: 1, track: 1 });
    }
    // New run starting behind the line; heading unknown on the first frame.
    accumulate(m, { x: 0, z: 0, dist: 900, lap: 1, track: 1, newRun: true });
    expect(m.pendingSF).not.toBeNull();
    // First forward movement resolves the S/F heading.
    accumulate(m, { x: 0, z: 5, dist: 905, lap: 1, track: 1 });
    expect(m.pendingSF).toBeNull();
    expect(m.startFinish).not.toBeNull();
    expect(Math.abs(Math.hypot(m.startFinish.hx, m.startFinish.hz) - 1)).toBeLessThan(1e-6);
  });

  it('holds the trail while paused (no movement)', () => {
    const m = createModel();
    drive(m, { to: 30 });
    const len = m.trail.length;
    // Same position/distance repeated → paused, no new points.
    for (let i = 0; i < 5; i++) {
      accumulate(m, { x: 30, z: 0, dist: 30, lap: 1, track: 1 });
    }
    expect(m.trail.length).toBe(len);
  });

  it('tags trail points driven during a pit stop', () => {
    const m = createModel();
    drive(m, { to: 15, pit: false });
    drive(m, { from: 18, to: 30, pit: true });
    expect(m.trail.some(p => p.pit === true)).toBe(true);
    expect(m.trail.some(p => p.pit === false)).toBe(true);
  });

  it('captures a start/finish crossing on the first lap boundary', () => {
    const m = createModel();
    drive(m, { from: 0, to: 30, lap: 1 });
    // Lap increments → start/finish crossing recorded, fresh trail.
    accumulate(m, { x: 33, z: 0, dist: 33, lap: 2, track: 1 });
    expect(m.startFinish).not.toBeNull();
    expect(Number.isFinite(m.startFinish.x)).toBe(true);
    expect(Number.isFinite(m.startFinish.hx)).toBe(true);
  });

  it('records off-track markers on the rising edge, spaced apart', () => {
    const m = createModel();
    accumulate(m, { x: 0, z: 0, dist: 0, lap: 1, track: 1, offTrack: false });
    accumulate(m, { x: 2, z: 0, dist: 2, lap: 1, track: 1, offTrack: true });
    // Still off-track but no rising edge → no new marker.
    accumulate(m, { x: 4, z: 0, dist: 4, lap: 1, track: 1, offTrack: true });
    expect(m.offMarks.length).toBe(1);
    // Back on track, then off again far away → second marker.
    accumulate(m, { x: 20, z: 0, dist: 20, lap: 1, track: 1, offTrack: false });
    accumulate(m, { x: 40, z: 0, dist: 40, lap: 1, track: 1, offTrack: true });
    expect(m.offMarks.length).toBe(2);
  });

  it('markPitStart records spaced pit markers', () => {
    const m = createModel();
    markPitStart(m, 10, 10);
    markPitStart(m, 11, 10); // too close → ignored
    markPitStart(m, 40, 40);
    expect(m.pitMarks.length).toBe(2);
  });

  it('resetModel clears trail, markers and start/finish', () => {
    const m = createModel();
    drive(m, { to: 30 });
    markPitStart(m, 5, 5);
    accumulate(m, { x: 33, z: 0, dist: 33, lap: 2, track: 1, offTrack: true });
    resetModel(m);
    expect(m.trail).toEqual([]);
    expect(m.pitMarks).toEqual([]);
    expect(m.offMarks).toEqual([]);
    expect(m.startFinish).toBeNull();
  });
});

describe('offTrackFromRaw', () => {
  it('is true only when driving_line saturates at the ±127 cap', () => {
    expect(offTrackFromRaw({ driving_line: 0 })).toBe(false);
    expect(offTrackFromRaw({ driving_line: 100 })).toBe(false);
    expect(offTrackFromRaw({ driving_line: OFFTRACK_DRIVING_LINE_CAP })).toBe(true);
    expect(offTrackFromRaw({ driving_line: -OFFTRACK_DRIVING_LINE_CAP })).toBe(true);
  });

  it('handles missing/invalid input', () => {
    expect(offTrackFromRaw(null)).toBe(false);
    expect(offTrackFromRaw({})).toBe(false);
  });
});

describe('TrackMap widget', () => {
  let root;
  let tm;
  let rafSpy;

  beforeEach(() => {
    // Run scheduled renders synchronously.
    rafSpy = vi.spyOn(global, 'requestAnimationFrame').mockImplementation((cb) => {
      cb();
      return 0;
    });
    root = document.createElement('div');
    document.body.appendChild(root);
    tm = new TrackMap(root);
    vi.clearAllMocks();
  });

  afterEach(() => {
    root.remove();
    rafSpy.mockRestore();
  });

  it('show()/hide()/toggle() track visibility', () => {
    expect(tm.isVisible()).toBe(false);
    tm.show();
    expect(tm.isVisible()).toBe(true);
    expect(root.hidden).toBe(false);
    tm.hide();
    expect(tm.isVisible()).toBe(false);
    expect(root.hidden).toBe(true);
    tm.toggle();
    expect(tm.isVisible()).toBe(true);
  });

  it('moves the racing-line gauge dot and label from driving_line', () => {
    tm.show();
    tm.onTelemetry({ is_race_on: false, raw: { driving_line: 63, pos_x: 0, pos_z: 0, dist_m: 0, lap_number: 1, track_ordinal: 1 } });
    const dot = root.querySelector('[data-track="gauge-dot"]');
    const val = root.querySelector('[data-track="gauge-val"]');
    // 63/127 ≈ 0.496 → (0.496+1)/2 ≈ 74.8%
    expect(parseFloat(dot.style.left)).toBeGreaterThan(70);
    expect(parseFloat(dot.style.left)).toBeLessThan(80);
    expect(val.textContent).toBe('right 63');
  });

  it('labels a centred line as "on line"', () => {
    tm.show();
    tm.onTelemetry({ is_race_on: false, raw: { driving_line: 0, pos_x: 0, pos_z: 0, dist_m: 0, lap_number: 1, track_ordinal: 1 } });
    expect(root.querySelector('[data-track="gauge-val"]').textContent).toBe('on line');
  });

  it('accumulates only while racing', () => {
    tm.show();
    // Not racing → no accumulation.
    tm.onTelemetry({ is_race_on: false, raw: { driving_line: 0, pos_x: 1, pos_z: 1, dist_m: 1, lap_number: 1, track_ordinal: 1 } });
    // Racing → accumulates.
    for (let x = 0; x <= 30; x++) {
      tm.onTelemetry({ is_race_on: true, raw: { driving_line: 0, pos_x: x, pos_z: x % 2 ? 3 : -3, dist_m: x, lap_number: 1, track_ordinal: 1 } });
    }
    tm.clear(); // should not throw, resets map
    expect(root.querySelector('[data-track="empty"]').hidden).toBe(false);
  });

  it('ignores telemetry with no raw block', () => {
    tm.show();
    expect(() => tm.onTelemetry({ is_race_on: true })).not.toThrow();
  });

  it('drops a pit marker and colours the trail on pit start', () => {
    tm.show();
    for (let x = 0; x <= 30; x++) {
      tm.onTelemetry({ is_race_on: true, raw: { driving_line: 0, pos_x: x, pos_z: 0, dist_m: x, lap_number: 1, track_ordinal: 1 } });
    }
    tm.onPitStart();
    // A subsequent sample is flagged as a pit segment.
    tm.onTelemetry({ is_race_on: true, raw: { driving_line: 0, pos_x: 33, pos_z: 0, dist_m: 33, lap_number: 1, track_ordinal: 1 } });
    tm.onPitEnd();
    expect(() => tm.onTelemetry({ is_race_on: true, raw: { driving_line: 0, pos_x: 36, pos_z: 0, dist_m: 36, lap_number: 1, track_ordinal: 1 } })).not.toThrow();
  });

  it('toggles the off-track marker button label', () => {
    const btn = root.querySelector('[data-track="off-toggle"]');
    expect(btn.textContent).toContain('✓');
    btn.click();
    expect(btn.textContent).toContain('✗');
    btn.click();
    expect(btn.textContent).toContain('✓');
  });
});
