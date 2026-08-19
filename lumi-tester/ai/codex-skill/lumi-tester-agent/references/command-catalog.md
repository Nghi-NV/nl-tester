# Lumi Tester Command Catalog

Concise reference for Lumi Tester YAML commands. Search `references/commands.csv` for the full parameter matrix.

## 1. Header Declaration

```yaml
platform: android # android | ios | web | macos | windows | android_auto
appId: com.example.app # package, bundleId, app path, or binary path
url: https://example.com # for Web tests
jig: "profiles/jig.yaml" # Hardware Jig profile or port
defaultTimeout: 10000
---
```

## 2. Core Navigation & Interaction

- `launchApp`: Launch the app or open the web URL (`clearState: true` for clean slate).
- `stopApp`: Terminate the active application.
- `tap`: Tap an element via structured selector (`id`, `text`, `type`, `align`, `offset`).
- `longPress`, `doubleTap`, `rightClick`: Extended gestures on elements.
- `inputText`: Type text into focused field (focus with `tap` first).
- `eraseText`: Clear text from the active field.
- `swipe`: Directional swipe (`direction: up | down | left | right`).
- `scrollUntilVisible`: Scroll container until target element appears.
- `back`, `pressHome`, `hideKeyboard`: Platform navigation actions.

## 3. Assertions & Synchronization

- `see`: Assert element is visible (`exact: true`, `timeout: 5000`).
- `notSee`: Assert element is absent / not visible.
- `waitUntilVisible` / `waitUntilNotVisible`: Polling assertion for dynamic transitions.
- `assertVar`: Assert variable equality or pattern match.
- `assertColor`: Verify pixel color at point or region.
- `wait`: Fixed delay in ms (use sparingly; prefer `waitUntilVisible`).

## 4. Control Flow & Reusability

- `repeat`: Execute nested commands N times (`times: 3`).
- `retry`: Retry flaky nested commands (`maxRetries: 2`).
- `conditional`: Branching logic (`condition: { visible: "Skip" }, then: [...]`).
- `runFlow`: Reusable subflow execution (`runFlow: ./subflows/login.yaml`).

## 5. Variables, Scripts & Utilities

- `setVar`: Store runtime variables (`name: token, value: "${DATA}"`).
- `runScript`: Host shell command or JS context script (`command: "./setup.sh"`).
- `evalScript`: Inline JavaScript expression evaluation.
- `httpRequest`: Direct HTTP API request (`method: GET, url: "..."`).
- `mockLocation` / `stopMockLocation`: GPS simulation (`file: "route.gpx", speed: 40`).
- `screenshot`: Capture visual artifact (`path: "screen.png"`).
- `startRecording` / `stopRecording`: Capture MP4 video recording.

## 6. Hardware Automation (`hw*`)

See dedicated reference: [references/hardware.md](file:///references/hardware.md).

- **Relays**: `hwPowerOn`, `hwPowerOff`, `hwPowerCycle`, `hwPowerOffAll`.
- **Servos**: `hwClick`, `hwPress`, `hwRelease`, `hwRotate`, `hwRepeatClick`, `hwConfigureServo`.
- **Sensors**: `hwSeeLed`, `hwSeeLedBlink`, `hwSeeLedOff`, `hwSensorLight`, `hwCalibrateColor`.
- **Diagnostics**: `hwReadServo`, `hwReadRelay`, `hwReadColor`, `hwDiagnostics`, `hwSafeState`.

## 7. Command Best Practices

- Wait before tap: Use `waitUntilVisible` before `tap` when navigating between screens.
- Stable selectors: Prioritize `id` / `accessibilityId` over dynamic text or coordinates.
- Sub-element taps: Use `align: right` or `offset: "85%,50%"` instead of manual coordinates.
