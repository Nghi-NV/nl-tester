# Lumi Tester Flow Patterns

Use these patterns as starting points. Always replace selectors with discovered
stable selectors from `selectors.csv` and validate before running.

## Contents

- Current Android app smoke
- Login
- Onboarding skip
- Search
- Settings toggle
- Permission dialog
- GPS route
- Web form
- Failure recovery pattern

## Current Android App Smoke

Use when the user asks to test the app currently open on a connected Android
device, especially when multiple devices are attached.

Discover the target first:

```bash
adb devices -l
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> exec-out uiautomator dump /dev/tty
```

Then write the flow with the discovered package and semantic selectors:

```yaml
platform: android
appId: com.example.current
tags:
  - smoke
  - current-app
defaultTimeout: 10000
---
- launchApp:
    appId: com.example.current
    permissions:
      all: allow
- waitUntilVisible:
    accessibilityId: "Screen title"
- waitUntilVisible:
    accessibilityId: "Primary action"
- tap:
    accessibilityId: "Primary action"
- waitForAnimationToEnd
- screenshot: "current_app_smoke.png"
```

Adaptation rules:

- Use current focus as the source of truth for `appId`; do not reuse a nearby
  YAML file unless it targets the same package.
- If launch restores a nested screen, inspect the hierarchy first and navigate
  back with a real command such as `back` or a semantic button tap. Do not use
  coordinates.
- `conditional.condition.visible` checks visible text, not Android
  `content-desc`; do not use it for `accessibilityId` readiness.
- When UI XML exposes `content-desc`, use `accessibilityId` or `desc`.
- Avoid `point` selectors unless the hierarchy and OCR expose no stable target.
- If the failure XML package is not the expected `appId`, debug launch/crash or
  wrong target before changing selectors.

## Login

Use when the user asks for sign in, authentication, or account smoke tests.

```yaml
platform: android
appId: com.example.app
tags:
  - smoke
  - login
defaultTimeout: 15000
---
- launchApp
- waitUntilVisible:
    id: "email"
- tap:
    id: "email"
- inputText: "${USER_EMAIL}"
- tap:
    id: "password"
- inputText: "${USER_PASSWORD}"
- hideKeyboard
- tap:
    id: "login_button"
- waitUntilVisible:
    text: "Home"
    exact: true
- see:
    text: "Home"
    exact: true
```

Adaptation rules:

- If credentials are sensitive, use `env: { file: ".env" }`.
- If keyboard covers the button, use `hideKeyboard` before `tap`.
- If login sometimes shows onboarding, use `conditional` or `runFlow`.

## Onboarding Skip

Use for first-run screens, tutorials, and permission prompts.

```yaml
platform: android
appId: com.example.app
tags:
  - onboarding
---
- launchApp:
    clearState: true
- retry:
    maxRetries: 3
    commands:
      - conditional:
          condition:
            visible: "Skip"
          then:
            - tap:
                text: "Skip"
                exact: true
- waitUntilVisible:
    text: "Home"
    exact: true
```

Adaptation rules:

- Prefer exact text for buttons like Skip/Next only if the app language is fixed.
- For permission dialogs, prefer `text` because system dialogs often lack app ids.

## Search

Use for search boxes, filtering lists, or command palette style flows.

```yaml
platform: android
appId: com.example.app
tags:
  - search
---
- launchApp
- tap:
    id: "search"
- inputText: "bedroom"
- waitUntilVisible:
    text: "Bedroom"
    exact: true
- see:
    text: "Bedroom"
    exact: true
```

Adaptation rules:

- If results load remotely, use `waitUntilVisible`, not fixed `wait`.
- If the target is below the fold, use `scrollUntilVisible`.
- For dynamic result counts, use `regex`.

## Settings Toggle

Use for settings lists, switches, and repeated rows.

```yaml
platform: android
appId: com.example.app
tags:
  - settings
---
- launchApp
- tap:
    desc: "Open menu"
- tap:
    text: "Settings"
    exact: true
- scrollUntilVisible:
    text: "Bedroom"
    direction: down
- tap:
    rightOf:
      text: "Bedroom"
      exact: true
    type: "android.widget.Switch"
- see:
    text: "Bedroom"
    exact: true
```

Adaptation rules:

- In repeated rows, prefer relative selectors over `type + index`.
- Use `scrollUntilVisible` before relative row actions.

## Permission Dialog

Use when flows trigger OS permission prompts.

```yaml
platform: android
appId: com.example.app
tags:
  - permission
---
- launchApp
- conditional:
    condition:
      visible: "While using the app"
    then:
      - tap:
          text: "While using the app"
          exact: true
- conditional:
    condition:
      visible: "Allow"
    then:
      - tap:
          text: "Allow"
          exact: true
```

Adaptation rules:

- Permission text varies by OS version; inspect screenshot/XML before hardcoding.
- Keep permission handling in a reusable `runFlow`.

## GPS Route

Use for location playback, maps, navigation, and geofence tests.

```yaml
platform: android
appId: com.example.app
tags:
  - gps
---
- launchApp
- mockLocation:
    file: ./routes/home-to-office.gpx
    speed: 30
    loop: false
- waitForLocation:
    lat: 21.0278
    lon: 105.8342
    tolerance: 80
    timeout: 60000
- see:
    text: "Arrived"
- stopMockLocation
```

Adaptation rules:

- Always stop mock location in cleanup flows.
- Use `waitForMockCompletion` for full route playback assertions.

## Web Form

Use for browser tests.

```yaml
platform: web
url: https://example.com/login
browser: chromium
tags:
  - web
---
- launchApp
- tap:
    css: "[data-testid='email']"
- inputText: "test@example.com"
- tap:
    css: "[data-testid='password']"
- inputText: "secret"
- tap:
    role: "button"
    text: "Sign in"
- waitUntilVisible:
    text: "Dashboard"
    exact: true
```

Adaptation rules:

- Prefer `css: [data-testid=...]` over visual text when available.
- Use `role + text` for accessible buttons and links.
- Avoid brittle `xpath` or deep CSS chains.

## Failure Recovery Pattern

Use after any failed run:

```bash
lumi-tester list ./flow.yaml --json
lumi-tester run ./flow.yaml --platform android --command-index <failedIndex> --report --snapshot --events-jsonl --output ./output
```

Process:

1. Read `run.json`.
2. Read the failed command screenshot/XML/log.
3. If selector failed, use `suggest_selectors` on the XML artifact when MCP is available.
4. Patch the smallest selector or wait issue.
5. Rerun the failed command.
6. Rerun the whole flow.
