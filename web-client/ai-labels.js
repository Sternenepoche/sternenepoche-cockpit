// One small label per displayed image, including images added by the game UI.
(() => {
  'use strict';
  const labels = new Map();
  const selector = 'img:not(.art-color):not([data-auth-qr]), canvas, [data-ai-image]';
  let scheduled = false;
  const resize = new ResizeObserver(schedule);

  function schedule() {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(update);
  }

  function update() {
    scheduled = false;
    for (const [source, label] of labels) {
      if (!source.isConnected || label.parentElement !== hostFor(source)) {
        resize.unobserve(source);
        label.remove();
        labels.delete(source);
      }
    }
    for (const source of document.querySelectorAll(selector)) {
      if(source.closest('.art-shell,.tile-image,.item-info-dialog,.planet-pick,.bs-kachel,.bs-banner,.bs-detail-bild,.bs-auftrag,.planet-bild,.gt-tabelle,#techinfo'))continue;
      if (labels.has(source)) continue;
      const host = hostFor(source);
      if (!host) continue;
      // Kleine Bilder (Kosten-Symbole, Listen) tragen nie eine sichtbare Marke – gar nicht erst anlegen.
      if (source.tagName === 'IMG' && source.complete && source.naturalWidth && Math.min(source.offsetWidth, source.offsetHeight) < 64) continue;
      if (getComputedStyle(host).position === 'static') host.style.position = 'relative';
      const label = document.createElement('span');
      label.className = 'ai-image-label';
      label.textContent = 'KI';
      label.setAttribute('aria-hidden', 'true');
      if (source.hasAttribute('data-ai-image')) label.classList.add('ai-background-label');
      if (source === document.body) label.classList.add('ai-page-label');
      host.append(label);
      labels.set(source, label);
      resize.observe(source);
    }
    for (const [source, label] of labels) {
      // Nur schreiben, was sich ändert: jedes Setzen löste neue Mutationen und am Handy sichtbares Blinken aus.
      const hidden = Math.min(source.offsetWidth, source.offsetHeight) < 64 || !source.getClientRects().length || !source.offsetWidth || !source.offsetHeight
        || (source.tagName === 'IMG' && (!source.complete || !source.naturalWidth));
      if (label.hidden !== hidden) label.hidden = hidden;
      if (hidden || source.hasAttribute('data-ai-image')) continue;
      // Sibling overlays preserve existing grid, flex and absolute image layouts.
      const left = `${source.offsetLeft + source.offsetWidth - 2}px`, top = `${source.offsetTop + source.offsetHeight - 2}px`;
      if (label.style.left !== left) label.style.left = left;
      if (label.style.top !== top) label.style.top = top;
    }
  }

  function hostFor(source) {
    return source.hasAttribute('data-ai-image') ? source : source.parentElement;
  }

  function start() {
    new MutationObserver(records => {
      if (records.some(record => record.type === 'attributes'
        ? !record.target.classList.contains('ai-image-label') && !record.target.closest('#galaxy-labels')
        : [...record.addedNodes, ...record.removedNodes].some(node =>
          node.nodeType === 1 && !node.classList.contains('ai-image-label')))) schedule();
    }).observe(document.body, {subtree: true, childList: true, attributes: true,
      attributeFilter: ['src', 'hidden', 'class', 'open']});
    document.addEventListener('load', schedule, true);
    document.addEventListener('error', schedule, true);
    window.addEventListener('resize', schedule);
    document.fonts?.ready.then(schedule);
    schedule();
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, {once: true});
  else start();
})();
