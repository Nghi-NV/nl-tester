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

App launch/crash/abort:

- Treat `appCrashed`, `FATAL EXCEPTION`, `Force finishing`, `START_ABORTED`,
  `am_crash`, tombstone output, or a missing app process as app/runtime
  failures, not selector failures.
- Check the event stream, reports, current focus, process state, and recent logs:

```bash
rg -n 'appCrashed|START_ABORTED|FATAL EXCEPTION|Force finishing|am_crash|tombstone' ./output
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> shell pidof <appId>
adb -s <serial> logcat -d -v time | rg -i '<appId>|FATAL EXCEPTION|START_ABORTED|am_crash|tombstone'
```

- If `clearState` causes launch aborts or data-dependent crashes, rerun once
  without `clearState` to separate first-run app failures from test authoring
  mistakes.
