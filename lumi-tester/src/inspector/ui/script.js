// State
let fileOk = false;
let selectors = [];
let clickX = 0, clickY = 0;
let selectedIdx = -1;
let insertMode = false;
let insertAt = -1;
let showBoxes = true;
let cachedElements = [];
let packages = [];
let pendingOpenAction = null;
let confirmResolver = null;

// Utils
function toast(msg) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.classList.add('show');
  setTimeout(() => t.classList.remove('show'), 2000);
}

function setStatus(msg) {
  document.getElementById('status').textContent = msg;
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// Toggle boxes
function toggleBoxes() {
  showBoxes = !showBoxes;
  document.getElementById('boxToggle').classList.toggle('on', showBoxes);
  drawOverlay();
}

// Capture
async function capture() {
  const btn = document.getElementById('captureBtn');
  btn.disabled = true;
  setStatus('Capturing...');

  const skip = !showBoxes;

  try {
    const r = await fetch(`/api/screenshot?skip_hierarchy=${skip}`);
    if (!r.ok) throw new Error('Failed');
    const d = await r.json();

    document.getElementById('placeholder').style.display = 'none';
    const img = document.getElementById('screen');

    const imageLoaded = new Promise((resolve) => { img.onload = resolve; });
    img.src = 'data:image/jpeg;base64,' + d.data;
    img.style.display = 'block';

    await imageLoaded;

    if (!skip) {
      try {
        const h = await fetch('/api/hierarchy');
        const hd = await h.json();
        cachedElements = hd.elements || [];
      } catch (e) { cachedElements = []; }
    } else {
      cachedElements = [];
    }

    syncOverlaySize();
    drawOverlay();

    setStatus(`Captured ${d.width}×${d.height}`);
  } catch (e) {
    console.error(e);
    setStatus('Capture failed');
  }

  btn.disabled = false;
}

function syncOverlaySize() {
  const canvas = document.getElementById('overlay');
  const img = document.getElementById('screen');

  if (img.style.display === 'none') return;

  // Use offsetWidth/Height for rendered size
  canvas.width = img.offsetWidth;
  canvas.height = img.offsetHeight;

  // Position canvas exactly over image
  canvas.style.width = img.offsetWidth + 'px';
  canvas.style.height = img.offsetHeight + 'px';
}

function drawOverlay() {
  const canvas = document.getElementById('overlay');
  const img = document.getElementById('screen');
  const ctx = canvas.getContext('2d');

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  if (!showBoxes || !cachedElements.length || img.style.display === 'none') return;
  if (!img.naturalWidth || !img.naturalHeight) return;

  const scaleX = canvas.width / img.naturalWidth;
  const scaleY = canvas.height / img.naturalHeight;

  ctx.strokeStyle = 'rgba(88, 166, 255, 0.5)';
  ctx.lineWidth = 1;

  cachedElements.forEach(el => {
    if (el.bounds) {
      const x = el.bounds.left * scaleX;
      const y = el.bounds.top * scaleY;
      const w = (el.bounds.right - el.bounds.left) * scaleX;
      const h = (el.bounds.bottom - el.bounds.top) * scaleY;
      if (w > 3 && h > 3) {
        ctx.strokeRect(x, y, w, h);
      }
    }
  });
}

// Redraw overlay on window resize
window.addEventListener('resize', () => {
  syncOverlaySize();
  drawOverlay();
});

// Right-click handler
document.getElementById('screen').addEventListener('contextmenu', async (e) => {
  e.preventDefault();

  const img = e.target;
  const rect = img.getBoundingClientRect();
  clickX = Math.round((e.clientX - rect.left) * img.naturalWidth / rect.width);
  clickY = Math.round((e.clientY - rect.top) * img.naturalHeight / rect.height);

  // Calculate menu position immediately to keep in viewport
  const menu = document.getElementById('menu');
  let left = e.clientX;
  let top = e.clientY;

  if (left + 250 > window.innerWidth) left = window.innerWidth - 260;
  if (top + 300 > window.innerHeight) top = window.innerHeight - 310;

  menu.style.left = left + 'px';
  menu.style.top = top + 'px';

  menu.innerHTML = '<div class="menu-item">Loading...</div>';
  menu.classList.add('show');

  setStatus(`Inspecting ${clickX}, ${clickY}...`);

  try {
    const r = await fetch(`/api/element-at?x=${clickX}&y=${clickY}`);
    const d = await r.json();
    selectors = d.selectors || [];

    renderSelectors();
    renderMenu();
    setStatus(`Found ${selectors.length} selectors`);
  } catch (e) {
    selectors = [];
    menu.classList.remove('show');
    setStatus('Inspection failed');
  }
});

function renderSelectors() {
  const el = document.getElementById('sels');
  if (!selectors.length) {
    el.innerHTML = '<div style="color:var(--muted);font-size:10px">No element found at this position</div>';
    return;
  }

  el.innerHTML = selectors.map((s, i) => `
        <div class="sel-item ${s.is_stable ? 'good' : 'warn'}" onclick="copySelector(${i})">
            <span>${s.selector_type}: ${escapeHtml(s.value.substring(0, 35))}${s.value.length > 35 ? '...' : ''}</span>
            <span style="color:var(--muted)">${s.score}</span>
        </div>
    `).join('');
}

function copySelector(i) {
  navigator.clipboard.writeText(selectors[i].yaml);
  toast('Copied to clipboard');
}

// Close menu on click outside
document.addEventListener('click', (e) => {
  if (!e.target.closest('.menu')) {
    document.getElementById('menu').classList.remove('show');
  }
});

function renderMenu() {
  const menu = document.getElementById('menu');
  let html = '';

  // 1. Selector Actions (with submenus)
  if (selectors.length) {
    html += `<div class="menu-head">Context Actions</div>`;
    html += createSelectorMenu('tap', 'Tap', '👆');
    html += createSelectorMenu('longPress', 'Long Press', '✋');
    html += createSelectorMenu('doubleTap', 'Double Tap', '👆');
    html += `<div class="menu-div"></div>`;
    html += createSelectorMenu('see', 'Assert Visible', '👁️');
    html += createSelectorMenu('notSee', 'Assert Not Visible', '🚫');
    html += `<div class="menu-div"></div>`;
  }

  // 2. Input
  html += `<div class="menu-head">Input</div>`;
  html += `<div class="menu-item" onclick="handleAction('inputText')">
            <span class="emoji">⌨️</span> Input Text...
        </div>`;
  html += `<div class="menu-div"></div>`;

  // 3. App Control
  html += `<div class="menu-head">App Control</div>`;
  html += `<div class="menu-item" onclick="handleAction('open')">
            <span class="emoji">📱</span> Open App (Package ID)...
        </div>`;
  html += `<div class="menu-item" onclick="handleAction('openClear')">
            <span class="emoji">🔄</span> Open App (Clear State)...
        </div>`;
  html += `<div class="menu-div"></div>`;

  // 4. Navigation
  html += `<div class="menu-head">Navigation</div>`;
  html += menuItem('back', 'Back');
  html += menuItem('swipeUp', 'Swipe Up');
  html += menuItem('swipeDown', 'Swipe Down');
  html += menuItem('swipeLeft', 'Swipe Left');
  html += menuItem('swipeRight', 'Swipe Right');
  html += `<div class="menu-div"></div>`;
  html += menuItem('wait', 'Wait...');
  html += menuItem('hideKeyboard', 'Hide Keyboard');

  menu.innerHTML = html;
}

function createSelectorMenu(action, label, icon) {
  const opts = selectors.map((s, i) => {
    const val = s.value.length > 30 ? s.value.substring(0, 28) + '...' : s.value;
    return `
             <div class="submenu-item" onclick="event.stopPropagation(); handleAction('${action}', ${i})">
                <span class="submenu-type">${s.selector_type}</span>
                <span>${escapeHtml(val)}</span>
             </div>`;
  }).join('');

  return `
        <div class="menu-item-group">
            <div class="menu-head-item" onclick="toggleSubmenu(this)">
                <span class="emoji">${icon}</span> ${label}
                <span class="arrow">▶</span>
            </div>
            <div class="submenu-container">
                ${opts}
            </div>
        </div>`;
}

function toggleSubmenu(el) {
  el.classList.toggle('open');
  const container = el.nextElementSibling;
  if (container) container.classList.toggle('open');
}

function menuItem(action, label) {
  let icon = '';
  switch (action) {
    case 'back': icon = '⬅️'; break;
    case 'swipeUp': icon = '⬆️'; break;
    case 'swipeDown': icon = '⬇️'; break;
    case 'swipeLeft': icon = '⬅️'; break;
    case 'swipeRight': icon = '➡️'; break;
    case 'wait': icon = '⏱️'; break;
    case 'hideKeyboard': icon = '⌨️'; break;
  }
  return `<div class="menu-item" onclick="handleAction('${action}')"><span class="emoji">${icon}</span> ${label}</div>`;
}

async function handleAction(action, selectorIdx) {
  document.getElementById('menu').classList.remove('show');

  switch (action) {
    case 'tap':
    case 'longPress':
    case 'doubleTap':
    case 'see':
    case 'notSee':
      if (selectorIdx === undefined) selectorIdx = 0;
      if (!selectors[selectorIdx]) return;
      await executeAndAdd(action, selectors[selectorIdx]);
      break;

    case 'open':
    case 'openClear':
      pendingOpenAction = action;
      showPkgDialog();
      break;

    case 'inputText':
      const text = prompt('Text to input:');
      if (!text) return;
      await addCommand({ type: 'inputText', text });
      if (selectors.length) {
        await execute('tap', clickX, clickY);
        await execute('inputText', clickX, clickY, text);
      }
      break;

    case 'wait':
      const ms = prompt('Wait duration (ms):', '1000');
      if (!ms) return;
      await addCommand({ type: 'wait', ms: parseInt(ms) });
      break;

    default:
      await addCommand({ type: action });
      break;
  }
}

// Confirm Dialog Logic
function confirmDialog(msg, title = 'Confirm') {
  document.getElementById('confirmMsg').textContent = msg;
  document.getElementById('confirmTitle').textContent = title;
  document.getElementById('confirmDialog').style.display = 'flex';
  return new Promise(resolve => {
    confirmResolver = resolve;
  });
}

function resolveConfirm(result) {
  document.getElementById('confirmDialog').style.display = 'none';
  if (confirmResolver) confirmResolver(result);
}

async function deleteCmd(idx) {
  const ok = await confirmDialog('Are you sure you want to delete this command?', 'Delete Command');
  if (!ok) return;

  try {
    await fetch('/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'delete', index: idx })
    });
    loadCommands();
  } catch (e) {
    console.error(e);
    toast('Failed to delete');
  }
}

// Package Dialog
async function showPkgDialog() {
  document.getElementById('pkgDialog').style.display = 'flex';
  document.getElementById('pkgList').innerHTML = 'Loading packages...';

  try {
    const r = await fetch('/api/packages');
    const d = await r.json();
    packages = d.packages || [];
    filterPackages();
  } catch (e) {
    document.getElementById('pkgList').innerHTML = 'Failed to load packages';
  }
  document.getElementById('pkgSearch').focus();
}

function closePkgDialog() {
  document.getElementById('pkgDialog').style.display = 'none';
  pendingOpenAction = null;
}

function filterPackages() {
  const q = document.getElementById('pkgSearch').value.toLowerCase();
  const list = document.getElementById('pkgList');
  const filtered = packages.filter(p => p.toLowerCase().includes(q));

  list.innerHTML = filtered.map(p =>
    `<div class="pkg-item" onclick="selectPackage('${p}')">${p}</div>`
  ).join('');
}

async function selectPackage(pkg) {
  closePkgDialog();
  if (!pendingOpenAction) return;

  const clearState = pendingOpenAction === 'openClear';
  await addCommand({ type: 'open', app: pkg, clearState });
  toast(`Added open ${pkg}`);
}

// Command Management
async function executeAndAdd(action, selector) {
  await execute(action, clickX, clickY);
  await addCommand({
    type: action,
    selector_type: selector.selector_type,
    value: selector.value
  });
  if (['tap', 'longPress', 'doubleTap'].includes(action)) {
    setTimeout(capture, 600);
  }
}

async function execute(action, x, y, text) {
  try {
    await fetch('/api/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action, x, y, text, selector: selectors[0] || null })
    });
  } catch (e) {
    console.error('Execute failed:', e);
  }
}

async function addCommand(cmd) {
  if (!fileOk) {
    toast('Open a file first');
    return;
  }
  try {
    await fetch('/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        action: 'insert',
        index: insertMode && insertAt >= 0 ? insertAt : undefined,
        command: cmd
      })
    });
    toast('✓ Added');
    loadCommands();
    if (insertMode) {
      insertMode = false;
      insertAt = -1;
    }
  } catch (e) {
    toast('Failed to add command');
  }
}

async function openFile() {
  const path = document.getElementById('file').value;
  try {
    const r = await fetch('/api/file', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path })
    });
    if (r.ok) {
      fileOk = true;
      loadCommands();
    } else {
      toast('File not found');
    }
  } catch (e) {
    toast('Error opening file');
  }
}

async function newFile() {
  const path = document.getElementById('file').value;
  // For now assume server handles it or file exists
  // In real scenario we might need 'create' flag
  openFile();
}

async function loadCommands() {
  try {
    const r = await fetch('/api/file/commands');
    const d = await r.json();
    if (d.success && d.commands) {
      renderCommands(d.commands);
    } else {
      toast('Sync failed: ' + (d.message || 'Unknown'));
    }
  } catch (e) {
    console.error(e);
    toast('Sync error');
  }
}

// Commands Render
function renderCommands(cmds) {
  const el = document.getElementById('cmds');
  if (!el) return;

  if (!cmds || !cmds.length) {
    el.innerHTML = '<div class="placeholder" style="padding:20px;text-align:center"><div style="font-size:10px;color:var(--muted)">No commands found</div></div>';
    return;
  }

  el.innerHTML = cmds.map((cmd, i) => {
    const typeMatch = cmd.match(/^- (\w+):/);
    const type = typeMatch ? typeMatch[1] : 'unknown';
    const body = cmd.replace(/^- \w+:\s*/, '').trim();

    return `
        <div class="cmd ${i === selectedIdx ? 'selected' : ''}">
            <button class="cmd-del" onclick="event.stopPropagation(); deleteCmd(${i})">×</button>
            <div class="cmd-header" onclick="selectCmd(${i})">
                <span class="cmd-idx">${i + 1}</span>
                <button class="cmd-play" onclick="event.stopPropagation(); playCmd(${i})">▶</button>
                <span class="cmd-type">${type}</span>
            </div>
            <div class="cmd-body">${escapeHtml(body)}</div>
        </div>
        `;
  }).join('');

  // Auto-scroll to bottom if at end
  el.scrollTop = el.scrollHeight;
}

function selectCmd(i) {
  selectedIdx = i;
  const els = document.querySelectorAll('.cmd');
  els.forEach((e, idx) => e.classList.toggle('selected', idx === i));
}

function setInsertMode() {
  if (selectedIdx === -1) {
    toast('Select a command first to insert after/before');
    return;
  }
  insertMode = true;
  insertAt = selectedIdx + 1; // Insert after by default
  toast(`Mode: Insert after #${selectedIdx + 1}`);
}

async function playCmd(i) {
  try {
    const r = await fetch(`/api/play-command/${i}`, { method: 'POST' });
    if (r.ok) {
      toast(`Played #${i + 1}`);
      setTimeout(capture, 800);
    } else {
      toast('Failed to play');
    }
  } catch (e) {
    toast('Error');
  }
}

// Auto Init
window.addEventListener('DOMContentLoaded', () => {
  const fileInput = document.getElementById('file');
  if (fileInput.value) {
    openFile();
  }
  capture();
});
