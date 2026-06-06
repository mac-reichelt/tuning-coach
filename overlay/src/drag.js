/**
 * drag.js — Lightweight draggable utility for fixed-position overlay panels.
 *
 * Converts right/bottom CSS anchoring to left/top on first drag so the panel
 * stays where the user drops it.  Uses Pointer Events for compatibility with
 * SimHub's Chromium runtime.
 *
 * @param {HTMLElement} el      The fixed-position element to make moveable.
 * @param {HTMLElement} handle  The child element the user grabs to drag.
 */
export function makeDraggable(el, handle) {
  let startX = 0, startY = 0, startL = 0, startT = 0;

  handle.style.cursor = 'grab';

  handle.addEventListener('pointerdown', (e) => {
    // Convert right/bottom anchoring to left/top so we can offset freely.
    const rect    = el.getBoundingClientRect();
    el.style.left   = `${rect.left}px`;
    el.style.top    = `${rect.top}px`;
    el.style.right  = 'auto';
    el.style.bottom = 'auto';

    startX = e.clientX;
    startY = e.clientY;
    startL = rect.left;
    startT = rect.top;

    handle.style.cursor = 'grabbing';
    el.style.userSelect = 'none';
    handle.setPointerCapture(e.pointerId);
  });

  handle.addEventListener('pointermove', (e) => {
    if (!handle.hasPointerCapture(e.pointerId)) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;
    el.style.left = `${startL + dx}px`;
    el.style.top  = `${startT + dy}px`;
  });

  handle.addEventListener('pointerup', () => {
    handle.style.cursor = 'grab';
    el.style.userSelect = '';
  });
}
