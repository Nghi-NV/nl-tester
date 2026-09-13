# Lumi Tester Testcase Design

Use this reference when asked to create enough test cases for an app or web
feature, convert testcase documents into YAML, or organize generated tests.

## Contents

- Coverage loop
- Research inputs
- SRS requirements format
- Systematic app exploration loop
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
   writing YAML. When no spec/existing tests are available, run the
   "Systematic App Exploration Loop" below first - do not start writing
   YAML from a guess at what the app contains.
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
surfaces. Mark uncertain expectations as `exploratory` until confirmed. See
"Systematic App Exploration Loop" below for the concrete process - this is
the normal case for a real assignment ("test this app"), not a fallback.

When the user DOES have a real SRS/requirements document, do not leave it as
loose prose you re-read from scratch every session - convert it into the
structured `requirements/` format below once, then treat it as the normative
coverage source alongside the exploration loop. See "SRS Requirements
Format" for the concrete schema and the mandatory coverage check.

## SRS Requirements Format

A pure-YAML, multi-file format for turning a real SRS into something an
agent can mechanically check itself against - designed after auditing
`google-labs-code/design.md` (a YAML-frontmatter spec for design tokens) and
adapting only what transfers: one normative machine-readable block, a fixed
small vocabulary reused everywhere, and an explicit field for intentional
omissions instead of leaving them to prose that can be forgotten. Unlike
DESIGN.md this is pure YAML, no markdown prose layer - every prior draft
that allowed free-text explanation sections turned out to be exactly where
acceptance criteria got lost, so nothing here is expressible only as prose.

**Why this exists**: exploring a running app (the loop below) only proves
what is CURRENTLY VISIBLE. It cannot prove a requirement was silently never
built, gated behind a flag, or removed - clicking a UI can never demonstrate
the absence of a feature the spec promised. Cross-checking against a real
requirements source is the only way to catch that class of gap, which is
why this section exists as a companion to the exploration loop, not a
replacement for it.

### File Layout

```text
requirements/
  index.yaml     # metadata, policy, includes list, reconciliation
  common.yaml    # shared: cross-cutting items (popups, alerts, validation
                 # rules, network/session error behavior) used by 2+ modules
  <module>.yaml  # requirements: for one module/feature area (one file per
                 # module keeps each file short enough to actually read)
```

Same shape as `runFlow` composing subflow files: `index.yaml` is the entry
point, everything else is included and merged into one in-memory model - not
a new concept to learn.

### `index.yaml`

```text
version: alpha
name: <app/feature name>
source: <path to the original SRS>
author: <who created this file>
created_at: 2026-09-11
updated_at: 2026-09-11
tags: [speaker, iot]        # classification tags for the whole set

platforms:                   # optional - every build the app ships on. One
                              # entry per platform this SRS covers, using the
                              # SAME field names a testcase file's own
                              # frontmatter already uses for that platform
                              # (`appId` for android/ios/macos/windows, `url`
                              # for web) - not a new naming scheme to learn.
  android:
    appId: com.example.app
  ios:
    appId: com.example.app
  web:
    url: https://example.com
  macos:
    appId: /Applications/Example.app
  windows:
    appId: C:\Program Files\Example\Example.exe

screens:                     # optional - the flat list of screen/page ids
                              # this app has, declared once so every
                              # requirement's `targets` and every shared
                              # item's `targets` reference the SAME id
                              # instead of each file inventing its own name
                              # for "the login screen". Formalizes what was
                              # previously left to an ad-hoc `screen-map.csv`.
  - login
  - register
  - dashboard
  - settings

policy:                      # required test tags per requirement priority -
  must: [smoke, negative]    # stated once here, never repeated per requirement
  should: [regression]
  could: [exploratory]

common: requirements/common.yaml    # or a list, once shared items outgrow one file:
# common:
#   - requirements/common.yaml
#   - requirements/common_hardware.yaml
includes:
  - requirements/auth.yaml
  - requirements/settings.yaml

reconciliation:              # filled in DURING exploration, not authored upfront
  missing_targets: []        # requirement has no matching screen/button found
  undocumented_targets: []   # screen/button found with no matching requirement
  shared_gaps: []            # a shared item's `targets` list has an unverified entry
```

`common` accepts either shape - a single path (the original form) or a list
of paths, read in order and merged. Split by concern once one file gets hard
to scan (e.g. `common_auth.yaml`, `common_hardware.yaml`) rather than as a
default; most projects never need more than one file. A path in the list
that does not exist yet is not an error, same tolerance as the single-path
form - a project may declare a file it hasn't populated yet.

`platforms`/`screens` are pure metadata - the Rust coverage checker
(`requirements-coverage`) ignores unknown top-level fields, so adding them
never breaks an existing `index.yaml` that omits both. Use them when the
same requirement set spans more than one platform build, or the project is
big enough that `targets` values need one shared vocabulary instead of every
module file inventing its own screen names. A single-platform, single-module
project can omit both and lose nothing.

When one requirement's `expect`/`reject` genuinely differs by platform (e.g.
the OS permission dialog's exact button text), do not split it into two
requirement ids - keep one id, prefix the differing lines inline
(`"Android: ..."` / `"iOS: ..."`), and only add the platform name to `tags`
when the whole requirement is platform-exclusive, not just one bullet
inside it.

### `common.yaml` (cross-cutting `shared:` items)

```text
shared:
  error_toast:
    label: Thông báo lỗi chung   # optional - free-text display name, see below
    targets: all             # or a list of specific screens/buttons
    expect:
      - Hiện trong 1s sau khi lỗi xảy ra
      - Tự tắt sau 3s, không chặn thao tác khác
  network_offline:
    targets: all
    expect:
      - "Hiện {error_toast}, không crash, cho phép retry"
```

`label` is optional and purely for display - the map key (`error_toast` above)
is the technical id actually used inside `{error_toast}` reference tokens, so
it stays a safe identifier (letters/digits/`_`/`.`/`-`). `label` exists for
the common case where the natural name for a shared item is free-text (e.g.
Vietnamese with diacritics and spaces) that could never work as a `{...}`
token - same split as a requirement's `REQ-ID` key vs. its free-text `title`.
Omit it when the id already reads fine on its own.

`targets` is deliberately not called "screens" - on a hardware/jig test
target (see `hardware`/`camera` modules), the applicable unit is a
button/region id like `device_2.button_1`, not a UI screen. The same field
name covers both; only what a coverage check cross-references against
differs (`screen-map.csv` for UI, the camera/jig profile's button list for
hardware) - add `domain: ui | hardware` at the `index.yaml` level when a
project needs to be explicit about which.

### `<module>.yaml` (one file per module)

```text
requirements:
  REQ-AUTH-001:
    title: Đăng nhập bằng số điện thoại
    priority: must              # must | should | could - drives `policy` above
    tags: [login, otp]           # only if it differs from the file-level default
    expect:
      - SĐT + OTP hợp lệ -> vào Home
      - "Sai OTP 3 lần -> khoá 5 phút, hiện {error_toast}"
    reject:
      - SĐT không tồn tại
      - OTP hết hạn
    edge:
      - "Mất mạng giữa lúc gửi OTP ({network_offline})"

  REQ-AUTH-002:
    title: Quên mật khẩu
    priority: should
    skip: "Chưa triển khai trong bản build hiện tại (SRS mục 4.3)"
```

Exactly six keywords, reused everywhere, matching how `tap`/`see`/`wait` get
reused across YAML testcases instead of inventing a new verb per situation:
`priority`, `expect`, `reject`, `edge`, `skip`, `targets`. A `{name}` inside
any string is a reference into `common.yaml`'s `shared:` map - written
inline where it is relevant, not declared in a separate "uses" list.

Deliberately NOT included: a `status`/`testcases` field on each requirement.
Coverage must never be self-reported by whoever last edited the file - it is
computed from `cases.csv` (which yaml file references this requirement id)
and whether that yaml file exists on disk, the same "verify, don't trust a
claimed state" discipline as everywhere else in this document. Wire this
into the same coverage check invoked by "Stop Conditions" below: no
requirement may be reported as covered - and no suite may be reported as
"done" - unless it is either non-`skip`ped with at least one real, existing
testcase file referencing its id in `cases.csv`, or explicitly `skip`ped
with a `reason`. Left with neither is a gap that must be surfaced to the
user, never silently dropped.

## Systematic App Exploration Loop

Use this when you are handed an app/page with **no spec, no existing tests,
no walkthrough** - the actual default case for most real assignments, not an
edge case. The goal is a written screen map you can point to as proof nothing
was skipped, not a mental impression of "I clicked around a bit."

This is a breadth-first traversal: fully inventory the CURRENT screen before
following any one path deep, so sibling actions are not left unexplored
because you got absorbed in one flow.

1. **Launch cold.** `launchApp: { clearState: true }` once, record the
   first-run/onboarding screens separately (they often never appear again in
   the same session) - then relaunch normally (no `clearState`) for the main
   traversal so you are not repeatedly fighting onboarding.
2. **Inventory the current screen - including what is off-screen.** Call
   `suggest_selectors` (or `lumi-tester suggest-selectors --platform <p>
   --device <d>`) with **no** `query`/`point` and a high `limit` (e.g. 50),
   `includeNonClickable: true`. This returns every visible/interactive
   element with bounds and a ranked selector each - the screen's full
   inventory, not just what you happened to notice. **This only sees what is
   currently rendered.** Long lists are commonly virtualized on mobile - an
   item below the fold does not exist in the tree at all until scrolled into
   view. If the screen scrolls, `scrollUntilVisible`/`swipe` to the bottom
   and re-run the inventory at each stop before treating the screen as fully
   catalogued; do not assume the first dump is complete just because nothing
   obviously indicates more content. Pair with a screenshot
   (`inspector_get /api/screenshot` or `--snapshot` on a real run) for a
   visual record next to the element list.
3. **Record the screen** in a screen map (see format below) before touching
   anything else on it: an id, how you reached it (parent screen + the exact
   action), the screenshot path, and the full element list from step 2.
4. **Turn every clickable element into a candidate edge** to explore from
   this screen - not just the ones that look obviously important. An
   element you skip because it "looks like a settings toggle" is exactly the
   kind of thing coverage gaps are made of.
5. **Follow one edge at a time.** Tap it, wait for the UI to settle, repeat
   step 2 on the result:
   - New element inventory / new screenshot -> a new screen. Add it to the
     map and continue the loop from it.
   - Same screen re-appears (a toggle, a no-op tap, a modal that closed
     itself) -> mark the edge explored, no new screen.
   - A dialog/sheet/permission prompt appeared over the same screen -> still
     a new screen entry (dialogs are exactly where coverage gaps hide).
6. **Navigate back to the parent screen** after each edge before trying the
   next sibling edge on the same screen - explore breadth-first, not by
   drilling arbitrarily deep down the first interesting path and losing track
   of what else was on the starting screen.
7. **Explicitly track these edge types separately** - they are the ones most
   often missed because they are not a plain tap:
   - Long-press / context menu (`longPress`, not `tap`)
   - Swipe-to-reveal row actions, pull-to-refresh (`swipe`)
   - Empty state vs. populated state (may need seeded data / no data as two
     separate visits to the same screen)
   - Error/offline state (`setNetwork`/airplane-mode-style simulation)
   - Permission dialogs - allow and deny are two separate branches, not one
   - Deep links / notification / share-target entry points - these are not
     reachable by tapping from within the app at all; try them explicitly
     (`openLink`) as their own root-level entries in the map
   - Destructive actions (delete, logout, clear data) - explore these last on
     a given branch, note them as "requires re-seed/relaunch to continue"
     rather than tapping through blind, since they invalidate the rest of
     that branch's state
8. **Stop when** every discovered screen has zero unexplored edges, or you
   hit an explicit budget (state the number you used, e.g. "40 screens/40
   dialogs for this app" - do not silently stop and imply completeness).
   Report the budget and the remaining unexplored-edge count if you stop
   early; do not claim full coverage you did not reach.

   **Default budget when none is given.** If the user did not state a time
   box or scope, do not stop after an arbitrary small number of screens on
   your own judgment - that under-delivers silently. Default to: visit every
   top-level navigation destination (every tab/menu entry reachable from the
   first screen) at least once and record it in the screen map with a
   `smoke`-level test, before stopping. Only leave a top-level destination at
   `status: partial`/unvisited if it is genuinely out of scope (destructive,
   needs unavailable data/role) or you explicitly ask the user whether to go
   deeper - never because time ran out without saying so. Depth *within* a
   given destination (every sub-page, every list item) is where "state your
   budget and stop" applies; breadth *across* top-level destinations is not
   optional by default.
9. **One traversal is one role/state - repeat it, do not assume it
   generalizes.** A single BFS pass only maps what one account/data state can
   see. `Actors/roles` and `States` in the Coverage Model below are not just
   theory to apply to screens you already found - a different role or state
   commonly exposes screens/elements the first pass never reaches at all
   (an admin menu, an "upgrade" banner only shown to free-tier accounts, a
   resume-onboarding screen only shown to a partially-registered account).
   Run the loop again per role/state that plausibly changes the UI, and merge
   the resulting screen maps rather than treating the first one as the app's
   full surface.
10. **Feed the map into the Coverage Model below.** For each discovered
   screen, run it through the coverage checklist (happy/edge/negative,
   permissions, states, network) to generate testcases - exploration produces
   the inventory, the Coverage Model/Test Design Techniques sections below
   turn that inventory into actual test cases.
11. **Re-run after app updates, and diff the screen maps.** A screen map is a
    snapshot of one app version. On the next version, re-run the loop and
    compare against the saved map - new screens/elements are coverage gaps
    waiting to happen, removed ones are dead test cases to retire, changed
    ones are where selectors are most likely to break next.

### What Clicking Can't Discover

The traversal above only maps what UI navigation can reach. It structurally
cannot find these - each needs its own deliberate investigation, not more
clicking, and none of them show up as an "unexplored edge" in the screen map:

- **Time/schedule-dependent behavior**: features gated by date, session
  expiry, subscription renewal, rate limits that reset on a timer.
- **Feature flags / A-B variants / gradual rollouts**: the build you are
  exploring may not have the flag enabled that another user's build does -
  ask what flags exist rather than assuming what you see is everything.
- **Concurrency/race conditions**: the same account active on two
  devices/sessions at once, two rapid duplicate submits, a background sync
  racing a foreground edit.
- **Third-party integration failure modes**: payment gateway down or
  declined, push notification service unreachable, social login provider
  down or revoked, map/analytics SDK failure - these paths are invisible
  unless the dependency is deliberately made to fail.
- **Accessibility**: screen reader label correctness (distinct from having
  *any* accessible name, which the exploration loop already surfaces),
  font-scaling/zoom layouts, color contrast, keyboard-only navigation on web
  and desktop.
- **Localization**: other locales, RTL layout, string length overflow in
  translated text, locale-specific date/number/currency formatting.
- **Performance**: cold-start time, memory growth over a long session, large
  dataset scroll/render performance, battery drain.
- **Security**: auth bypass attempts, expired/tampered tokens, insecure local
  storage of sensitive data, session fixation/hijacking.

Treat this list the same way as the screen map: name which of these are in
scope, which are explicitly out of scope for this assignment, and which need
a specialist (security/perf/accessibility) rather than silently omitting
them.

### Screen Map Format

Keep this next to the generated suite (e.g. `screen-map.csv` in the feature
folder) so "did we cover everything discovered" is a file someone can check,
not a claim:

```text
screen_id,reached_from,action,screenshot,element_count,notable_elements,edge_types_seen,status,testcases_yaml
home,-,launchApp,home.png,18,"Search bar; 3 tabs; FAB","tap",explored,smoke/001_home.yaml
search_results,home,"tap: Search",search.png,12,"Result list; empty state not seen yet","tap,swipe",partial,regression/010_search.yaml
settings,home,"tap: Settings tab",settings.png,9,"Logout button; Notifications toggle","tap,longPress",explored,regression/020_settings.yaml
logout_confirm,settings,"tap: Logout",logout.png,3,"Confirm/Cancel dialog",tap,explored,regression/021_logout.yaml
```

`status` is one of `explored` (zero unexplored edges), `partial` (budget hit
or a branch deferred, say why), or `skipped-destructive` (identified but not
executed to avoid corrupting the rest of the traversal - note the risk
instead).

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
- For macOS and Windows, pair `clearState: true` with a header-level
  `desktopState.clear` plan. Use `mode: autoSafe` for app-scoped defaults and
  `mode: manual` only when explicit paths, Keychain services, or HKCU registry
  keys are known.
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
  screens/                  # also skipped by directory runs (page-object-style
    home_screen.yaml         # per-screen selector definitions, if used)
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
around the main files. Directories named `subflows/` and `screens/` are
skipped during `run`'s directory collection; call those reusable flows
explicitly with `runFlow`. If a scenario needs per-file setup, call an
explicit `runFlow` inside that scenario or run self-contained files
separately.

`validate` and `list` do **not** apply the `subflows/`/`screens/` exclusion -
they parse and report every YAML file found recursively, subflows included.
Only `run`'s own collected file list reflects what actually executes
standalone; `list --json` output may include command indexes for files that
never run on their own.

**Hook scope, verified against the runner source**: `setup.yaml`/`teardown.yaml`
only run when they sit directly in the exact folder passed to `run` - the
lookup does **not** cascade into nested feature folders, even though `run`
still collects and executes every YAML file found recursively underneath.

```text
auto_test/
  setup.yaml          # runs once IF you `run auto_test/`
  android/
    setup.yaml         # runs once IF you `run auto_test/android/` directly -
    home/               # but is SILENTLY SKIPPED (not run as a hook, not run
      setup.yaml        # as a test either) if you instead `run auto_test/`
      open_home.yaml     # and this file still gets executed either way
```

Two consequences worth planning around:

1. **One global before/after for the whole suite**: put exactly one
   `setup.yaml`/`teardown.yaml` at whatever root you always pass to `run` (the
   "run root"), not scattered across nested feature folders expecting them to
   combine - they will not.
2. **Isolated per-feature before/after**: run each feature folder as its own
   `lumi-tester run <feature-folder>` invocation (e.g. one per line in a
   script, or one per CI matrix job) so each folder's own `setup.yaml` is the
   one actually picked up.

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
env: { file: ".env" }
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

Place the credential file beside `setup.yaml` because the setup flow loads it:

```text
tests/generated/account/settings/.env
USER_EMAIL=test@example.com
USER_PASSWORD=replace-with-secret
```

Add that `.env` path to `.gitignore` and commit only a placeholder
`.env.example` alongside it. Never commit a `.env` file with real
credentials - `${VAR_NAME}` substitution exists specifically so secrets never
need to appear in the YAML itself.

To set the same values via a real OS environment variable instead (preferred
for CI/shared secrets - nothing touches disk in the repo), export it before
running:

```bash
# bash/zsh (macOS/Linux) - persists for the rest of the shell session
export USER_EMAIL="test@example.com"
export USER_PASSWORD="replace-with-secret"
lumi-tester run ./test.yaml --platform android

# bash/zsh - inline, scoped to this one command only
USER_EMAIL="test@example.com" USER_PASSWORD="replace-with-secret" \
  lumi-tester run ./test.yaml --platform android
```

```powershell
# Windows PowerShell - persists for the rest of the session
$env:USER_EMAIL = "test@example.com"
$env:USER_PASSWORD = "replace-with-secret"
lumi-tester run .\test.yaml --platform android
```

```cmd
:: Windows cmd.exe
set USER_EMAIL=test@example.com
set USER_PASSWORD=replace-with-secret
lumi-tester run test.yaml --platform android
```

In CI, set the same variable names as pipeline/repo secrets (GitHub Actions
`env:`/`secrets.*`, GitLab CI variables, etc.) instead of a committed `.env`
file - the YAML does not change between local and CI runs, only where the
value comes from.

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
- End every flow file by navigating back to one known, documented screen
  (e.g. the app's home screen) - never assume the next flow file in the
  suite will re-establish state on its own. Discovered live: a flow that
  tapped deep into a settings sub-page and ended there (no return-to-home
  step) made the *next* flow's first assertion fail, even though each flow
  passed 100% in isolation - the failure only showed up when running the
  whole suite in sequence. Verify this with a full-suite run
  (`lumi-tester run <dir> --continue-on-failure`), not just running each
  file alone - isolation-only testing hides this class of bug.

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
- If this suite came from the Systematic App Exploration Loop: every screen
  map entry is `explored` or has a documented reason it is `partial`/
  `skipped-destructive`; the loop ran per role/state that plausibly changes
  the UI, not just once; and each "What Clicking Can't Discover" item is
  explicitly marked in-scope, out-of-scope, or needs-specialist - not silently
  dropped.
- If a `requirements/` directory (see "SRS Requirements Format") exists for
  this project: every requirement id across `index.yaml`'s `includes` is
  either `skip`ped with a reason, or referenced from at least one row in
  `cases.csv` whose `yaml` file actually exists on disk. Compute this - do
  not eyeball it or trust a status someone hand-wrote.
