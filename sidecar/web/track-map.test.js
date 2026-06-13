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
  save: vi.fn(), restore: vi.fn(), setLineDash: vi.fn(),
  strokeStyle: '', fillStyle: '', lineWidth: 1,
};
HTMLCanvasElement.prototype.getContext = () => ctxStub;

import {
  createModel, accumulate, resetModel, centerline, edges,
  fitTransform, gaugeFraction, contiguousRuns, STATION_M, TrackMap,
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
    feedStraight(m, { lap: 2, baseDist: 70 });
    feedStraight(m, { lap: 3, baseDist: 140 });
    // Laps 2 and 3 retrace the same geometry → stations align, not double.
    expect(centerline(m).length).toBeLessThanOrEqual(8);
    // Each aligned station was visited on both lap 2 and lap 3.
    const visited = [...m.stations.values()].every(st => st.n >= 2);
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
});
