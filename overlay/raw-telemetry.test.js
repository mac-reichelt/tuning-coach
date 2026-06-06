/**
 * Tests for fmt (formatting helper) and RawTelemetryPanel.
 *
 * Coverage for fmt:
 *  - null / undefined → 'N/A'
 *  - boolean true/false → 'true' / 'false'
 *  - integer values → no decimal places
 *  - non-integer floats → exactly 3 decimal places
 *  - string passthrough
 *
 * Coverage for RawTelemetryPanel:
 *  - panel starts hidden, button reads 'Raw Data'
 *  - toggle() shows the panel and changes button text to 'Hide Raw'
 *  - second toggle() hides the panel again and reverts button text
 *  - update() followed by toggle() renders one <tr> per field, sorted by key
 *  - null values receive the raw-null CSS class; non-null values do not
 *  - update() while hidden does not render until toggle() makes it visible
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// makeDraggable uses setPointerCapture which jsdom stubs incompletely;
// mock the module so the panel constructor never touches pointer APIs.
vi.mock('./src/drag.js', () => ({ makeDraggable: vi.fn() }));

import { fmt, RawTelemetryPanel } from './src/raw-telemetry.js';

// ── fmt ───────────────────────────────────────────────────────────────────────

describe('fmt', () => {
  it('returns N/A for null', () => {
    expect(fmt(null)).toBe('N/A');
  });

  it('returns N/A for undefined', () => {
    expect(fmt(undefined)).toBe('N/A');
  });

  it('returns "true" for boolean true', () => {
    expect(fmt(true)).toBe('true');
  });

  it('returns "false" for boolean false', () => {
    expect(fmt(false)).toBe('false');
  });

  it('returns the integer as a string with no decimal point', () => {
    expect(fmt(42)).toBe('42');
  });

  it('returns 0 as a string with no decimal point', () => {
    expect(fmt(0)).toBe('0');
  });

  it('returns a float to exactly 3 decimal places', () => {
    expect(fmt(3.14159)).toBe('3.142');
  });

  it('formats a float with trailing significant digits', () => {
    expect(fmt(2.5)).toBe('2.500');
  });

  it('returns a string value unchanged', () => {
    expect(fmt('hello')).toBe('hello');
  });
});

// ── RawTelemetryPanel ─────────────────────────────────────────────────────────

describe('RawTelemetryPanel', () => {
  let panel;
  let btn;
  let rtp;

  beforeEach(() => {
    panel = document.createElement('div');
    btn   = document.createElement('button');
    document.body.appendChild(panel);
    document.body.appendChild(btn);
    rtp = new RawTelemetryPanel(panel, btn);
  });

  afterEach(() => {
    panel.remove();
    btn.remove();
    vi.clearAllMocks();
  });

  // ── Initial state ──────────────────────────────────────────────

  it('starts with the panel hidden', () => {
    expect(panel.hidden).toBe(true);
  });

  it('starts with button text "Raw Data"', () => {
    expect(btn.textContent).toBe('Raw Data');
  });

  // ── toggle() ──────────────────────────────────────────────────

  it('toggle() makes the panel visible', () => {
    rtp.toggle();
    expect(panel.hidden).toBe(false);
  });

  it('toggle() changes button text to "Hide Raw"', () => {
    rtp.toggle();
    expect(btn.textContent).toBe('Hide Raw');
  });

  it('second toggle() hides the panel again', () => {
    rtp.toggle();
    rtp.toggle();
    expect(panel.hidden).toBe(true);
  });

  it('second toggle() reverts button text to "Raw Data"', () => {
    rtp.toggle();
    rtp.toggle();
    expect(btn.textContent).toBe('Raw Data');
  });

  // ── update() + rendering ──────────────────────────────────────

  it('update() then toggle() renders one row per field', () => {
    rtp.update({ speed_mps: 10.5, lap_number: 3 });
    rtp.toggle();
    expect(panel.querySelectorAll('tbody tr').length).toBe(2);
  });

  it('rows are sorted alphabetically by field key', () => {
    rtp.update({ zzz: 1, aaa: 2 });
    rtp.toggle();
    const labels = [...panel.querySelectorAll('.raw-key')].map(td => td.textContent);
    expect(labels[0]).toContain('aaa');
    expect(labels[1]).toContain('zzz');
  });

  it('null values receive the raw-null CSS class', () => {
    rtp.update({ speed_mps: null });
    rtp.toggle();
    expect(panel.querySelector('.raw-null')).not.toBeNull();
  });

  it('non-null values do not receive the raw-null CSS class', () => {
    rtp.update({ speed_mps: 42 });
    rtp.toggle();
    expect(panel.querySelector('.raw-null')).toBeNull();
  });

  it('update() while hidden does not render rows until panel is shown', () => {
    rtp.update({ speed_mps: 10 });
    // Panel is still hidden — tbody should be empty
    expect(panel.querySelectorAll('tbody tr').length).toBe(0);

    rtp.toggle();
    // Now visible — should have rendered
    expect(panel.querySelectorAll('tbody tr').length).toBe(1);
  });

  it('uses the LABELS override for known field names', () => {
    rtp.update({ speed_mps: 5 });
    rtp.toggle();
    expect(panel.querySelector('.raw-key').textContent).toBe('Speed (m/s)');
  });
});
