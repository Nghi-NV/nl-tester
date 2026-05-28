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
- For iOS, check `idb`.
- For Web/video capture, check `ffmpeg`.
