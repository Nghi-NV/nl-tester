# Lumi extension manifest

Lumi IDE imports a single JSON manifest. It stores extensions in the app's configuration directory; it does not execute installed files. A newer manifest with the same `id` updates the installed version and preserves its enabled state.

```json
{
  "schemaVersion": 1,
  "lumiApiVersion": 1,
  "id": "team.login-flows",
  "name": "Login flows",
  "publisher": "Team",
  "version": "1.2.0",
  "description": "Reusable login flow authoring helpers.",
  "contributes": {
    "docs": [
      { "title": "Login selectors", "content": "Prefer stable test IDs." }
    ],
    "snippets": [
      { "prefix": "login", "description": "Basic login flow", "body": "- launchApp\n- tap: login" }
    ],
    "templates": [
      { "name": "Login smoke test", "fileName": "login_smoke.yaml", "content": "platform: android\n---\n- launchApp\n" }
    ],
    "selectorPacks": [
      { "name": "Account", "description": "Common account selectors", "selectors": ["id: login_button", "text: Welcome"] }
    ],
    "inspectorGuides": [
      {
        "name": "Web checkout",
        "description": "Prefer stable test IDs and confirm the target against its visible label.",
        "platforms": ["web"],
        "selectorExamples": ["id: checkout-submit", "text: Pay now"]
      }
    ],
    "reportViews": [
      { "id": "login-summary", "label": "Login summary", "description": "Login-related run summary." }
    ]
  }
}
```

The current manager previews docs and all contribution metadata, copies snippets, and creates template files in the open workspace root. Enabled `inspectorGuides` appear in UI Inspector for their declared platforms, with selector examples users can copy. Selector packs and report view declarations remain informational metadata; they do not execute extension code or alter the test engine. Contribution strings are bounded by the 1 MiB manifest limit. Template `fileName` must be a single file name, so a template cannot escape the workspace root.

`schemaVersion` and `lumiApiVersion` must both be `1`. Extension `version` uses `major.minor.patch`; an update must be strictly newer. Extension IDs use lowercase letters, digits, dots, underscores, and hyphens.
