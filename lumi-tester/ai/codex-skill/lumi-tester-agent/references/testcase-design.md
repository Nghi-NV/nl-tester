# Lumi Tester Testcase Design

Use this reference when asked to create enough test cases for an app or web
feature, convert testcase documents into YAML, or organize generated tests.

## Coverage Loop

1. Identify the feature, platform, app identity, target environment, user roles,
   and data dependencies.
2. Build a small coverage model before writing YAML. Include screens/pages,
   inputs, permissions, states, roles, network conditions, and integrations.
3. Generate testcase candidates with the techniques below.
4. Collapse redundant cases with risk and pairwise thinking; keep one stable
   smoke path and focused edge/negative cases.
5. Group tests by required setup. Do not run files independently when they
   require login, onboarding, seeded data, permissions, or a specific state.
6. Write setup/login/group flows first, then leaf scenario files.
7. Validate every generated YAML file, then run by folder/group with reports and
   artifacts.

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
- Regression selection: tag critical smoke, risky changed flows, and full
  regression separately.

## App And Web Coverage Checklist

Functional:

- Happy path, alternate path, cancel/back path, retry path.
- Empty, loading, success, partial success, error, timeout.
- Create, read, update, delete, undo, duplicate, idempotency.
- Search/filter/sort/pagination/infinite scroll.
- Deep link, notification entry, share/open-with, browser refresh/back/forward.

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

## Grouping Strategy

Use folders when scenarios share state:

```text
tests/generated/<feature>/
  README.md                 # optional human traceability outside skill output
  data/
    users.csv
  setup/
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
  teardown/
    logout.yaml
    cleanup.yaml
```

When the repo already has a test layout, follow it instead of forcing this
shape. Keep generated tests under a feature folder such as
`tests/generated/<feature>/` or the repo's equivalent, so artifacts from a
testcase batch stay together.

Run a folder/group when files depend on shared setup:

```bash
lumi-tester validate tests/generated/login --json
lumi-tester list tests/generated/login --json
lumi-tester run tests/generated/login --platform android --report --snapshot --events-jsonl --output ./output/login
```

Run a single file only when it is explicitly self-contained.

## YAML Authoring Rules From Testcases

- Put testcase id and requirement id in `tags` when available.
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
Requirement | Risk | Platform | State | Role | Data class | Permission | Network | Expected result | YAML file
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
