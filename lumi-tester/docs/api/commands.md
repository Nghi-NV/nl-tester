# 📖 lumi-tester Command Reference

Tài liệu này liệt kê chi tiết đầy đủ **tất cả 138 lệnh (commands & aliases)** có thể sử dụng trong file YAML của `lumi-tester` trên các nền tảng **Android**, **Android Auto**, **iOS**, **Web**, **macOS**, **Windows** và **Hardware Jig Controller**.

Mỗi lệnh đều có ví dụ minh họa đầy đủ **tất cả các biến số và tham số có thể có** (từ dạng viết tắt đơn giản đến dạng đối tượng nâng cao).

---

## 📌 Bảng tra cứu nhanh lệnh (Quick Access Table Index)

| Danh mục | Lệnh & Kịch bản áp dụng |
| :--- | :--- |
| 🔍 **Selectors** | [`text`](#các-loại-selector-chính), [`id`](#các-loại-selector-chính), [`regex`](#các-loại-selector-chính), [`type`](#-tìm-hiểu-về-type-element-type), [`point`](#các-loại-selector-chính), [`ocr`](#-ocr-selector-nhận-diện-quang-học), [`relative`](#vị-trí-tương-đối-relative-positioning), [`auto-scroll`](#tự-động-cuộn-auto-scroll) |
| 📱 **App Lifecycle** | [`launchApp` / `open`](#open--launchapp), [`stopApp` / `stop`](#stopapp--stop), [`installApp`](#installapp), [`uninstallApp`](#uninstallapp), [`clearAppData`](#clearappdata), [`backgroundApp`](#backgroundapp) |
| 🗺️ **Navigation** | [`back`](#back), [`pressHome` / `home`](#presshome--home), [`hideKeyboard`](#hidekeyboard--hidekbd), [`openLink`](#openlink--deeplink), [`navigate`](#navigate) |
| 👆 **Interaction** | [`tap`](#tapon--tap), [`longPress`](#longpresson--longpress), [`doubleTap`](#doubletapon--doubletap), [`rightClick`](#rightclick--contextclick), [`inputText` / `write` / `type`](#inputtext--write--type), [`eraseText`](#erasetext--clear), [`tapAt` / `inputAt`](#tapat--inputat), [`pasteText`](#pastetext) |
| 📜 **Scroll & Swipe** | [`swipeLeft` / `swipeRight` / `swipeUp` / `swipeDown`](#swipeleft--swiperight--swipeup--swipedown--swipe), [`scrollUntilVisible` / `scrollTo`](#scrolluntilvisible--scrollto) |
| 👁️ **Assertions** | [`see` / `assertVisible`](#assertvisible--see), [`notSee`](#assertnotvisible--notsee), [`waitUntilVisible`](#waituntilvisible--waitsee), [`waitUntilNotVisible`](#waituntilnotvisible--waitnotsee), [`extendedWaitUntil`](#extendedwaituntil), [`assertColor`](#assertcolor--checkcolor), [`assertScreenshot`](#assertscreenshot) |
| ⏳ **Wait & Delays** | [`wait` / `await`](#wait--await), [`waitForAnimationToEnd`](#waitforanimationtoend) |
| 📦 **Variables** | [`find` / `define`](#find--define), [`setVar`](#setvar), [`assertVar`](#assertvar), [`generate`](#generate) |
| 🔀 **Control Flow** | [`repeat`](#repeat), [`retry`](#retry), [`runFlow`](#runflow), [`conditional`](#conditional) |
| 🐍 **Scripting & Python** | [`runPython` / `execPython` / `python`](#-scripting--python-integration-tích-hợp-script--python), [`runScript`](#runscript), [`evalScript`](#evalscript), [`assertTrue`](#asserttrue--assert) |
| 🌐 **Network & DB** | [`httpRequest`](#httprequest), [`setNetwork`](#setnetwork), [`airplaneMode`](#airplanemode--toggleairplanemode), [`dbQuery`](#dbquery) |
| 📋 **Clipboard & Files** | [`setClipboard` / `getClipboard` / `copyTextFrom`](#-clipboard--files-clipboard--quản-lý-file), [`pushFile` / `pullFile`](#pushfile--pullfile) |
| 📸 **Artifacts & Reports** | [`screenshot`](#screenshot--takescreenshot--snapshot), [`startRecording` / `stopRecording`](#startrecording--stoprecording), [`exportReport`](#exportreport), [`sendLarkMessage` / `lark`](#sendlarkmessage--lark) |
| 📍 **GPS Simulation** | [`mockLocation` / `gps`](#mocklocation--gps), [`mockLocationControl`](#stopmocklocation--mocklocationcontrol), [`waitForLocation`](#waitforlocation--waitformockcompletion) |
| 🎬 **GIF Recording** | [`captureGifFrame`](#-gif-recording-tạo-gif-minh-họa), [`buildGif`](#-gif-recording-tạo-gif-minh-họa) |
| ⚙️ **Hardware Relay/Servo** | [`connectJig`](#connectjig--disconnectjig), [`turnOn` / `turnOff` / `powerCycle`](#turnon--turnoff--turnoffall--powercycle), [`clickButton` / `holdButton` / `releaseButton`](#clickbutton--pressbutton--holdbutton--releasebutton--releaseallbuttons--repeatclick), [`rotateServo` / `startRepeatClick`](#rotateservo--configureservo--startrepeatclick--stoprepeatclick), [`readServo` / `readRelay`](#readservo--readrelay) |
| 🎨 **Hardware LED & Sensor** | [`seeLedColor` / `seeLedBlink`](#seeledcolor--seeledblink--seeledoff), [`setSensorLight`](#setsensorlight--setbrightnessthresholds--waitforbrightness--waitforcct), [`waitForBrightness` / `waitForCct`](#setsensorlight--setbrightnessthresholds--waitforbrightness--waitforcct), [`calibrateColor` / `saveCalibration`](#calibratecolor--calibratebrightness--addcctpoint--savecalibration--loadcalibration--resetcalibration--erasecalibration) |
| 📷 **Camera Profile** | [`assertDeviceState`](#-camera-profile-assertions-kiểm-tra-trạng-thái-qua-camera), [`waitDeviceState`](#-camera-profile-assertions-kiểm-tra-trạng-thái-qua-camera), [`assertDeviceTransition`](#-camera-profile-assertions-kiểm-tra-trạng-thái-qua-camera), [`waitLedPattern`](#-camera-profile-assertions-kiểm-tra-trạng-thái-qua-camera) |
| 🔊 **Audio & Media** | [`playMedia` / `stopMedia`](#-audio--media-playback-âm-thanh--truyền-thông), [`startAudioCapture` / `verifyAudioDucking`](#startaudiocapture--stopaudiocapture--verifyaudioducking) |
| 🛠️ **System Settings** | [`rotate`](#rotate--setorientation), [`press`](#press--presskey), [`inputRandomEmail` / `inputRandomNumber`](#random-inputs-inputrandomemail--inputrandomnumber--inputrandompersonname--inputrandomtext), [`setVolume` / `lockDevice`](#system-controls-opennotifications--openquicksettings--setvolume--lockdevice--unlockdevice--selectdisplay--setlocale), [`assertPerformance` / `setCpuThrottling`](#performance-profiling-startprofiling--stopprofiling--assertperformance--setcputhrottling--setnetworkconditions) |

---

## 🔍 Selectors & Global Parameters

Nhiều lệnh tương tác (như `tap`, `see`, `scrollTo`) sử dụng chung một bộ tham số để xác định phần tử trên màn hình.

### Các loại Selector chính
| Trường | Alias | Mô tả |
| :--- | :--- | :--- |
| `text` | - | Tìm theo văn bản hiển thị (khớp chính xác hoặc chứa chuỗi). |
| `id` | - | Resource ID (Android/Web), Accessibility ID (iOS), hoặc Name/AutomationId (Windows/macOS). |
| `regex` | - | Khớp văn bản bằng biểu thức chính quy (Regex). |
| `desc` | `contentDesc`, `accessibilityId` | Tìm theo mô tả nội dung (Accessibility Label). |
| `type` | `element_type` | Loại phần tử (Class View / XCUIElement / HTML tag). |
| `point` | - | Tọa độ tuyệt đối `"x,y"` hoặc phần trăm `"x%,y%"`. |
| `align` | - | Căn chỉnh điểm tương tác trong bounds của element: `left` (10%), `right` (90%), `top` (10%), `bottom` (90%), `center` (50%). |
| `offset` | - | Độ lệch tương đối % bên trong bounds của element: `"X%,Y%"` (VD: `"85%,50%"`). |
| `css` | - | (Chỉ Web) CSS Selector. |
| `xpath` | - | XPath Selector. |
| `image` | - | Template matching theo ảnh mẫu. |
| `ocr` | - | Tìm text bằng nhận diện quang học (OCR). Hỗ trợ regex. |

---

### 🎯 Căn chỉnh vị trí trong phần tử (`align` & `offset`)
Khi cần tương tác vào một thành phần con trong một view/item phức hợp (ví dụ: nút gạt Switch nằm ở cạnh phải của hàng cài đặt, icon xóa ở góc, v.v.):
- **`align` (Preset)**: `left` (10%,50%), `right` (90%,50%), `top` (50%,10%), `bottom` (50%,90%), `center` (50%,50%).
- **`offset` (Custom %)**: Chỉ định chính xác vị trí tương đối `"X%,Y%"` tính từ góc trên-trái của phần tử (0% đến 100%).

```yaml
# Tap vào nút toggle ở cạnh phải của Switch
- tap:
    type: "Switch"
    index: 1
    align: right

# Tap vào vị trí 85% chiều rộng, 50% chiều cao của hàng item
- tap:
    id: "settings_row"
    offset: "85%,50%"
```

---

---

### 🧱 Tìm hiểu về `type` (Element Type)
Trường `type` giúp thu hẹp phạm vi tìm kiếm bằng cách chỉ định loại thành phần:
- **Android**: `Button`, `EditText`, `TextView`, `ImageView`, `CheckBox`, `Switch`.
- **iOS**: `Button`, `TextField`, `SecureTextField`, `StaticText`, `Image`, `Cell`.
- **Web**: `input`, `button`, `a`, `span`, `div`, `p`.

---

### 🔍 Giải thích về Regex
- `\d+`: Dãy số bất kỳ.
- `\d{6}`: Mã OTP 6 chữ số.
- `.+`: Đoạn chữ bất kỳ.
- `(Nam|Nữ)`: Khớp "Nam" hoặc "Nữ".

---

### Vị trí tương đối (Relative Positioning)
- `rightOf`, `leftOf`, `above`, `below`.
```yaml
- tap:
    rightOf: "Username"
    type: "EditText"
    timeout: 5000
    optional: false
```

### 📷 OCR Selector (Nhận diện quang học)
```yaml
- tap:
    ocr:
      text: "Start Game"
      index: 0
      region: "bottom-half" # top-left, top-right, bottom-left, bottom-right, top-half, bottom-half, center
```

### Tự động cuộn (Auto-scroll)
```yaml
- tap:
    text: "Save"
    scrollable:
      enable: true
      index: 0
```

---

## 📱 App Management & Lifecycle (Quản lý Ứng dụng)

### `open` / `launchApp`
**Mô tả**: Mở một ứng dụng trên thiết bị. Hỗ trợ Package name (Android), Bundle ID (iOS), `.app` (macOS), hoặc `.exe` path (Windows). Trên macOS/Windows cần cấu hình `desktopState.clear` ở header để xóa dữ liệu desktop state.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt (Shorthand)
- launchApp

# 2. Đầy đủ các biến số đối tượng
- launchApp:
    appId: "com.example.app"
    clearState: true
    permissions:
      notifications: "allow"
      location: "always"
      camera: "deny"
      microphone: "allow"
```

### `stopApp` / `stop`
**Mô tả**: Dừng (kill) ứng dụng đang chạy.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt (Shorthand) - Dừng app mặc định từ header
- stopApp

# 2. Chỉ định appId cụ thể
- stopApp: "com.example.app"
```

### `installApp`
**Mô tả**: Cài đặt file ứng dụng (.apk, .ipa, .app path) vào thiết bị.

Ví dụ đầy đủ các tham số:
```yaml
- installApp: "./builds/app-debug.apk"
```

### `uninstallApp`
**Mô tả**: Gỡ cài đặt ứng dụng theo package/bundle ID.

Ví dụ đầy đủ các tham số:
```yaml
- uninstallApp: "com.example.app"
```

### `clearAppData`
**Mô tả**: Xóa dữ liệu và cache của ứng dụng Android. Không dùng lệnh này cho macOS/Windows; desktop cần `desktopState.clear` trong header và `launchApp: { clearState: true }`.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Xóa app mặc định
- clearAppData

# 2. Chỉ định package name cụ thể
- clearAppData: "com.example.app"
```

### `backgroundApp`
**Mô tả**: Đưa ứng dụng xuống nền trong khoảng thời gian quy định rồi tự mở lại.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt thời gian ms
- backgroundApp: 5000

# 2. Đối tượng cấu hình
- backgroundApp:
    durationMs: 5000
```

---

## 🗺️ Navigation & Links (Điều hướng)

### `back`
**Mô tả**: Nhấn nút Back hệ thống (Esc trên Desktop).

```yaml
- back
```

### `pressHome` / `home`
**Mô tả**: Nhấn nút Home để về màn hình chính.

```yaml
- home
```

### `hideKeyboard` / `hideKbd`
**Mô tả**: Ẩn bàn phím ảo.

```yaml
- hideKeyboard
```

### `openLink` / `deepLink`
**Mô tả**: Mở Deep Link hoặc URL quy định.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Deep link đơn giản
- openLink: "myapp://settings/profile"

# 2. Mở URL Web
- openLink: "https://example.com/checkout?user=123"
```

### `navigate`
**Mô tả**: Điều hướng trang Web tới URL mới.

```yaml
- navigate: "https://example.com/dashboard"
```

---

## 👆 Interaction & Input (Tương tác & Nhập liệu)

### `tapOn` / `tap`
**Mô tả**: Nhấn (click) vào phần tử hoặc tọa độ.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt theo Text
- tap: "Login"

# 2. Đầy đủ các trường biến số đối tượng
- tap:
    id: "btn_submit"
    text: "Submit"
    type: "Button"
    index: 0
    align: right # left | right | top | bottom | center
    offset: "85%,50%" # Custom offset trong bounds của element
    timeout: 5000
    optional: true
    scrollable:
      enable: true
      index: 0
```

### `longPressOn` / `longPress`
**Mô tả**: Nhấn giữ phần tử trên màn hình.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt
- longPress: "Delete Item"

# 2. Đầy đủ các trường đối tượng
- longPress:
    id: "item_row_1"
    align: left
    offset: "15%,50%"
    durationMs: 2000
    timeout: 5000
```

### `doubleTapOn` / `doubleTap`
**Mô tả**: Nhấn nhanh 2 lần liên tiếp.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt
- doubleTap: "photo_thumbnail"

# 2. Đầy đủ đối tượng
- doubleTap:
    id: "photo_thumbnail"
    align: center
    timeout: 3000
```

### `rightClick` / `contextClick`
**Mô tả**: Click chuột phải (dùng trên Web, macOS, Windows).

Ví dụ đầy đủ các tham số:
```yaml
- rightClick:
    text: "File.txt"
    id: "file_node_12"
    align: right
```

### `click`
**Mô tả**: Lệnh click đơn giản trên Web.

```yaml
- click: "Sign In"
```

### `inputText` / `write` / `type`
**Mô tả**: Nhập văn bản vào ô đang focus hoặc ô chỉ định.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Nhập vào ô đang focus
- inputText: "user@example.com"

# 2. Nhập vào ô có selector cụ thể với tất cả tham số
- write:
    text: "Xin chào Việt Nam"
    selector: "input_address"
    id: "field_address"
    unicode: true
    clearFirst: true
```

### `eraseText` / `clear`
**Mô tả**: Xóa ký tự trong ô đang nhập.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Xóa toàn bộ
- eraseText

# 2. Xóa N ký tự
- eraseText: 5
```

### `tapAt` / `inputAt`
**Mô tả**: Tương tác theo loại phần tử và thứ tự index (Fallback khi không có ID/Text).

Ví dụ đầy đủ các tham số:
```yaml
- tapAt:
    type: "Button"
    index: 1

- inputAt:
    type: "EditText"
    index: 0
    text: "admin@example.com"
```

### `pasteText`
**Mô tả**: Dán nội dung clipboard vào ô đang chọn.

```yaml
- pasteText
```

---

## 📜 Scroll & Swipe (Cuộn & Vuốt)

### `swipeLeft` / `swipeRight` / `swipeUp` / `swipeDown` / `swipe`
**Mô tả**: Vuốt màn hình theo hướng chỉ định.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt hướng
- swipeLeft
- swipeUp

# 2. Đầy đủ các trường đối tượng
- swipe:
    direction: "up"
    distance: 0.8  # Tỷ lệ chiều dài vuốt (0.1 -> 1.0)
    duration: 500  # Thời gian vuốt (ms)
    startX: "50%"
    startY: "80%"
    endX: "50%"
    endY: "20%"
```

### `scrollUntilVisible` / `scrollTo`
**Mô tả**: Cuộn màn hình liên tục cho đến khi thấy phần tử mục tiêu xuất hiện.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt theo Text
- scrollTo: "Bottom Link"

# 2. Đầy đủ các biến số đối tượng
- scrollUntilVisible:
    id: "target_card"
    text: "Mục 50"
    direction: "down"  # down, up, left, right
    maxScrolls: 15
    scrollDistance: 0.7
```

---

## 👁️ Assertions & Visibility (Kiểm tra giao diện)

### `assertVisible` / `see`
**Mô tả**: Kiểm tra phần tử hiển thị trên màn hình.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt
- see: "Welcome Back"

# 2. Đầy đủ biến số
- assertVisible:
    id: "header_title"
    text: "Trang chủ"
    type: "TextView"
    timeout: 8000
    soft: true  # soft assertion: không ngắt flow khi không thấy
```

### `assertNotVisible` / `notSee`
**Mô tả**: Kiểm tra phần tử KHÔNG xuất hiện trên màn hình.

Ví dụ đầy đủ các tham số:
```yaml
- notSee: "Error Message"

- assertNotVisible:
    id: "loading_spinner"
    timeout: 5000
```

### `waitUntilVisible` / `waitSee`
**Mô tả**: Chờ cho đến khi phần tử xuất hiện.

Ví dụ đầy đủ các tham số:
```yaml
- waitSee:
    id: "dashboard_loaded"
    text: "Hoàn tất"
    timeout: 10000
```

### `waitUntilNotVisible` / `waitNotSee`
**Mô tả**: Chờ cho đến khi phần tử biến mất (VD: Loading overlay).

Ví dụ đầy đủ các tham số:
```yaml
- waitNotSee:
    id: "loading_overlay"
    timeout: 15000
```

### `extendedWaitUntil`
**Mô tả**: Chờ nhiều điều kiện visible / notVisible cùng lúc.

Ví dụ đầy đủ các tham số:
```yaml
- extendedWaitUntil:
    timeout: 15000
    visible:
      id: "success_dialog"
    notVisible:
      id: "spinner"
```

### `assertColor` / `checkColor`
**Mô tả**: Kiểm tra màu pixel tại tọa độ chỉ định.

Ví dụ đầy đủ các tham số:
```yaml
- assertColor:
    point: "50%,50%"
    color: "#00FF00"
    tolerance: 5
```

### `assertScreenshot`
**Mô tả**: So sánh ảnh màn hình hiện tại với ảnh mẫu baseline.

Ví dụ đầy đủ các tham số:
```yaml
- assertScreenshot:
    baseline: "baselines/home.png"
    tolerancePercent: 1.5
```

### `assertClipboard`
**Mô tả**: Kiểm tra nội dung clipboard.

```yaml
- assertClipboard: "Copied Token"
```

---

## ⏳ Wait & Delays (Tạm dừng)

### `wait` / `await`
**Mô tả**: Tạm dừng cố định khoảng thời gian (ms).

```yaml
- wait: 2000 # Tạm dừng 2s
```

### `waitForAnimationToEnd`
**Mô tả**: Chờ hiệu ứng chuyển cảnh ổn định.

```yaml
- waitForAnimationToEnd
```

---

## 📦 Variables & Selectors (Biến & Bộ chọn tái sử dụng)

### `find` / `define`
**Mô tả**: Định nghĩa bộ chọn selector và gán tên biến để tái sử dụng.

Ví dụ đầy đủ các tham số:
```yaml
- find:
    name: "submit_btn"
    id: "btn_submit"
    text: "Gửi dữ liệu"
    type: "Button"

- tap: "${submit_btn}"
```

### `setVar`
**Mô tả**: Gán giá trị vào biến.

Ví dụ đầy đủ các tham số:
```yaml
- setVar:
    name: "user_name"
    value: "John Doe"
```

### `assertVar`
**Mô tả**: Kiểm tra giá trị biến.

Ví dụ đầy đủ các tham số:
```yaml
- assertVar:
    name: "user_name"
    equals: "John Doe"
    contains: "John"
```

### `generate`
**Mô tả**: Sinh dữ liệu ngẫu nhiên (Faker) và lưu vào biến.

Ví dụ đầy đủ các tham số:
```yaml
- generate:
    name: "random_email"
    type: "email" # uuid, email, phone, name, address, number, date
    length: 8
```

---

## 🔀 Control Flow & Logic (Luồng điều khiển)

### `repeat`
**Mô tả**: Vòng lặp thực thi danh sách lệnh.

Ví dụ đầy đủ các tham số:
```yaml
- repeat:
    times: 3
    commands:
      - tap: "Next"
      - wait: 500
```

### `retry`
**Mô tả**: Thử lại khối lệnh nếu xảy ra lỗi.

Ví dụ đầy đủ các tham số:
```yaml
- retry:
    maxRetries: 3
    delayMs: 1000
    commands:
      - tap: "Submit"
      - see: "Success"
```

### `runFlow`
**Mô tả**: Chạy file test sub-flow YAML khác.

Ví dụ đầy đủ các tham số:
```yaml
- runFlow:
    path: "./common/login.yaml"
    vars:
      user: "admin"
      pass: "123456"
```

### `conditional`
**Mô tả**: Cấu trúc rẽ nhánh If / Then / Else theo phần tử UI.

Ví dụ đầy đủ các tham số:
```yaml
- conditional:
    condition:
      visible: "Update Available"
    then:
      - tap: "Skip"
    else:
      - log: "No update popup"
```

---

## 🐍 Scripting & Python Integration (Tích hợp Script & Python)

### `runPython` / `execPython` / `python`
**Mô tả**: Thực thi file Python hoặc đoạn mã Python inline, truyền tham số và lưu biến kết quả vào context test flow.

Ví dụ đầy đủ tất cả các biến số có thể:
```yaml
# 1. Thực thi đoạn mã inline và lưu stdout
- runPython:
    code: |
      import sys, json
      print(json.dumps({"token": "ABC123XYZ", "status": 200}))
    saveVar: "raw_output"

# 2. Đầy đủ các biến số với file script, args, env, timeout, pythonPath và saveVars
- runPython:
    script: "./scripts/helper.py"
    args: ["--mode", "test", "--serial", "$DEVICE_SERIAL"]
    env:
      API_SECRET: "my_secret_key"
    timeoutMs: 15000
    pythonPath: "./venv/bin/python"
    saveVars:
      auth_token: "token"
      res_status: "status"
```

### `runScript`
**Mô tả**: Thực thi lệnh Shell script trên máy Host.

Ví dụ đầy đủ các tham số:
```yaml
- runScript:
    command: "python3"
    args: ["process.py", "--flag"]
    saveOutput: "script_result"
    timeoutMs: 10000
```

### `evalScript`
**Mô tả**: Thực thi mã JavaScript để tính toán biểu thức.

```yaml
- evalScript: "Date.now()"
```

### `assertTrue` / `assert`
**Mô tả**: Kiểm tra biểu thức điều kiện logic.

```yaml
- assertTrue: "${status_code} == 200"
```

---

## 🌐 Network & Database (Mạng & Cơ sở dữ liệu)

### `httpRequest`
**Mô tả**: Gửi HTTP REST API request.

Ví dụ đầy đủ các tham số:
```yaml
- httpRequest:
    url: "https://api.example.com/login"
    method: "POST"
    headers:
      Content-Type: "application/json"
      Authorization: "Bearer ${token}"
    body:
      username: "admin"
      password: "password123"
    timeoutMs: 10000
    saveResponse:
      "$.data.token": "api_token"
      "$.status": "api_status"
```

### `setNetwork`
**Mô tả**: Bật/tắt WiFi hoặc Mobile Data trên Android.

Ví dụ đầy đủ các tham số:
```yaml
- setNetwork:
    wifi: true
    data: false
```

### `airplaneMode` / `toggleAirplaneMode`
**Mô tả**: Bật/Tắt chế độ máy bay.

```yaml
- airplaneMode
```

### `dbQuery`
**Mô tả**: Thực hiện truy vấn SQL vào cơ sở dữ liệu.

Ví dụ đầy đủ các tham số:
```yaml
- dbQuery:
    connection: "postgres://user:pass@localhost:5432/db"
    query: "SELECT status, role FROM users WHERE id = ?"
    params: ["123"]
    save:
      "status": "user_status"
      "role": "user_role"
```

---

## 📋 Clipboard & Files (Clipboard & Quản lý File)

### `setClipboard` / `getClipboard` / `assertClipboard` / `copyTextFrom` / `pasteText`
Ví dụ đầy đủ các tham số:
```yaml
- setClipboard: "SecretOTP123"

- getClipboard: "my_copied_code"

- copyTextFrom:
    id: "otp_label"

- pasteText
```

### `pushFile` / `pullFile`
**Mô tả**: Truyền file giữa máy Host và thiết bị Android.

Ví dụ đầy đủ các tham số:
```yaml
- pushFile:
    source: "./config.json"
    destination: "/sdcard/config.json"

- pullFile:
    source: "/sdcard/log.txt"
    destination: "./output/log.txt"
```

---

## 📸 Artifacts & Reporting (Báo cáo & Ảnh chụp)

### `screenshot` / `takeScreenshot` / `snapshot`
**Mô tả**: Chụp ảnh màn hình thiết bị.

```yaml
- screenshot: "login_screen.png"
```

### `startRecording` / `stopRecording`
**Mô tả**: Quay video màn hình kiểm thử.

```yaml
- startRecording: "test_run.mp4"
- wait: 5000
- stopRecording
```

### `exportReport`
**Mô tả**: Xuất báo cáo kết quả ra định dạng HTML / JSON.

Ví dụ đầy đủ các tham số:
```yaml
- exportReport:
    path: "./output/report.json"
    format: "json" # json, html
```

### `sendLarkMessage` / `lark`
**Mô tả**: Gửi thông báo kết quả qua Lark / Feishu Bot.

Ví dụ đầy đủ các tham số:
```yaml
- lark:
    webhook: "https://open.larksuite.com/open-apis/bot/v2/hook/xxx"
    title: "Kết quả kiểm thử Automated"
    content: "Test suite chạy hoàn tất 100% pass!"
    status: "success" # success, failed
```

---

## 📍 Location & GPS Simulation (Giả lập vị trí GPS)

### `mockLocation` / `gps`
**Mô tả**: Phát di chuyển vị trí GPS giả lập theo tuyến đường file GPX/KML.

Ví dụ đầy đủ các tham số:
```yaml
- gps:
    file: "./routes/hanoi_drive.gpx"
    speed: 50 # km/h
    loop: true
    intervalMs: 1000
```

### `stopMockLocation` / `mockLocationControl`
Ví dụ đầy đủ các tham số:
```yaml
- mockLocationControl:
    speed: 80
    pause: false

- stopMockLocation
```

### `waitForLocation` / `waitForMockCompletion`
Ví dụ đầy đủ các tham số:
```yaml
- waitForLocation:
    lat: 21.0278
    lon: 105.8342
    tolerance: 15.0

- waitForMockCompletion: 60000
```

---

## 🎬 GIF Recording (Tạo GIF minh họa)

### `captureGifFrame` / `startGifCapture` / `stopGifCapture` / `buildGif`
Ví dụ đầy đủ các tham số:
```yaml
- captureGifFrame: "step1"
- tap: "Next"
- captureGifFrame: "step2"
- buildGif:
    frames: ["step1", "step2"]
    output: "result.gif"
    delay: 500
```

---

## ⚙️ Hardware Jig Controller (Điều khiển Phần cứng Jig STM32/Relay/Servo)

Lumi Tester hỗ trợ giao tiếp trực tiếp qua Cổng COM/TTY tới mạch nạp và bộ Jig điều khiển phần cứng tự động hóa cho thiết bị IoT / Smart Home.

### `connectJig` / `disconnectJig`
**Mô tả**: Kết nối tới mạch phần cứng Jig Controller qua cổng RS232/USB Serial.

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt cổng COM
- connectJig: "COM5"

# 2. Đầy đủ các tham số đối tượng
- connectJig:
    port: "COM5"
    baudrate: 115200
    timeoutMs: 3000
```

### `turnOn` / `turnOff` / `turnOffAll` / `powerCycle`
**Mô tả**: Điều khiển đóng/ngắt các kênh Rơ-le (Relay) cấp nguồn phần cứng.

Ví dụ đầy đủ các tham số:
```yaml
- turnOn: 1       # Bật nguồn kênh 1
- turnOff: 1      # Tắt nguồn kênh 1
- turnOffAll      # Tắt toàn bộ rơ-le
- powerCycle:     # Khởi động lại nguồn (Tắt 2s rồi bật lại)
    channel: 1
    offMs: 2000
```

### `clickButton` / `pressButton` / `holdButton` / `releaseButton` / `releaseAllButtons` / `repeatClick`
**Mô tả**: Điều khiển động cơ Servo nhấn nút vật lý trên thiết bị.

Ví dụ đầy đủ các tham số:
```yaml
- clickButton: 1         # Click nút vật lý kênh 1
- pressButton: 1         # Nhấn đè nút kênh 1
- holdButton: 1          # Nhấn giữ nút (cho chế độ Pairing / Reset)
- releaseButton: 1       # Nhả nút kênh 1
- releaseAllButtons      # Nhả tất cả các nút về vị trí nghỉ
- repeatClick:           # Nhấn nhấp nhả 3 lần liên tiếp
    channel: 1
    count: 3
    intervalMs: 300
```

### `rotateServo` / `configureServo` / `startRepeatClick` / `stopRepeatClick`
**Mô tả**: Cấu hình góc xoay Servo chi tiết và điều khiển vòng lặp nhấn nhả tự động phần cứng trên STM32.

Ví dụ đầy đủ các tham số:
```yaml
- rotateServo:
    channel: 1
    angle: 90
    speed: 50

- configureServo:
    channel: 1
    pressAngle: 75
    releaseAngle: 0
    pressDurationMs: 200

- startRepeatClick:
    channel: 1
    periodMs: 1500

- stopRepeatClick: 1
```

### `readServo` / `readRelay`
**Mô tả**: Đọc trạng thái phản hồi từ Servo và Relay.

```yaml
- readServo: 1
- readRelay: 1
```

---

## 🎨 Hardware LED & Color Sensor (Cảm biến màu & LED Phần cứng)

### `seeLedColor` / `seeLedBlink` / `seeLedOff`
**Mô tả**: Kiểm tra và đợi trạng thái đèn LED phần cứng (RED, GREEN, BLUE, YELLOW, CYAN, MAGENTA, PINK, WHITE, OFF).

Ví dụ đầy đủ các tham số:
```yaml
# 1. Viết tắt màu
- seeLedColor: "GREEN"

# 2. Đầy đủ các trường đối tượng
- seeLedColor:
    channel: 1
    expected: "BLUE"
    timeoutMs: 5000

- seeLedBlink:
    channel: 1
    periodMs: 500

- seeLedOff: 1   # Đợi LED tắt
```

### `setSensorLight` / `setBrightnessThresholds` / `waitForBrightness` / `waitForCct`
**Mô tả**: Điều khiển đèn chiếu cảm biến màu và cài đặt ngưỡng độ sáng / nhiệt độ màu (CCT Kelvin).

Ví dụ đầy đủ các tham số:
```yaml
- setSensorLight: "on"

- setBrightnessThresholds:
    channel: 1
    offBelowPercent: 30
    onAbovePercent: 70

- waitForBrightness:
    channel: 1
    minPercent: 70

- waitForCct:
    channel: 1
    minKelvin: 2700
    maxKelvin: 6500
```

### `calibrateColor` / `calibrateBrightness` / `addCctPoint` / `saveCalibration` / `loadCalibration` / `resetCalibration` / `eraseCalibration`
**Mô tả**: Hiệu chỉnh và lưu trữ dữ liệu cân bằng trắng / màu sắc cảm biến vào bộ nhớ Flash.

Ví dụ đầy đủ các tham số:
```yaml
- calibrateColor:
    channel: 1
    color: "RED"

- saveCalibration
- loadCalibration
```

### `enterSafeState` / `systemDiagnostics` / `readColor` / `readSensorLight`
**Mô tả**: Ngắt an toàn khẩn cấp, chẩn đoán hệ thống MCU và đọc giá trị màu RGBC thực tế.

Ví dụ đầy đủ các tham số:
```yaml
- enterSafeState
- systemDiagnostics
- readColor: 1
- readSensorLight
```

---

## 📷 Camera Profile Assertions (Kiểm tra trạng thái qua Camera)

### `assertDeviceState` / `waitDeviceState` / `assertDeviceTransition` / `waitLedPattern` / `getDeviceState`
**Mô tả**: Nhận diện vùng đèn LED thiết bị qua Camera profile.

Ví dụ đầy đủ các tham số:
```yaml
- assertDeviceState:
    button: "power_led"
    expect: "GREEN"

- waitDeviceState:
    button: "status_led"
    expect: "BLUE"
    withinMs: 5000

- assertDeviceTransition:
    button: "status_led"
    from: "OFF"
    to: "RED"

- waitLedPattern:
    button: "status_led"
    expect: "PINK"
    count: 3

- getDeviceState:
    saveAs: "current_device_leds"
```

---

## 🔊 Audio & Media Playback (Âm thanh & Truyền thông)

### `playMedia` / `stopMedia`
**Mô tả**: Phát file âm thanh mẫu (.wav, .mp3) ra loa thiết bị.

Ví dụ đầy đủ các tham số:
```yaml
- playMedia:
    file: "./audio/voice_command.wav"
    loopPlayback: false
    volume: 80

- stopMedia
```

### `startAudioCapture` / `stopAudioCapture` / `verifyAudioDucking`
**Mô tả**: Ghi âm tín hiệu mic và kiểm tra hiện tượng giảm âm lượng (Audio Ducking).

Ví dụ đầy đủ các tham số:
```yaml
- startAudioCapture:
    duration: 10000
    savePath: "./output/recorded_voice.wav"

- stopAudioCapture

- verifyAudioDucking:
    minDuckingCount: 1
    volumeDropThreshold: 0.3
```

---

## 🛠️ Device & System Settings (Thiết lập Thiết bị & Hệ thống)

### `rotate` / `setOrientation`
**Mô tả**: Xoay màn hình thiết bị.

```yaml
- rotate: "landscape"

- setOrientation: "LANDSCAPE"
```

### `press` / `pressKey`
**Mô tả**: Nhấn phím cứng hoặc phím hệ thống (ENTER, BACK, HOME, VOLUME_UP, DPAD_CENTER).

Ví dụ đầy đủ các tham số:
```yaml
- press: "ENTER"

- pressKey:
    key: "BACK"
    times: 2
```

### Random Inputs: `inputRandomEmail` / `inputRandomNumber` / `inputRandomPersonName` / `inputRandomText`
**Mô tả**: Nhập ngẫu nhiên dữ liệu vào ô đang chọn.

Ví dụ đầy đủ các tham số:
```yaml
- inputRandomEmail
- inputRandomNumber: { length: 6 }
- inputRandomPersonName
- inputRandomText: { length: 10 }
```

### System Controls: `openNotifications` / `openQuickSettings` / `setVolume` / `lockDevice` / `unlockDevice` / `selectDisplay` / `setLocale`
Ví dụ đầy đủ các tham số:
```yaml
- openNotifications
- openQuickSettings
- setVolume: 80
- lockDevice
- unlockDevice
- display: 1
- locale: "vi_VN"
```

### Performance Profiling: `startProfiling` / `stopProfiling` / `assertPerformance` / `setCpuThrottling` / `setNetworkConditions`
Ví dụ đầy đủ các tham số:
```yaml
- startProfiling:
    samplingIntervalMs: 500

- wait: 10000

- stopProfiling:
    savePath: "./output/profile.trace"

- assertPerformance:
    metric: "memory" # memory, cpu, fps
    limit: "250MB"

- setCpuThrottling: 4.0

- setNetworkConditions: "slow-3g"
```
