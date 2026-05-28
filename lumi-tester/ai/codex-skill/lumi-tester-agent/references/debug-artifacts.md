# Lumi Tester Debug Artifacts

Use this reference when a run fails and the user wants a diagnosis or a patch.

## Artifact Priority

1. `run.json`: first place to find summary, failed command indexes, errors, and
   artifact paths. This should exist even without `--report` after executor
   finalization.
2. `test-results.json`: report-mode session data for HTML/JUnit generation.
3. `events.jsonl`: event stream. Useful for timing/order issues.
4. `fail_*_cmdN_*.png`: screenshot at failure.
5. `fail_*_cmdN_*.xml`: UI hierarchy at failure.
6. `fail_*_cmdN_*.log`: recent device/browser logs.

## Event JSONL

Each line is one serialized `TestEvent`. Useful event types include:

- `sessionStarted`
- `flowStarted`
- `commandStarted`
- `commandPassed`
- `commandFailed`
- `commandSkipped`
- `appCrashed`
- `sessionFinished`

Use `commandFailed.index` to rerun:

```bash
lumi-tester run ./test.yaml --platform android --command-index <index> --report --snapshot --events-jsonl --output ./output
```

## Common Failure Diagnosis

Wrong app or package mismatch:

- Compare the expected `appId` with the package shown in current focus, failure
  UI XML, screenshot, and logs.
- If the failure UI XML package differs from the expected app, stop tuning
  selectors. First diagnose wrong device selection, stale YAML `appId`, launch
  failure, crash, or a system dialog overlay.
- For "current app" requests, rediscover foreground package/activity instead of
  reusing an unrelated YAML file:

```bash
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> exec-out uiautomator dump /dev/tty
```

Element not found:

- Prefer replacing `point` with `id`, `desc`, or exact `text`.
- If text exists in screenshot but not XML, try `ocr`.
- If multiple elements match, add `index`, `type`, or a relative anchor.

Assertion timeout:

- Add `waitUntilVisible` before interaction/assertion.
- Increase `defaultTimeout` only after selector quality is verified.

Wrong text input:

- Ensure the target field is tapped before `inputText`.
- Do not combine selector and text entry in one command unless verified by
  `validate --json` and local examples.

Runtime dependency failure:

- Run `doctor --platform <platform> --json`.
- For Android, check `adb`.
- For iOS, check `idb`; on macOS install it with
  `brew tap facebook/fb && brew install idb-companion` when missing.
- For Web/video capture, check `ffmpeg`.

Platform-specific target checks:

- Android: compare `appId` with `mCurrentFocus`, UI XML `package`, `pidof`, and
  recent `logcat` lines.
- iOS: compare `appId` with the bundle id from `simctl listapps`, the
  frontmost app from simulator/device tooling, the accessibility hierarchy, and
  recent device logs. If a system permission alert is frontmost, handle the
  dialog or set permission state before tuning selectors.
- Web: compare the requested `url` with the actual browser page URL, title,
  screenshot, DOM/hierarchy artifact, console errors, failed network requests,
  and storage/session state. If the page is still loading, blocked by auth, or
  redirected, fix navigation/setup before tuning selectors.

Use platform-specific evidence to classify the failure before editing YAML:

- Wrong target: foreground app/page does not match `appId` or `url`.
- Setup/state issue: login, onboarding, permission, storage, seed data, or
  `clearState` changed the expected screen.
- App/runtime issue: crash, aborted launch, browser console fatal error, or
  failed required network call.
- Selector issue: expected screen is correct but the target element selector is
  absent, unstable, duplicated, or appears after a wait.

App launch/crash/abort:

- Treat `appCrashed`, `FATAL EXCEPTION`, `Force finishing`, `START_ABORTED`,
  `am_crash`, tombstone output, iOS process exit/crash logs, browser page crash,
  console fatal errors, or a missing app process as app/runtime failures, not
  selector failures.
- Check the event stream, reports, current focus, process state, and recent logs:

```bash
rg -n 'appCrashed|START_ABORTED|FATAL EXCEPTION|Force finishing|am_crash|tombstone' ./output
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> shell pidof <appId>
adb -s <serial> logcat -d -v time | rg -i '<appId>|FATAL EXCEPTION|START_ABORTED|am_crash|tombstone'
xcrun simctl listapps booted | rg '<bundleId>'
xcrun simctl spawn booted log show --last 5m --style compact | rg -i '<bundleId>|crash|exception|abort'
rg -n 'console|network|pageerror|crash|ERR_|4[0-9]{2}|5[0-9]{2}' ./output
```

- If `clearState` causes launch aborts or data-dependent crashes, rerun once
  without `clearState` to separate first-run app failures from test authoring
  mistakes.
