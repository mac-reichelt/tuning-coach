/**
 * Tests for TelemetryStatus.
 *
 * Coverage:
 *  - constructor sets initial data-state='none' and renders child spans
 *  - update(true)  → state='live',   text='Live'
 *  - update(false) → state='paused', text='Paused'
 *  - stale timer fires after STALE_MS (3 000 ms) and resets state to 'none'
 *  - repeated update() calls reset the stale timer (last writer wins)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { TelemetryStatus } from './src/telemetry-status.js';

describe('TelemetryStatus', () => {
  let el;
  let ts;

  beforeEach(() => {
    el = document.createElement('div');
    document.body.appendChild(el);
    ts = new TelemetryStatus(el);
  });

  afterEach(() => {
    el.remove();
    vi.useRealTimers();
  });

  // ── Construction ──────────────────────────────────────────────

  it('sets data-state to none on construction', () => {
    expect(el.dataset.state).toBe('none');
  });

  it('renders a .ts-dot and .ts-text span on construction', () => {
    expect(el.querySelector('.ts-dot')).not.toBeNull();
    expect(el.querySelector('.ts-text')).not.toBeNull();
  });

  it('shows "No data" text on construction', () => {
    expect(el.querySelector('.ts-text').textContent).toBe('No data');
  });

  // ── update(true) ──────────────────────────────────────────────

  it('update(true) sets data-state to live', () => {
    ts.update(true);
    expect(el.dataset.state).toBe('live');
  });

  it('update(true) sets visible text to Live', () => {
    ts.update(true);
    expect(el.querySelector('.ts-text').textContent).toBe('Live');
  });

  // ── update(false) ─────────────────────────────────────────────

  it('update(false) sets data-state to paused', () => {
    ts.update(false);
    expect(el.dataset.state).toBe('paused');
  });

  it('update(false) sets visible text to Paused', () => {
    ts.update(false);
    expect(el.querySelector('.ts-text').textContent).toBe('Paused');
  });

  // ── Stale timer ───────────────────────────────────────────────

  it('reverts to none after exactly 3 000 ms without an update', () => {
    vi.useFakeTimers();
    ts.update(true);
    expect(el.dataset.state).toBe('live');

    vi.advanceTimersByTime(3_000);
    expect(el.dataset.state).toBe('none');
  });

  it('does not revert before 3 000 ms has elapsed', () => {
    vi.useFakeTimers();
    ts.update(true);

    vi.advanceTimersByTime(2_999);
    expect(el.dataset.state).toBe('live');
  });

  it('repeated update() calls reset the stale timer', () => {
    vi.useFakeTimers();
    ts.update(true);

    // Advance to just before stale, then call update() again to reset the timer
    vi.advanceTimersByTime(2_900);
    ts.update(true);

    // 2 900 ms since the second update — still live
    vi.advanceTimersByTime(2_900);
    expect(el.dataset.state).toBe('live');

    // Past 3 000 ms since the second update — now stale
    vi.advanceTimersByTime(101);
    expect(el.dataset.state).toBe('none');
  });
});
