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
      if (labels.has(source)) continue;
      const host = hostFor(source);
      if (!host) continue;
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
      label.hidden = Math.min(source.offsetWidth, source.offsetHeight) < 64 || !source.getClientRects().length || !source.offsetWidth || !source.offsetHeight
        || (source.tagName === 'IMG' && (!source.complete || !source.naturalWidth));
      if (source.hasAttribute('data-ai-image')) continue;
      // Sibling overlays preserve existing grid, flex and absolute image layouts.
      label.style.left = `${source.offsetLeft + source.offsetWidth - 2}px`;
      label.style.top = `${source.offsetTop + source.offsetHeight - 2}px`;
    }
  }

  function hostFor(source) {
    return source.hasAttribute('data-ai-image') ? source : source.parentElement;
  }

  function start() {
    new MutationObserver(records => {
      if (records.some(record => record.type === 'attributes'
        ? !record.target.classList.contains('ai-image-label')
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
