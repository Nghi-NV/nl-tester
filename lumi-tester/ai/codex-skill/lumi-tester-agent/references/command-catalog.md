# Lumi Tester Command Catalog

Use this reference before writing or repairing Lumi YAML. Prefer the canonical
names below; aliases are accepted but should not be the first choice in new
tests.

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

Use `appId` for Android/iOS app tests, `url` for Web tests, and both only when
the flow intentionally bridges app and web context.

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
    when:
      visible:
        text: "Skip"
    commands:
      - tap:
          text: "Skip"
```

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

`runScript`: run JavaScript against variables/context.

```yaml
- runScript: "vars.count = 1 + 1"
```

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
    name: login_screen
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
    latitude: 21.0278
    longitude: 105.8342
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
