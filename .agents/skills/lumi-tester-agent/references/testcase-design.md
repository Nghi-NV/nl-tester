# Lumi Tester Testcase Design

Guidelines for structuring robust test suites, designing coverage models, and organizing automated YAML tests.

## 1. Test Suite Directory Layout

Group tests by feature module or execution flow to ensure reliable state management:

```text
e2e/workspaces/my_app/
├── setup.yaml                  # Root setup (e.g. login, permission grant)
├── teardown.yaml               # Root teardown (e.g. logout, clear data)
├── subflows/                   # Reusable subflows
│   ├── login.yaml
│   └── clear_cache.yaml
├── auth/                       # Feature group folder
│   ├── login_valid.yaml
│   ├── login_invalid.yaml
│   └── password_reset.yaml
├── settings/
│   ├── profile_update.yaml
│   └── notifications_toggle.yaml
└── hardware/                   # Hardware/IoT test flows
    ├── profiles/
    │   └── jig_switch.yaml
    └── power_cycle_test.yaml
```

## 2. Setup & Subflow Reusability

Avoid repeating common multi-step sequences across test files. Use `runFlow`:

```yaml
# In leaf test: auth/profile_update.yaml
platform: android
appId: com.example.app
tags: [regression, settings]
---
- runFlow: ../subflows/login.yaml
- tap: { id: "profile_tab" }
- inputText: "New Display Name"
- tap: { id: "save_btn" }
- see: { text: "Profile Updated" }
```

## 3. Data-Driven Testing (DDT via CSV)

For parameterized testing across multiple datasets, declare `data` in the header:

```yaml
platform: web
url: "https://example.com/login"
data: "testdata/users.csv"
---
- launchApp
- tap: { id: "username" }
- inputText: "${username}"
- tap: { id: "password" }
- inputText: "${password}"
- tap: { id: "login_btn" }
- see: { text: "${expected_greeting}" }
```

## 4. Test Hierarchy & Tagging Strategy

Use `tags` in headers to enable targeted CI/CD execution:

- `smoke`: Fast, critical happy paths (< 2 minutes).
- `regression`: Comprehensive feature verification.
- `hardware`: Tests requiring physical serial/Jig connection.
- `flaky-quarantine`: Tests under investigation.

Run subsets easily with `--tags`:
```bash
lumi-tester run ./e2e --platform android --tags smoke --report
```

## 5. Test Authoring Principles
- **Atomic & Independent**: Each test flow should define its setup or declare dependencies cleanly.
- **Fail Fast**: Check preconditions (`waitUntilVisible`) before performing destructive actions.
- **Explicit Assertions**: Always end interactions with verifiable state checks (`see`, `assertVar`, `hwSeeLed`).
