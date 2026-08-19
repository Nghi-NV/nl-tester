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
- tap: "Submit|Xác nhận|Đăng nhập" # Multilingual regex shorthand
- waitUntilVisible: "^(Dashboard|Trang chủ)"
- see: "Welcome|Xin chào"
```

## 2. Multilingual & Dynamic Regex Pattern

```yaml
# Shorthand regex allows tests to pass seamlessly across English and Vietnamese locales
- tap: "Accept|Đồng ý|Cho phép"
- waitUntilVisible: "^(Welcome|Chào mừng).*$"
- see: "Success|Thành công"
```

## 3. Sub-Element Positioning Pattern (`align` & `offset`)

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

## 4. Hardware + App Hybrid Verification Pattern

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

## 5. GPS Navigation Simulation Pattern

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

## 6. Subflow & Error Recovery Pattern

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

## 7. Web & Desktop Pattern

```yaml
platform: web # or macos / windows
url: "https://example.com/dashboard"
---
- launchApp
- waitUntilVisible: { role: "button", text: "Get Started" }
- tap: { role: "button", text: "Get Started" }
- screenshot: "web_dashboard.png"
```
