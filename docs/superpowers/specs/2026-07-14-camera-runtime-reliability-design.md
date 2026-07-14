# Camera Runtime Reliability Design

## Goal

Make camera-backed Lumi Tester flows reliable across interactive use, CI, macOS,
Linux, and Windows without changing existing camera command semantics.

The work addresses the reviewed failures in FFmpeg snapshot capture, observation
side effects, HTTP frame deadlines, camera launcher discovery, platform
selection, and the brittle Lumi Life home selector.

## Scope

This design covers two delivery phases:

1. Camera runtime correctness: FFmpeg process handling, explicit observation,
   RTSP versus HTTP frame-source selection, and bounded frame deadlines.
2. Compatibility and test stability: cross-platform discovery, environment
   precedence, platform detection, schema documentation, and selector cleanup.

The following are explicitly out of scope:

- Replacing FFmpeg with a new decoder.
- Adding a general-purpose HTTP client dependency.
- Reworking camera detection or calibration algorithms.
- Changing the existing camera YAML command names or state model.
- Refactoring unrelated executor, driver, or recorder code.

## Architectural Decisions

### Frame sources remain explicit

`CameraSession` continues to support two sources:

- RTSP uses a warm `FfmpegGrabber` and remains the default when `camera.rtsp` is
  configured.
- HTTP uses an explicitly configured `camera.server` and requests
  `/api/frame.jpg` from that server.

Starting an observation UI must not change the selected frame source. In
particular, the executor must not overwrite an RTSP configuration with a
localhost server URL.

### Observation is opt-in

The executor starts or opens a live view only when `camera.observe: true`.

- With `server`, it opens `<server>/view` and does not start another server.
- With `rtsp` and `profile`, it starts the read-only local observation server,
  then opens its `/view` route after the server is ready.
- With `observe: false`, it opens no browser, binds no port, and starts no
  observation task.

Observation failure is logged but does not silently replace the RTSP source.
Camera commands subsequently use the source declared in YAML.

### Deadlines flow downward

Camera frame acquisition accepts a caller-supplied deadline. Every blocking
operation derives its remaining timeout from that deadline:

- TCP connect/read/write timeouts never exceed the remaining command time.
- HTTP retries stop at the caller deadline.
- Camera polling loops do not start another frame request after expiry.

One-shot operations use a bounded default deadline. Wait and blink commands use
their configured command deadlines. A lower layer must not introduce an
independent 20-second retry window.

### FFmpeg output is drained while the process runs

Snapshot capture must consume stdout and stderr concurrently so FFmpeg cannot
block on a full OS pipe. The parent waits for one of two outcomes:

- FFmpeg exits: join both reader threads, validate status, and decode stdout.
- Deadline expires: kill and reap FFmpeg, join both readers, and return a
  timeout error.

Errors continue to redact RTSP credentials.

### Discovery is platform-neutral and deterministic

Camera launcher discovery uses `Path::components()` rather than searching for
slash-delimited strings. RTSP value precedence is:

1. Explicit CLI `--rtsp` argument.
2. Process environment `CAMERA_RTSP`.
3. `CAMERA_RTSP` from the discovered `.env` file.
4. RTSP stored in the selected profile where the low-level command already
   supports it.

File discovery remains deterministic by sorting candidates. Doctor output
identifies the selected profile and test YAML so an unintended match is visible.

### Camera test uses normal platform detection

`lumi-tester camera test` validates the selected YAML and then uses the same
`detect_platform()` behavior as `lumi-tester run`. Android remains the fallback
only when the YAML supplies no platform.

### Stable selectors precede coordinates

The Lumi Life `select_home` subflow taps the home using a stable hierarchy
selector. A percentage point is allowed only if device evidence shows the
element cannot be selected through id, description, accessibility, text, or
regex. If a coordinate is necessary, the YAML must explain the evidence in a
short comment and retain a visible-state assertion before and after the tap.

## Component Changes

### `src/camera/stream.rs`

- Introduce a bounded FFmpeg child-output collection helper.
- Drain stdout and stderr concurrently.
- Kill and reap timed-out children.
- Preserve JPEG decoding and credential-redacted errors.

### `src/camera/session.rs`

- Pass deadlines into frame acquisition.
- Bound HTTP connect, write, read, and retry behavior by the remaining time.
- Keep RTSP restart behavior within the same deadline.
- Test server-frame acquisition using a local fake TCP server.

### `src/runner/executor.rs`

- Restore the `observe` guard.
- Stop mutating `camera.server` when a local view starts.
- Pass command deadlines to camera session operations.
- Keep observation tasks best-effort and isolated from detection sessions.

### `src/camera/launcher.rs`

- Make profile discovery path-component based.
- Read process environment before `.env`.
- Add tests for Windows separators and environment precedence without depending
  on the developer machine's environment.

### `src/main.rs`

- Reuse normal platform detection for `camera test`.
- Preserve all existing shortcut and low-level camera commands.

### Schema, documentation, and VS Code metadata

- Document `camera.server` and `camera.observe` in the bundled schema.
- Update VS Code camera header metadata if that metadata exists in the current
  extension schema.
- Document that `server` selects an existing HTTP frame source while `observe`
  controls whether a live UI is opened.

### Lumi Life flow

- Replace the new coordinate-only home tap with the strongest verified selector.
- Validate and list the flow before any device execution.

## Error Handling

- FFmpeg timeout errors state the elapsed limit and redact the RTSP URL.
- HTTP frame errors report the server authority and final cause without
  retrying beyond the command deadline.
- Unsupported server schemes fail immediately with the supported `http://`
  format.
- An unresolved RTSP or server variable fails before starting a session.
- Observation startup failures are emitted as warnings and never redirect the
  detection source.
- Discovery errors name the missing input and show an explicit override command.

## Testing Strategy

All behavior changes follow red-green TDD.

1. FFmpeg process tests use a temporary executable/script that writes more than
   the OS pipe capacity and a second script that never exits. Tests prove large
   output is drained and timeout kills the child.
2. HTTP tests use a local TCP listener for successful JPEG, connection failure,
   delayed response, and deadline expiry.
3. Executor tests prove `observe: false` creates no observation task and does
   not modify the configured source; `observe: true` creates one task.
4. Launcher tests cover process-environment precedence and path-component based
   profile matching.
5. CLI tests prove `camera test` honors a non-Android YAML platform.
6. Schema tests prove `server` and `observe` validate in single and named camera
   configurations.
7. Lumi YAML is checked with `validate --json` and `list --json`; device testing
   is limited to the changed selector flow and camera smoke flow.

The final verification set is:

```bash
cargo fmt --check
cargo test --lib
cargo test camera_ -- --nocapture
cargo run -- validate e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml --json
cargo run -- validate e2e/workspaces/lumi_life/subflows/select_home.yaml --json
cargo run -- list e2e/workspaces/lumi_life/subflows/select_home.yaml --json
```

Runtime verification additionally uses a real camera only after all offline
checks pass.

## Delivery Order

Phase 1 lands runtime fixes as independently reviewable commits:

1. FFmpeg timeout-safe output capture.
2. Deadline-aware camera session I/O.
3. Explicit, isolated observation behavior.

Phase 2 lands compatibility changes:

1. Cross-platform launcher discovery and environment precedence.
2. Platform detection and camera schema/documentation.
3. Stable Lumi Life selector plus targeted device verification.

## Acceptance Criteria

- An FFmpeg snapshot larger than the OS pipe capacity completes without
  deadlock.
- A hung FFmpeg process is killed and reaped at the configured deadline.
- `observe: false` never opens a browser or binds an observation port.
- Starting observation does not alter the camera source used by assertions.
- HTTP camera reads cannot outlive the parent command deadline.
- Profile discovery works with Windows and Unix path components.
- Process `CAMERA_RTSP` overrides `.env`.
- `camera test` honors the YAML platform.
- Camera schema and extension metadata describe `server` and `observe`.
- The changed Lumi Life flow validates and uses the strongest available
  selector.
- Offline tests pass before any hardware test is attempted.
