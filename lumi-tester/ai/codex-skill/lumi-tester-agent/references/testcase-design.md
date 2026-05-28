# Lumi Tester Testcase Design

Use this reference when asked to create enough test cases for an app or web
feature, convert testcase documents into YAML, or organize generated tests.

## Contents

- Coverage loop
- Research inputs
- Coverage model
- Test design techniques
- App and web coverage checklist
- Grouping strategy
- Generated suite example
- YAML authoring rules from testcases
- Coverage matrix template
- Stop conditions

## Coverage Loop

1. Identify the feature, platform, app identity, target environment, user roles,
   and data dependencies.
2. Research the system from product artifacts and runtime behavior before
   writing YAML.
3. Build a small coverage model. Include screens/pages, inputs, permissions,
   states, roles, network conditions, integrations, and platform differences.
4. Generate testcase candidates with the techniques below.
5. Collapse redundant cases with risk and pairwise thinking; keep one stable
   smoke path and focused edge/negative cases.
6. Group tests by required setup. Do not run files independently when they
   require login, onboarding, seeded data, permissions, or a specific state.
7. Write root `setup.yaml`/`teardown.yaml` or explicit `runFlow` setup flows
   first, then leaf scenario files.
8. Validate every generated YAML file, then run by folder/group with reports and
   artifacts.

## Research Inputs

Use every available source to avoid shallow happy-path suites:

- Product requirements, user stories, acceptance criteria, bug reports, release
  notes, analytics funnels, support tickets, API docs, and design files.
- Existing manual testcases, unit/integration tests, Playwright/Appium/Maestro
  tests, QA checklists, and production incidents.
- Runtime exploration: app navigation, UI XML/accessibility tree, DOM,
  screenshots, network logs, permissions, deep links, storage/state, and logs.
- Platform contracts: Android/iOS permission behavior, browser differences,
  responsive breakpoints, OS version differences, and app lifecycle events.

When requirements are incomplete, explore the app/web surface and create a
coverage map from observable screens, forms, actions, states, and error
surfaces. Mark uncertain expectations as `exploratory` until confirmed.

## Coverage Model

Before writing YAML, create a compact model:

- Actors/roles: anonymous, user, admin, wrong role, expired or disabled account.
- Entry points: cold launch, deep link, notification, share/open-with, browser
  URL, refresh/back/forward, resumed app.
- States: fresh install, logged in, logged out, onboarding complete, seeded
  data, empty data, cached data, migrated data, offline/online.
- Objects/data: valid, invalid, duplicate, missing, large, deleted, archived,
  permission-restricted, server-generated, localized.
- Operations: create, view, edit, delete, undo, submit, cancel, retry, search,
  filter, sort, paginate, upload/download, sync.
- Oracles: visible text/state, persisted data, navigation, disabled/enabled
  controls, error messages, permissions, network side effects, no duplicate
  submits, no leaked sensitive data.

Turn the model into a matrix, then choose rows by risk. High-risk rules need
positive, negative, boundary, state-transition, and permission/network variants.

## Test Design Techniques

- Equivalence partitioning: choose one representative from each valid and
  invalid input class.
- Boundary value analysis: test min, max, just below, just above, empty, and
  oversized values where limits exist.
- Decision tables: cover combinations of rules, permissions, roles, feature
  flags, payment/subscription status, and validation messages.
- State transition testing: cover allowed and disallowed transitions such as
  logged out -> logged in, draft -> saved, offline -> online, pending -> done.
- Pairwise/combinatorial testing: when many parameters interact, cover pairs
  instead of full Cartesian explosion unless risk requires more.
- Use-case testing: cover end-to-end user journeys, not only isolated widgets.
- Error guessing/risk testing: add cases for flaky backends, expired sessions,
  duplicate submits, retry, timeout, empty data, slow media, and interrupted
  navigation.
- Exploratory chartering: when behavior is unknown, run focused exploration for
  one area, save artifacts, then convert stable findings into regression cases.
- CRUD matrix: for each entity, cover create/read/update/delete plus duplicate,
  undo, permissions, stale object, and concurrent or repeated submit behavior.
- Lifecycle testing: cover cold start, background/foreground, rotation/resize,
  refresh, app kill/relaunch, and resume from interrupted flows when supported.
- Accessibility/i18n smoke: cover stable accessibility labels, dynamic text,
  long localized strings, RTL if relevant, and font scale/responsive layout.
- Regression selection: tag critical smoke, risky changed flows, and full
  regression separately.

## App And Web Coverage Checklist

Functional:

- Happy path, alternate path, cancel/back path, retry path.
- Empty, loading, success, partial success, error, timeout.
- Create, read, update, delete, undo, duplicate, idempotency.
- Search/filter/sort/pagination/infinite scroll.
- Deep link, notification entry, share/open-with, browser refresh/back/forward.
- App/web lifecycle: cold launch, background/resume, refresh, reconnect,
  interrupted action, duplicate tap/submit.

Inputs:

- Required/missing fields, invalid format, duplicate value, max length, unicode,
  emoji, leading/trailing spaces, multiline, paste, keyboard hide/show.
- Numeric boundaries, date/time/timezone, currency/locale, file size/type.

Auth and session:

- Logged out, logged in, expired session, wrong role, disabled account.
- Login prerequisites should live in setup flows or grouped folders, not copied
  into every leaf test.

Permissions and privacy:

- First-run permission allow, deny, deny forever, revoke after grant.
- Android runtime permissions, iOS permission dialogs, camera/microphone/photos,
  location while-in-use/always, notifications, storage.
- Permission states use `allow` or `deny`.
- Android supported short keys include `camera`, `microphone`/`mic`,
  `location`/`gps`, `coarse_location`, `contacts`, `phone`/`call`, `sms`,
  `storage`/`files`, `write_storage`, `calendar`, `notifications`, and `all`.
- iOS permission mutation is simulator-only. Supported keys include `calendar`,
  `contacts`, `contacts-limited`, `location`/`gps`, `fine_location`,
  `coarse_location`, `location-always`, `background_location`, `photos`,
  `gallery`, `photos-add`, `microphone`, `record_audio`, `camera`,
  `media-library`, `motion`, `sensors`, `reminders`, `siri`, `faceid`,
  `homekit`, `health`, and `all`.
- Do not assume `permissions: { all: allow }` is always correct. Use it for
  smoke setup only when the testcase requires pre-granted permissions.

State and data:

- Fresh install, existing user data, migrated data, cache present, cache cleared.
- Use `clearState: true` only for first-run/reset cases. It may log out users,
  remove seeded data, trigger onboarding, or expose app launch crashes.
- For authenticated or data-dependent suites, prefer explicit setup/login flows
  and seeded data over `clearState` in every file.

Environment:

- Online/offline, slow network, API failure, server error, retry.
- Portrait/landscape, small/large screen, font scale, dark/light mode.
- Android/iOS version differences and Web browser differences when relevant.

Security-focused Web/API smoke:

- Authentication, authorization, session management, input validation, upload,
  redirect/deep-link handling, and sensitive data exposure checks.

Web-specific:

- Browser back/forward, reload, direct URL access, responsive breakpoints,
  focus/keyboard navigation, form autofill, cookies/local storage/session
  storage, file upload/download, tabs/windows, and cross-browser differences.

Mobile-specific:

- Runtime permissions, app lifecycle, orientation, keyboard overlays, OS dialogs,
  push/deep-link entry, no-network/airplane-like behavior, device locale/time,
  and small/large screen variants.

## Grouping Strategy

Use folders when scenarios share state:

```text
tests/generated/<feature>/
  cases.csv                 # testcase matrix: id, risk, tags, yaml path
  setup.yaml                # auto-runs once before collected main files
  teardown.yaml             # auto-runs once after collected main files
  data/
    users.csv
  subflows/                 # skipped by directory runs; call with runFlow
    login.yaml
    seed_data.yaml
    grant_permissions.yaml
  smoke/
    001_open_feature.yaml
    002_primary_happy_path.yaml
  regression/
    validation/
    permissions/
    state/
    web/
    ios/
    android/
```

When the repo already has a test layout, follow it instead of forcing this
shape. Keep generated tests under a feature folder such as
`tests/generated/<feature>/` or the repo's equivalent, so artifacts from a
testcase batch stay together.

Directory runs automatically skip files named `setup.yaml`, `setup.yml`,
`teardown.yaml`, and `teardown.yml`, then execute root setup/teardown hooks
around the main files. Directories named `subflows/` are skipped during
directory collection; call those reusable flows explicitly with `runFlow`.
If a scenario needs per-file setup, call an explicit `runFlow` inside that
scenario or run self-contained files separately. Nested directory hooks are not
auto-applied unless that nested directory is the folder passed to `run`.

Run a folder/group when files depend on shared setup:

```bash
lumi-tester validate tests/generated/login --json
lumi-tester list tests/generated/login --json
lumi-tester run tests/generated/login --platform android --report --snapshot --events-jsonl --output ./output/login
```

Run a single file only when it is explicitly self-contained.

When a scenario file lives in a subdirectory, use relative paths from that file
for explicit setup flows:

```yaml
- runFlow: "../subflows/login.yaml"
```

## Generated Suite Example

Use this shape when converting a testcase matrix into runnable files. Keep
shared login, permissions, and seeded state outside leaf tests.

`tests/generated/account/settings/setup.yaml`:

```yaml
platform: android
appId: com.example.app
tags:
  - setup
defaultTimeout: 15000
---
- launchApp:
    appId: com.example.app
    permissions:
      notifications: allow
- waitUntilVisible:
    accessibilityId: "Login"
    timeout: 15000
- runFlow: "./subflows/login.yaml"
- waitUntilVisible:
    accessibilityId: "Settings"
    timeout: 15000
```

`tests/generated/account/settings/subflows/login.yaml`:

```yaml
platform: android
appId: com.example.app
tags:
  - subflow
  - login
defaultTimeout: 15000
---
- tap:
    accessibilityId: "Email"
- inputText: "${USER_EMAIL}"
- tap:
    accessibilityId: "Password"
- inputText: "${USER_PASSWORD}"
- hideKeyboard
- tap:
    accessibilityId: "Login"
```

`tests/generated/account/settings/regression/001_toggle_notifications.yaml`:

```yaml
platform: android
appId: com.example.app
tags:
  - regression
  - settings
  - TC-SETTINGS-001
defaultTimeout: 10000
---
- waitUntilVisible:
    accessibilityId: "Settings"
    timeout: 15000
- tap:
    accessibilityId: "Notifications"
- see:
    accessibilityId: "Notifications enabled"
```

Validate and run the folder, not the leaf file, when the suite depends on root
setup or shared state:

```bash
lumi-tester validate tests/generated/account/settings --json
lumi-tester list tests/generated/account/settings --json
lumi-tester run tests/generated/account/settings --platform android --report --snapshot --events-jsonl --output ./output/account-settings
```

Do not copy this selector text blindly. Replace selectors with values from UI
XML/accessibility tree/DOM, then validate before running.

## YAML Authoring Rules From Testcases

- Put testcase id and requirement id in `tags` when available.
- Keep a `cases.csv` or equivalent matrix near generated YAML when creating a
  suite from many testcase rows.
- Keep one user intent per scenario file unless the testcase is an end-to-end
  journey.
- Use `launchApp` followed by selector-based readiness waits.
- Use semantic selectors from UI XML/DOM/accessibility tree. Avoid coordinates.
- Put login, permission setup, mock location, seeded data, and cleanup in
  reusable `runFlow` files when multiple tests need them.
- For permission testcases, write separate flows for allow and deny behavior.
- For clear-state testcases, make the reset explicit in the testcase name and
  expected assertions.

## Coverage Matrix Template

Use this compact table before writing YAML:

```text
Requirement | Source | Risk | Platform | State | Role | Entry point | Data class | Permission | Network | Expected result | YAML file
```

Suggested `cases.csv` columns:

```csv
id,requirement,source,risk,platform,tags,state,role,entry_point,data_class,permission,network,expected,yaml
```

Mark each row as one of:

- `smoke`: must pass on every build.
- `regression`: broader behavior coverage.
- `negative`: invalid input/error/security behavior.
- `exploratory`: needs artifacts or manual confirmation before automation.

## Stop Conditions

Before claiming coverage is enough, verify:

- Every requirement/user story has at least one testcase or a documented reason.
- Each high-risk rule has positive and negative coverage.
- Boundary and invalid data are covered for user inputs.
- Permission, clearState, auth/session, and offline behavior are intentionally
  included or explicitly out of scope.
- Test files validate, grouped dependencies are runnable, and reports/artifacts
  are produced for debug.
