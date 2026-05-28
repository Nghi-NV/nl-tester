# Lumi Tester Command Catalog

Use this reference before writing or repairing Lumi YAML. Prefer the canonical
names below; aliases are accepted but should not be the first choice in new
tests.

## Contents

- Header
- Core app and navigation
- Element actions
- Assertions and waits
- Control flow
- Variables, data, and scripts
- Artifacts and media
- GPS and device state
- Visual assertions
- Command selection rules

## Header

Common header fields:

```yaml
platform: android
appId: com.example.app
url: https://example.com
browser: chromium
tags:
  - smoke
defaultTimeout: 10000
speed: normal
closeWhenFinish: true
env:
  USER_EMAIL: test@example.com
---
```

Use `appId` for Android package names, iOS bundle ids, macOS `.app` paths or
bundle ids, and Windows executable paths. Use `url` for Web tests, and both only
when the flow intentionally bridges app and web context. For macOS/Windows
`clearState`, add a header-level `desktopState.clear` plan and then launch with
`clearState: true`.

## Core App And Navigation

`launchApp`: start the app or open the configured web URL.

```yaml
- launchApp
```

`stopApp`: stop the app under test.

```yaml
- stopApp
```

`back`, `pressHome`, `hideKeyboard`: platform navigation utilities.

```yaml
- back
- hideKeyboard
```

## Element Actions

`tap`: tap an element. Prefer structured selectors.

```yaml
- tap:
    id: "login_button"
- tap:
    text: "Login"
    exact: true
```

`longPress`, `doubleTap`, `rightClick`: same selector model as `tap`.

```yaml
- longPress:
    text: "Delete"
- doubleTap:
    id: "photo"
```

`inputText`: type into the currently focused field. Focus first.

```yaml
- tap:
    id: "email"
- inputText: "test@example.com"
```

`eraseText`: clear text from the active/found field.

```yaml
- eraseText
```

`swipe`: manual gesture. Use this when the direction is known but the target is
not a specific element.

```yaml
- swipe:
    direction: up
```

`scrollUntilVisible`: scroll until an element appears, then stop.

```yaml
- scrollUntilVisible:
    text: "Advanced settings"
    direction: down
```

## Assertions And Waits

`see`: assert that an element is visible.

```yaml
- see:
    text: "Welcome"
    exact: true
```

`notSee`: assert that an element is absent/not visible.

```yaml
- notSee:
    text: "Loading"
```

`waitUntilVisible`, `waitUntilNotVisible`: wait without treating the first miss
as failure.

```yaml
- waitUntilVisible:
    id: "dashboard"
    timeout: 15000
```

`wait`: fixed delay. Use only for animations, debounce, or external systems
after selector-based waits are not available.

```yaml
- wait: 1000
```

## Control Flow

`repeat`: run nested commands a fixed number of times or while a condition is
true.

```yaml
- repeat:
    times: 3
    commands:
      - tap:
          text: "Load more"
```

`retry`: retry flaky nested commands before failing.

```yaml
- retry:
    maxRetries: 2
    commands:
      - tap:
          text: "Continue"
      - see:
          text: "Done"
```

`runFlow`: call another YAML flow or inline a reusable block.

```yaml
- runFlow: ./login.yaml
```

`conditional`: run commands only when a condition is met.

```yaml
- conditional:
    condition:
      visible: "Skip"
    then:
      - tap:
          text: "Skip"
```

`condition.visible` and `condition.visibleRegex` check text. Do not use them
for Android `content-desc` or iOS accessibility identifiers unless the local
runner has been verified to support that selector form.

## Variables, Data, And Scripts

`setVar`, `assertVar`: store and assert runtime variables.

```yaml
- setVar:
    name: email
    value: test@example.com
- assertVar:
    name: email
    equals: test@example.com
```

`runScript`: run a host shell command or local script. A `.js` file path runs
against Lumi variables/context; other strings run through the host shell.

```yaml
- runScript:
    command: "./scripts/setup_db.sh"
    timeoutMs: 30000
- runScript: "./scripts/update_vars.js"
```

Use `evalScript` for inline JavaScript expressions.

`httpRequest`: call an HTTP endpoint as part of setup/assertion.

```yaml
- httpRequest:
    method: GET
    url: https://example.com/health
```

## Artifacts And Media

`screenshot`: capture a screenshot.

```yaml
- screenshot:
    path: "login_screen.png"
```

`startRecording`, `stopRecording`: record video.

```yaml
- startRecording
- stopRecording
```

`captureGifFrame`, `buildGif`, `startGifCapture`, `stopGifCapture`: GIF
workflow commands. Use for visual bug reports, not normal assertions.

## GPS And Device State

`mockLocation`, `stopMockLocation`: simulate GPS.

```yaml
- mockLocation:
    file: "./routes/home-to-office.gpx"
    speed: 30
    loop: false
- stopMockLocation
```

## Visual Assertions

`assertColor`: assert pixel/region color.

```yaml
- assertColor:
    point: "50%,50%"
    color: "#FFFFFF"
```

Use visual assertions only when accessibility/native selectors cannot express
the expected state.

## Command Selection Rules

- Use `waitUntilVisible` before `tap` when the screen is loading.
- Use `see` for user-visible outcomes, not for implementation details.
- Use `scrollUntilVisible` instead of repeated `swipe` when searching a list.
- Use `retry` around external/flaky transitions, not around parser errors.
- Use `runFlow` for login/setup blocks reused by multiple tests.
- Use `screenshot` for evidence, not as a substitute for assertions.
