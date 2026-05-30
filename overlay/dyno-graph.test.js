/**
 * Tests for DynoGraph.
 *
 * Coverage:
 *  - show()/hide()/toggle() update root.hidden and internal visibility state
 *  - onDynoUpdate with phase='waiting_for_ready' shows stop-wrap, hides retry
 *  - onDynoUpdate with phase='complete' shows retry, hides stop-wrap
 *  - phase title text reflects the active phase
 *  - onDynoUpdate with bins populates the stats section (Peak Power / Peak Torque)
 *  - unit toggle button cycles between 'SI' and 'Imperial' labels
 *
 * jsdom does not implement HTMLCanvasElement.getContext, so a ctx stub is
 * installed on the prototype before any test calls #drawGraph.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// makeDraggable uses pointer-capture APIs that jsdom stubs incompletely;
// mock the module so the DynoGraph constructor never touches them.
vi.mock('./src/drag.js', () => ({ makeDraggable: vi.fn() }));

// Minimal 2D-context stub — jsdom's canvas returns null from getContext by default.
// Properties like strokeStyle/fillStyle are writable on a plain object.
const ctxStub = {
  clearRect:    vi.fn(),
  beginPath:    vi.fn(),
  moveTo:       vi.fn(),
  lineTo:       vi.fn(),
  stroke:       vi.fn(),
  fill:         vi.fn(),
  fillText:     vi.fn(),
  save:         vi.fn(),
  restore:      vi.fn(),
  setLineDash:  vi.fn(),
  strokeStyle:  '',
  fillStyle:    '',
  lineWidth:    1,
  font:         '',
  textAlign:    '',
};

// Assign before any test runs; DynoGraph only calls getContext inside #drawGraph,
// which is triggered by onDynoUpdate — well after module initialisation.
HTMLCanvasElement.prototype.getContext = () => ctxStub;

import { DynoGraph } from './src/dyno-graph.js';

/** Three-bin sample dataset covering a meaningful RPM range. */
const SAMPLE_BINS = [
  { rpm: 1_000, power_w: 10_000, torque_nm: 100 },
  { rpm: 5_000, power_w: 50_000, torque_nm: 200 },
  { rpm: 8_000, power_w: 40_000, torque_nm: 150 },
];

describe('DynoGraph', () => {
  let root;
  let dg;

  beforeEach(() => {
    root = document.createElement('div');
    document.body.appendChild(root);
    dg = new DynoGraph(root);
    vi.clearAllMocks();
  });

  afterEach(() => {
    root.remove();
  });

  // ── Visibility ────────────────────────────────────────────────

  it('show() makes root visible (root.hidden = false)', () => {
    dg.show();
    expect(root.hidden).toBe(false);
  });

  it('hide() sets root.hidden to true', () => {
    dg.hide();
    expect(root.hidden).toBe(true);
  });

  it('toggle() hides root when currently visible', () => {
    dg.show();
    dg.toggle();
    expect(root.hidden).toBe(true);
  });

  it('toggle() shows root when currently hidden', () => {
    dg.hide();
    dg.toggle();
    expect(root.hidden).toBe(false);
  });

  // ── Phase: waiting_for_ready ──────────────────────────────────

  it('waiting_for_ready shows stop-wrap element', () => {
    dg.onDynoUpdate({ phase: 'waiting_for_ready' });
    expect(root.querySelector('[data-dyno="stop-wrap"]').hidden).toBe(false);
  });

  it('waiting_for_ready hides retry button', () => {
    dg.onDynoUpdate({ phase: 'waiting_for_ready' });
    expect(root.querySelector('[data-dyno="retry"]').hidden).toBe(true);
  });

  it('waiting_for_ready sets title to contain "Setup"', () => {
    dg.onDynoUpdate({ phase: 'waiting_for_ready' });
    expect(root.querySelector('[data-dyno="title"]').textContent).toContain('Setup');
  });

  // ── Phase: complete ───────────────────────────────────────────

  it('complete shows retry button', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: [] });
    expect(root.querySelector('[data-dyno="retry"]').hidden).toBe(false);
  });

  it('complete hides stop-wrap element', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: [] });
    expect(root.querySelector('[data-dyno="stop-wrap"]').hidden).toBe(true);
  });

  it('complete hides graph-wrap when bins array is empty', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: [] });
    expect(root.querySelector('[data-dyno="graph-wrap"]').hidden).toBe(true);
  });

  it('complete shows graph-wrap when bins are present', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(root.querySelector('[data-dyno="graph-wrap"]').hidden).toBe(false);
  });

  it('complete title reads "Dyno — Complete"', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: [] });
    expect(root.querySelector('[data-dyno="title"]').textContent).toBe('Dyno — Complete');
  });

  // ── Stats section ─────────────────────────────────────────────

  it('onDynoUpdate with bins renders Peak Power stat', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(root.querySelector('[data-dyno="stats"]').innerHTML).toContain('Peak Power');
  });

  it('onDynoUpdate with bins renders Peak Torque stat', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(root.querySelector('[data-dyno="stats"]').innerHTML).toContain('Peak Torque');
  });

  it('onDynoUpdate with bins renders Redline stat', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(root.querySelector('[data-dyno="stats"]').innerHTML).toContain('Redline');
  });

  it('onDynoUpdate with bins renders Power Band Start stat', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(root.querySelector('[data-dyno="stats"]').innerHTML).toContain('Power Band Start');
  });

  // ── Unit toggle ───────────────────────────────────────────────

  it('unit toggle button starts with text "SI"', () => {
    expect(root.querySelector('[data-dyno="unit-toggle"]').textContent).toBe('SI');
  });

  it('clicking unit toggle switches button text to "Imperial"', () => {
    root.querySelector('[data-dyno="unit-toggle"]').click();
    expect(root.querySelector('[data-dyno="unit-toggle"]').textContent).toBe('Imperial');
  });

  it('clicking unit toggle twice cycles back to "SI"', () => {
    const btn = root.querySelector('[data-dyno="unit-toggle"]');
    btn.click();
    btn.click();
    expect(btn.textContent).toBe('SI');
  });

  it('unit toggle with bins re-renders stats using imperial units', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    root.querySelector('[data-dyno="unit-toggle"]').click();
    const stats = root.querySelector('[data-dyno="stats"]').innerHTML;
    // Imperial power label
    expect(stats).toContain('HP');
  });

  it('unit toggle back to SI shows kW unit label', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    const btn = root.querySelector('[data-dyno="unit-toggle"]');
    btn.click(); // → Imperial
    btn.click(); // → SI
    const stats = root.querySelector('[data-dyno="stats"]').innerHTML;
    expect(stats).toContain('kW');
  });

  // ── canvas drawing ────────────────────────────────────────────

  it('clearRect is called when bins are rendered', () => {
    dg.onDynoUpdate({ phase: 'complete', bins: SAMPLE_BINS });
    expect(ctxStub.clearRect).toHaveBeenCalled();
  });
});
