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
| `env` | `vars`, `var`| Map | `env: { KEY: "value" }` định nghĩa biến trực tiếp, hoặc `env: { file: ".env" }` đọc từ file `.env` thật (khuyên dùng cho dữ liệu nhạy cảm - xem mục "Biến môi trường & Dữ liệu nhạy cảm" bên dưới). |
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
5.  **Theo Vị trí tương đối**: (Aliases: `rightOf`, `leftOf`, `above`, `below`). Tự động tìm kiếm các thành phần không tên/id nằm cạnh các nhãn (text, contentDesc) ổn định.
    ```yaml
    # Tap vào Slider nằm dưới nhãn "30%"
    - tap:
        type: "View"
        below: "30%"
        offset: "50%,50%"

    # Nếu có nhiều hơn 1 phần tử cùng loại trong hướng đó, chỉ định index:
    - tap:
        type: "View"
        index: 1
        below: "Brightness"
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
8.  **Kéo vuốt liên tục (`drag`)**: Dùng cho các thanh trượt Slider, Seekbar, sắp xếp kéo thả (Reorder), vẽ Canvas mượt mà trên **Android**, **iOS**, **Web**, **macOS**, **Windows**.
    ```yaml
    # Kéo thanh trượt từ mốc 0% sang 80%
    - drag:
        from:
          type: "View"
          below: "Brightness"
          offset: "0%,50%"
        to:
          type: "View"
          below: "Brightness"
          offset: "80%,50%"
        duration: 500 # ms
    ```

---

## 🔀 Cấu trúc Điều khiển Hiện đại: `when:`, `forEach:`, `match:`

Để kịch bản test luôn phẳng (flat), dễ đọc và không bị lồng block sâu, Lumi Tester hỗ trợ các cấu trúc điều khiển tự nhiên:

### 1. Modifier Inline `when:` (Loại bỏ If-Then lồng nhau)
Thay vì tạo cả một block `conditional:` cồng kềnh, bạn có thể gắn trực tiếp `when:` vào bất kỳ câu lệnh nào:
```yaml
# Nhấn Đồng ý KHI nhìn thấy Popup Cookies
- tap: "Accept Cookies"
  when: { visible: "Accept Cookies" }

# Nhấn Để sau KHI có Popup Update
- tap: "Remind Later"
  when: { visible: "Update Available" }

# Điều kiện theo biến môi trường hoặc biểu thức logic
- tap: "Debug Mode"
  when: "${ENV} == 'staging'"
```

### 2. Vòng lặp Khai báo `forEach:` (Duyệt mảng & Data-Driven)
Duyệt qua danh sách mảng tĩnh, danh sách đối tượng hoặc mảng động từ biến Context:
```yaml
# Duyệt mảng tĩnh
- forEach:
    item: channel
    in: [1, 2, 3, 4]
    commands:
      - hwClick:
          channel: "${channel}"
          duration: 150
      - wait: 500

# Data-Driven với danh sách đối tượng
- forEach:
    item: user
    in:
      - { email: "admin@test.com", pass: "123456" }
      - { email: "guest@test.com", pass: "password" }
    commands:
      - tap: "Email"
      - inputText: "${user.email}"
      - tap: "Password"
      - inputText: "${user.pass}"
      - tap: "Login"
```

### 3. Khớp Nhánh `match:` (Thay thế Switch-Case)
Cấu trúc phẳng với Key-Value gọn gàng, trực quan:
```yaml
- match: "${USER_ROLE}"
  cases:
    admin:
      - tap: "Admin Panel"
      - see: "System Settings"
    editor:
      - tap: "Content Editor"
    viewer:
      - see: "Read Only"
  default:
    - see: "Unauthorized"
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

# Chạy test theo dữ liệu CSV/JSON (Data-Driven Testing)
lumi-tester run path/to/test.yaml --data users.csv --platform android
```

### 2. Thực thi qua VS Code Extension:
- **`▶ Run All`**: Chạy toàn bộ test flow (nằm ở đầu file / dòng `---`).
- **`▷ Run [i]`**: Chỉ chạy riêng câu lệnh thứ `i` để kiểm tra nhanh selector.
- **`▶ Run from [i]`**: Chạy từ câu lệnh thứ `i` đến hết file (tiếp tục flow từ điểm mong muốn).

---

## 🔐 Biến môi trường & Dữ liệu nhạy cảm

Không bao giờ viết trực tiếp mật khẩu, token, số điện thoại... vào file YAML.
Tham chiếu bằng `${TEN_BIEN}` ở bất kỳ trường chuỗi nào (giá trị selector,
`inputText`, header), rồi cấp giá trị bằng 1 trong 2 cách:

1. **Biến môi trường OS thật** (khuyên dùng cho CI/secret dùng chung) - export
   trước khi chạy, giá trị không bao giờ nằm trên đĩa trong repo:

   ```bash
   # bash/zsh (macOS/Linux) - tồn tại cho cả phiên shell
   export USER_EMAIL="test@example.com"
   export USER_PASSWORD="replace-with-secret"
   lumi-tester run ./test.yaml --platform android

   # bash/zsh - chỉ áp dụng cho đúng lệnh này
   USER_EMAIL="test@example.com" USER_PASSWORD="replace-with-secret" \
     lumi-tester run ./test.yaml --platform android
   ```

   ```powershell
   # Windows PowerShell
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

   Trên CI, khai báo cùng tên biến này ở phần secrets/variables của pipeline
   (GitHub Actions `env:`/`secrets.*`, GitLab CI variables...) - file YAML
   không đổi giữa local và CI, chỉ nguồn giá trị đổi.

2. **File `.env` cục bộ** (tiện cho dev local): trỏ header vào file bằng cú
   pháp đặc biệt `env: { file: ".env" }` - Lumi tự đọc file đó (định dạng
   chuẩn `KEY=value`, comment bằng `#`, có thể có tiền tố `export `) và biến
   mỗi key thành `${KEY}` dùng được trong flow:

   ```yaml
   platform: android
   appId: com.example.app
   env: { file: ".env" }
   ---
   - tap:
       accessibilityId: "Email"
   - inputText: "${USER_EMAIL}"
   - tap:
       accessibilityId: "Password"
   - inputText: "${USER_PASSWORD}"
   ```

   Chỉ commit file mẫu (`.env.example`) kèm placeholder, thêm `.env` thật vào
   `.gitignore` - **không bao giờ** commit file `.env` chứa credential thật.
   Không dùng `env: { KEY: "value" }` (map thường) cho dữ liệu nhạy cảm vì
   giá trị sẽ nằm thẳng trong YAML đã commit.

## 🤝 Best Practices

1.  **Sử dụng `setup.yaml` & `teardown.yaml`**: Để tái sử dụng code login/logout.
2.  **Tránh Tọa độ Cứng**: Luôn ưu tiên Text, ID, hoặc `align`/`offset`. Nếu dùng tọa độ, hãy dùng percentage.
3.  **Sâu chuỗi sub-flows**: Dùng `runFlow` để module hóa kịch bản.
4.  **Không hardcode dữ liệu nhạy cảm**: Dùng `${VAR}` + biến môi trường/`.env` - xem mục "Biến môi trường & Dữ liệu nhạy cảm" phía trên.

## 📁 Tổ chức thư mục

```text
tests/
├── setup.yaml          # tự chạy 1 lần trước các file chính
├── teardown.yaml       # tự chạy 1 lần sau các file chính
├── data/
├── subflows/            # Sub-flows (login.yaml...) - gọi bằng `runFlow`
└── scenarios/          # Test chính
```

`subflows/` (và `screens/` nếu dùng cho định nghĩa selector theo màn hình) là
2 tên thư mục đặc biệt duy nhất bị bỏ qua khi `run` thu thập file - đặt đúng
tên này thì các file bên trong mới không bị chạy như 1 test độc lập, chỉ chạy
được qua `runFlow`. Đặt tên khác (ví dụ `common/`) sẽ khiến file trong đó bị
chạy như test bình thường.

**Phạm vi của `setup.yaml`/`teardown.yaml` (đã kiểm chứng qua source code)**:
chỉ chạy khi nằm đúng ở thư mục được truyền trực tiếp vào `run` - **không**
tự động áp dụng cho các thư mục con lồng bên trong, dù `run` vẫn duyệt đệ quy
và chạy hết file yaml trong các thư mục con đó.

```text
auto_test/
├── setup.yaml           # chạy 1 lần NẾU bạn `run auto_test/`
└── android/
    ├── setup.yaml       # chạy 1 lần NẾU bạn `run auto_test/android/` trực
    │                    #   tiếp - nhưng bị BỎ QUA hoàn toàn (không chạy như
    │                    #   hook, cũng không chạy như test) nếu thay vào đó
    │                    #   bạn `run auto_test/`
    └── home/
        └── open_home.yaml  # vẫn được chạy trong cả 2 trường hợp trên
```

Hệ quả cần lưu ý khi thiết kế:

- Muốn **1 before/after áp dụng cho toàn bộ suite**: đặt đúng 1 cặp
  `setup.yaml`/`teardown.yaml` ở thư mục gốc mà bạn luôn `run` ("run root"),
  không rải rác theo từng thư mục feature con rồi mong chúng gộp lại.
- Muốn **before/after cô lập theo từng feature**: chạy từng thư mục feature
  bằng lệnh `run` riêng (một dòng script hoặc một job CI cho mỗi feature) để
  đúng `setup.yaml` của thư mục đó được dùng.
