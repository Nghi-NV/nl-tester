const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const root = path.resolve(__dirname, "..");
const html = fs.readFileSync(path.join(root, "src/camera/ui/calibrate.html"), "utf8");
const script = fs.readFileSync(path.join(root, "src/camera/ui/script.js"), "utf8");

test("calibration UI exposes a region-scoped state learning panel", () => {
  assert.match(html, /id="state-panel"/);
  assert.match(html, /id="state-region-title"/);
  assert.match(html, /id="state-presets"/);
  assert.match(html, /id="state-name"/);
  assert.match(html, /id="btn-learn-state"/);
  assert.match(html, /id="region-state-list"/);
});

test("state learning panel refreshes when region selection or models change", () => {
  assert.match(script, /function renderStatePanel\(/);
  assert.match(script, /function learnStateFromPanel\(/);
  assert.match(script, /renderStatePanel\(\);/);
  assert.match(script, /\$\("learn-region"\)\.addEventListener\("change", \(\) => \{/);
});
