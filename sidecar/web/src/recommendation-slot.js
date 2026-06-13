/**
 * recommendation-slot.js — Recommendation slide-in panel.
 *
 * Renders a static placeholder card when no recommendation is present
 * (so layout is stable) and a live card when a recommendation arrives.
 *
 * Dismiss / snooze / history-toggle buttons are present but no-op until
 * Phase 7 wires real heuristics data.
 *
 * WS payload contract (additive — no schema_version bump per ADR-0002):
 *
 * {
 *   "type": "recommendation",
 *   "schema_version": 1,
 *   "t_ms": 1738012345678,
 *   "data": {
 *     "id":         "<ulid>",
 *     "session_id": "<ulid>",
 *     "lap_number": 3,
 *     "category":   "springs",
 *     "title":      "Front bottoming out",
 *     "detected":   "Front suspension >95% travel on 3 of 4 corners.",
 *     "cause":      "Insufficient front spring rate / ride height.",
 *     "adjustment": {
 *       "summary":   "Front spring rate 85 → 92 N/mm",
 *       "parameter": "spring_rate_front",
 *       "from":      85.0, "to": 92.0, "unit": "N/mm", "step": 1.0
 *     },
 *     "expected_outcome": "Eliminates bottoming on T1/T3.",
 *     "confidence": "high",
 *     "caveats":    ["Assumes smooth driving style"],
 *     "alternatives": [],
 *     "driving_style_assumed": "smooth",
 *     "locked_fallback_used": false
 *   }
 * }
 *
 * Usage:
 *   import { RecommendationSlot } from './recommendation-slot.js';
 *   const slot = new RecommendationSlot(
 *     document.getElementById('recommendation-slot')
 *   );
 *   // To show with placeholder:
 *   slot.showPlaceholder();
 *   // To render a real recommendation:
 *   slot.showRecommendation(data);
 */

export class RecommendationSlot {
  /** @type {HTMLElement} */
  #root;

  /** @type {HTMLElement} */
  #body;

  /** @type {HTMLElement} */
  #history;

  /** @type {Array<object>} */
  #historyItems = [];

  /** @type {ReturnType<typeof setTimeout> | null} Pending snooze re-show timer. */
  #snoozeTimer = null;

  /** Snooze duration: hide the panel, then auto-restore after this many ms. */
  static #SNOOZE_MS = 60_000;

  /** @type {number} Maximum number of history items to retain. */
  static #MAX_HISTORY = 20;

  /**
   * Escape all HTML special characters to prevent XSS.
   * @param {unknown} s
   * @returns {string}
   */
  static #esc(s) {
    return String(s ?? '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  /** @type {boolean} */
  #historyOpen = false;

  /**
   * @param {HTMLElement} rootEl  The #recommendation-slot container.
   */
  constructor(rootEl) {
    this.#root = rootEl;
    this.#build();
    this.showPlaceholder();
  }

  /** Slide the panel into view. Cancels any pending snooze. */
  show() {
    this.#clearSnooze();
    this.#root.classList.add('visible');
  }

  /** Slide the panel out of view. */
  hide() {
    this.#root.classList.remove('visible');
  }

  /** Whether the panel is currently visible. */
  isVisible() {
    return this.#root.classList.contains('visible');
  }

  /** Toggle visibility (used by the Coach overlay button). */
  toggle() {
    if (this.isVisible()) {
      this.hide();
    } else {
      this.show();
    }
  }

  /**
   * Temporarily hide the panel, then automatically restore it after `ms`.
   * Unlike dismiss (which stays hidden until reopened), snooze guarantees the
   * coach reappears so it is never lost.
   * @param {number} [ms]  Snooze duration in milliseconds.
   */
  snooze(ms = RecommendationSlot.#SNOOZE_MS) {
    this.hide();
    this.#clearSnooze();
    this.#snoozeTimer = setTimeout(() => {
      this.#snoozeTimer = null;
      this.#root.classList.add('visible');
    }, ms);
  }

  #clearSnooze() {
    if (this.#snoozeTimer !== null) {
      clearTimeout(this.#snoozeTimer);
      this.#snoozeTimer = null;
    }
  }

  /** Render the placeholder card (no active recommendation). */
  showPlaceholder() {
    this.#body.innerHTML = `
      <div class="rec-placeholder">
        <div class="rec-placeholder-icon">🏎</div>
        <div class="rec-placeholder-text">
          No recommendation yet.<br>
          Keep driving — the coach is watching.
        </div>
      </div>
    `;
    this.show();
  }

  /**
   * Render a live recommendation card.
   * @param {object} data  Recommendation `data` payload from the WS envelope.
   */
  showRecommendation(data) {
    if (!data) { this.showPlaceholder(); return; }

    // Archive to history
    this.#historyItems.unshift(data);
    if (this.#historyItems.length > RecommendationSlot.#MAX_HISTORY) {
      this.#historyItems.splice(RecommendationSlot.#MAX_HISTORY, this.#historyItems.length);
    }
    this.#rebuildHistory();

    const adj = data.adjustment ?? {};
    const esc = RecommendationSlot.#esc;

    this.#body.innerHTML = `
      <div class="rec-card">
        <div class="rec-card-title">${esc(data.title)}</div>
        <div class="rec-card-detected">${esc(data.detected)}</div>
        ${adj.summary ? `<div class="rec-card-adjustment">${esc(adj.summary)}</div>` : ''}
        ${data.confidence ? `
          <div class="rec-card-confidence" data-confidence="${esc(data.confidence)}">
            Confidence: <span>${esc(data.confidence)}</span>
          </div>
        ` : ''}
      </div>
    `;

    this.show();
  }

  // ── Private ────────────────────────────────────────────────

  #build() {
    this.#root.innerHTML = `
      <div class="rec-header">
        <span class="rec-title">Coach</span>
        <div class="rec-controls">
          <button class="rec-btn" data-action="snooze" title="Snooze">Snooze</button>
          <button class="rec-btn" data-action="history" title="History">History</button>
          <button class="rec-btn" data-action="dismiss" title="Dismiss">✕</button>
        </div>
      </div>
      <div class="rec-body" data-rec="body"></div>
      <div class="rec-history" data-rec="history"></div>
    `;

    this.#body    = this.#root.querySelector('[data-rec="body"]');
    this.#history = this.#root.querySelector('[data-rec="history"]');

    // Wire up control buttons (no-op stubs until Phase 7)
    this.#root.addEventListener('click', (ev) => {
      const btn = ev.target.closest('[data-action]');
      if (!btn) return;
      const action = btn.dataset.action;
      if (action === 'dismiss') {
        this.hide();
      } else if (action === 'snooze') {
        this.snooze();
      } else if (action === 'history') {
        this.#toggleHistory();
      }
    });
  }

  #toggleHistory() {
    this.#historyOpen = !this.#historyOpen;
    this.#history.classList.toggle('open', this.#historyOpen);
  }

  #rebuildHistory() {
    const esc = RecommendationSlot.#esc;
    this.#history.innerHTML = this.#historyItems.length === 0
      ? '<div class="rec-history-item">No history yet.</div>'
      : this.#historyItems.map((item) =>
          `<div class="rec-history-item">Lap ${item.lap_number ?? '?'}: ${
            esc(item.title ?? 'Recommendation')
          }</div>`
        ).join('');
  }
}
