# Python Integration Guide for Lumi Tester

Lumi Tester provides native support for executing Python scripts and inline Python snippets directly inside YAML flows using `runPython` (aliases: `execPython`, `python`).

This feature enables seamless integration with external APIs, custom database queries, hardware test logic, AI model inference, and data extraction.

---

## 🚀 Quick Example

```yaml
platform: android
appId: com.example.app
---
# 1. Simple shorthand execution
- runPython: "./scripts/pre_check.py"

# 2. Execute script with arguments and extract multiple JSON variables
- runPython:
    script: "./scripts/authenticate.py"
    args:
      - "--device"
      - "$DEVICE_SERIAL"
      - "--env"
      - "staging"
    env:
      SECRET_KEY: "my_api_secret"
    timeoutMs: 15000
    saveVars:
      access_token: "token"
      user_status: "user.status"

# 3. Use extracted variables in app flow
- tap:
    id: "token_input"
- inputText: "$access_token"

# 4. Run inline Python code
- runPython:
    code: |
      import sys, json
      print(json.dumps({"status": "PASS", "received_token": sys.argv[1]}))
    args: ["$access_token"]
    saveVar: "result_json"
```

---

## ⚙️ Command Reference (`runPython` / `execPython` / `python`)

| Parameter | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `script` | `string` | Optional* | Path to `.py` script file relative to flow directory. |
| `code` | `string` | Optional* | Inline Python source code snippet to execute. |
| `args` | `list<string>` | Optional | Arguments passed to the script (sys.argv). Supports variable substitution (`$VAR`). |
| `env` | `map<string, string>` | Optional | Custom environment variables passed to the Python process. |
| `timeoutMs` | `number` | Optional | Execution timeout in milliseconds (default: `30000` / 30s). |
| `pythonPath` | `string` | Optional | Custom Python interpreter binary (e.g. `python3`, `python`, or `./venv/bin/python`). |
| `saveVar` | `string` | Optional | Variable name to store trimmed `stdout` text output. |
| `saveVars` | `list` \| `map` | Optional | Extract multiple variables from `stdout` JSON output. |

*\* Either `script` or `code` must be specified when using object format.*

---

## 📦 Variable Extraction (`saveVar` & `saveVars`)

### 1. `saveVar` (Single String Output)
Stores the complete trimmed `stdout` text into a single variable.

```yaml
- runPython:
    code: "print('GENERATED_TOKEN_12345')"
    saveVar: "my_token"

- inputText: "$my_token"
```

### 2. `saveVars` with List (Direct Key Extraction)
When Python prints a JSON dictionary, extract fields directly into matching variable names:

```yaml
- runPython:
    code: |
      import json
      print(json.dumps({"token": "abc123", "user_id": 999}))
    saveVars:
      - token
      - user_id

# Creates variables: $token = "abc123", $user_id = "999"
```

### 3. `saveVars` with Map (Custom Variable Mapping & Dot Notation)
Map variable names to specific JSON fields or nested properties using dot notation:

```yaml
- runPython:
    code: |
      import json
      print(json.dumps({
        "status": "success",
        "data": {
          "session_token": "xyz789",
          "user": { "role": "admin" }
        }
      }))
    saveVars:
      sessionToken: "data.session_token"
      userRole: "data.user.role"

# Creates variables: $sessionToken = "xyz789", $userRole = "admin"
```

---

## 🐍 Python Environment Resolution

Lumi Tester automatically resolves the Python interpreter in the following order:

1. `pythonPath` specified in the `runPython` command parameters.
2. System default: `python` on Windows, `python3` on macOS / Linux.
3. Virtualenv paths if specified (e.g., `pythonPath: "./.venv/bin/python"` or `pythonPath: ".\\.venv\\Scripts\\python.exe"`).

---

## 🛠 Troubleshooting

- **Non-zero Exit Codes**: If the Python script exits with code `!= 0`, Lumi Tester logs `stderr` output and fails the step.
- **JSON Parsing Errors**: When using `saveVars`, ensure the script prints clean JSON to `stdout` without extraneous debug `print()` statements.
