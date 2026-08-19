# Lumi Tester Flow Patterns

Canonical flow templates and best practices for creating automated tests across platforms.

## 1. Authentication & Form Input Pattern

```yaml
platform: android
appId: com.example.app
tags: [smoke, login]
defaultTimeout: 15000
---
- launchApp
- waitUntilVisible: { id: "email_input" }
- tap: { id: "email_input" }
- inputText: "${USER_EMAIL:-test@example.com}"
- tap: { id: "password_input" }
- inputText: "${USER_PASS:-secret123}"
- hideKeyboard
- tap: { id: "submit_btn" }
- waitUntilVisible: { text: "Dashboard", exact: true }
- see: { text: "Dashboard" }
```

## 2. Sub-Element Positioning Pattern (`align` & `offset`)

For toggles, checkboxes, or icon buttons located on row edges:

```yaml
# Tap toggle switch on the right side of a list row
- tap:
    type: "Switch"
    index: 0
    align: right # Presets: left (10%), right (90%), top (10%), bottom (90%), center (50%)

# Custom percentage offset within element bounds
- tap:
    id: "settings_item"
    offset: "85%,50%"
```

## 3. Hardware + App Hybrid Verification Pattern

For IoT/Smart devices requiring physical interaction + App assertion:

```yaml
platform: android
appId: com.lumi.lifenext
jig: "profiles/jig_switch_sample.yaml"
---
- launchApp
- hwPowerOn: 1
- hwClick: 1 # Physical button pressed via Servo
- waitUntilVisible: { text: "Light 1 is ON" }
- hwSeeLedBlink: { channel: 1, color: "BLUE", count: 2 } # LED assertion
- tap: { id: "turn_off_btn" }
- hwSeeLedOff: 1
```

## 4. GPS Navigation Simulation Pattern

```yaml
platform: android
appId: com.example.map
---
- launchApp
- mockLocation:
    file: "./routes/commute.gpx"
    speed: 45
    loop: false
- waitUntilVisible: { id: "speedometer" }
- stopMockLocation
```

## 5. Subflow & Error Recovery Pattern

```yaml
# Main test flow reusing subflow
- runFlow: ./subflows/login.yaml
- conditional:
    condition: { visible: "Rate App" }
    then:
      - tap: { text: "Later" }
- retry:
    maxRetries: 2
    commands:
      - tap: { id: "refresh_btn" }
      - waitUntilVisible: { id: "content_list" }
```

## 6. Web & Desktop Pattern

```yaml
platform: web # or macos / windows
url: "https://example.com/dashboard"
---
- launchApp
- waitUntilVisible: { role: "button", text: "Get Started" }
- tap: { role: "button", text: "Get Started" }
- screenshot: "web_dashboard.png"
```
