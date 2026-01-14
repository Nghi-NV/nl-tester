# 📖 lumi-tester Command Reference

Tài liệu này liệt kê chi tiết tất cả các lệnh (commands) có thể sử dụng trong file YAML của `lumi-tester`.

---

## 📱 App Management (Quản lý Ứng dụng)

### `open` / `launchApp`
Mở một ứng dụng.

**Tham số:**
- `appId`: Package name (Android) hoặc Bundle ID (iOS).
- `clearState`: `true` để xóa dữ liệu app trước khi mở (Clean Install).
- `clearKeychain`: `true` để xóa Keychain (iOS Simulator only).
- `stopApp`: `true` để dừng app trước khi mở (default: true).
- `permissions`: Map các quyền cần cấp (`{ all: "deny" }` hoặc `{ notifications: "allow" }`).

```yaml
- open: "com.example.app"
- launchApp:
    appId: "com.example.app"
    clearState: true
    permissions:
      notifications: "allow"
```

### `stopApp`
Dừng ứng dụng đang test.
```yaml
- stopApp
```

### `clearAppData`
Xóa dữ liệu của ứng dụng (Reset).
```yaml
- clearAppData: "com.example.app"
```

### `installApp`
Cài đặt file APK.
```yaml
- installApp: "./app-debug.apk"
```

### `uninstallApp`
Gỡ cài đặt ứng dụng.
```yaml
- uninstallApp: "com.example.app"
```

### `backgroundApp`
Đưa ứng dụng xuống background trong một khoảng thời gian.
```yaml
- backgroundApp:
    durationMs: 5000 # default
```

### `selectDisplay` / `display`
Chọn màn hình để tương tác (Ví dụ: Android Auto).
```yaml
- selectDisplay: "0" # Main display
- display: "1"       # Secondary display
```

### `setLocale`
Thay đổi ngôn ngữ thiết bị.
```yaml
- setLocale: "en_US"
```

---

## 👆 Interaction (Tương tác)

### `tap`
Chạm vào một phần tử. Hỗ trợ nhiều cách tìm phần tử.

**Tham số:**
- `text`: Tìm theo văn bản chính xác.
- `id`: Tìm theo Resource ID (Android) hoặc ID (Web).
- `css`: Tìm theo CSS Selector (Web only).
- `xpath`: Tìm theo XPath.
- `point`: Tìm theo tọa độ (`x,y` hoặc `x%,y%`).
- `regex`: Tìm theo Regex (hỗ trợ `\d+`, `[...]`, `(...)`).
- `index`: Số thứ tự nếu có nhiều kết quả (0-based).
- `type`: Loại element (ví dụ "Button", "EditText").
- `optional`: `true` để không báo lỗi nếu không tìm thấy.

```yaml
- tap: "Login"
- tap: 
    id: "btn_login"
- tap: 
    point: "50%,80%"
- tap: 
    regex: "Confirm.*"
```

### `doubleTap`
Chạm nhanh 2 lần. Tham số tương tự `tap`.
```yaml
- doubleTap: "Like"
- doubleTap: 
    id: "btn_like"
```

### `longPress`
Nhấn và giữ (mặc định 1000ms).
```yaml
- longPress: "Save Image"
```

### `rightClick` / `contextClick`
Chuột phải (Web/Desktop).
```yaml
- rightClick: "Item"
```

### `tapAt`
Chạm vào element theo index và loại (không cần text/id).
```yaml
- tapAt:
    type: "Button"
    index: 2
```

### `inputText` / `write`
Nhập văn bản vào ô input đang focus hoặc tìm theo selector.

**Tham số:**
- `text`: Nội dung cần nhập.
- `unicode`: `true` để dùng chế độ nhập Unicode (hỗ trợ tiếng Việt, ký tự đặc biệt) thông qua `AdbIME` (Android only).

```yaml
- inputText: "hello"
- inputText:
    text: "xin chào"
    unicode: true
```

### `inputAt`
Nhập văn bản vào element theo index và loại.
```yaml
- inputAt:
    type: "EditText"
    index: 0
    text: "My Name"
```

### `eraseText`
Xóa văn bản trong ô input đang focus.
- **iOS**: Sử dụng thuật toán Triple-tap select-all + space replacement để đảm bảo xóa sạch.
```yaml
- eraseText
```

### `hideKeyboard`
Ẩn bàn phím ảo.
```yaml
- hideKeyboard
```

### `press`
Nhấn phím vật lý (Home, Back, Enter...).
```yaml
- press: "Enter"
- press: "Back"
```

### `home` / `pressHome`
Nhấn Home.
```yaml
- home
```

### `back`
Nhấn Back.
```yaml
- back
```

---

## 📜 Scroll & Swipe

### `swipe`
Vuốt màn hình.
- `direction`: `up`, `down`, `left`, `right`.
- `duration`: Thời gian vuốt (ms).
- `distance`: Khoảng cách vuốt (0-1).

```yaml
- swipe: "up"
- swipe:
    direction: "left"
    duration: 500
    from:
        id: "container_view" # Swipe bắt đầu từ element này
```

### `scrollTo`
Cuộn tới khi thấy element.
```yaml
- scrollTo:
    text: "Footer Link"
    direction: "down"
    maxScrolls: 10
    from:
        id: "scrollable_container" # Scroll bên trong container này
```

---

## ⚙️ System & Settings (Hệ thống)

### `openNotifications`
Mở thanh thông báo.
```yaml
- openNotifications
```

### `openQuickSettings`
Mở Quick Settings.
```yaml
- openQuickSettings
```

### `setVolume`
Chỉnh âm lượng.
```yaml
- setVolume: 50
```

### `lockDevice` / `unlockDevice`
Khóa/Mở khóa màn hình.
```yaml
- lockDevice
- unlockDevice
```

### `setNetwork`
Bật tắt WiFi/Data.
```yaml
- setNetwork:
    wifi: true
    data: false
```

### `airplaneMode`
Bật/Tắt chế độ máy bay.
```yaml
- airplaneMode
```

### `setOrientation`
Xoay màn hình (Advanced).
- Modes: `Portrait`, `Landscape`, `UpsideDown`, `LandscapeLeft`, `LandscapeRight`.
```yaml
- setOrientation: { mode: "LandscapeLeft" }
```

### `rotate`
Ra lệnh xoay màn hình (Simple).
```yaml
- rotate: "landscape"
```

---

## ⚡ Performance Testing

### `startProfiling`
Bắt đầu ghi nhận số liệu hiệu năng (CPU, RAM).
```yaml
- startProfiling:
    samplingIntervalMs: 1000
    package: "com.example.app"
```

### `stopProfiling`
Dừng ghi nhận và lưu báo cáo.
```yaml
- stopProfiling:
    savePath: "perf_report.json"
```

### `assertPerformance`
Kiểm tra hiệu năng không vượt quá ngưỡng.
```yaml
- assertPerformance:
    metric: "memory"
    limit: "200MB"
```

### `setCpuThrottling`
Giới hạn tốc độ CPU (giả lập máy yếu).
```yaml
- setCpuThrottling: 2.0 # Chậm hơn 2x
```

### `setNetworkConditions`
Giả lập mạng yếu.
```yaml
- setNetworkConditions: "3g" # edge, 3g, 4g, wifi
```

---

## 👁️ Assertions (Kiểm tra)

### `see` / `assertVisible`
Kiểm tra phần tử hiển thị.
```yaml
- see: "Welcome"
- see: 
    regex: "User \\d+"
```

### `notSee` / `assertNotVisible`
Kiểm tra phần tử KHÔNG hiển thị.
```yaml
- notSee: "Loading..."
```

### `waitNotSee`
Chờ cho tới khi phần tử biến mất (ví dụ chờ loading tắt).
```yaml
- waitNotSee:
    id: "loading_spinner"
    timeout: 10000
```

### `extendedWaitUntil`
Chờ điều kiện phức tạp với timeout tùy chỉnh.
```yaml
- extendedWaitUntil:
    visible: { text: "Success" }
    timeout: 30000
```

### `assert` / `assertTrue`
Kiểm tra điều kiện logic hoặc expression.
```yaml
- assert:
    condition: "${count} > 5"
```

### `assertVar`
So sánh giá trị biến.
```yaml
- assertVar:
    name: "status"
    equals: "active"
```

### `assertColor` / `checkColor`
Kiểm tra màu sắc pixel.
```yaml
- assertColor:
    point: "50%,50%"
    color: "#FF0000"
    tolerance: 10
```

### `assertScreenshot`
So sánh màn hình hiện tại với ảnh mẫu (Visual Regression).
```yaml
- assertScreenshot: "baseline/home.png"
```

### `assertClipboard`
Kiểm tra nội dung clipboard.
```yaml
- assertClipboard: "copied_text"
```

---

## 📋 Clipboard & Data Transfer

### `setClipboard`
Gán nội dung vào clipboard.
```yaml
- setClipboard: "123456"
```

### `getClipboard`
Lấy nội dung clipboard lưu vào biến.
```yaml
- getClipboard:
    name: "my_clip"
```

### `copyTextFrom`
Copy text từ một element.
```yaml
- copyTextFrom:
    id: "otp_code"
- copyTextFrom:
   text: "Code:" # Tìm element chứa text này và copy toàn bộ nội dung
```

### `pasteText`
Dán text từ clipboard.
```yaml
- pasteText
```

### `pushFile`
Đẩy file từ máy tính vào thiết bị.
```yaml
- pushFile:
    src: "./data.json"
    dest: "/sdcard/Download/data.json"
```

### `pullFile`
Lấy file từ thiết bị về máy tính.
```yaml
- pullFile:
    src: "/sdcard/photo.jpg"
    dest: "./evidence/photo.jpg"
```

---

## 🎲 Random Inputs (Dữ liệu ngẫu nhiên)

### `generate`
Sinh dữ liệu giả (faker) và lưu biến.
```yaml
- generate:
    name: "email"
    type: "email" # name, phone, uuid, password, number
```

### `inputRandomEmail`
Nhập email ngẫu nhiên vào ô focus.
```yaml
- inputRandomEmail
```

### `inputRandomName`
Nhập tên ngẫu nhiên.
```yaml
- inputRandomName
```

### `inputRandomText`
Nhập văn bản ngẫu nhiên.
```yaml
- inputRandomText:
    length: 10
```

### `inputRandomNumber`
Nhập số ngẫu nhiên.
```yaml
- inputRandomNumber:
    length: 6
```

---

## ⚙️ Logic & Control Flow

### `wait`
Chờ (ms).
```yaml
- wait: 1000
```

### `waitForAnimationToEnd`
Chờ UI ổn định (không còn chuyển động).
```yaml
- waitForAnimationToEnd
```

### `setVar`
Đặt biến.
```yaml
- setVar:
    name: "counter"
    value: 1
```

### `runFlow`
Chạy sub-flow.
```yaml
- runFlow: "subflows/login.yaml"
# Inline variables
- runFlow:
    path: "subflows/login.yaml"
    env:
      username: "test"
```

### `repeat`
Lặp lại.
```yaml
- repeat:
    times: 5
    commands: [...]
    
- repeat:
    while: 
        notSee: "End"
    commands: [...]
```

### `retry`
Thử lại khi lỗi.
```yaml
- retry:
    times: 3
    commands: [...]
```

### `conditional`
Điều kiện If-Else.
```yaml
- conditional:
    if: 
      - see: "Popup" # Supports visible, visibleRegex, notVisible
    then:
      - tap: "Close"
    else:
      - log: "No popup"
```

### `runScript`
Chạy Shell script trên máy tính.
```yaml
- runScript: "echo 'Hello' > log.txt"
```

### `evalScript`
Chạy javascript/script nhỏ để tính toán.
```yaml
- evalScript: "Date.now()"
```

### `httpRequest`
Gửi API request.
```yaml
- httpRequest:
    url: "https://api.example.com/status"
    method: "GET"
    saveResponse:
      status: "status_code" # Save specific field to var
```

### `dbQuery`
Thực thi SQL query.
```yaml
- dbQuery:
    connection: "postgres://..."
    query: "SELECT status FROM users WHERE id = 1"
    save:
      status: "user_status"
```

### `openLink` / `deepLink`
Mở Deep Link.
```yaml
- openLink: "myapp://home"
```

---

## 📷 Media & GIF

### `takeScreenshot`
```yaml
- takeScreenshot: "screen.png"
```

### `startRecording` / `stopRecording`
```yaml
- startRecording: "video"
- stopRecording
```

### `startGifCapture` / `stopGifCapture`
Tự động chụp frame để làm GIF.
```yaml
- startGifCapture:
    interval: 500
    maxFrames: 100
- ... operations ...
- stopGifCapture: "demo.gif"
```

### `captureFrame` / `createGif`
Tự tạo GIF thủ công.
```yaml
- captureFrame: "step1"
- createGif:
    frames: ["step1", "step2"]
    output: "manual.gif"
```

---

## 📍 Mock Location

### `mockLocation` / `gps`
Mô phỏng vị trí GPS.
```yaml
- gps:
    file: "route.gpx"
    speed: 40
    loop: true
```

### `mockLocationControl`
Điều khiển GPS động khi đang chạy.
```yaml
- mockLocationControl:
    speed: 60
    pause: true
```

### `stopMockLocation`
```yaml
- stopMockLocation
```
