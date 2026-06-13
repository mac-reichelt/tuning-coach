/**
 * Tests for RecommendationSlot urgency rendering.
 *
 * Coverage:
 *  - critical payload renders a "Live" badge with data-urgency="critical"
 *  - deferred payload renders a "Lap review" badge with data-urgency="deferred"
 *  - missing/unknown urgency falls back to deferred
 *  - expected_outcome is rendered when present
 *  - title/detected/adjustment are HTML-escaped (XSS guard)
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { RecommendationSlot } from './src/recommendation-slot.js';

function makeRec(overrides = {}) {
  return {
    id: 'R1',
    session_id: '1',
    lap_number: 3,
    category: 'anti_roll',
    title: 'Mid-corner understeer',
    detected: 'Front slip angle averaged 1.4° more than rear.',
    cause: 'Front-limited balance.',
    adjustment: { summary: 'Soften front ARB ~2 clicks', parameter: 'anti_roll_front', to: -2, step: 1, unit: 'clicks (Δ)' },
    expected_outcome: 'Shifts load transfer rearward.',
    confidence: 'medium',
    urgency: 'deferred',
    ...overrides,
  };
}

describe('RecommendationSlot urgency', () => {
  let el;
  let slot;

  beforeEach(() => {
    el = document.createElement('div');
    document.body.appendChild(el);
    slot = new RecommendationSlot(el);
  });

  afterEach(() => {
    el.remove();
  });

  it('renders a Live badge for critical recommendations', () => {
    slot.showRecommendation(makeRec({ urgency: 'critical', title: 'Front brake lockup' }));
    const badge = el.querySelector('.rec-card-badge');
    expect(badge).not.toBeNull();
    expect(badge.dataset.urgency).toBe('critical');
    expect(badge.textContent).toBe('Live');
    expect(el.querySelector('.rec-card').dataset.urgency).toBe('critical');
  });

  it('renders a Lap review badge for deferred recommendations', () => {
    slot.showRecommendation(makeRec({ urgency: 'deferred' }));
    const badge = el.querySelector('.rec-card-badge');
    expect(badge.dataset.urgency).toBe('deferred');
    expect(badge.textContent).toBe('Lap review');
  });

  it('falls back to deferred when urgency is missing or unknown', () => {
    slot.showRecommendation(makeRec({ urgency: undefined }));
    expect(el.querySelector('.rec-card-badge').dataset.urgency).toBe('deferred');

    slot.showRecommendation(makeRec({ urgency: 'bogus' }));
    expect(el.querySelector('.rec-card-badge').dataset.urgency).toBe('deferred');
  });

  it('renders the expected outcome when present', () => {
    slot.showRecommendation(makeRec());
    const outcome = el.querySelector('.rec-card-outcome');
    expect(outcome).not.toBeNull();
    expect(outcome.textContent).toContain('load transfer rearward');
  });

  it('escapes HTML in user-facing fields', () => {
    slot.showRecommendation(makeRec({ title: '<img src=x onerror=alert(1)>' }));
    const title = el.querySelector('.rec-card-title');
    expect(title.querySelector('img')).toBeNull();
    expect(title.textContent).toContain('<img');
  });
});
