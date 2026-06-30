(() => {
  "use strict";

  const WARP = [500, 500];
  const TEMPLATES = [
    { label: "Công tắc 1 nút", rows: 1, cols: 1 },
    { label: "Công tắc 2 nút", rows: 1, cols: 2 },
    { label: "Công tắc 3 nút", rows: 1, cols: 3 },
    { label: "Công tắc 4 nút (2×2)", rows: 2, cols: 2 },
    { label: "Công tắc 4 nút (1×4)", rows: 1, cols: 4 },
    { label: "Công tắc 6 nút (2×3)", rows: 2, cols: 3 },
    { label: "Công tắc 8 nút (2×4)", rows: 2, cols: 4 },
    { label: "Công tắc 10 nút (2×5)", rows: 2, cols: 5 },
    { label: "Ổ cắm đôi (1×2)", rows: 1, cols: 2 },
    { label: "Cảm biến (1 LED)", rows: 1, cols: 1, regionPrefix: "status" },
    { label: "Home Controller (hàng LED)", rows: 1, cols: 5, regionPrefix: "status" },
    { label: "Tùy chỉnh…", custom: true },
  ];
  const STATE_PRESETS = ["WHITE", "YELLOW", "RED", "BLUE", "GREEN", "PINK", "OFF"];

  const $ = (id) => document.getElementById(id);
  const srcCanvas = $("src");
  const ctx = srcCanvas.getContext("2d");
  const warpCanvas = $("warp");
  const wctx = warpCanvas.getContext("2d");
  const statusEl = $("status");

  let info = null;
  let corners = []; // [[x,y], ...] in source pixels, order TL,TR,BR,BL
  let layout = { type: "grid", rows: 2, cols: 2, cell_fill: 0.6 };
  let regionPrefix = "button";
  let buttons = [];
  let states = []; // legacy/global fallback rules
  let stateModels = {}; // {"button_1.ON": {hsv:[...]}}, preferred
  let labCameras = [];
  let labDevices = [];
  let activeCameraId = "camera_1";
  let activeDeviceId = "switch_4gang";
  let dragIdx = -1;
  let verifiedProfileKey = null;

  let currentFrame = null;
  let frameLoaded = false;

  function setStatus(text, cls) {
    statusEl.textContent = text;
    statusEl.className = "status" + (cls ? " " + cls : "");
  }

  function slugify(value, fallback) {
    const out = String(value || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");
    return out || fallback;
  }

  function activeDevice() {
    return labDevices.find((device) => device.id === activeDeviceId) || labDevices[0] || null;
  }

  function activeCamera() {
    return labCameras.find((camera) => camera.id === activeCameraId) || labCameras[0] || null;
  }

  function qualifiedRegionId(button) {
    const deviceId = activeDeviceId || button.deviceId || button.device_id || "device";
    const regionId = button.id || slugify(button.label, "region");
    return `${deviceId}.${regionId}`;
  }

  function displayDeviceLabel(device) {
    return (device && (device.label || device.id)) || "Thiết bị";
  }

  // ── Grid → button ROIs (mirrors profile::grid_buttons) ──────────────
  function gridButtons(rows, cols, fill) {
    const [W, H] = WARP;
    const cw = W / cols,
      ch = H / rows;
    const f = Math.min(Math.max(fill, 0.1), 1);
    const out = [];
    let idx = 1;
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        const cx = (c + 0.5) * cw,
          cy = (r + 0.5) * ch;
        const rw = cw * f,
          rh = ch * f;
        const id =
          regionPrefix === "status" && idx === 1
            ? "status"
            : regionPrefix + "_" + idx;
        out.push({
          deviceId: activeDeviceId,
          id,
          label: regionPrefix === "button" ? "Nút " + idx : "LED " + idx,
          kind: regionPrefix === "button" ? "button_led" : "status_led",
          roi: [
            Math.max(0, Math.round(cx - rw / 2)),
            Math.max(0, Math.round(cy - rh / 2)),
            Math.round(rw),
            Math.round(rh),
          ],
          mask: "ellipse",
          allowedStates:
            regionPrefix === "button"
              ? ["ON", "OFF"]
              : ["RED", "GREEN", "PINK", "YELLOW", "WHITE", "OFF"],
        });
        idx++;
      }
    }
    return out;
  }

  function expandedSearchRoi(roi) {
    const [x, y, w, h] = roi || [0, 0, 1, 1];
    const side = Math.max(80, Math.min(180, Math.round(Math.max(w, h) * 3), WARP[0], WARP[1]));
    const cx = x + w / 2;
    const cy = y + h / 2;
    const sx = Math.max(0, Math.min(WARP[0] - side, Math.round(cx - side / 2)));
    const sy = Math.max(0, Math.min(WARP[1] - side, Math.round(cy - side / 2)));
    return [
      sx,
      sy,
      side,
      side,
    ];
  }

  function regenButtons() {
    if (layout.type === "grid") {
      buttons = gridButtons(layout.rows, layout.cols, layout.cell_fill || 0.6);
    }
    buttons.forEach((button) => (button.deviceId = activeDeviceId));
    updateRegionSelect();
    syncActiveDeviceFromUi();
    renderDeviceList();
    updateQuality();
  }

  function buildProfile() {
    const profileButtons = buttons.map((button) => {
      const [x, y, w, h] = button.roi;
      const minSide = Math.max(1, Math.min(w, h));
      return {
        id: button.id,
        deviceId: activeDeviceId,
        label: button.label,
        kind: button.kind,
        roi: button.roi,
        searchRoi: button.searchRoi || button.search_roi || expandedSearchRoi(button.roi),
        mask: button.mask,
        allowedStates: button.allowedStates || button.allowed_states || [],
        expectedCenter: button.expectedCenter || [
          Math.round(x + w / 2),
          Math.round(y + h / 2),
        ],
        maxCenterDrift: button.maxCenterDrift || Math.max(6, Math.round(minSide * 0.35)),
      };
    });
    return {
      name: $("name").value.trim() || null,
      camera: { rtsp: null, transport: null },
      activeCameraId,
      activeDeviceId,
      lab: buildLabProfile(),
      geometry: { corners: corners.map((c) => [c[0], c[1]]), warp: WARP },
      layout: layout,
      buttons: profileButtons,
      states: states.map(normalizeStateRule),
      stateModels: stateModels,
      min_ratio: 0.05,
      min_margin: 0.05,
    };
  }

  function buildLabProfile() {
    syncActiveDeviceFromUi();
    return {
      name: $("name").value.trim() || "lumi_lab",
      activeCameraId,
      activeDeviceId,
      cameras: labCameras.map((camera) => ({
        id: camera.id,
        label: camera.label,
        rtsp: null,
        transport: null,
      })),
      devices: labDevices.map((device) => ({
        id: device.id,
        label: device.label,
        cameraId: device.cameraId || activeCameraId,
        kind: device.kind || "switch",
        regions: (device.regions || []).map((button) => ({
          ...normalizeButton(button),
          deviceId: device.id,
        })),
      })),
    };
  }

  function profileKey() {
    return JSON.stringify(buildProfile());
  }

  function normalizeButton(button) {
    return {
      id: button.id,
      deviceId: button.deviceId || button.device_id,
      label: button.label,
      kind: button.kind,
      roi: button.roi,
      searchRoi: button.searchRoi || button.search_roi,
      mask: button.mask || "ellipse",
      allowedStates: button.allowedStates || button.allowed_states || [],
      expectedCenter: button.expectedCenter || button.expected_center,
      maxCenterDrift: button.maxCenterDrift || button.max_center_drift,
    };
  }

  function normalizeStateRule(rule) {
    const out = {
      name: rule.name,
      hsv: rule.hsv || [],
      source: rule.source,
    };
    const type = rule.type || rule.rule_type;
    const darkMaxV = rule.darkMaxV ?? rule.dark_max_v;
    const whiteMaxS = rule.whiteMaxS ?? rule.white_max_s;
    const whiteMinV = rule.whiteMinV ?? rule.white_min_v;
    if (type) out.type = type;
    if (darkMaxV != null) out.darkMaxV = darkMaxV;
    if (whiteMaxS != null) out.whiteMaxS = whiteMaxS;
    if (whiteMinV != null) out.whiteMinV = whiteMinV;
    return out;
  }

  function clearVerify() {
    verifiedProfileKey = null;
    const out = $("verify-result");
    if (out) out.classList.add("hidden");
    updateQuality();
  }

  function updateRegionSelect() {
    const sel = $("learn-region");
    if (!sel) return;
    const current = sel.value;
    sel.innerHTML = "";
    const all = document.createElement("option");
    all.value = "__all";
    all.textContent = "Tất cả vùng";
    sel.appendChild(all);
    buttons.forEach((button, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = `${displayDeviceLabel(activeDevice())} · ${button.label}`;
      sel.appendChild(option);
    });
    if ([...sel.options].some((option) => option.value === current)) {
      sel.value = current;
    } else if (buttons.length > 0) {
      sel.value = "0";
    }
    updateRegionEditor();
    updateTargetPreview();
    updateQuality();
  }

  function syncActiveDeviceFromUi() {
    let device = activeDevice();
    if (!device) return;
    const idInput = $("device-id");
    const labelInput = $("device-label");
    if (idInput) {
      const nextId = slugify(idInput.value, device.id);
      if (nextId && nextId !== device.id) {
        const oldId = device.id;
        device.id = nextId;
        activeDeviceId = nextId;
        buttons.forEach((button) => (button.deviceId = nextId));
        const renamed = {};
        Object.entries(stateModels).forEach(([key, rule]) => {
          renamed[key.startsWith(oldId + ".") ? nextId + key.slice(oldId.length) : key] = rule;
        });
        stateModels = renamed;
      }
    }
    if (labelInput && labelInput.value.trim()) {
      device.label = labelInput.value.trim();
    }
    device.cameraId = activeCameraId;
    device.layout = layout;
    device.regions = buttons.map((button) => ({ ...normalizeButton(button), deviceId: device.id }));
  }

  function applyActiveDeviceToUi() {
    const device = activeDevice();
    if (!device) return;
    activeDeviceId = device.id;
    activeCameraId = device.cameraId || activeCameraId;
    if ($("device-id")) $("device-id").value = device.id;
    if ($("device-label")) $("device-label").value = device.label || device.id;
    if ($("active-device-title")) $("active-device-title").textContent = displayDeviceLabel(device);
    if (device.layout) layout = device.layout;
    if (device.regions && device.regions.length) {
      buttons = device.regions.map(normalizeButton).map((button) => ({ ...button, deviceId: device.id }));
    } else {
      regenButtons();
      return;
    }
    updateRegionSelect();
    renderDeviceList();
    updateTargetPreview();
    drawWarpOverlay();
  }

  function renderDeviceList() {
    const list = $("device-list");
    if (!list) return;
    list.innerHTML = "";
    labDevices.forEach((device) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "device-item" + (device.id === activeDeviceId ? " active" : "");
      const regions = (device.regions || []).length || buttons.length;
      btn.innerHTML = `<strong>${device.label || device.id}</strong><span>${device.id} · ${regions} LED</span>`;
      btn.addEventListener("click", () => {
        syncActiveDeviceFromUi();
        activeDeviceId = device.id;
        applyActiveDeviceToUi();
        clearVerify();
      });
      list.appendChild(btn);
    });
    const summary = $("lab-summary");
    if (summary) summary.textContent = `${labCameras.length || 1} camera · ${labDevices.length || 1} thiết bị`;
  }

  function renderCameraSelect() {
    const sel = $("camera-select");
    if (!sel) return;
    sel.innerHTML = "";
    labCameras.forEach((camera) => {
      const option = document.createElement("option");
      option.value = camera.id;
      option.textContent = camera.label || camera.id;
      sel.appendChild(option);
    });
    sel.value = activeCameraId;
    sel.onchange = () => {
      activeCameraId = sel.value;
      const device = activeDevice();
      if (device) device.cameraId = activeCameraId;
      clearVerify();
      renderDeviceList();
    };
  }

  function updateTargetPreview() {
    const preview = $("target-preview");
    if (!preview) return;
    const index = selectedRegionIndex();
    const button = buttons[index] || buttons[0];
    preview.textContent = button ? qualifiedRegionId(button) : `${activeDeviceId}.button_1`;
  }

  function selectedRegionIndexes(scope) {
    const selected = $("learn-region").value;
    if (scope === "all") {
      return buttons.map((_, index) => index);
    }
    if (selected === "__all") return [];
    const index = parseInt(selected, 10);
    return Number.isFinite(index) && buttons[index] ? [index] : [];
  }

  function selectedRegionIndex() {
    const selected = $("learn-region").value;
    const index = parseInt(selected, 10);
    return Number.isFinite(index) && buttons[index] ? index : -1;
  }

  function updateRegionEditor() {
    const index = selectedRegionIndex();
    const disabled = index < 0;
    const idInput = $("region-id");
    const labelInput = $("region-label");
    const kindInput = $("region-kind");
    [idInput, labelInput, kindInput].forEach((el) => (el.disabled = disabled));
    if (disabled) {
      idInput.value = "";
      labelInput.value = "";
      kindInput.value = "button_led";
      renderStatePanel();
      return;
    }
    const button = buttons[index];
    idInput.value = button.id || "";
    labelInput.value = button.label || "";
    kindInput.value = button.kind || "button_led";
    renderStatePanel();
  }

  function selectRegion(index) {
    if (!buttons[index]) return;
    $("learn-region").value = String(index);
    updateRegionEditor();
    updateTargetPreview();
    renderStatePanel();
    drawWarpOverlay();
  }

  function applyRegionEditor() {
    const index = selectedRegionIndex();
    if (index < 0) return;
    const button = buttons[index];
    const id = $("region-id").value.trim();
    const label = $("region-label").value.trim();
    const kind = $("region-kind").value.trim();
    if (id) button.id = id;
    if (label) button.label = label;
    if (kind) button.kind = kind;
    button.deviceId = activeDeviceId;
    clearVerify();
    updateRegionSelect();
    $("learn-region").value = String(index);
    updateRegionEditor();
    drawWarpOverlay();
  }

  // ── Coordinate mapping (display px → source px) ──────────────────────
  function toSource(ev) {
    const rect = srcCanvas.getBoundingClientRect();
    const sx = srcCanvas.width / rect.width;
    const sy = srcCanvas.height / rect.height;
    return [(ev.clientX - rect.left) * sx, (ev.clientY - rect.top) * sy];
  }

  function nearestCorner(pt) {
    let best = -1,
      bestD = 1e9;
    corners.forEach((c, i) => {
      const d = Math.hypot(c[0] - pt[0], c[1] - pt[1]);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    });
    return bestD < 40 ? best : -1;
  }

  // ── Source canvas rendering ─────────────────────────────────────────
  function drawSrc() {
    if (frameLoaded && currentFrame) {
      ctx.drawImage(currentFrame, 0, 0, srcCanvas.width, srcCanvas.height);
    } else {
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, srcCanvas.width, srcCanvas.height);
    }
    if (corners.length === 0) return;
    ctx.lineWidth = 3;
    ctx.strokeStyle = "#43d17a";
    ctx.beginPath();
    corners.forEach((c, i) =>
      i === 0 ? ctx.moveTo(c[0], c[1]) : ctx.lineTo(c[0], c[1])
    );
    if (corners.length === 4) ctx.closePath();
    ctx.stroke();
    corners.forEach((c, i) => {
      ctx.fillStyle = "#43d17a";
      ctx.beginPath();
      ctx.arc(c[0], c[1], 7, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#0f1115";
      ctx.font = "bold 16px sans-serif";
      ctx.fillText(String(i + 1), c[0] - 4, c[1] + 5);
    });
  }

  srcCanvas.addEventListener("mousedown", (ev) => {
    if (info && info.observe) return;
    const pt = toSource(ev);
    if (corners.length >= 4) {
      dragIdx = nearestCorner(pt);
      if (dragIdx < 0) {
        corners = [pt];
      }
    } else {
      corners.push(pt);
    }
    clearVerify();
    updateCornerCount();
    drawSrc();
  });
  srcCanvas.addEventListener("mousemove", (ev) => {
    if (dragIdx < 0) return;
    corners[dragIdx] = toSource(ev);
    clearVerify();
    drawSrc();
  });
  window.addEventListener("mouseup", () => (dragIdx = -1));

  $("btn-clear").addEventListener("click", () => {
    corners = [];
    clearVerify();
    updateCornerCount();
    drawSrc();
  });

  function updateCornerCount() {
    $("corner-count").textContent = corners.length + " / 4 góc";
    updateQuality();
  }

  // ── Template selector ───────────────────────────────────────────────
  function initTemplates() {
    const sel = $("template");
    TEMPLATES.forEach((t, i) => {
      const o = document.createElement("option");
      o.value = String(i);
      o.textContent = t.label;
      sel.appendChild(o);
    });
    sel.value = "3"; // default 2x2
    applyTemplate();
    sel.addEventListener("change", applyTemplate);
    $("rows").addEventListener("change", applyCustomGrid);
    $("cols").addEventListener("change", applyCustomGrid);
  }

  function templateKind() {
    const selected = TEMPLATES[parseInt($("template").value, 10)] || TEMPLATES[3];
    return selected.label.replace(/[()×]/g, "").toLowerCase().replace(/\s+/g, "_");
  }

  function ensureLabDefaults(profile) {
    const profileLab = profile && profile.lab;
    labCameras = (profileLab && profileLab.cameras && profileLab.cameras.length
      ? profileLab.cameras
      : [
          {
            id: profile?.activeCameraId || profile?.active_camera_id || "camera_1",
            label: "Camera 1",
          },
        ]).map((camera) => ({
      id: camera.id || "camera_1",
      label: camera.label || camera.id || "Camera 1",
      rtsp: null,
      transport: null,
    }));
    activeCameraId =
      (profileLab && (profileLab.activeCameraId || profileLab.active_camera_id)) ||
      profile?.activeCameraId ||
      profile?.active_camera_id ||
      labCameras[0].id;

    const profileButtons = ((profile && (profile.buttons || profile.regions)) || []).map(normalizeButton);
    labDevices = (profileLab && profileLab.devices && profileLab.devices.length
      ? profileLab.devices
      : [
          {
            id: profile?.activeDeviceId || profile?.active_device_id || "switch_4gang",
            label: profile?.name || "Công tắc 4 nút",
            cameraId: activeCameraId,
            kind: "switch_4",
            regions: profileButtons,
          },
        ]).map((device) => ({
      id: device.id || slugify(device.label, "device_1"),
      label: device.label || device.id || "Thiết bị",
      cameraId: device.cameraId || device.camera_id || activeCameraId,
      kind: device.kind || "switch",
      layout: device.layout,
      regions: (device.regions || []).map(normalizeButton),
    }));
    activeDeviceId =
      (profileLab && (profileLab.activeDeviceId || profileLab.active_device_id)) ||
      profile?.activeDeviceId ||
      profile?.active_device_id ||
      labDevices[0].id;
    if (!labDevices.some((device) => device.id === activeDeviceId)) {
      activeDeviceId = labDevices[0].id;
    }
  }

  function addDevice() {
    syncActiveDeviceFromUi();
    const next = labDevices.length + 1;
    const device = {
      id: `device_${next}`,
      label: `Thiết bị ${next}`,
      cameraId: activeCameraId,
      kind: templateKind(),
      layout: { ...layout },
      regions: gridButtons(layout.rows || 2, layout.cols || 2, layout.cell_fill || 0.6).map((button) => ({
        ...button,
        deviceId: `device_${next}`,
      })),
    };
    labDevices.push(device);
    activeDeviceId = device.id;
    applyActiveDeviceToUi();
    clearVerify();
  }

  function addCamera() {
    syncActiveDeviceFromUi();
    const next = labCameras.length + 1;
    const camera = {
      id: `camera_${next}`,
      label: `Camera ${next}`,
      rtsp: null,
      transport: null,
    };
    labCameras.push(camera);
    activeCameraId = camera.id;
    const device = activeDevice();
    if (device) device.cameraId = activeCameraId;
    renderCameraSelect();
    renderDeviceList();
    clearVerify();
    setStatus(`Đã thêm ${camera.label}. Mở calibration bằng RTSP tương ứng khi cần canh camera này.`, "ok");
  }

  function applyTemplate() {
    const t = TEMPLATES[parseInt($("template").value, 10)];
    if (t.custom) {
      $("grid-custom").classList.remove("hidden");
      applyCustomGrid();
    } else {
      $("grid-custom").classList.add("hidden");
      regionPrefix = t.regionPrefix || "button";
      layout = { type: "grid", rows: t.rows, cols: t.cols, cell_fill: 0.6 };
      regenButtons();
    }
    const device = activeDevice();
    if (device) {
      device.kind = templateKind();
      device.layout = layout;
      device.regions = buttons.map((button) => ({ ...normalizeButton(button), deviceId: device.id }));
    }
    clearVerify();
    renderDeviceList();
  }
  function applyCustomGrid() {
    const rows = Math.max(1, parseInt($("rows").value, 10) || 1);
    const cols = Math.max(1, parseInt($("cols").value, 10) || 1);
    regionPrefix = "button";
    layout = { type: "grid", rows, cols, cell_fill: 0.6 };
    regenButtons();
    clearVerify();
  }

  // ── Learn colors ────────────────────────────────────────────────────
  async function learn(stateName, scope) {
    stateName = normalizeStateName(stateName);
    if (!stateName) {
      setStatus("Nhập tên trạng thái trước khi học", "err");
      return;
    }
    if (corners.length !== 4) {
      setStatus("Cần chọn đủ 4 góc trước khi học màu", "err");
      return;
    }
    const regionIndexes = selectedRegionIndexes(scope);
    if (regionIndexes.length === 0) {
      setStatus("Cần chọn một region cụ thể để học màu này", "err");
      return;
    }
    setStatus("Đang học màu " + stateName + "…");
    try {
      for (const index of regionIndexes) {
        const res = await fetch("/api/learn", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            geometry: { corners: corners.map((c) => [c[0], c[1]]), warp: WARP },
            rois: [buttons[index].roi],
          }),
        });
        const data = await res.json();
        if (data.error) {
          setStatus(data.error, "err");
          return;
        }
        upsertStateModel(buttons[index], stateName, data.ranges);
        allowStateForRegion(index, stateName);
      }
      clearVerify();
      renderStates();
      renderStatePanel();
      setStatus(`Đã học ${stateName} cho ${regionIndexes.length} vùng`, "ok");
    } catch (e) {
      setStatus("Lỗi học màu: " + e, "err");
    }
  }

  async function autoDetectLeds() {
    if (corners.length !== 4) {
      setStatus("Cần chọn đủ 4 góc trước khi tự tìm LED", "err");
      return;
    }
    setStatus("Đang tự tìm LED trong ảnh hiện tại…");
    try {
      const res = await fetch("/api/propose-leds", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(buildProfile()),
      });
      const data = await res.json();
      if (data.error) {
        setStatus(data.error, "err");
        return;
      }
      let found = 0;
      (data.proposals || []).forEach((proposal, index) => {
        if (!buttons[index] || !proposal.found) return;
        buttons[index].roi = proposal.roi;
        buttons[index].searchRoi = proposal.searchRoi || proposal.search_roi || buttons[index].searchRoi || expandedSearchRoi(proposal.roi);
        buttons[index].mask = proposal.mask || "ellipse";
        delete buttons[index].expectedCenter;
        delete buttons[index].maxCenterDrift;
        found += 1;
      });
      syncActiveDeviceFromUi();
      clearVerify();
      drawWarpOverlay();
      updateRegionSelect();
      setStatus(
        found > 0
          ? `Đã tự tìm ${found}/${buttons.length} vùng LED`
          : "Chưa tìm thấy LED sáng rõ. Hãy bật đèn hoặc kéo ROI thủ công.",
        found > 0 ? "ok" : "err"
      );
    } catch (e) {
      setStatus("Lỗi tự tìm LED: " + e, "err");
    }
  }
  function upsertState(name, ranges) {
    const ex = states.find((s) => s.name === name);
    if (name === "OFF") {
      const darkMaxV = Math.min(
        90,
        Math.max(20, ...((ranges || []).map((r) => (r.upper && r.upper[2]) || 45)))
      );
      const rule = { name, type: "dark", darkMaxV, source: "learned" };
      if (ex) {
        Object.assign(ex, rule);
        delete ex.hsv;
      }
      else states.push(rule);
      return;
    }
    if (ex) {
      ex.hsv = ranges;
      delete ex.type;
      delete ex.darkMaxV;
    } else states.push({ name, hsv: ranges, source: "learned" });
  }

  function upsertStateModel(button, name, ranges) {
    name = normalizeStateName(name);
    const key = `${qualifiedRegionId(button)}.${name}`;
    if (name === "OFF") {
      const darkMaxV = Math.min(
        90,
        Math.max(20, ...((ranges || []).map((r) => (r.upper && r.upper[2]) || 45)))
      );
      stateModels[key] = { type: "dark", darkMaxV, source: "learned" };
      return;
    }
    stateModels[key] = { hsv: ranges, source: "learned" };
  }

  function allowStateForRegion(index, name) {
    name = normalizeStateName(name);
    const button = buttons[index];
    if (!button) return;
    button.allowedStates = button.allowedStates || [];
    if (!button.allowedStates.some((state) => state.toUpperCase() === name)) {
      button.allowedStates.push(name);
    }
  }

  function normalizeStateName(value) {
    return String(value || "")
      .trim()
      .toUpperCase()
      .replace(/[^A-Z0-9_]+/g, "_")
      .replace(/^_+|_+$/g, "");
  }

  function stateModelKey(button, stateName) {
    return `${qualifiedRegionId(button)}.${normalizeStateName(stateName)}`;
  }

  function stateModelStateName(key) {
    return normalizeStateName(key.split(".").pop());
  }

  function hsvToCss(r) {
    // crude preview: mid of range, OpenCV H 0-179 → 0-360
    const h = ((r.lower[0] + r.upper[0]) / 2) * 2;
    const s = (r.lower[1] + r.upper[1]) / 2 / 255;
    const v = (r.lower[2] + r.upper[2]) / 2 / 255;
    return `hsl(${h}, ${Math.round(s * 100)}%, ${Math.round(v * 60)}%)`;
  }
  function renderStates() {
    const el = $("states-info");
    if (!el) return;
    el.innerHTML = "";
    states.forEach((s) => {
      const chip = document.createElement("span");
      chip.className = "chip";
      const sw = document.createElement("span");
      sw.className = "swatch";
      if (s.type === "dark") sw.style.background = "#111";
      else if (s.hsv && s.hsv[0]) sw.style.background = hsvToCss(s.hsv[0]);
      chip.appendChild(sw);
      chip.appendChild(document.createTextNode(s.name));
      el.appendChild(chip);
    });
    Object.entries(stateModels).forEach(([key, s]) => {
      const chip = document.createElement("span");
      chip.className = "chip";
      const sw = document.createElement("span");
      sw.className = "swatch";
      if (s.type === "dark") sw.style.background = "#111";
      else if (s.hsv && s.hsv[0]) sw.style.background = hsvToCss(s.hsv[0]);
      chip.appendChild(sw);
      chip.appendChild(document.createTextNode(key.replace(activeDeviceId + ".", "")));
      el.appendChild(chip);
    });
    renderStatePanel();
    updateQuality();
  }

  function renderStatePanel() {
    const panel = $("state-panel");
    if (!panel) return;
    const title = $("state-region-title");
    const presetWrap = $("state-presets");
    const list = $("region-state-list");
    const index = selectedRegionIndex();
    const button = buttons[index];
    if (title) title.textContent = button ? qualifiedRegionId(button) : "Chưa chọn vùng";

    if (presetWrap && presetWrap.childElementCount === 0) {
      STATE_PRESETS.forEach((name) => {
        const preset = document.createElement("button");
        preset.type = "button";
        preset.className = "state-preset";
        preset.textContent = name;
        preset.addEventListener("click", () => {
          $("state-name").value = name;
          $("state-name").focus();
        });
        presetWrap.appendChild(preset);
      });
    }

    if (!list) return;
    list.innerHTML = "";
    if (!button) {
      list.textContent = "Chọn một vùng LED để xem các trạng thái đã học.";
      list.className = "region-state-list empty";
      return;
    }

    const names = new Set([
      ...(button.allowedStates || button.allowed_states || []).map(normalizeStateName),
      ...states.map((state) => normalizeStateName(state.name)),
    ]);
    const prefix = `${qualifiedRegionId(button)}.`;
    Object.keys(stateModels)
      .filter((key) => key.startsWith(prefix))
      .forEach((key) => names.add(stateModelStateName(key)));

    if (names.size === 0) {
      list.textContent = "Vùng này chưa có state. Nhập tên state rồi bấm học từ ảnh hiện tại.";
      list.className = "region-state-list empty";
      return;
    }
    list.className = "region-state-list";
    [...names].filter(Boolean).sort().forEach((name) => {
      const chip = document.createElement("span");
      const scopedRule = stateModels[stateModelKey(button, name)];
      const globalRule = states.find((state) => normalizeStateName(state.name) === name);
      const rule = scopedRule || globalRule;
      chip.className = "chip state-chip" + (scopedRule ? " learned" : "");
      const sw = document.createElement("span");
      sw.className = "swatch";
      if (rule && rule.type === "dark") sw.style.background = "#111";
      else if (rule && rule.hsv && rule.hsv[0]) sw.style.background = hsvToCss(rule.hsv[0]);
      chip.appendChild(sw);
      chip.appendChild(document.createTextNode(name));
      const meta = document.createElement("small");
      meta.textContent = scopedRule ? "đã học vùng này" : "rule chung";
      chip.appendChild(meta);
      list.appendChild(chip);
    });
  }

  function learnStateFromPanel() {
    const input = $("state-name");
    const name = normalizeStateName(input && input.value);
    if (!name) {
      setStatus("Nhập tên trạng thái trước khi học", "err");
      if (input) input.focus();
      return;
    }
    if (input) input.value = name;
    learn(name, "selected");
  }

  // ── Detection loop ──────────────────────────────────────────────────
  // The warped preview image must only be drawn once it has finished decoding;
  // drawing right after setting `.src` paints an empty (black) bitmap.
  let warpImg = new Image();
  let warpReady = false;
  let latestDevice = null;
  let driftWarning = null;
  warpImg.onload = () => {
    warpReady = true;
    drawWarpOverlay();
  };

  async function detectTick() {
    if (corners.length === 4) {
      try {
        const res = await fetch("/api/detect", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(buildProfile()),
        });
        if (!res.ok) {
          setStatus(`Detect lỗi HTTP ${res.status}`, "err");
          return;
        }
        const data = await res.json();
        if (!data.error) {
          latestDevice = data.states;
          renderTable(data.states);
          if (data.warped) {
            warpImg.src = "data:image/jpeg;base64," + data.warped;
          } else {
            drawWarpOverlay();
          }
        }
      } catch (e) {
        setStatus("Detect lỗi: " + e, "err");
      }
    }
    setTimeout(detectTick, 500);
  }

  function badge(state) {
    const cls =
      state === "OK"
        ? "on"
        : state === "UNKNOWN" ||
            state === "AMBIGUOUS" ||
            state === "MISALIGNED" ||
            state === "UNSTABLE"
        ? "unknown"
        : state === "OFF"
          ? "off"
          : "on";
    return `<span class="badge ${cls}">${state}</span>`;
  }

  function confidenceClass(reading) {
    const confidence = reading.confidence || 0;
    const margin = reading.margin || 0;
    if (reading.status !== "MATCH" || confidence < 0.2 || margin < 0.05) return "bad";
    if (confidence < 0.5 || margin < 0.12) return "warn";
    return "ok";
  }

  function confidenceHtml(reading) {
    const confidence = Math.round((reading.confidence || 0) * 100);
    const cls = confidenceClass(reading);
    const label = cls === "ok" ? "Tốt" : cls === "warn" ? "Cần theo dõi" : "Không chắc";
    return (
      `<div class="confidence ${cls}">` +
      `<strong>${label} · ${confidence}%</strong>` +
      `<div class="confidence-bar"><span style="width:${Math.max(0, Math.min(100, confidence))}%"></span></div>` +
      "</div>"
    );
  }

  // Draw the (already-loaded) warped frame plus ROI + state overlay.
  function drawWarpOverlay() {
    if (!warpReady) return;
    warpCanvas.width = WARP[0];
    warpCanvas.height = WARP[1];
    wctx.drawImage(warpImg, 0, 0, WARP[0], WARP[1]);
    const readings = (latestDevice && latestDevice.buttons) || [];
    const selectedIndex = selectedRegionIndex();
    buttons.forEach((b, i) => {
      const reading = readings[i] || { state: "UNKNOWN" };
      const known = reading.status === "MATCH";
      const selected = i === selectedIndex;
      const color = selected ? "#2b6cff" : known ? "#43d17a" : "#ff5d5d";
      const [rx, ry, rw, rh] = b.roi;
      wctx.lineWidth = selected ? 4 : 2;
      wctx.strokeStyle = color;
      const ellipse = (b.mask || "ellipse").toLowerCase() === "ellipse";
      if (ellipse) {
        wctx.beginPath();
        wctx.ellipse(rx + rw / 2, ry + rh / 2, rw / 2, rh / 2, 0, 0, Math.PI * 2);
        wctx.stroke();
        if (selected) {
          wctx.save();
          wctx.globalAlpha = 0.28;
          wctx.strokeRect(rx, ry, rw, rh);
          wctx.restore();
        }
      } else {
        wctx.strokeRect(rx, ry, rw, rh);
      }
      const label = `${b.label}: ${reading.state}`;
      wctx.font = "bold 13px sans-serif";
      const textWidth = Math.ceil(wctx.measureText(label).width);
      const labelX = Math.max(0, Math.min(rx, WARP[0] - textWidth - 10));
      const labelY = Math.max(0, ry - 20);
      wctx.fillStyle = "rgba(15,17,21,0.82)";
      wctx.fillRect(labelX, labelY, textWidth + 10, 19);
      wctx.fillStyle = color;
      wctx.fillText(label, labelX + 5, labelY + 14);
      // resize handle (bottom-right)
      wctx.fillRect(rx + rw - 9, ry + rh - 9, 9, 9);
    });
  }

  // ── Draggable / resizable ROIs on the warped image ──────────────────
  // LEDs often sit at the corner of a button, not its centre, so the tester
  // nudges each ROI box onto the actual LED (works for any device layout).
  function toWarp(ev) {
    const rect = warpCanvas.getBoundingClientRect();
    return [
      ((ev.clientX - rect.left) * WARP[0]) / rect.width,
      ((ev.clientY - rect.top) * WARP[1]) / rect.height,
    ];
  }
  let warpDrag = { idx: -1, mode: null, off: [0, 0] };
  warpCanvas.addEventListener("mousedown", (ev) => {
    if (info && info.observe) return;
    const [x, y] = toWarp(ev);
    for (let i = buttons.length - 1; i >= 0; i--) {
      const [rx, ry, rw, rh] = buttons[i].roi;
      if (Math.abs(x - (rx + rw)) < 16 && Math.abs(y - (ry + rh)) < 16) {
        selectRegion(i);
        warpDrag = { idx: i, mode: "resize", off: [0, 0] };
        return;
      }
      if (x >= rx && x <= rx + rw && y >= ry && y <= ry + rh) {
        selectRegion(i);
        warpDrag = { idx: i, mode: "move", off: [x - rx, y - ry] };
        return;
      }
    }
  });
  warpCanvas.addEventListener("mousemove", (ev) => {
    if (warpDrag.idx < 0) return;
    const [x, y] = toWarp(ev);
    const b = buttons[warpDrag.idx];
    if (warpDrag.mode === "move") {
      b.roi[0] = Math.max(0, Math.min(WARP[0] - b.roi[2], Math.round(x - warpDrag.off[0])));
      b.roi[1] = Math.max(0, Math.min(WARP[1] - b.roi[3], Math.round(y - warpDrag.off[1])));
    } else {
      b.roi[2] = Math.max(12, Math.min(WARP[0] - b.roi[0], Math.round(x - b.roi[0])));
      b.roi[3] = Math.max(12, Math.min(WARP[1] - b.roi[1], Math.round(y - b.roi[1])));
    }
    delete b.expectedCenter;
    delete b.maxCenterDrift;
    b.searchRoi = expandedSearchRoi(b.roi);
    clearVerify();
    drawWarpOverlay();
  });
  window.addEventListener("mouseup", () => (warpDrag.idx = -1));

  function renderTable(device) {
    const tbody = $("results").querySelector("tbody");
    tbody.innerHTML = "";
    const readings = (device && device.buttons) || [];
    let okCount = 0;
    let weakCount = 0;
    let misalignedCount = 0;
    buttons.forEach((b, i) => {
      const reading = readings[i] || { state: "UNKNOWN", confidence: 0 };
      if (reading.status === "MATCH") okCount += 1;
      if (reading.status === "MISALIGNED") misalignedCount += 1;
      if (reading.status !== "MATCH" || confidenceClass(reading) !== "ok") weakCount += 1;
      const tr = document.createElement("tr");
      const second = reading.secondBest
        ? `${reading.secondBest.state} ${Math.round((reading.secondBest.confidence || 0) * 100)}%`
        : "-";
      tr.innerHTML =
        `<td><strong>${b.label}</strong><br><span class="muted">${qualifiedRegionId(b)}</span></td>` +
        `<td>${badge(reading.state)}</td>` +
        `<td>${confidenceHtml(reading)}</td>` +
        `<td class="muted advanced-col">${reading.status || "UNKNOWN"} · margin ${Math.round((reading.margin || 0) * 100)}%</td>` +
        `<td class="muted">next ${second}</td>`;
      tbody.appendChild(tr);
    });
    const summary = $("live-summary");
    if (summary) summary.textContent = `${okCount}/${buttons.length} vùng nhận diện được`;
    driftWarning =
      misalignedCount > 0
        ? `${misalignedCount} vùng lệch khỏi vị trí LED đã học. Camera hoặc thiết bị có thể đã xê dịch.`
        : weakCount >= Math.max(2, Math.ceil(buttons.length / 2))
          ? "Nhiều vùng LED đang yếu/không chắc. Kiểm tra camera, ánh sáng hoặc bấm Tự tìm LED."
          : null;
    updateQuality();
  }

  function setCheck(name, ok, text) {
    const el = document.querySelector(`[data-check="${name}"]`);
    if (!el) return;
    el.classList.toggle("ok", !!ok);
    el.classList.toggle("bad", ok === false);
    const detail = el.querySelector("em");
    if (detail) detail.textContent = text;
  }

  function learnedRegionCount() {
    const keys = Object.keys(stateModels);
    return buttons.filter((button) =>
      keys.some((key) => key.startsWith(qualifiedRegionId(button) + "."))
    ).length;
  }

  function updateQuality() {
    const cameraOk = !!currentFrame && frameLoaded;
    const cornersOk = corners.length === 4;
    const regionsOk = buttons.length > 0 && buttons.every((button) => button.roi && button.roi[2] >= 12 && button.roi[3] >= 12);
    const learnedCount = learnedRegionCount();
    const hasGlobalModels = states.length > 0;
    const learnedOk = learnedCount > 0 || hasGlobalModels;
    const verifiedOk = verifiedProfileKey === profileKey();

    setCheck("camera", cameraOk, cameraOk ? `${srcCanvas.width}×${srcCanvas.height}` : "Chưa có frame");
    setCheck("corners", cornersOk, `${corners.length} / 4 góc`);
    setCheck("regions", regionsOk, regionsOk ? `${buttons.length} vùng LED` : "Chưa có vùng LED");
    setCheck(
      "learned",
      learnedOk,
      learnedCount > 0
        ? `${learnedCount}/${buttons.length} vùng có model riêng`
        : hasGlobalModels
          ? "Đang dùng model màu chung"
          : "Chưa học ON/OFF"
    );
    setCheck("verified", verifiedOk, verifiedOk ? "Đã verify profile hiện tại" : "Cần verify 5s");

    const card = $("quality-decision");
    if (!card) return;
    card.classList.remove("ready", "warn", "bad");
    const title = card.querySelector(".decision-title");
    const copy = card.querySelector(".decision-copy");
    if (verifiedOk) {
      card.classList.add("ready");
      title.textContent = "Profile dùng được";
      copy.textContent = "Có thể lưu profile và dùng trong test automation.";
    } else if (cameraOk && cornersOk && regionsOk && learnedOk) {
      card.classList.add("warn");
      title.textContent = driftWarning ? "Cần kiểm tra căn chỉnh" : "Sẵn sàng verify";
      copy.textContent = driftWarning || "Chạy kiểm chứng 5 giây trước khi lưu để bắt lỗi rung, lệch ROI hoặc màu chưa ổn định.";
    } else {
      card.classList.add("bad");
      title.textContent = "Chưa đủ dữ liệu";
      copy.textContent = "Hoàn tất các mục checklist trước khi lưu profile.";
    }
  }

  async function verifyProfile() {
    const out = $("verify-result");
    out.classList.remove("hidden");
    if (corners.length !== 4) {
      setStatus("Cần đủ 4 góc trước khi kiểm chứng", "err");
      out.textContent = "Chưa thể kiểm chứng: thiếu 4 góc thiết bị.";
      return;
    }
    if (states.length === 0 && Object.keys(stateModels).length === 0) {
      setStatus("Hãy học ít nhất 1 trạng thái màu trước khi kiểm chứng", "err");
      out.textContent = "Chưa thể kiểm chứng: chưa có trạng thái màu.";
      return;
    }
    setStatus("Đang kiểm chứng profile trong 5 giây…");
    out.textContent = "Đang lấy mẫu camera…";
    try {
      const res = await fetch("/api/verify", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          profile: buildProfile(),
          durationMs: 5000,
          sampleMs: 200,
        }),
      });
      const data = await res.json();
      renderVerifyResult(data);
      verifiedProfileKey = data.ok ? profileKey() : null;
      setStatus(data.ok ? "Profile ổn định" : "Profile còn vùng chưa ổn", data.ok ? "ok" : "err");
    } catch (e) {
      out.textContent = "Lỗi kiểm chứng: " + e;
      setStatus("Lỗi kiểm chứng profile", "err");
    }
  }

  function renderVerifyResult(data) {
    const out = $("verify-result");
    if (data.error) {
      out.textContent = data.error;
      return;
    }
    const regions = data.regions || [];
    const rows = regions
      .map((r) => {
        const stateList = Object.entries(r.states || {})
          .map(([name, count]) => `${name}:${count}`)
          .join(", ");
        const missing = (r.missingStates || []).join(", ");
        const status = r.unstable
          ? "UNSTABLE"
          : r.misalignedCount
            ? "MISALIGNED"
            : r.ambiguousCount
              ? "AMBIGUOUS"
              : r.unknownCount
                ? "UNKNOWN"
                : "OK";
        return (
          "<tr>" +
          `<td>${r.id || r.label}</td>` +
          `<td>${badge(status)}</td>` +
          `<td class="muted">${r.matchCount}/${r.samples}</td>` +
          `<td class="muted">min ${Math.round((r.minConfidence || 0) * 100)}%</td>` +
          `<td class="muted">${stateList || "-"}${missing ? " · thiếu " + missing : ""}</td>` +
          "</tr>"
        );
      })
      .join("");
    out.innerHTML =
      `<div class="${data.ok ? "verify-ok" : "verify-bad"}">` +
      `${data.ok ? "Đạt" : "Chưa đạt"} · ${data.sampleCount || 0} frame mới · ${data.durationMs || 0}ms` +
      `${data.fresh ? "" : " · frame camera không mới"}` +
      "</div>" +
      `<table><tbody>${rows}</tbody></table>`;
  }

  // ── Save ────────────────────────────────────────────────────────────
  $("btn-verify").addEventListener("click", verifyProfile);

  $("btn-save").addEventListener("click", async () => {
    if (corners.length !== 4) {
      setStatus("Cần đủ 4 góc trước khi lưu", "err");
      return;
    }
    if (states.length === 0 && Object.keys(stateModels).length === 0) {
      setStatus("Hãy học ít nhất 1 trạng thái màu (ON/OFF)", "err");
      return;
    }
    if (verifiedProfileKey !== profileKey()) {
      setStatus("Cần kiểm chứng profile 5s thành công trước khi lưu", "err");
      $("save-msg").textContent = "Chưa lưu: profile hiện tại chưa được kiểm chứng hoặc đã thay đổi sau lần kiểm chứng.";
      return;
    }
    try {
      const res = await fetch("/api/save", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(buildProfile()),
      });
      const data = await res.json();
      if (data.error) $("save-msg").textContent = "❌ " + data.error;
      else $("save-msg").textContent = "✅ Đã lưu: " + data.path;
    } catch (e) {
      $("save-msg").textContent = "❌ " + e;
    }
  });

  // ── Frame polling ───────────────────────────────────────────────────
  function pollFrame() {
    const next = new Image();
    next.onload = () => {
      currentFrame = next;
      frameLoaded = true;
      drawSrc();
      updateQuality();
      setTimeout(pollFrame, 200);
    };
    next.onerror = () => setTimeout(pollFrame, 1000);
    next.src = "/api/frame.jpg?ts=" + Date.now();
  }

  // ── Boot ────────────────────────────────────────────────────────────
  async function boot() {
    $("btn-auto-leds").addEventListener("click", autoDetectLeds);
    $("btn-learn-on").addEventListener("click", () => learn("ON", "selected"));
    $("btn-learn-off").addEventListener("click", () => learn("OFF", "selected"));
    $("btn-learn-state").addEventListener("click", learnStateFromPanel);
    $("state-name").addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        learnStateFromPanel();
      }
    });
    $("learn-region").addEventListener("change", () => {
      updateRegionEditor();
      updateTargetPreview();
      drawWarpOverlay();
    });
    ["region-id", "region-label", "region-kind"].forEach((id) => {
      $(id).addEventListener("change", applyRegionEditor);
      $(id).addEventListener("blur", applyRegionEditor);
    });
    $("btn-add-camera").addEventListener("click", addCamera);
    $("btn-add-device").addEventListener("click", addDevice);
    ["device-id", "device-label"].forEach((id) => {
      $(id).addEventListener("change", () => {
        syncActiveDeviceFromUi();
        renderDeviceList();
        updateRegionSelect();
        clearVerify();
      });
      $(id).addEventListener("blur", () => {
        syncActiveDeviceFromUi();
        renderDeviceList();
        updateRegionSelect();
        clearVerify();
      });
    });
    initTemplates();

    try {
      info = await (await fetch("/api/info")).json();
    } catch (e) {
      setStatus("Không lấy được thông tin camera", "err");
      return;
    }
    srcCanvas.width = info.width;
    srcCanvas.height = info.height;
    srcCanvas.style.aspectRatio = `${info.width} / ${info.height}`;

    if (info.observe) {
      document.body.classList.add("observe");
      $("mode-tag").textContent = "Quan sát";
    }

    if (info.profile) {
      const p = info.profile;
      corners = (p.geometry && p.geometry.corners) || [];
      layout = p.layout || layout;
      states = (p.states || []).map(normalizeStateRule);
      stateModels = p.stateModels || p.state_models || {};
      if (p.name) $("name").value = p.name;
      ensureLabDefaults(p);
      const device = activeDevice();
      if (device && device.regions && device.regions.length) {
        buttons = device.regions.map(normalizeButton).map((button) => ({ ...button, deviceId: device.id }));
      } else {
        buttons = (p.buttons || p.regions || []).map(normalizeButton).map((button) => ({
          ...button,
          deviceId: activeDeviceId,
        }));
      }
    } else {
      ensureLabDefaults(null);
    }

    if (layout.type === "grid" && buttons.length === 0) regenButtons();
    buttons.forEach((button) => (button.deviceId = activeDeviceId));
    const device = activeDevice();
    if (device) {
      device.regions = buttons.map((button) => ({ ...normalizeButton(button), deviceId: device.id }));
    }
    renderCameraSelect();
    renderDeviceList();
    applyActiveDeviceToUi();
    renderStates();
    updateCornerCount();
    updateQuality();

    setStatus(`Đã kết nối · ${info.width}×${info.height}`, "ok");
    pollFrame();
    detectTick();
  }

  boot();
})();
