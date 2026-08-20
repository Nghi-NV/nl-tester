// State Management
let showBoxes = true;
let cachedElements = [];
let currentSelectors = [];
let currentAttributes = {};
let currentHierarchy = [];
let currentActionType = 'tap'; // 'tap' | 'see' | 'wait' | 'inputText' | 'copyText' | 'longPress'
let activeTab = 'selectors'; // 'selectors' | 'attributes'
let allApps = [];
let selectedAppTarget = null;
let scaleX = 1;
let scaleY = 1;
let zoomLevel = 1.0;
let lastSelectedBounds = null;

// Initialization
window.addEventListener('DOMContentLoaded', () => {
  capture();
  loadPackages();

  const overlay = document.getElementById('overlay');
  
  // Click on canvas to inspect
  overlay.addEventListener('click', (e) => {
    const rect = overlay.getBoundingClientRect();
    const x = (e.clientX - rect.left) / scaleX / zoomLevel;
    const y = (e.clientY - rect.top) / scaleY / zoomLevel;
    inspectAt(Math.round(x), Math.round(y), (e.clientX - rect.left) / zoomLevel, (e.clientY - rect.top) / zoomLevel);
  });

  // Mouse move for hover tooltip
  overlay.addEventListener('mousemove', (e) => {
    const rect = overlay.getBoundingClientRect();
    const x = (e.clientX - rect.left) / scaleX / zoomLevel;
    const y = (e.clientY - rect.top) / scaleY / zoomLevel;
    handleCanvasHover(x, y, e.clientX, e.clientY);
  });

  overlay.addEventListener('mouseleave', () => {
    hideHoverTooltip();
  });

  // Wheel zoom with Ctrl or Cmd
  const canvasContainer = document.getElementById('canvasContainer');
  canvasContainer.addEventListener('wheel', (e) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      if (e.deltaY < 0) {
        zoomIn();
      } else {
        zoomOut();
      }
    }
  }, { passive: false });

  window.addEventListener('resize', () => {
    requestAnimationFrame(syncOverlaySize);
  });

  if (window.ResizeObserver) {
    const ro = new ResizeObserver(() => {
      requestAnimationFrame(syncOverlaySize);
    });
    const cc = document.getElementById('canvasContainer');
    const sp = document.getElementById('screenPanel');
    const app = document.querySelector('.app-container');
    if (cc) ro.observe(cc);
    if (sp) ro.observe(sp);
    if (app) ro.observe(app);
  }
});

// Toast Feedback Notification
function showToast(msg) {
  const t = document.getElementById('toast');
  const msgEl = document.getElementById('toastMsg');
  if (!t) return;
  if (msgEl) msgEl.textContent = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2200);
}

function escapeHtml(s) {
  if (!s) return '';
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function formatYamlHighlight(yamlText) {
  if (!yamlText) return '';
  const escaped = escapeHtml(yamlText.trim());
  return escaped
    .split('\n')
    .map(line => {
      // 1. Action command e.g. "- tap:", "- see:", "- waitUntilVisible:"
      let l = line.replace(/^(\s*-\s+)([\w]+)(:)/, '$1<span class="yaml-cmd">$2</span>$3');
      // 2. Map keys e.g. "    id:", "    text:", "    type:", "    above:", "    below:", "    align:", "    offset:", "    point:", "    index:"
      l = l.replace(/^(\s*)([\w]+)(:)/, '$1<span class="yaml-key">$2</span>$3');
      // 3. String values in quotes e.g. &quot;...&quot;
      l = l.replace(/(:\s*)(&quot;.*?&quot;)/g, '$1<span class="yaml-str">$2</span>');
      // 4. Numbers e.g. 19
      l = l.replace(/(:\s*)(\b\d+\b)/g, '$1<span class="yaml-num">$2</span>');
      // 5. Enum keywords e.g. right, left, top, bottom, center, true, false
      l = l.replace(/(:\s*)\b(true|false|right|left|top|bottom|center)\b/g, '$1<span class="yaml-enum">$2</span>');
      return l;
    })
    .join('\n');
}

// Zoom & Viewport Controls
function updateZoom() {
  const wrapper = document.getElementById('screenWrapper');
  const text = document.getElementById('zoomLevelText');
  if (wrapper) {
    wrapper.style.transform = `scale(${zoomLevel})`;
  }
  if (text) {
    text.textContent = `${Math.round(zoomLevel * 100)}%`;
  }
}

function zoomIn() {
  if (zoomLevel < 3.0) {
    zoomLevel = Math.min(3.0, +(zoomLevel + 0.15).toFixed(2));
    updateZoom();
  }
}

function zoomOut() {
  if (zoomLevel > 0.4) {
    zoomLevel = Math.max(0.4, +(zoomLevel - 0.15).toFixed(2));
    updateZoom();
  }
}

function resetZoom() {
  zoomLevel = 1.0;
  updateZoom();
}

function fitToScreen() {
  zoomLevel = 1.0;
  updateZoom();
  syncOverlaySize();
}

// Hover Tooltip on Canvas
function handleCanvasHover(x, y, clientX, clientY) {
  if (!cachedElements.length) {
    hideHoverTooltip();
    return;
  }

  let best = null;
  let minArea = Infinity;

  for (const el of cachedElements) {
    if (el.bounds) {
      const b = el.bounds;
      if (x >= b.left && x <= b.right && y >= b.top && y <= b.bottom) {
        const area = (b.right - b.left) * (b.bottom - b.top);
        if (area > 0 && area < minArea) {
          minArea = area;
          best = el;
        }
      }
    }
  }

  if (best) {
    showHoverTooltip(best, clientX, clientY);
  } else {
    hideHoverTooltip();
  }
}

function showHoverTooltip(el, clientX, clientY) {
  const tooltip = document.getElementById('hoverTooltip');
  if (!tooltip) return;

  const shortClass = el.class ? el.class.split('.').pop() : 'View';
  const label = el.text ? `"${el.text.slice(0, 18)}"` : (el.resource_id ? `#${el.resource_id.split('/').pop()}` : '');
  const w = el.bounds ? el.bounds.right - el.bounds.left : 0;
  const h = el.bounds ? el.bounds.bottom - el.bounds.top : 0;

  tooltip.innerHTML = `<strong>${escapeHtml(shortClass)}</strong> ${escapeHtml(label)} <span style="opacity:0.6;font-size:9.5px">${w}×${h}px</span>`;
  tooltip.style.display = 'block';
  tooltip.style.left = `${clientX - document.getElementById('canvasContainer').getBoundingClientRect().left}px`;
  tooltip.style.top = `${clientY - document.getElementById('canvasContainer').getBoundingClientRect().top}px`;
}

function hideHoverTooltip() {
  const tooltip = document.getElementById('hoverTooltip');
  if (tooltip) tooltip.style.display = 'none';
}

// Capture Logic
async function capture() {
  const btn = document.querySelector('.btn-refresh');
  const statusText = document.getElementById('statusText');
  if (btn) btn.disabled = true;
  if (statusText) statusText.textContent = 'Capturing device screen & hierarchy...';

  try {
    const r = await fetch(`/api/screenshot?skip_hierarchy=false`);
    if (!r.ok) throw new Error('Capture failed: ' + r.statusText);
    const d = await r.json();

    const img = document.getElementById('screen');
    const placeholder = document.getElementById('placeholder');

    await new Promise((resolve) => {
      img.onload = () => resolve();
      img.onerror = () => resolve();
      img.src = 'data:image/jpeg;base64,' + d.data;
    });

    img.style.display = 'block';
    if (placeholder) placeholder.style.display = 'none';

    // Fetch hierarchy
    try {
      const h = await fetch('/api/hierarchy');
      if (h.ok) {
        const hd = await h.json();
        cachedElements = hd.elements || [];
      }
    } catch (e) {
      console.warn("Hierarchy fetch failed", e);
    }

    syncOverlaySize();
    if (statusText) statusText.textContent = `Connected • ${d.width} × ${d.height} px`;

  } catch (e) {
    console.error(e);
    if (statusText) statusText.textContent = 'Screen capture failed';
    showToast('Screen capture failed');
  } finally {
    if (btn) btn.disabled = false;
  }
}

// Canvas & Overlay Alignment (Pixel-Perfect Mathematical Aspect Ratio Containment)
function syncOverlaySize() {
  const canvas = document.getElementById('overlay');
  const img = document.getElementById('screen');
  const wrapper = document.getElementById('screenWrapper');
  const container = document.getElementById('canvasContainer');

  if (!img || img.style.display === 'none' || !img.naturalWidth || !container || !wrapper) return;

  const padX = 24;
  const padY = 40;
  const availW = Math.max(40, container.clientWidth - padX);
  const availH = Math.max(40, container.clientHeight - padY);

  const imgRatio = img.naturalWidth / img.naturalHeight;
  const contRatio = availW / availH;

  let targetW, targetH;

  if (contRatio > imgRatio) {
    // Height is constraint -> fit to full available height
    targetH = availH;
    targetW = Math.round(targetH * imgRatio);
  } else {
    // Width is constraint -> fit to full available width
    targetW = availW;
    targetH = Math.round(targetW / imgRatio);
  }

  // Explicitly size wrapper, img, and canvas so nothing overflows or gets cropped
  wrapper.style.width = targetW + 'px';
  wrapper.style.height = targetH + 'px';

  img.style.width = targetW + 'px';
  img.style.height = targetH + 'px';

  canvas.width = targetW;
  canvas.height = targetH;
  canvas.style.width = targetW + 'px';
  canvas.style.height = targetH + 'px';
  canvas.style.left = '0px';
  canvas.style.top = '0px';

  scaleX = targetW / img.naturalWidth;
  scaleY = targetH / img.naturalHeight;

  drawOverlay(lastSelectedBounds);
}

function drawOverlay(highlightBounds = null) {
  const canvas = document.getElementById('overlay');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Draw light subtle boundaries for all elements if toggled on
  if (showBoxes && cachedElements.length) {
    ctx.strokeStyle = 'rgba(56, 189, 248, 0.25)';
    ctx.lineWidth = 1;

    cachedElements.forEach(el => {
      if (el.bounds) {
        const x = Math.round(el.bounds.left * scaleX);
        const y = Math.round(el.bounds.top * scaleY);
        const w = Math.round((el.bounds.right - el.bounds.left) * scaleX);
        const h = Math.round((el.bounds.bottom - el.bounds.top) * scaleY);
        ctx.strokeRect(x + 0.5, y + 0.5, w, h);
      }
    });
  }

  // Draw active selected element with high-precision glowing box
  if (highlightBounds) {
    const x = Math.round(highlightBounds.left * scaleX);
    const y = Math.round(highlightBounds.top * scaleY);
    const w = Math.round((highlightBounds.right - highlightBounds.left) * scaleX);
    const h = Math.round((highlightBounds.bottom - highlightBounds.top) * scaleY);

    // Glowing fill
    ctx.fillStyle = 'rgba(56, 189, 248, 0.18)';
    ctx.fillRect(x, y, w, h);

    // Sharp glowing border
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 2;
    ctx.strokeRect(x, y, w, h);
  }
}

// Inspect element at coordinates
async function inspectAt(x, y, clickX, clickY) {
  hideHoverTooltip();

  // Visual tap indicator ring on canvas
  const canvas = document.getElementById('overlay');
  const ctx = canvas.getContext('2d');
  ctx.fillStyle = 'rgba(56, 189, 248, 0.9)';
  ctx.beginPath();
  ctx.arc(clickX * scaleX, clickY * scaleY, 6, 0, 2 * Math.PI);
  ctx.fill();

  const statusText = document.getElementById('statusText');
  if (statusText) statusText.textContent = `Analyzing element at (${x}, ${y})...`;

  try {
    const res = await fetch(`/api/element-at?x=${Math.round(x)}&y=${Math.round(y)}`);
    const data = await res.json();

    if (data.found) {
      currentSelectors = data.selectors || [];
      currentAttributes = data.attributes || {};
      currentHierarchy = data.hierarchy || [];
      lastSelectedBounds = data.bounds;

      drawOverlay(data.bounds);

      // Hero Header Meta
      const shortClass = data.element_class ? data.element_class.split('.').pop() : 'View';
      const badge = document.getElementById('selectionBadge');
      badge.textContent = shortClass;
      badge.classList.add('selected');

      const metaEl = document.getElementById('elementMeta');
      metaEl.style.display = 'flex';
      document.getElementById('metaClass').textContent = shortClass;

      if (data.bounds) {
        const w = data.bounds.right - data.bounds.left;
        const h = data.bounds.bottom - data.bounds.top;
        document.getElementById('metaDim').textContent = `${w} × ${h} px`;
      }

      const resMeta = document.getElementById('metaRes');
      if (data.attributes && data.attributes['resource-id']) {
        const shortId = data.attributes['resource-id'].split('/').pop();
        resMeta.textContent = `#${shortId}`;
        resMeta.style.display = 'inline-block';
      } else {
        resMeta.style.display = 'none';
      }

      renderAppInfo(data.app_id);
      renderBreadcrumbs(currentHierarchy);
      renderSelectors();
      renderAttributes(currentAttributes);

      // Show sections
      document.getElementById('breadcrumbBar').style.display = currentHierarchy.length > 0 ? 'flex' : 'none';
      document.getElementById('actionBar').style.display = 'block';
      document.getElementById('tabBar').style.display = 'flex';
      document.getElementById('emptyState').style.display = 'none';

      if (statusText) statusText.textContent = `Selected ${shortClass} at (${x}, ${y})`;
    } else {
      currentSelectors = data.selectors || [];
      currentAttributes = {};
      currentHierarchy = [];
      lastSelectedBounds = null;

      const badge = document.getElementById('selectionBadge');
      badge.textContent = 'No Selection';
      badge.classList.remove('selected');
      clearDetails();
      if (statusText) statusText.textContent = `No element found at (${x}, ${y})`;
    }
  } catch (e) {
    console.error(e);
    showToast('Inspection failed');
    if (statusText) statusText.textContent = 'Inspection failed';
  }
}

function clearDetails() {
  document.getElementById('appInfo').textContent = '';
  document.getElementById('elementMeta').style.display = 'none';
  document.getElementById('breadcrumbBar').style.display = 'none';
  document.getElementById('actionBar').style.display = 'none';
  document.getElementById('tabBar').style.display = 'none';
  document.getElementById('selectorsList').innerHTML = '';
  document.getElementById('attributesGrid').innerHTML = '';
  document.getElementById('emptyState').style.display = 'flex';
  drawOverlay();
}

function renderAppInfo(appId) {
  const el = document.getElementById('appInfo');
  if (appId) {
    el.textContent = appId;
  } else {
    el.textContent = '';
  }
}

// Breadcrumbs Trail
function renderBreadcrumbs(hierarchy) {
  const container = document.getElementById('breadcrumbList');
  if (!container) return;

  if (!hierarchy || hierarchy.length === 0) {
    container.innerHTML = '';
    return;
  }

  container.innerHTML = hierarchy.map((node, index) => {
    let label = node.short_class;
    let icon = '⊞';
    if (index === 0) icon = '❖';
    else if (index === hierarchy.length - 1) icon = '◈';

    if (node.resource_id) {
      const shortId = node.resource_id.split('/').pop();
      label += `#${shortId}`;
    } else if (node.text) {
      label += ` "${node.text.slice(0, 12)}"`;
    }

    const activeClass = node.is_target ? 'active' : '';
    const arrow = index < hierarchy.length - 1 ? '<span class="breadcrumb-arrow">›</span>' : '';

    return `
      <div class="breadcrumb-chip ${activeClass}" onclick="selectBreadcrumbNode(${index})" title="${escapeHtml(node.class_name)}">
        <span style="opacity:0.6">${icon}</span>
        <span>${escapeHtml(label)}</span>
      </div>
      ${arrow}
    `;
  }).join('');
}

function selectBreadcrumbNode(index) {
  const node = currentHierarchy[index];
  if (!node) return;

  const centerX = Math.round((node.bounds.left + node.bounds.right) / 2);
  const centerY = Math.round((node.bounds.top + node.bounds.bottom) / 2);

  inspectAt(centerX, centerY, centerX, centerY);
}

// Action Segmented Control
function setActionType(action) {
  currentActionType = action;

  document.querySelectorAll('.action-segment').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.action === action);
  });

  renderSelectors();
}

function transformYamlForAction(rawYaml, actionType) {
  if (!rawYaml) return '';

  if (actionType === 'tap') return rawYaml;
  if (actionType === 'see') return rawYaml.replace(/^- tap:/g, '- see:');
  if (actionType === 'wait') return rawYaml.replace(/^- tap:/g, '- waitUntilVisible:');
  if (actionType === 'longPress') return rawYaml.replace(/^- tap:/g, '- longPress:');
  if (actionType === 'copyText') return rawYaml.replace(/^- tap:/g, '- copyTextFrom:');
  if (actionType === 'inputText') return `${rawYaml}\n- inputText: "example_text"`;

  return rawYaml;
}

// Tab Switching
function switchTab(tab) {
  activeTab = tab;

  document.getElementById('tabSelectorsBtn').classList.toggle('active', tab === 'selectors');
  document.getElementById('tabAttributesBtn').classList.toggle('active', tab === 'attributes');

  document.getElementById('selectorsView').style.display = tab === 'selectors' ? 'block' : 'none';
  document.getElementById('attributesView').style.display = tab === 'attributes' ? 'block' : 'none';
}

// Render Selectors Stack
function renderSelectors() {
  const list = document.getElementById('selectorsList');
  const countBadge = document.getElementById('selectorsCountBadge');
  if (!list) return;

  if (countBadge) countBadge.textContent = currentSelectors.length;

  if (!currentSelectors || currentSelectors.length === 0) {
    list.innerHTML = '<div class="studio-empty-state"><div class="empty-title">No selectors found</div></div>';
    return;
  }

  list.innerHTML = currentSelectors.map((s, i) => {
    const scoreClass = s.is_stable ? 'stable' : 'unstable';
    const baseYaml = s.yaml || s.value || '';
    const transformedYaml = transformYamlForAction(baseYaml, currentActionType);
    const displayValue = formatYamlHighlight(transformedYaml);

    let typePillClass = '';
    if (s.selector_type.includes('id')) typePillClass = 'id-type';
    else if (s.selector_type.includes('relative')) typePillClass = 'relative-type';

    return `
      <div class="selector-card ${scoreClass}">
        <div class="sel-topbar">
          <div class="sel-badge-group">
            <span class="sel-type-pill ${typePillClass}">${escapeHtml(s.selector_type)}</span>
          </div>
          <div class="sel-score-meter">
            <span class="score-dot"></span>
            <span>${s.score} pts</span>
          </div>
        </div>

        <div class="code-preview-frame">
          <div class="code-preview-top">
            <div class="code-dots">
              <span class="code-dot"></span>
              <span class="code-dot"></span>
              <span class="code-dot"></span>
            </div>
            <span class="code-lang-label">YAML</span>
          </div>
          <pre class="sel-value">${displayValue}</pre>
        </div>

        ${s.description ? `
          <div class="sel-desc-row">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"></polyline></svg>
            <span>${escapeHtml(s.description)}</span>
          </div>
        ` : ''}

        <div class="sel-btn-group">
          <button class="btn-card-secondary" onclick="copyToClipboard(${i})">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
            <span>Copy</span>
          </button>
          <button class="btn-card-primary" onclick="insertToEditor(${i})">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 10 20 15 15 20"></polyline><path d="M4 4v7a4 4 0 0 0 4 4h12"></path></svg>
            <span>Insert to Flow</span>
          </button>
        </div>
      </div>
    `;
  }).join('');
}

// Render Attributes Grid
function renderAttributes(attrs) {
  const grid = document.getElementById('attributesGrid');
  const countBadge = document.getElementById('attributesCountBadge');
  if (!grid) return;

  const entries = Object.entries(attrs || {});
  if (countBadge) countBadge.textContent = entries.length;

  if (entries.length === 0) {
    grid.innerHTML = '<div class="studio-empty-state"><div class="empty-title">No attributes available</div></div>';
    return;
  }

  grid.innerHTML = entries.map(([key, val]) => {
    const isBool = val === 'true' || val === 'false';
    const boolClass = val === 'true' ? 'bool-true' : (val === 'false' ? 'bool-false' : '');

    return `
      <div class="attr-card-row" data-key="${escapeHtml(key.toLowerCase())}" data-val="${escapeHtml(val.toLowerCase())}">
        <span class="attr-key">${escapeHtml(key)}</span>
        <span class="attr-val ${boolClass}">${escapeHtml(val)}</span>
        <button class="attr-copy-icon-btn" onclick="copyTextDirect('${escapeHtml(val)}')" title="Copy ${escapeHtml(key)}">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
        </button>
      </div>
    `;
  }).join('');
}

function filterAttributes() {
  const q = (document.getElementById('attrSearchInput').value || '').toLowerCase().trim();
  const rows = document.querySelectorAll('.attr-card-row');

  rows.forEach(row => {
    const key = row.dataset.key || '';
    const val = row.dataset.val || '';
    if (key.includes(q) || val.includes(q)) {
      row.style.display = 'flex';
    } else {
      row.style.display = 'none';
    }
  });
}

function copyTextDirect(text) {
  if (!text) return;
  navigator.clipboard.writeText(text).catch(() => {});
  if (window.parent && window.parent !== window) {
    window.parent.postMessage({ type: 'copySelector', value: text }, '*');
  }
  showToast(`Copied: ${text.slice(0, 24)}...`);
}

// Copy with Selection support
async function copyToClipboard(idx) {
  const selectedText = window.getSelection() ? window.getSelection().toString().trim() : '';
  const s = currentSelectors[idx];
  const baseYaml = s ? (s.yaml || s.value || '') : '';
  const transformed = transformYamlForAction(baseYaml, currentActionType);
  const text = selectedText || transformed;
  if (!text) return;

  let copied = false;
  if (navigator.clipboard && window.isSecureContext) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
    } catch (e) {}
  }

  if (!copied) {
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.top = '0';
      ta.style.left = '0';
      ta.style.width = '2em';
      ta.style.height = '2em';
      ta.style.padding = '0';
      ta.style.border = 'none';
      ta.style.outline = 'none';
      ta.style.boxShadow = 'none';
      ta.style.background = 'transparent';
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      copied = true;
    } catch (e) {}
  }

  if (window.parent && window.parent !== window) {
    window.parent.postMessage({
      type: 'copySelector',
      value: text
    }, '*');
  }

  showToast(selectedText ? 'Selection Copied' : 'YAML Copied to Clipboard');
}

// Global copy event listener: guarantees manual Cmd+C / Ctrl+C works in VS Code webview iframe
document.addEventListener('copy', (e) => {
  const selected = window.getSelection() ? window.getSelection().toString() : '';
  if (selected) {
    if (e.clipboardData) {
      e.clipboardData.setData('text/plain', selected);
    }
    if (window.parent && window.parent !== window) {
      window.parent.postMessage({
        type: 'copySelector',
        value: selected
      }, '*');
    }
  }
});

function insertToEditor(idx) {
  const s = currentSelectors[idx];
  if (!s || !window.parent) return;
  const baseYaml = s.yaml || s.value || '';
  const transformed = transformYamlForAction(baseYaml, currentActionType);

  window.parent.postMessage({
    type: 'insertSelector',
    value: transformed,
    selector: s
  }, '*');
  showToast('Inserted to VS Code Flow');
}

// App Selection & Search
async function loadPackages() {
  try {
    const res = await fetch('/api/packages');
    if (!res.ok) return;
    const data = await res.json();
    const rawList = data.packages || [];
    allApps = rawList.map(item => {
      if (item.includes('|')) {
        const parts = item.split('|').map(s => s.trim());
        const namePart = parts[0] || '';
        const isRunning = namePart.includes('[Running]');
        const name = namePart.replace('[Running]', '').trim();
        const bundleId = parts[1] || '';
        const path = parts[2] || '';
        return { name, bundleId, path, isRunning, raw: item };
      } else if (item.includes('(') && item.includes(')')) {
        const m = item.match(/^(.*?)\s*\((.*?)\)$/);
        if (m) {
          return { name: m[2], bundleId: m[1], path: '', isRunning: true, raw: item };
        }
      }
      return { name: item, bundleId: item, path: '', isRunning: false, raw: item };
    });
  } catch (e) {
    console.warn("Failed to load packages", e);
  }
}

function showAppDropdown() {
  filterApps();
  const dropdown = document.getElementById('appDropdown');
  if (dropdown) dropdown.style.display = 'block';
}

function filterApps() {
  const input = document.getElementById('appSearchInput');
  const dropdown = document.getElementById('appDropdown');
  const clearBtn = document.getElementById('clearAppBtn');
  if (!dropdown) return;
  const q = (input ? input.value : '').toLowerCase().trim();

  if (clearBtn) {
    clearBtn.style.display = q.length > 0 || selectedAppTarget ? 'block' : 'none';
  }

  const filtered = allApps.filter(app => {
    return app.name.toLowerCase().includes(q) ||
           app.bundleId.toLowerCase().includes(q) ||
           app.path.toLowerCase().includes(q);
  });

  if (filtered.length === 0) {
    dropdown.innerHTML = '<div style="padding:12px;color:var(--text-muted);font-size:11.5px;text-align:center;">No matching window or package found</div>';
    dropdown.style.display = 'block';
    return;
  }

  dropdown.innerHTML = filtered.slice(0, 50).map((app, idx) => `
    <div class="app-dropdown-item" onclick="selectApp(${allApps.indexOf(app)})">
      <div class="app-item-row">
        <div class="app-icon-box">
          ${app.path ? `<img class="app-icon-img" src="/api/app-icon?path=${encodeURIComponent(app.path)}" onerror="this.parentElement.innerHTML='💻'">` : '📱'}
        </div>
        <div class="app-item-info">
          <div class="app-name">
            <span>${escapeHtml(app.name)}</span>
            ${app.isRunning ? '<span class="app-badge-running">ACTIVE</span>' : ''}
          </div>
          <div class="app-detail">${escapeHtml(app.path || app.bundleId)}</div>
        </div>
      </div>
    </div>
  `).join('');
  dropdown.style.display = 'block';
}

async function selectApp(idx) {
  const input = document.getElementById('appSearchInput');
  const dropdown = document.getElementById('appDropdown');
  const clearBtn = document.getElementById('clearAppBtn');
  const statusText = document.getElementById('statusText');
  const app = allApps[idx];
  if (!app) return;

  const target = app.path || app.bundleId || app.name;
  selectedAppTarget = target;
  if (input) input.value = app.name;
  if (dropdown) dropdown.style.display = 'none';
  if (clearBtn) clearBtn.style.display = 'block';

  showToast(`Attached: ${app.name}`);
  if (statusText) statusText.textContent = `Target: ${app.name}`;

  try {
    await fetch('/api/target-app', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ app_id: target })
    });
    await capture();
  } catch (e) {
    console.error("Failed to set target app", e);
  }
}

async function clearAppSelection() {
  const input = document.getElementById('appSearchInput');
  const dropdown = document.getElementById('appDropdown');
  const clearBtn = document.getElementById('clearAppBtn');
  const statusText = document.getElementById('statusText');
  selectedAppTarget = null;
  if (input) input.value = '';
  if (dropdown) dropdown.style.display = 'none';
  if (clearBtn) clearBtn.style.display = 'none';

  showToast('Full Screen Mode');
  if (statusText) statusText.textContent = 'Full Screen';

  try {
    await fetch('/api/target-app', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ app_id: '' })
    });
    await capture();
  } catch (e) {
    console.error("Failed to clear target app", e);
  }
}

function enableHorizontalWheelScroll(elem) {
  if (!elem) return;
  elem.addEventListener('wheel', (e) => {
    if (e.deltaY !== 0 && !e.shiftKey) {
      e.preventDefault();
      elem.scrollLeft += e.deltaY;
    }
  }, { passive: false });
}

function initPanelResizer() {
  const resizer = document.getElementById('panelResizer');
  const screenPanel = document.getElementById('screenPanel');
  const appContainer = document.querySelector('.app-container');

  if (!resizer || !screenPanel || !appContainer) return;

  const isOneColumn = () => window.innerWidth <= 680;

  function applyLayoutDimensions() {
    if (isOneColumn()) {
      screenPanel.style.width = '';
      screenPanel.style.maxWidth = '';
      const savedHeight = localStorage.getItem('lumi_panel_height');
      if (savedHeight) {
        const h = parseInt(savedHeight, 10);
        if (h >= 160 && h <= window.innerHeight - 180) {
          screenPanel.style.flex = `0 0 ${h}px`;
          screenPanel.style.height = `${h}px`;
        }
      } else {
        screenPanel.style.flex = '';
        screenPanel.style.height = '';
      }
    } else {
      screenPanel.style.height = '';
      screenPanel.style.maxHeight = '';
      const savedWidth = localStorage.getItem('lumi_panel_width');
      if (savedWidth) {
        const w = parseInt(savedWidth, 10);
        if (w >= 220 && w <= window.innerWidth - 250) {
          screenPanel.style.flex = `0 0 ${w}px`;
          screenPanel.style.width = `${w}px`;
        }
      } else {
        screenPanel.style.flex = '';
        screenPanel.style.width = '';
      }
    }
    requestAnimationFrame(syncOverlaySize);
  }

  // Initial layout application
  applyLayoutDimensions();
  window.addEventListener('resize', applyLayoutDimensions);

  let isDragging = false;
  let activePointerId = null;

  resizer.addEventListener('pointerdown', (e) => {
    isDragging = true;
    activePointerId = e.pointerId;
    resizer.setPointerCapture(e.pointerId);
    resizer.classList.add('is-dragging');
    document.body.style.userSelect = 'none';
    e.preventDefault();
  });

  let rAF = null;

  resizer.addEventListener('pointermove', (e) => {
    if (!isDragging) return;
    const containerRect = appContainer.getBoundingClientRect();

    if (isOneColumn()) {
      // Vertical resizing (Height) in 1-column mode
      const minHeight = 160;
      const maxHeight = Math.max(minHeight, containerRect.height - 180);
      const newHeight = Math.max(minHeight, Math.min(maxHeight, e.clientY - containerRect.top));

      screenPanel.style.flex = `0 0 ${newHeight}px`;
      screenPanel.style.height = `${newHeight}px`;
      screenPanel.style.maxHeight = 'none';
    } else {
      // Horizontal resizing (Width) in 2-column mode
      const minWidth = 220;
      const maxWidth = Math.max(minWidth, containerRect.width - 250);
      const newWidth = Math.max(minWidth, Math.min(maxWidth, e.clientX - containerRect.left));

      screenPanel.style.flex = `0 0 ${newWidth}px`;
      screenPanel.style.width = `${newWidth}px`;
      screenPanel.style.maxWidth = 'none';
    }

    if (rAF) cancelAnimationFrame(rAF);
    rAF = requestAnimationFrame(() => {
      syncOverlaySize();
    });
  });

  const stopDragging = () => {
    if (isDragging) {
      isDragging = false;
      if (rAF) cancelAnimationFrame(rAF);
      if (activePointerId !== null) {
        try { resizer.releasePointerCapture(activePointerId); } catch (_) {}
        activePointerId = null;
      }
      resizer.classList.remove('is-dragging');
      document.body.style.userSelect = '';

      if (isOneColumn()) {
        const h = screenPanel.getBoundingClientRect().height;
        localStorage.setItem('lumi_panel_height', Math.round(h).toString());
      } else {
        const w = screenPanel.getBoundingClientRect().width;
        localStorage.setItem('lumi_panel_width', Math.round(w).toString());
      }
      requestAnimationFrame(syncOverlaySize);
    }
  };

  resizer.addEventListener('pointerup', stopDragging);
  resizer.addEventListener('pointercancel', stopDragging);
}

document.addEventListener('DOMContentLoaded', () => {
  initPanelResizer();

  const breadcrumbList = document.getElementById('breadcrumbList');
  const actionContainer = document.querySelector('.action-segment-container');
  if (breadcrumbList) enableHorizontalWheelScroll(breadcrumbList);
  if (actionContainer) enableHorizontalWheelScroll(actionContainer);

  const searchInput = document.getElementById('appSearchInput');
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const q = searchInput.value.toLowerCase().trim();
        if (!q) {
          clearAppSelection();
          return;
        }
        const match = allApps.find(a => 
          a.name.toLowerCase().includes(q) ||
          a.bundleId.toLowerCase().includes(q) ||
          a.path.toLowerCase().includes(q)
        );
        if (match) {
          selectApp(allApps.indexOf(match));
        }
      } else if (e.key === 'Escape') {
        const dropdown = document.getElementById('appDropdown');
        if (dropdown) dropdown.style.display = 'none';
      }
    });
  }
});

document.addEventListener('click', (e) => {
  const wrapper = document.querySelector('.app-search-wrapper');
  if (wrapper && !wrapper.contains(e.target)) {
    const dropdown = document.getElementById('appDropdown');
    if (dropdown) dropdown.style.display = 'none';
  }
});
