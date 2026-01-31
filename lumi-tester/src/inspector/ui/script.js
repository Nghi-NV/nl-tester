// State
let showBoxes = true;
let cachedElements = [];
let currentSelectors = [];
let scaleX = 1;
let scaleY = 1;

// Init
window.addEventListener('DOMContentLoaded', () => {
  capture();

  // Interaction listeners
  const overlay = document.getElementById('overlay');
  overlay.addEventListener('click', (e) => {
    const rect = overlay.getBoundingClientRect();
    const x = (e.clientX - rect.left) / scaleX;
    const y = (e.clientY - rect.top) / scaleY;
    inspectAt(Math.round(x), Math.round(y), e.clientX - rect.left, e.clientY - rect.top);
  });

  window.addEventListener('resize', syncOverlaySize);
});

// UI Feedback
function showToast(msg) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2000);
}

function escapeHtml(s) {
  if (!s) return '';
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function toggleBoxes() {
  showBoxes = !showBoxes;
  document.getElementById('boxToggle').classList.toggle('on', showBoxes);
  drawOverlay();
}

// Capture Logic
async function capture() {
  const btn = document.querySelector('.btn-icon');
  if (btn) btn.disabled = true;
  document.getElementById('status').textContent = 'Capturing...';

  try {
    const r = await fetch(`/api/screenshot?skip_hierarchy=false`);
    if (!r.ok) throw new Error('Capture failed');
    const d = await r.json();

    const img = document.getElementById('screen');
    const loadPromise = new Promise(resolve => img.onload = resolve);
    img.src = 'data:image/jpeg;base64,' + d.data;
    img.style.display = 'block';
    document.getElementById('placeholder').style.display = 'none';

    await loadPromise;

    // Fetch hierarchy
    try {
      const h = await fetch('/api/hierarchy');
      const hd = await h.json();
      cachedElements = hd.elements || [];
    } catch (e) {
      console.warn("Hierarchy fetch failed", e);
    }

    syncOverlaySize();
    document.getElementById('status').textContent = `Captured ${d.width}×${d.height}`;

  } catch (e) {
    console.error(e);
    document.getElementById('status').textContent = 'Capture failed';
    showToast('Capture failed');
  } finally {
    if (btn) btn.disabled = false;
  }
}

// Canvas & Overlay Alignment Fix
function syncOverlaySize() {
  const canvas = document.getElementById('overlay');
  const img = document.getElementById('screen');
  const wrapper = document.getElementById('screenWrapper');

  if (!img || img.style.display === 'none') return;

  const containerWidth = img.clientWidth;
  const containerHeight = img.clientHeight;
  const naturalWidth = img.naturalWidth;
  const naturalHeight = img.naturalHeight;

  if (!naturalWidth) return;

  const imageRatio = naturalWidth / naturalHeight;
  const containerRatio = containerWidth / containerHeight;

  let renderWidth, renderHeight, offsetX = 0, offsetY = 0;

  if (containerRatio > imageRatio) {
    // Height is the limiting factor
    renderHeight = containerHeight;
    renderWidth = renderHeight * imageRatio;
    offsetX = (containerWidth - renderWidth) / 2;
  } else {
    // Width is the limiting factor
    renderWidth = containerWidth;
    renderHeight = renderWidth / imageRatio;
    offsetY = (containerHeight - renderHeight) / 2;
  }

  // Align canvas to the RENDERED image content
  canvas.width = renderWidth;
  canvas.height = renderHeight;
  canvas.style.width = renderWidth + 'px';
  canvas.style.height = renderHeight + 'px';
  canvas.style.left = offsetX + 'px';
  canvas.style.top = offsetY + 'px';

  scaleX = renderWidth / naturalWidth;
  scaleY = renderHeight / naturalHeight;

  drawOverlay();
}

function drawOverlay() {
  const canvas = document.getElementById('overlay');
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (!showBoxes || !cachedElements.length) return;

  ctx.strokeStyle = 'rgba(88, 166, 255, 0.4)';
  ctx.lineWidth = 1;

  cachedElements.forEach(el => {
    if (el.bounds) {
      const x = el.bounds.left * scaleX;
      const y = el.bounds.top * scaleY;
      const w = (el.bounds.right - el.bounds.left) * scaleX;
      const h = (el.bounds.bottom - el.bounds.top) * scaleY;
      ctx.strokeRect(x, y, w, h);
    }
  });
}

// Interaction
// Global offsets for click handling (though relative to canvas is easier)
async function inspectAt(x, y, clickX, clickY) {
  // Visual feedback on the canvas (clickX/Y already relative to canvas)
  drawOverlay();
  const ctx = document.getElementById('overlay').getContext('2d');
  ctx.fillStyle = 'rgba(255, 255, 0, 0.6)';
  ctx.beginPath();
  ctx.arc(clickX, clickY, 8, 0, 2 * Math.PI);
  ctx.fill();

  document.getElementById('status').textContent = `Inspecting (${x}, ${y})...`;

  try {
    const res = await fetch(`/api/element-at?x=${Math.round(x)}&y=${Math.round(y)}`);
    const data = await res.json();

    if (data.found) {
      document.getElementById('selectionBadge').textContent = 'Selected';
      document.getElementById('selectionBadge').style.background = 'var(--green)';
      renderAppInfo(data.app_id);
      renderCommands(data.supported_commands);
      renderDetails(data.selectors);
    } else {
      document.getElementById('selectionBadge').textContent = 'No Selection';
      document.getElementById('selectionBadge').style.background = 'var(--muted)';
      clearDetails();
    }
  } catch (e) {
    console.error(e);
    showToast('Inspection failed');
    document.getElementById('status').textContent = 'Inspection failed';
  }
}

function clearDetails() {
  document.getElementById('appInfo').textContent = '';
  document.getElementById('commandsSection').style.display = 'none';
  const container = document.getElementById('detailsContent');
  // Preserve static children (commands section) but show empty state
  // Simplest is to just re-render the structure or hide lists
  // But wait, our HTML structure has changed.
  // Let's target the dynamic containers directly.

  // Actually detailsContent now has static children (commands section)
  // We should just clear the selectors list part?
  // The current renderDetails replaces innerHTML of a container.
  // We need a specific container for selectors now.
  // Wait, the HTML update added 'commandsSection' INSIDE 'detailsContent'.
  // renderDetails currently does `container.innerHTML = ...`. This will wipe out commandsSection!

  // FIX: We need to change renderDetails to target a specific container or append.
  // Let's modify renderDetails first.

  // Remove all children except commandsSection
  Array.from(container.children).forEach(child => {
    if (child.id !== 'commandsSection') {
      container.removeChild(child);
    }
  });

  // Show empty state
  container.insertAdjacentHTML('beforeend', `
    <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" style="margin-bottom: 12px; opacity: 0.5;">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <circle cx="8.5" cy="8.5" r="1.5"></circle>
            <polyline points="21 15 16 10 5 21"></polyline>
        </svg>
        <div>Click on an element to inspect</div>
    </div>
  `);
}

function renderAppInfo(appId) {
  const el = document.getElementById('appInfo');
  if (appId) {
    el.textContent = `appId: ${appId}`;
  } else {
    el.textContent = '';
  }
}

function renderCommands(commands) {
  const section = document.getElementById('commandsSection');
  const list = document.getElementById('commandsList');

  if (!commands || commands.length === 0) {
    section.style.display = 'none';
    return;
  }

  section.style.display = 'block';
  // Collapse by default (ensure list is hidden if logic requires, but HTML defaults to none)

  list.innerHTML = commands.map(cmd =>
    `<div class="command-item">- ${cmd}</div>`
  ).join('');
}

function toggleSection(listId, arrowId) {
  const list = document.getElementById(listId);
  const arrow = document.getElementById(arrowId);
  const header = arrow.parentElement;

  if (list.style.display === 'none') {
    list.style.display = 'block';
    header.classList.add('expanded');
  } else {
    list.style.display = 'none';
    header.classList.remove('expanded');
  }
}

// Rendering
function renderDetails(selectors) {
  // We need to inject selectors AFTER the commands section
  // Let's assume we have a separate container or we manage the DOM carefully.
  // The HTML provided has:
  // detailsContent -> [commandsSection, emptyState]
  // We should replace emptyState with selectors or append selectors.

  // Better approach: Have a dedicated div for selectors.
  // Since I can't easily change HTML again without another tool call (I could but it's cleaner to use what I have),
  // I will clear everything EXCEPT commandsSection? No, that's messy.

  // Let's look at the HTML structure again.
  // <div id="detailsContent">
  //    <div id="commandsSection">...</div>
  //    <div class="empty-state">...</div>
  // </div>

  // I should remove the empty state and append selector cards.
  // OR, simply clear everything and re-add commandsSection + selectors.
  // But clearDetails needs to restore empty state.

  const container = document.getElementById('detailsContent');

  // Store reference to commands section
  const commandsSection = document.getElementById('commandsSection');

  // Clear container but keep commandsSection
  // This is tricky if I overwrite innerHTML.
  // Hack: Since I need to fix renderDetails anyway, let's create a selectors container in JS if missing?
  // Or just create the DOM elements.

  // Let's do:
  // 1. Remove all children except commandsSection
  Array.from(container.children).forEach(child => {
    if (child.id !== 'commandsSection') {
      container.removeChild(child);
    }
  });

  const badge = document.getElementById('selectorCountBadge');
  if (badge) badge.textContent = selectors.length;

  if (!selectors || selectors.length === 0) {
    // Show empty state
    container.insertAdjacentHTML('beforeend', `
      <div class="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" style="margin-bottom: 12px; opacity: 0.5;">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              <circle cx="8.5" cy="8.5" r="1.5"></circle>
              <polyline points="21 15 16 10 5 21"></polyline>
          </svg>
          <div>No selectors found for this element.</div>
      </div>
    `);
    return;
  }

  const cardsHtml = selectors.map((s, i) => {
    const scoreClass = s.is_stable ? 'stable' : 'unstable';
    // Format value more nicely for "type" or "regex" or "point"
    let displayValue = escapeHtml(s.value);

    // Suggest the key: value format
    if (s.selector_type === 'type') {
      displayValue = `<span style="color:#79c0ff">type:</span> ${displayValue}`;
      if (s.index) {
        displayValue += ` <span style="color:#79c0ff">index:</span> ${s.index}`;
      }
    } else if (s.selector_type === 'text') {
      displayValue = `<span style="color:#79c0ff">text:</span> ${displayValue}`;
      if (s.index) {
        displayValue += ` <span style="color:#79c0ff">index:</span> ${s.index}`;
      }
    } else if (s.selector_type === 'point') {
      displayValue = `<span style="color:#79c0ff">point:</span> ${displayValue}`;
    } else if (s.selector_type === 'id') {
      displayValue = `<span style="color:#79c0ff">id:</span> ${displayValue}`;
    }

    return `
      <div class="selector-card ${scoreClass}">
        <div class="sel-header">
          <span class="sel-type">${s.selector_type}</span>
          <span class="sel-score">${s.score} pts</span>
        </div>
        <div class="sel-value">${displayValue}</div>
        ${s.description ? `<div class="sel-desc">${escapeHtml(s.description)}</div>` : ''}
        <div class="sel-actions">
          <button class="btn btn-outline btn-sm" onclick="copyToClipboard(${i})">Copy</button>
          <button class="btn btn-primary btn-sm" onclick="insertToEditor(${i})">Insert</button>
        </div>
      </div>
    `;
  }).join('');

  // Append new cards
  container.insertAdjacentHTML('beforeend', cardsHtml);
}

// External Actions
function copyToClipboard(idx) {
  const s = currentSelectors[idx];
  if (!s) return;
  navigator.clipboard.writeText(s.yaml).then(() => {
    showToast('Copied YAML');
  });
}

function insertToEditor(idx) {
  const s = currentSelectors[idx];
  if (!s || !window.parent) return;
  window.parent.postMessage({
    type: 'insertSelector',
    value: s.yaml,
    selector: s
  }, '*');
  showToast('Inserted to VSCode');
}
