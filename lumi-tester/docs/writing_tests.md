# ✍️ Hướng dẫn Viết Test

Tài liệu này giúp bạn hiểu rõ cấu trúc file kịch bản test và cách tổ chức một test flow hiệu quả.

---

## 📄 Cấu trúc File YAML

`lumi-tester` chấp nhận hai định dạng file để phù hợp với nhu cầu đơn giản hoặc phức tạp.

### 1. Định dạng Phân tách (Header --- Steps)
Đây là định dạng khuyến nghị cho các test thực tế. Sử dụng dấu `---` để tách biệt phần khai báo cấu hình và danh sách các lệnh thực thi.

```yaml
appId: com.example.app
platform: android
tags:
  - smoke
  - regression
---
- launchApp
- tap: "Login"
```

### 2. Định dạng Map (Single Block)
Phù hợp khi bạn muốn định nghĩa toàn bộ test trong một cấu trúc map duy nhất, hoặc khi Test Flow được lồng vào một hệ thống khác.

```yaml
appId: com.example.app
steps: # Hoặc 'commands'
  - open: "com.example.app"
  - tap: "Login"
```

---

## 📋 Các trường Header (Khai báo)

Phần Header nằm phía trên dấu `---`. Nếu không có dấu `---`, các trường này có thể khai báo cùng cấp với `steps`.

| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `appId` | - | String | Package name (Android), Bundle ID (iOS), `.app` path/bundle id (macOS), hoặc `.exe` path (Windows). |
| `url` | - | String | URL khởi tạo (Web). |
| `platform` | - | String | `android`, `android_auto`, `ios`, `web`, `macos`, `windows`. |
| `desktopState` | - | Map | Cấu hình xóa state cho desktop; dùng `desktopState.clear` cùng `launchApp: { clearState: true }` trên macOS/Windows. |
| `env` | `vars`, `var`| Map | Định nghĩa biến môi trường (Key-Value) hoặc load từ file (`file: path`). |
| `data` | - | String | Path tới file dữ liệu (CSV/JSON). |
| `defaultTimeout` | - | Number | Thời gian chờ mặc định (ms) cho các lệnh. |
| `tags` | - | Array | Danh sách nhãn phân loại test. |
| `speed` | - | String | Tốc độ: `turbo`, `fast`, `normal`, `safe`. |
| `browser` | - | String | (Web) `Chrome`, `Firefox`, `Webkit`. |
| `closeWhenFinish`| - | Boolean | Tự động đóng app khi kết thúc. |
| `steps` | `commands` | Array | Danh sách các lệnh (Dùng trong định dạng Map). |

---

## 💡 Ví dụ đầy đủ kịch bản kiểm thử (Full Test Flow Examples)

### 1. 🤖 Android Mobile Test (Đăng nhập & Kiểm tra Trang chủ)
```yaml
platform: android
appId: com.example.smartapp
defaultTimeout: 10000
tags:
  - mobile
  - smoke
---
- launchApp:
    clearState: true
    permissions:
      notifications: "allow"
      location: "while_in_use"

# Chờ màn hình đăng nhập hiển thị
- waitSee:
    id: "login_container"

# Nhập email và mật khẩu
- tap:
    id: "input_email"
- inputText: "user@example.com"

- tap:
    id: "input_password"
- inputText: "Secret123"

- tap:
    id: "btn_login"

# Xác nhận vào được Dashboard
- see:
    text: "Chào mừng"
    exact: false
- screenshot: "android_dashboard.png"
```

---

### 2. 🍏 iOS Mobile Test (Xác thực & Accessibility ID)
```yaml
platform: ios
appId: com.example.iosapp
defaultTimeout: 12000
tags:
  - ios
  - regression
---
- launchApp:
    clearState: true

- tap:
    desc: "LoginButton"
    type: "Button"

- type:
    text: "ios_tester@example.com"
    selector: "EmailField"

- hideKeyboard

- tap: "Submit"
- see: "Welcome Page"
```

---

### 3. 🌐 Web Automation Test (Chrome Multi-step & API Call)
```yaml
platform: web
url: "https://shop.example.com"
browser: Chrome
defaultTimeout: 15000
---
- launchApp

# Gửi HTTP API lấy Token khuyến mãi
- httpRequest:
    url: "https://api.example.com/promo/active"
    method: "GET"
    saveResponse:
      "$.promo_code": "PROMO_CODE"

- tap:
    css: ".nav-login-btn"

- inputText: "testuser@gmail.com"
- press: "Enter"

- scrollTo: "Mã giảm giá"
- tap:
    css: "#promo_input"
- write: "$PROMO_CODE"

- see: "Áp dụng thành công"
```

---

### 4. 🐍 Python Integration Test (Thực thi mã Python & Kiểm tra Biến)
```yaml
platform: android
appId: com.example.iotapp
---
# Gọi script Python tạo mã xác thực JWT ngẫu nhiên
- runPython:
    code: |
      import time, json
      payload = {
        "timestamp": int(time.time()),
        "token": "AUTH_XYZ999",
        "role": "admin"
      }
      print(json.dumps(payload))
    saveVars:
      generated_token: "token"
      user_role: "role"

- assertTrue: "${user_role} == 'admin'"

- tap:
    id: "auth_token_field"
- write: "$generated_token"
```

---

### 5. ⚙️ Hardware Jig Controller Test (Relay, Servo & LED Sensor)
```yaml
platform: android
appId: com.lumi.smarthome
jig: "profiles/jig_switch_sample.yaml" # hoặc jig: "COM5"
---
# 1. Cấp nguồn rơ-le kênh 1 cho thiết bị Smart Switch
- hwPowerOn: 1
- wait: 2000

# 2. Điều khiển động cơ Servo nhấn giữ nút Pairing trong 5 giây
- hwPress: 1
- wait: 5000
- hwRelease: 1

# 3. Kiểm tra đèn LED phần cứng nhấp nháy màu xanh dương (Pairing Mode)
- hwSeeLedBlink:
    channel: 1
    color: "BLUE"
    count: 2
    timeoutMs: 8000

# 4. Ngắt nguồn hoàn toàn sau khi hoàn tất test
- hwPowerOffAll
```

---

### 6. 🚗 Android Auto / Automotive Test (Điều hướng Bản đồ & Media)
```yaml
platform: android_auto
appId: com.example.naviapp
---
- selectDisplay: "1" # Màn hình trung tâm ô tô DHU
- launchApp

- tap:
    point: "50%,20%" # Chọn ô tìm kiếm đường đi
- inputText: "Hà Nội"
- press: "ENTER"

- see: "Bắt đầu chỉ đường"
- tap: "Bắt đầu chỉ đường"
```

---

### 7. 💻 macOS Desktop Test (App Lifecycle & Clear State)
```yaml
platform: macos
appId: /Applications/LumiDesktop.app
desktopState:
  clear:
    mode: autoSafe
---
- launchApp:
    clearState: true

- see: "Setup Wizard"
- tap: "Next"
- screenshot: "macos_wizard.png"
```

---

### 8. 📍 GPS Simulation Test (Giả lập di chuyển theo file GPX)
```yaml
platform: android
appId: com.example.tracker
---
- launchApp

# Bắt đầu phát tọa độ di chuyển tốc độ 60km/h
- gps:
    file: "./routes/hanoi_to_haiphong.gpx"
    speed: 60
    loop: true

- wait: 5000
- waitForLocation:
    lat: 20.8449
    lon: 106.6881
    tolerance: 50.0

- stopMockLocation
```

---

### 9. 📈 Performance Profiling Test (Đo CPU/RAM & Assert)
```yaml
platform: android
appId: com.example.heavyapp
---
- startProfiling:
    samplingIntervalMs: 500

- launchApp
- repeat:
    times: 5
    commands:
      - swipeLeft
      - wait: 1000

- stopProfiling:
    savePath: "./output/profile_result.json"

- assertPerformance:
    metric: "memory"
    limit: "200MB"
```

---

## 🔍 Cách tìm Elements (Selectors)

`lumi-tester` hỗ trợ nhiều cách để xác định element trên màn hình:

1.  **Theo Text**: Tìm văn bản hiển thị (case-insensitive).
    ```yaml
    - tap: "Login"
    ```
2.  **Theo Resource ID**: ID định danh trong code. (Alias: `id`)
    ```yaml
    - tap:
        id: "btn_login"
    ```
3.  **Theo Tọa độ**: Phù hợp khi element không có định danh. (Alias: `point`)
    ```yaml
    - tap:
        point: "50%,80%"
    ```
4.  **Theo Regex**: Tìm theo biểu mẫu của chữ. (Alias: `regex`)
    ```yaml
    - see:
        regex: "OTP: \\d{6}"
    ```
5.  **Theo Vị trí tương đối**: (Aliases: `rightOf`, `leftOf`, `above`, `below`)
    ```yaml
    - tap:
        rightOf: "Username"
        type: "EditText"
    ```
6.  **Theo Mô tả (Accessibility)**: (Aliases: `desc`, `contentDesc`, `accessibilityId`)
    ```yaml
    - tap:
        desc: "Nút Lưu"
    ```
7.  **Căn chỉnh vị trí trong phần tử (`align` & `offset`)**: Cho phép click vào cạnh trái/phải/trên/dưới hoặc vị trí tương đối % bên trong bounds của element (hữu ích cho switch toggle, menu item có icon/nút).
    ```yaml
    # Tap vào toggle bên phải của Switch hàng thứ 2
    - tap:
        type: "Switch"
        index: 1
        align: right  # Presets: left (10%), right (90%), top (10%), bottom (90%), center (50%)

    # Custom offset theo phần trăm kích thước element
    - tap:
        id: "item_row"
        offset: "85%,50%"
    ```

---

## ⚡ Thực thi & Debugging linh hoạt (Execution & Debugging)

Khi phát triển hoặc gỡ lỗi kịch bản test, `lumi-tester` và VS Code Extension cung cấp các chế độ chạy nhanh:

### 1. Thực thi qua CLI:
```bash
# Chạy toàn bộ file
lumi-tester run path/to/test.yaml --platform android

# Chỉ chạy một câu lệnh duy nhất (0-based index)
lumi-tester run path/to/test.yaml --command-index 2

# Chạy từ câu lệnh này đến hết file
lumi-tester run path/to/test.yaml --from-command-index 2

# Lặp lại test N lần liên tiếp (stress/stability testing)
lumi-tester run path/to/test.yaml --repeat 5
```

### 2. Thực thi qua VS Code Extension:
- **`▶ Run All`**: Chạy toàn bộ test flow (nằm ở đầu file / dòng `---`).
- **`▷ Run [i]`**: Chỉ chạy riêng câu lệnh thứ `i` để kiểm tra nhanh selector.
- **`▶ Run from [i]`**: Chạy từ câu lệnh thứ `i` đến hết file (tiếp tục flow từ điểm mong muốn).

---

## 🤝 Best Practices

1.  **Sử dụng `setup.yaml` & `teardown.yaml`**: Để tái sử dụng code login/logout.
2.  **Tránh Tọa độ Cứng**: Luôn ưu tiên Text, ID, hoặc `align`/`offset`. Nếu dùng tọa độ, hãy dùng percentage.
3.  **Sâu chuỗi sub-flows**: Dùng `runFlow` để module hóa kịch bản.

## 📁 Tổ chức thư mục

```text
tests/
├── setup.yaml
├── data/
├── common/             # Sub-flows (Login.yaml)
└── scenarios/          # Test chính
```
