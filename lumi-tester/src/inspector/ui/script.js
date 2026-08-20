// State
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
let hoveredElement = null;

// Init
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

  // Mouse move for hover tooltip & element preview
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

  window.addEventListener('resize', syncOverlaySize);
});

// UI Feedback Toast
function showToast(msg) {
  const t = document.getElementById('toast');
  if (!t) return;
  t.textContent = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2000);
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
      // 1. Line starting with "- action:" e.g. "- tap:", "- see:", "- waitUntilVisible:"
      let l = line.replace(/^(\s*-\s+)([\w]+)(:)/, '$1<span class="yaml-cmd">$2</span>$3');
      // 2. Map keys e.g. "    id:", "    text:", "    type:", "    above:", "    below:", "    align:", "    offset:", "    point:", "    index:"
      l = l.replace(/^(\s*)([\w]+)(:)/, '$1<span class="yaml-key">$2</span>$3');
      // 3. String values in quotes e.g. &quot;...&quot;
      l = l.replace(/(:\s*)(&quot;.*?&quot;)/g, '$1<span class="yaml-str">$2</span>');
      // 4. Number values e.g. 19
      l = l.replace(/(:\s*)(\b\d+\b)/g, '$1<span class="yaml-num">$2</span>');
      // 5. Enum / boolean keywords e.g. right, left, top, bottom, center, true, false
      l = l.replace(/(:\s*)\b(true|false|right|left|top|bottom|center)\b/g, '$1<span class="yaml-enum">$2</span>');
      return l;
    })
    .join('\n');
}

function toggleBoxes() {
  showBoxes = !showBoxes;
  document.getElementById('boxToggle').classList.toggle('on', showBoxes);
  drawOverlay();
}

// Zoom & Pan Controls
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

  // Find smallest element at (x, y)
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
  const label = el.text ? `"${el.text.slice(0, 20)}"` : (el.resource_id ? `#${el.resource_id.split('/').pop()}` : '');
  const w = el.bounds ? el.bounds.right - el.bounds.left : 0;
  const h = el.bounds ? el.bounds.bottom - el.bounds.top : 0;

  tooltip.textContent = `${shortClass} ${label} (${w}×${h})`;
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
  const btn = document.querySelector('.btn-icon');
  if (btn) btn.disabled = true;
  document.getElementById('status').textContent = 'Capturing screen...';

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
    document.getElementById('status').textContent = `Ready • ${d.width} × ${d.height} px`;

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
    renderHeight = containerHeight;
    renderWidth = renderHeight * imageRatio;
    offsetX = (containerWidth - renderWidth) / 2;
  } else {
    renderWidth = containerWidth;
    renderHeight = renderWidth / imageRatio;
    offsetY = (containerHeight - renderHeight) / 2;
  }

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

function drawOverlay(highlightBounds = null) {
  const canvas = document.getElementById('overlay');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (showBoxes && cachedElements.length) {
    ctx.strokeStyle = 'rgba(88, 166, 255, 0.35)';
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

  // Highlight specific selected element with glowing bounding box
  if (highlightBounds) {
    const x = highlightBounds.left * scaleX;
    const y = highlightBounds.top * scaleY;
    const w = (highlightBounds.right - highlightBounds.left) * scaleX;
    const h = (highlightBounds.bottom - highlightBounds.top) * scaleY;

    ctx.strokeStyle = 'var(--accent, #58a6ff)';
    ctx.lineWidth = 2;
    ctx.strokeRect(x, y, w, h);

    ctx.fillStyle = 'rgba(88, 166, 255, 0.18)';
    ctx.fillRect(x, y, w, h);
  }
}

// Interaction: Inspect element at coordinate
async function inspectAt(x, y, clickX, clickY) {
  hideHoverTooltip();
  drawOverlay();

  // Visual tap indicator dot
  const ctx = document.getElementById('overlay').getContext('2d');
  ctx.fillStyle = 'rgba(255, 215, 0, 0.8)';
  ctx.beginPath();
  ctx.arc(clickX * scaleX, clickY * scaleY, 7, 0, 2 * Math.PI);
  ctx.fill();

  document.getElementById('status').textContent = `Inspecting (${x}, ${y})...`;

  try {
    const res = await fetch(`/api/element-at?x=${Math.round(x)}&y=${Math.round(y)}`);
    const data = await res.json();

    if (data.found) {
      currentSelectors = data.selectors || [];
      currentAttributes = data.attributes || {};
      currentHierarchy = data.hierarchy || [];

      // Highlight element bounds on canvas
      if (data.bounds) {
        drawOverlay(data.bounds);
      }

      const shortClass = data.element_class ? data.element_class.split('.').pop() : 'Element';
      const badge = document.getElementById('selectionBadge');
      badge.textContent = `${shortClass} (${data.bounds ? (data.bounds.right - data.bounds.left) + '×' + (data.bounds.bottom - data.bounds.top) : ''})`;
      badge.style.background = 'var(--green)';

      renderAppInfo(data.app_id);
      renderCommands(data.supported_commands);
      renderBreadcrumbs(currentHierarchy);
      renderSelectors();
      renderAttributes(currentAttributes);

      // Show action & tab bars
      document.getElementById('breadcrumbBar').style.display = currentHierarchy.length > 0 ? 'flex' : 'none';
      document.getElementById('actionBar').style.display = 'flex';
      document.getElementById('tabBar').style.display = 'flex';
      document.getElementById('emptyState').style.display = 'none';

      document.getElementById('status').textContent = `Selected ${shortClass} at (${x}, ${y})`;
    } else {
      currentSelectors = data.selectors || [];
      currentAttributes = {};
      currentHierarchy = [];

      document.getElementById('selectionBadge').textContent = 'No Selection';
      document.getElementById('selectionBadge').style.background = 'var(--muted)';
      clearDetails();
      document.getElementById('status').textContent = `No element at (${x}, ${y})`;
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
  document.getElementById('breadcrumbBar').style.display = 'none';
  document.getElementById('actionBar').style.display = 'none';
  document.getElementById('tabBar').style.display = 'none';
  document.getElementById('selectorsList').innerHTML = '';
  document.getElementById('attributesTableBody').innerHTML = '';
  document.getElementById('emptyState').style.display = 'block';
  drawOverlay();
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

// Breadcrumbs Hierarchy Navigation
function renderBreadcrumbs(hierarchy) {
  const container = document.getElementById('breadcrumbList');
  if (!container) return;

  if (!hierarchy || hierarchy.length === 0) {
    container.innerHTML = '';
    return;
  }

  container.innerHTML = hierarchy.map((node, index) => {
    let label = node.short_class;
    if (node.resource_id) {
      const shortId = node.resource_id.split('/').pop();
      label += `#${shortId}`;
    } else if (node.text) {
      label += ` "${node.text.slice(0, 15)}"`;
    }

    const activeClass = node.is_target ? 'active' : '';
    const sep = index < hierarchy.length - 1 ? '<span class="breadcrumb-sep">›</span>' : '';

    return `
      <div class="breadcrumb-item ${activeClass}" onclick="selectBreadcrumbNode(${index})" title="${escapeHtml(node.class_name)}">
        <span>${escapeHtml(label)}</span>
      </div>
      ${sep}
    `;
  }).join('');
}

function selectBreadcrumbNode(index) {
  const node = currentHierarchy[index];
  if (!node) return;

  // Calculate center of this ancestor node and inspect it
  const centerX = Math.round((node.bounds.left + node.bounds.right) / 2);
  const centerY = Math.round((node.bounds.top + node.bounds.bottom) / 2);

  inspectAt(centerX, centerY, centerX, centerY);
}

// Action Switcher Pills
function setActionType(action) {
  currentActionType = action;

  // Update pill styles
  document.querySelectorAll('.action-pill').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.action === action);
  });

  // Re-render selectors with new action type
  renderSelectors();
}

function transformYamlForAction(rawYaml, actionType) {
  if (!rawYaml) return '';

  // If action is 'tap', keep original
  if (actionType === 'tap') {
    return rawYaml;
  }

  // Handle shorthand or map-based commands
  if (actionType === 'see') {
    return rawYaml.replace(/^- tap:/g, '- see:');
  }

  if (actionType === 'wait') {
    return rawYaml.replace(/^- tap:/g, '- waitUntilVisible:');
  }

  if (actionType === 'longPress') {
    return rawYaml.replace(/^- tap:/g, '- longPress:');
  }

  if (actionType === 'copyText') {
    return rawYaml.replace(/^- tap:/g, '- copyTextFrom:');
  }

  if (actionType === 'inputText') {
    // Tap field first, then input text
    return `${rawYaml}\n- inputText: "example_text"`;
  }

  return rawYaml;
}

// Tab Switching (Selectors vs Attributes)
function switchTab(tab) {
  activeTab = tab;

  document.getElementById('tabSelectorsBtn').classList.toggle('active', tab === 'selectors');
  document.getElementById('tabAttributesBtn').classList.toggle('active', tab === 'attributes');

  document.getElementById('selectorsView').style.display = tab === 'selectors' ? 'block' : 'none';
  document.getElementById('attributesView').style.display = tab === 'attributes' ? 'block' : 'none';
}

// Render Selectors Tab
function renderSelectors() {
  const list = document.getElementById('selectorsList');
  const countBadge = document.getElementById('selectorsCountBadge');
  if (!list) return;

  if (countBadge) countBadge.textContent = currentSelectors.length;

  if (!currentSelectors || currentSelectors.length === 0) {
    list.innerHTML = '<div class="empty-state">No selectors available for this element.</div>';
    return;
  }

  list.innerHTML = currentSelectors.map((s, i) => {
    const scoreClass = s.is_stable ? 'stable' : 'unstable';
    const baseYaml = s.yaml || s.value || '';
    const transformedYaml = transformYamlForAction(baseYaml, currentActionType);
    const displayValue = formatYamlHighlight(transformedYaml);

    return `
      <div class="selector-card ${scoreClass}">
        <div class="sel-header">
          <span class="sel-type">${escapeHtml(s.selector_type)}</span>
          <span class="sel-score">${s.score} pts</span>
        </div>
        <pre class="sel-value">${displayValue}</pre>
        ${s.description ? `<div class="sel-desc">${escapeHtml(s.description)}</div>` : ''}
        <div class="sel-actions">
          <button class="btn btn-outline btn-sm" onclick="copyToClipboard(${i})">
            <span>📋</span> Copy
          </button>
          <button class="btn btn-primary btn-sm" onclick="insertToEditor(${i})">
            <span>↳</span> Insert
          </button>
        </div>
      </div>
    `;
  }).join('');
}

// Render Attributes Tab
function renderAttributes(attrs) {
  const tbody = document.getElementById('attributesTableBody');
  const countBadge = document.getElementById('attributesCountBadge');
  if (!tbody) return;

  const entries = Object.entries(attrs || {});
  if (countBadge) countBadge.textContent = entries.length;

  if (entries.length === 0) {
    tbody.innerHTML = '<tr><td colspan="3" style="text-align:center;color:var(--muted);padding:16px;">No attributes available</td></tr>';
    return;
  }

  tbody.innerHTML = entries.map(([key, val]) => `
    <tr class="attr-row" data-key="${escapeHtml(key.toLowerCase())}" data-val="${escapeHtml(val.toLowerCase())}">
      <td class="attr-name">${escapeHtml(key)}</td>
      <td class="attr-val">${escapeHtml(val)}</td>
      <td>
        <button class="attr-copy-btn" onclick="copyTextDirect('${escapeHtml(val)}')" title="Copy ${escapeHtml(key)}">📋</button>
      </td>
    </tr>
  `).join('');
}

function filterAttributes() {
  const q = (document.getElementById('attrSearchInput').value || '').toLowerCase().trim();
  const rows = document.querySelectorAll('.attr-row');

  rows.forEach(row => {
    const key = row.dataset.key || '';
    const val = row.dataset.val || '';
    if (key.includes(q) || val.includes(q)) {
      row.style.display = '';
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
  showToast(`Copied: ${text.slice(0, 25)}...`);
}

// Copy to Clipboard (with selection support)
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

  showToast(selectedText ? 'Copied Selection' : 'Copied to Clipboard');
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
  showToast('Inserted to VS Code');
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
    dropdown.innerHTML = '<div style="padding:10px;color:var(--muted);font-size:12px">No matching application found</div>';
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
            ${app.isRunning ? '<span class="app-badge-running">Running</span>' : ''}
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
  const app = allApps[idx];
  if (!app) return;

  const target = app.path || app.bundleId || app.name;
  selectedAppTarget = target;
  if (input) input.value = app.name;
  if (dropdown) dropdown.style.display = 'none';
  if (clearBtn) clearBtn.style.display = 'block';

  showToast(`Selected: ${app.name}`);
  document.getElementById('status').textContent = `Target: ${app.name} (capturing...)`;

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
  selectedAppTarget = null;
  if (input) input.value = '';
  if (dropdown) dropdown.style.display = 'none';
  if (clearBtn) clearBtn.style.display = 'none';

  showToast('Full Screen Mode');
  document.getElementById('status').textContent = 'Full Screen (capturing...)';

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

document.addEventListener('DOMContentLoaded', () => {
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
