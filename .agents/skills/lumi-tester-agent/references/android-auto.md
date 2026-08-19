# Android Auto Testing

Use this reference for `platform: android_auto` flows backed by Android Auto
Desktop Head Unit (DHU).

## Contents

- Platform model
- Flow template
- Supported commands
- Selector limits
- Debug checklist

## Platform Model

- Android Auto uses an attached Android device plus DHU.
- `appId` is the Android package under test.
- The runner starts DHU and sends DHU console commands for car-screen input.
- No UI hierarchy is available; selector-based waits and assertions are not
  reliable on this platform.
- Treat Android Auto as a graphics/canvas style surface. Use screenshots,
  logs, and deterministic key/point actions.

## Flow Template

```yaml
platform: android_auto
appId: com.example.auto
tags:
  - android-auto
  - smoke
defaultTimeout: 15000
---
- launchApp
- wait: 2000
- tap:
    point: "50%,80%"
- press: navigation
- screenshot: output/android-auto-smoke.png
- stopApp
```

Agent rules:

- Run `doctor --platform android_auto --json` first.
- Run `devices --platform android` and choose the Android device serial.
- Pass the serial with `--device <serial>` when more than one Android device is
  connected.
- Use bounded fixed `wait` after `launchApp` because Android Auto has no UI
  hierarchy for `waitUntilVisible`.
- Prefer `press`/dpad-style actions over raw coordinates when a supported key
  exists.

## Supported Commands

Good Android Auto commands:

- `launchApp`
- `stopApp`
- `tap` with `point`
- `doubleTap` with `point`
- `swipeLeft`, `swipeRight`, `swipeUp`, `swipeDown`
- `back`, `pressHome`, `hideKeyboard`
- `openLink`
- `press`
- `screenshot`
- `wait`

Useful keys:

```yaml
- press: navigation
- press: search
- press: play_pause
- press: media_next
- press: media_previous
- press: back
```

Avoid these Android Auto commands unless local runtime support has changed:

- `waitUntilVisible`, `waitUntilNotVisible`, `see`, `notSee`
- `scrollUntilVisible`
- `inputText`, `eraseText`
- `longPress`, `rightClick`
- `assertScreenshot`
- `startRecording`, `stopRecording`

## Selector Limits

Android Auto tap is point-only in the current DHU driver:

```yaml
- tap:
    point: "50%,80%"
```

Do not use `id`, `text`, `accessibilityId`, `ocr`, `image`, relative selectors,
or `type` for Android Auto unless the local driver has been extended and
validated. The UI dump intentionally returns an empty hierarchy.

## Debug Checklist

When Android Auto fails:

1. Check `doctor --platform android_auto --json` for ADB, Android SDK, and DHU.
2. Confirm the selected Android serial with `devices --platform android`.
3. Inspect `events.jsonl`, screenshot artifacts, and Android logcat output.
4. If a tap misses, adjust the percentage point from the screenshot dimensions.
5. If launch fails, validate the Android package on the phone with ADB before
   changing DHU actions.
