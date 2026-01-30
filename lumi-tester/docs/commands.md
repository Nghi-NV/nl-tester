# 📖 lumi-tester Command Reference

Tài liệu này liệt kê chi tiết tất cả các lệnh (commands) có thể sử dụng trong file YAML của `lumi-tester`.

---

## � Selectors & Global Parameters

Nhiều lệnh tương tác (như `tap`, `see`, `scrollTo`) sử dụng chung một bộ tham số để xác định phần tử trên màn hình.

### Các loại Selector chính
| Trường | Alias | Mô tả |
| :--- | :--- | :--- |
| `text` | - | Tìm theo văn bản hiển thị. |
| `id` | - | Resource ID (Android/Web). |
| `regex` | - | Khớp văn bản bằng biểu thức chính quy (Regex). Xem giải thích bên dưới. |
| `desc` | `contentDesc`, `accessibilityId` | Tìm theo mô tả nội dung (Accessibility Label). |
| `type` | `element_type` | Loại của phần tử (Class name). Xem chi tiết bên dưới. |
| `point` | - | Tọa độ tuyệt đối `"x,y"` hoặc phần trăm `"x%,y%"`. |
| `css` | - | (Chỉ Web) CSS Selector. |
| `xpath` | - | XPath Selector. |
| `image` | - | Template matching theo ảnh mẫu. |

---

### 🧱 Tìm hiểu về `type` (Element Type)
Trường `type` giúp bạn thu hẹp phạm vi tìm kiếm bằng cách chỉ định loại "thành phần" trên màn hình. Mỗi nền tảng sẽ có các tên loại khác nhau:

**Android (Tên Class của View):**
- `Button`: Các nút bấm.
- `EditText`: Các ô nhập văn bản.
- `TextView`: Các đoạn văn bản hiển thị (nhãn).
- `ImageView`: Các hình ảnh, icon.
- `CheckBox`, `Switch`: Các nút gạt, đánh dấu.

**iOS (XCUIElement Types):**
- `Button`: Nút bấm.
- `TextField`, `SecureTextField`: Ô nhập liệu (thường và bảo mật).
- `StaticText`: Văn bản hiển thị.
- `Image`: Hình ảnh.
- `Cell`: Một dòng trong danh sách.

**Web (HTML Tags):**
- `input`: Các ô nhập liệu.
- `button`: Các nút bấm.
- `a`: Các đường dẫn (link).
- `span`, `div`, `p`: Các khối văn bản.

---

---

### 🔍 Giải thích về Regex (Dễ hiểu nhất)
**Regex** (biểu thức chính quy) giống như một **"bộ lọc thông minh"**. Thay vì tìm một từ chính xác, bạn mô tả cho máy tính biết "hình dáng" của từ đó.

**Các ký tự "thần kỳ" hay dùng:**
*   `\d+`: Đại diện cho **một dãy số bất kỳ**. (Ví dụ: `1`, `100`, `999`).
*   `.+`: Đại diện cho **một đoạn chữ bất kỳ**. (Ví dụ: `abc`, `Hello 123`).
*   `.*`: Giống `.+` nhưng có thể là **không có chữ nào** (chuỗi rỗng).
*   `\d{6}`: Tìm chính xác **6 con số** (Rất hay dùng để tìm mã OTP).
*   `(A|B)`: Tìm chữ A **HOẶC** chữ B. (Ví dụ: `(Nam|Nữ)`).
*   `^` và `$`: Đánh dấu bắt đầu và kết thúc (tìm chính xác cả câu).

**Ví dụ thực tế:**
*   `Mã OTP là: \d{6}`: Sẽ tìm thấy các câu như "Mã OTP là: 123456" hay "Mã OTP là: 987654".
*   `Chào mừng .+`: Sẽ tìm thấy "Chào mừng Nam", "Chào mừng Admin",... (bất cứ tên nào).
*   `Xác nhận (thành công|thất bại)`: Tìm thấy cả 2 trường hợp "Xác nhận thành công" hoặc "Xác nhận thất bại".

---

### Vị trí tương đối (Relative Positioning)
Dùng để tìm phần tử dựa trên một "mỏ neo" (Anchor) khác.
- `rightOf`, `leftOf`, `above`, `below`.
- Ví dụ:
```yaml
- tap:
    rightOf: "Username"
    type: "EditText"
```

### Tự động cuộn (Auto-scroll)
Nếu phần tử không có sẵn trên màn hình, bạn có thể kích hoạt tự động cuộn trong selector.
```yaml
tap:
  text: "Save"
  scrollable:
    enable: true
    index: 0 # Index của vùng cuộn nếu có nhiều vùng
```

---

## �📱 App Management (Quản lý Ứng dụng)

### `open` / `launchApp`
**Mô tả**: Mở một ứng dụng trên thiết bị. Có thể xóa dữ liệu app hoặc cấp quyền trước khi mở.

**Ví dụ**:
```yaml
# Mở đơn giản bằng appId
- open: "com.example.app"

# Mở với cấu hình nâng cao
- launchApp:
    appId: "com.example.app"
    clearState: true
    permissions:
      notifications: "allow"
      location: "always"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `appId` | `url` | String | - | Package name (Android) hoặc Bundle ID (iOS). |
| `clearState`| - | Boolean | `false` | Xóa dữ liệu ứng dụng (Clean Install) trước khi mở. |
| `clearKeychain`| - | Boolean | `false` | Xóa Keychain (chỉ áp dụng iOS Simulator). |
| `stopApp` | - | Boolean | `true` | Dừng ứng dụng nếu đang chạy trước khi mở lại. |
| `permissions`| - | Map | - | Danh sách quyền cần thiết lập (key là tên quyền, value là `allow`/`deny`). |

**Giá trị Enum/Đặc biệt**:
- `permissions`:
    - Key: `all`, `notifications`, `location`, `camera`, `microphone`, `storage`, v.v.
    - Value: `allow`, `deny`, `always`, `while_in_use`.

---

### `stopApp` / `stop`
**Mô tả**: Dừng (kill) ứng dụng đang chạy.

**Ví dụ**:
```yaml
- stopApp: "com.example.app"
```

---

### `installApp`
**Mô tả**: Cài đặt một ứng dụng từ file (.apk, .ipa) trên máy tính vào thiết bị.

**Ví dụ**:
```yaml
- installApp: "./builds/app-debug.apk"
```

---

### `uninstallApp`
**Mô tả**: Gỡ cài đặt ứng dụng khỏi thiết bị.

**Ví dụ**:
```yaml
- uninstallApp: "com.example.app"
```

---

### `backgroundApp`
**Mô tả**: Đưa ứng dụng vào nền (Background) trong một khoảng thời gian rồi tự động quay lại.

**Ví dụ**:
```yaml
- backgroundApp:
    seconds: 5 # Đưa vào nền 5 giây
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `seconds`| - | Number | Số giây để ứng dụng ở trong nền. |

---

### `clearAppData`
**Mô tả**: Xóa dữ liệu và cache của ứng dụng (Reset app).

**Ví dụ**:
```yaml
- clearAppData: "com.example.app"
```

---

### `installApp`
**Mô tả**: Cài đặt ứng dụng từ file cục bộ vào thiết bị.

**Ví dụ**:
```yaml
- installApp: "./apps/my_app_debug.apk"
```

---

### `uninstallApp`
**Mô tả**: Gỡ cài đặt ứng dụng khỏi thiết bị.

**Ví dụ**:
```yaml
- uninstallApp: "com.example.app"
```

---

### `backgroundApp`
**Mô tả**: Đưa ứng dụng xuống nền (background) trong một khoảng thời gian rồi tự động mở lại.

**Ví dụ**:
```yaml
- backgroundApp:
    durationMs: 5000 # Ở background 5 giây
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `appId` | String | App hiện tại | App ID cần đưa xuống background. |
| `durationMs`| Number | `5000` | Thời gian ở background (mili giây). |

---

### `back`
**Mô tả**: Quay lại màn hình trước đó (Nút Back hệ thống).
**Aliases**: `back`

**Ví dụ**:
```yaml
- back
```

---

### `pressHome` / `home`
**Mô tả**: Nhấn nút Home để về màn hình chính.
**Aliases**: `pressHome`, `home`

**Ví dụ**:
```yaml
- home
```

---

### `selectDisplay` / `display`
**Mô tả**: Chọn màn hình hiển thị để tương tác (dùng cho các hệ thống nhiều màn hình như Android Auto).

**Ví dụ**:
```yaml
- selectDisplay: "0" # Màn hình chính
- display: "1"       # Màn hình phụ
```

**Giá trị Enum/Đặc biệt**:
- `id`: Thường là `0` (Main), `1` (Secondary/External).

---

### `setLocale`
Change the device locale (Android only).

```yaml
- setLocale: "en_US"
```

### `sendLarkMessage`

Send a notification message to Lark/Feishu via Custom Bot.
Supports variable substitution (`${time}`, `${date}`) and embedding file content.
If `secret` is provided, the message will be signed (HMAC-SHA256).

```yaml
- sendLarkMessage:
    webhook: "https://open.larksuite.com/open-apis/bot/v2/hook/..."
    secret: "optional_secret_key"
    title: "Test Report ${date}"
    content: "All tests passed at ${time}"
    status: "success" # success, failure, info, warning
    files:
      - "./output/report.json"
```

## Clipboard

---

## 👆 Interaction (Tương tác)

### `tap` / `tapOn`
**Mô tả**: Chạm (Click) vào một phần tử trên màn hình hoặc theo tọa độ.

**Ví dụ**:
```yaml
# Tìm theo text
- tap: "Login"

# Tìm theo ID và chỉ định index thứ 2
- tap:
    id: "btn_action"
    index: 1

# Dùng vị trí tương đối
- tap:
    rightOf: "Username"
    type: "EditText"

# Chạm vào ảnh mẫu
- tap:
    image: "assets/btn_save.png"
    optional: true
```

**Tham số Selector**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `text` | - | String | Tìm phần tử chứa text chính xác (hoặc case-insensitive). |
| `id` | - | String | Tìm theo Resource ID (Android), ID (Web), hoặc Accessibility ID. |
| `css` | - | String | (Web) CSS Selector. |
| `xpath` | - | String | XPath selector. |
| `point` | - | String | Tọa độ cụ thể ("x,y" hoặc "x%,y%"). |
| `regex` | - | String | Tìm khớp theo biểu thức chính quy. |
| `index` | - | Number | Thứ tự của phần tử nếu tìm thấy nhiều kết quả (0-based). |
| `type` | `element_type` | String | Loại phần tử (EditText, Button, input, v.v.). |
| `desc` | `contentDesc`, `accessibilityId` | String | Tìm theo Content-Description. |
| `placeholder`| - | String | Tìm theo text placeholder. |
| `role` | - | String | Tìm theo ARIA role (Web) hoặc accessibility traits. |
| `image` | - | String | Path tới file ảnh để tìm kiếm bằng template matching. |

**Tham số Điều khiển**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `optional` | - | Boolean | `false` | Nếu `true`, test sẽ tiếp tục ngay cả khi không tìm thấy phần tử. |
| `exact` | - | Boolean | `false` | Buộc khớp text chính xác tuyệt đối (case-sensitive). |
| `retryTapIfNoChange`| - | Boolean | `true` | Thử nhấn lại nếu không thấy tín hiệu UI thay đổi. |
| `scrollable`| - | Object | - | Cấu hình tự động cuộn màn hình để tìm phần tử. |

**Shorthand Vị trí tương đối** (Sử dụng thay cho Selector chính):
- `rightOf`, `leftOf`, `above`, `below`. (Alias tương ứng: `rightOf`, `leftOf`).
- Mỗi mỏ neo có thể dùng text hoặc các trường selector đầy đủ.

---

### `doubleTap` / `doubleTapOn`
**Mô tả**: Chạm nhanh hai lần liên tiếp. Tham số tương tự `tap`.

**Ví dụ**:
```yaml
- doubleTap: "Item Name"

- doubleTapOn:
    id: "recycler_view"
    index: 0
```

---

### `longPress` / `longPressOn`
**Mô tả**: Nhấn và giữ một phần tử. Tham số tương tự `tap`.

**Ví dụ**:
```yaml
- longPress: "Hold Me"

- longPressOn:
    point: "50%,50%"
```

---

### `rightClick` / `contextClick`
**Mô tả**: Nhấn chuột phải (Context Menu). Tham số tương tự `tap`.

**Ví dụ**:
```yaml
- rightClick: "File.txt"

- contextClick:
    id: "item_id"
```

---

### `tapAt`
**Mô tả**: Chạm vào phần tử theo loại và thứ tự mà không cần text/ID.

**Ví dụ**:
```yaml
- tapAt:
    type: "Button"
    index: 1 # Chạm vào nút thứ 2 trên màn hình
```

---

### `inputText` / `write` / `type`
**Mô tả**: Nhập văn bản vào một phần tử hoặc ô đang focus.

**Ví dụ**:
```yaml
# Nhập vào ô đang focus
- write: "my password"

# Nhập tiếng Việt có hỗ trợ AdbIME
- write:
    text: "xin chào"
    unicode: true

# Tìm phần tử rồi mới nhập (Lệnh `type`)
- type:
    text: "admin"
    selector: "#user_login"
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `text` | String | - | Nội dung văn bản cần nhập. |
| `unicode` | Boolean | `false` | Dùng chế độ Unicode (Android AdbIME) cho tiếng Việt/Ký tự đặc biệt. |
| `selector` | String | - | (Chỉ lệnh `type`) Selector tìm phần tử trước khi nhập. |

---

### `inputAt`
**Mô tả**: Nhập văn bản vào phần tử theo loại và thứ tự.

**Ví dụ**:
```yaml
- inputAt:
    type: "EditText"
    index: 0
    text: "admin@example.com"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `type` | `element_type` | String | Loại phần tử (EditText, Button,...). |
| `index` | - | Number | Thứ tự tương ứng của loại phần tử đó. |
| `text` | - | String | Nội dung cần nhập. |

---

### `eraseText` / `clear`
**Mô tả**: Xóa văn bản trong ô nhập liệu đang focus.

**Ví dụ**:
```yaml
- clear:
    charCount: 10 # Xóa 10 ký tự

- eraseText: 5
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `charCount`| - | Number | Số lượng ký tự cần xóa. Nếu bỏ trống, sẽ xóa toàn bộ. |

---

### `hideKeyboard` / `hideKbd`
**Mô tả**: Ẩn bàn phím ảo nếu đang hiển thị.

**Ví dụ**:
```yaml
- hideKeyboard
```

---

### `press` / `pressKey`
**Mô tả**: Nhấn phím vật lý hoặc tổ hợp phím hệ thống.

**Ví dụ**:
```yaml
- press: "Enter"

- pressKey:
    key: "Back"
    times: 3 # Nhấn Back 3 lần
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `key` | String | - | Tên phím hoặc Keycode (số). |
| `times` | Value | `1` | Số lần nhấn (hỗ trợ số hoặc biến `${var}`). |

**Các phím phổ biến**:
- `Home`, `Back`, `Enter`, `Done`, `Menu`, `Search`, `Power`, `VolumeUp`, `VolumeDown`, `DpadUp`, `DpadDown`, `DpadLeft`, `DpadRight`, `DpadCenter`.

---

### `pasteText`
**Mô tả**: Dán văn bản từ clipboard vào vị trí con trỏ hiện tại.

**Ví dụ**:
```yaml
- pasteText
```

---

## 📜 Scroll & Swipe

### `swipe`
**Mô tả**: Vẩy (Vuốt) màn hình theo một hướng cụ thể.
**Aliases**: `swipeUp`, `swipeDown`, `swipeLeft`, `swipeRight`

**Ví dụ**:
```yaml
# Vuốt lên đơn giản
- swipe: "up"

# Dùng các lệnh chuyên biệt
- swipeLeft
- swipeRight
- swipeUp
- swipeDown

# Vuốt sang trái chậm với khoảng cách ngắn
- swipe:
    direction: "left"
    distance: 0.5
    duration: 1000
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `direction` | String | - | Hướng vuốt: `up`, `down`, `left`, `right`. |
| `duration` | Number | `500` | Thời gian thực hiện hành động (ms). |
| `distance` | Number | `0.8` | Khoảng cách vuốt (tỉ lệ 0.0 đến 1.0 của màn hình). |
| `from` | Selector | - | Bắt đầu vuốt từ vị trí của một phần tử cụ thể. |

---

### `scrollTo` / `scrollUntilVisible`
**Mô tả**: Cuộn màn hình liên tục cho đến khi thấy phần tử mục tiêu xuất hiện.

**Ví dụ**:
```yaml
# Cuộn tìm text
- scrollTo: "Footer Link"

# Cuộn trong một vùng cụ thể (container)
- scrollUntilVisible:
    id: "target_item"
    direction: "down"
    maxScrolls: 20
    from:
      id: "scroll_container"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| (Selector) | - | Mixed | - | Chấp nhận `text`, `id`, `regex`, v.v. |
| `direction` | - | String | `down` | Hướng cuộn: `down`, `up`, `left`, `right`. |
| `maxScrolls` | `numberScroll` | Number | `10` | Số lần cuộn tối đa trước khi dừng. |
| `from` | - | Selector | - | Chỉ định Container thực hiện cuộn. |
| `timeout` | - | Number | - | Thời gian chờ tối đa (ms). |

---

## ⚙️ System & Settings (Hệ thống)

### `openNotifications`
**Mô tả**: Kéo thanh thông báo hoặc trung tâm thông báo xuống.
**Aliases**: `openNotifications`

**Ví dụ**:
```yaml
- openNotifications
```

---

### `openQuickSettings`
**Mô tả**: Mở bảng cài đặt nhanh (Quick Settings).
**Aliases**: `openQuickSettings`

**Ví dụ**:
```yaml
- openQuickSettings
```

---

### `setVolume`
**Mô tả**: Điều chỉnh âm lượng của thiết bị.

**Ví dụ**:
```yaml
- setVolume: 75 # Đặt âm lượng 75%
```

---

### `setLocale` / `locale`
**Mô tả**: Thay đổi ngôn ngữ/vùng (Locale) của hệ thống.
**Aliases**: `locale`

**Ví dụ**:
```yaml
- setLocale: "vi_VN"
```

---

### `selectDisplay` / `display`
**Mô tả**: Chọn màn hình hiển thị để tương tác (dùng cho các hệ thống nhiều màn hình).
**Aliases**: `display`

**Ví dụ**:
```yaml
- selectDisplay: "1"
```

### `lockDevice` / `unlockDevice`
**Mô tả**: Khóa màn hình hoặc mở khóa thiết bị.
**Aliases**: `lockDevice`, `unlockDevice`

**Ví dụ**:
```yaml
- lockDevice
- unlockDevice
```

---

### `setNetwork`
**Mô tả**: Bật/Tắt các kết nối mạng (WiFi, Dữ liệu di động).
**Aliases**: `setNetwork`

**Ví dụ**:
```yaml
- setNetwork:
    wifi: true
    data: false
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `wifi` | - | Boolean | Bật/tắt WiFi. |
| `data` | - | Boolean | Bật/tắt Dữ liệu di động. |

---

### `airplaneMode` / `toggleAirplaneMode`
**Mô tả**: Chế độ máy bay.
**Aliases**: `airplaneMode`, `toggleAirplaneMode`

**Ví dụ**:
```yaml
- airplaneMode
```

---

### `setOrientation`
**Mô tả**: Đặt hướng xoay màn hình nâng cao.
**Aliases**: `setOrientation`

**Ví dụ**:
```yaml
- setOrientation:
    mode: "LANDSCAPE"
```

**Giá trị Enum/Đặc biệt**:
- `mode`: `PORTRAIT`, `LANDSCAPE`, `UPSIDE_DOWN`, `LANDSCAPE_LEFT`, `LANDSCAPE_RIGHT`.

---

### `rotate` / `rotateScreen`
**Mô tả**: Xoay màn hình nhanh giữa hai chế độ cơ bản.
**Aliases**: `rotate`, `rotateScreen`

**Ví dụ**:
```yaml
- rotate: "landscape"
- rotate: "portrait"
```

---

## ⚡ Performance Testing

### `startProfiling`
**Mô tả**: Bắt đầu ghi nhận số liệu hiệu năng (CPU, RAM, v.v.).
**Aliases**: `startProfiling`

**Ví dụ**:
```yaml
- startProfiling:
    samplingIntervalMs: 500 # 0.5 giây/mẫu
    package: "com.example.app"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `samplingIntervalMs`| - | Number | `1000` | Tần suất lấy mẫu (ms). |
| `package` | - | String | App hiện tại | Package name cần profile. |

---

### `stopProfiling`
**Mô tả**: Dừng ghi nhận và xuất báo cáo hiệu năng.
**Aliases**: `stopProfiling`

**Ví dụ**:
```yaml
- stopProfiling:
    savePath: "performance_report.json"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `savePath` | - | String | Đường dẫn lưu file báo cáo (JSON). |

---

### `assertPerformance`
**Mô tả**: Kiểm tra các chỉ số hiệu năng có nằm trong ngưỡng cho phép hay không.
**Aliases**: `assertPerformance`

**Ví dụ**:
```yaml
- assertPerformance:
    metric: "memory"
    limit: "250MB"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `metric` | - | Enum | Loại chỉ số: `cpu`, `memory`, `fps`, `jank`. |
| `limit` | - | String/Number | Ngưỡng giới hạn cho phép. |

**Giá trị Enum/Đặc biệt**:
- `metric`: `cpu`, `memory`, `fps`, `jank`.

---

### `setCpuThrottling`
**Mô tả**: Giới hạn tốc độ CPU (giả lập thiết bị cấu hình thấp).
**Aliases**: `setCpuThrottling`

**Ví dụ**:
```yaml
- setCpuThrottling: 2.0 # Giới hạn chậm hơn 2 lần
```

---

### `setNetworkConditions`
**Mô tả**: Thay đổi điều kiện mạng (giả lập mạng yếu).
**Aliases**: `setNetworkConditions`

**Ví dụ**:
```yaml
- setNetworkConditions: "slow-3g"
```

**Giá trị Enum/Đặc biệt**:
- Profile: `online`, `offline`, `slow-3g`, `fast-3g`, `4g`, `wifi`.

---

## 👁️ Assertions (Kiểm tra)

### `see` / `assertVisible`
**Mô tả**: Kiểm tra phần tử có hiển thị trên màn hình hay không.

**Ví dụ**:
```yaml
# Kiểm tra text đơn giản
- see: "Welcome"

# Kiểm tra nâng cao với soft assertion
- assertVisible:
    id: "user_profile_img"
    soft: true # Nếu không thấy cũng không làm dừng toàn bộ test suite
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| (Selector) | Mixed | - | Chấp nhận các trường selector như `text`, `id`, `regex`, v.v. |
| `timeout` | Number | `defaultTimeout` | Thời gian chờ tối đa cho phần tử xuất hiện (ms). |
| `soft` | Boolean | `false` | Nếu `true`, chỉ log lỗi và đánh dấu bước fail nhưng vẫn chạy tiếp. |
| `containsChild`| Selector | - | Kiểm tra phần tử cha có chứa một phần tử con cụ thể hay không. |

---

### `notSee` / `assertNotVisible`
**Mô tả**: Kiểm tra phần tử KHÔNG hiển thị trên màn hình.

**Ví dụ**:
```yaml
- notSee: "Logged Out"

- assertNotVisible:
    id: "error_icon"
```

---

### `waitUntilVisible` / `waitSee`
**Mô tả**: Chờ cho đến khi phần tử xuất hiện.

**Ví dụ**:
```yaml
- waitSee: "Welcome Home"

- waitUntilVisible:
    id: "main_content"
    timeout: 10000
```

---

### `waitNotSee` / `waitUntilNotVisible`
**Mô tả**: Chờ cho đến khi phần tử biến mất.

**Ví dụ**:
```yaml
- waitNotSee: "Loading..."

- waitUntilNotVisible:
    id: "progress_bar"
```

---

### `extendedWaitUntil`
**Mô tả**: Chờ điều kiện phức tạp với nhiều trạng thái.

**Ví dụ**:
```yaml
- extendedWaitUntil:
    timeout: 30000
    visible:
      id: "success_dialog"
    notVisible:
      id: "loading_overlay"
```

---

### `assert` / `assertTrue`
**Mô tả**: Kiểm tra một biểu thức logic hoặc giá trị biến.

**Ví dụ**:
```yaml
# Kiểm tra biểu thức chuỗi
- assert: "${items_count} > 0"

# Dùng cấu trúc struct
- assertTrue:
    condition: "${status} == 'active'"
    soft: true
```

---

### `assertVar`
**Mô tả**: So sánh trực tiếp giá trị của một biến.

**Ví dụ**:
```yaml
- assertVar:
    name: "user_role"
    expected: "admin"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `name` | - | String | Tên biến cần kiểm tra. |
| `expected`| - | String | Giá trị mong đợi. |

---

### `assertColor` / `checkColor`
**Mô tả**: Kiểm tra màu sắc tại một tọa độ điểm ảnh.

**Ví dụ**:
```yaml
- assertColor:
    point: "50%,50%"
    color: "#4CAF50" # Màu xanh lá
    tolerance: 5 # Sai số 5%
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `point` | String | - | Tọa độ ("x,y" hoặc "%"). |
| `color` | String | - | Mã màu (Hex, tên màu: `red`, `blue`,...). |
| `tolerance` | Number | `10` | Độ lệch màu cho phép (0-100%). |

---

### `assertScreenshot`
**Mô tả**: So sánh màn hình hiện tại với ảnh mẫu (Visual Regression).
**Aliases**: `assertScreenshot`

**Ví dụ**:
```yaml
- assertScreenshot: "baselines/home_screen.png"
```

---

### `assertClipboard`
**Mô tả**: Kiểm tra nội dung trong clipboard có khớp với mong đợi không.
**Aliases**: `assertClipboard`

**Ví dụ**:
```yaml
- assertClipboard: "Expected Text"
```

---

## 📋 Clipboard & Data Transfer

### `setClipboard`
**Mô tả**: Gán một chuỗi văn bản vào clipboard của thiết bị.
**Aliases**: `setClipboard`

**Ví dụ**:
```yaml
- setClipboard: "hello world"
```

---

### `getClipboard`
**Mô tả**: Lấy nội dung từ clipboard và lưu vào biến.
**Aliases**: `getClipboard`

**Ví dụ**:
```yaml
- getClipboard:
    name: "otp_code"
```

---

### `copyTextFrom`
**Mô tả**: Trích xuất text từ một phần tử UI và lưu vào clipboard hoặc biến.

**Ví dụ**:
```yaml
- copyTextFrom:
    id: "user_id_label"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| (Selector) | - | Mixed | Các trường selector (`id`, `text`,...). |

---

### `pushFile`
**Mô tả**: Đẩy file từ máy tính lên thiết bị.

**Ví dụ**:
```yaml
- pushFile:
    source: "./local/config.json"
    destination: "/sdcard/config.json"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `source` | - | String | Đường dẫn file trên máy tính. |
| `destination`| - | String | Đường dẫn đích trên thiết bị. |

---

### `pullFile`
**Mô tả**: Lấy file từ thiết bị về máy tính.

**Ví dụ**:
```yaml
- pullFile:
    source: "/sdcard/log.txt"
    destination: "./logs/device_log.txt"
```

---

## 🎲 Random Inputs

### `generate`
**Mô tả**: Sinh dữ liệu ngẫu nhiên (Faker) và lưu vào biến.

**Ví dụ**:
```yaml
- generate:
    name: "random_user"
    type: "name"

- generate:
    name: "expiry_date"
    type: "date"
    format: "YYYY-MM-DD"

- generate:
    name: "age"
    type: "number"
    format: "18-60"
```

**Giá trị Enum/Đặc biệt**:
- `type`: `uuid`, `email`, `phone`, `name`, `address`, `number`, `date`, `password`.

---

### `inputRandomEmail`
**Mô tả**: Nhập một địa chỉ email ngẫu nhiên vào ô đang focus.

**Ví dụ**:
```yaml
- inputRandomEmail
```

---

### `inputRandomName` / `inputRandomPersonName`
**Mô tả**: Nhập tên người ngẫu nhiên.
**Aliases**: `inputRandomPersonName`

**Ví dụ**:
```yaml
- inputRandomName
```

---

### `inputRandomText`
**Mô tả**: Nhập chuỗi văn bản ngẫu nhiên.
**Aliases**: `inputRandomText`

**Ví dụ**:
```yaml
- inputRandomText:
    length: 10
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `length` | - | Number | Độ dài chuỗi (mặc định 8). |

---

### `inputRandomNumber` / `inputRandomPhoneNumber`
**Mô tả**: Nhập chuỗi số ngẫu nhiên.
**Aliases**: `inputRandomNumber`, `inputRandomPhoneNumber`

**Ví dụ**:
```yaml
- inputRandomNumber:
    length: 6 # Ví dụ sinh mã OTP 6 số
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `length` | - | Number | Số lượng chữ số. |

---

## ⚙️ Logic & Control Flow

### `wait` / `await`
**Mô tả**: Dừng thực thi trong một khoảng thời gian cố định.

**Ví dụ**:
```yaml
- wait: 2000 # Chờ 2 giây
```

---

### `waitForAnimationToEnd`
**Mô tả**: Chờ cho đến khi các hiệu ứng chuyển cảnh (Animation) kết thúc và màn hình ổn định.

**Ví dụ**:
```yaml
- waitForAnimationToEnd
```

---

### `setVar`
**Mô tả**: Khai báo hoặc cập nhật giá trị cho một biến.

**Ví dụ**:
```yaml
- setVar:
    name: "is_logged_in"
    value: true

- setVar:
    name: "timestamp"
    value: "${evalScript: Date.now()}"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `name` | - | String | Tên biến. |
| `value`| - | Mixed | Giá trị gán cho biến. |

---

### `runFlow`
**Mô tả**: Chạy một file test flow khác như một kịch bản con (Sub-flow).
**Aliases**: `runFlow`

**Ví dụ**:
```yaml
- runFlow:
    path: "common/login.yaml"
    vars:
      user: "admin"
    when: "${is_logged_in} == false"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `path` | - | String | - | Đường dẫn tới file YAML flow. |
| `vars` | `env` | Map | - | Danh sách biến truyền vào cho sub-flow. |
| `when` | - | Expression | - | Điều kiện để chạy flow này. |
| `optional`| - | Boolean | `false` | Nếu `true`, sub-flow lỗi sẽ không làm dừng flow chính. |

---

### `repeat`
**Mô tả**: Vòng lặp thực thi một danh sách các lệnh.

**Ví dụ**:
```yaml
- repeat:
    times: 5
    commands:
      - tap: "Next"
      - wait: 500
```

**Tham số**:
- `times`: Số lần lặp.
- `while`: Lặp cho đến khi điều kiện (biến hoặc phần tử xuất hiện/biến mất) không còn thỏa mãn.
- `commands`: Danh sách các lệnh bên trong vòng lặp.

---

### `retry`
**Mô tả**: Thử lại một khối lệnh nếu có lỗi xảy ra.

**Ví dụ**:
```yaml
- retry:
    maxRetries: 3
    commands:
      - tap: "Submit"
      - see: "Success"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `maxRetries`| - | Number | `3` | Số lần thử lại tối đa. |
| `commands` | - | Sequence | - | Danh sách lệnh cần thực hiện lại. |

---

### `conditional`
**Mô tả**: Cấu trúc rẽ nhánh If-Then-Else dựa trên sự xuất hiện/biến mất của phần tử.

**Ví dụ**:
```yaml
- conditional:
    condition:
      visible: "Update Available"
    then:
      - tap: "Later"
    else:
      - log: "No update found"
```

**Tham số điều kiện (`condition`)**:
| Trường | Mô tả |
| :--- | :--- |
| `visible` | Kiểm tra text/id/... đang hiển thị. |
| `visibleRegex`| Kiểm tra khớp regex đang hiển thị. |
| `notVisible`| Kiểm tra phần tử KHÔNG hiển thị. |
| `notVisibleRegex`| Kiểm tra regex KHÔNG hiển thị. |

---

### `runScript`
**Mô tả**: Thực thi một lệnh Shell script trên máy tính đang chạy test (Host).

**Ví dụ**:
```yaml
- runScript: "scripts/setup_db.sh"

- runScript:
    command: "python3"
    args: ["process_data.py", "data.csv"]
    saveOutput: "python_result"
    timeoutMs: 30000
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `command` | - | String | Lệnh hoặc đường dẫn tới script. |
| `args` | - | Array | Danh sách tham số truyền vào script. |
| `saveOutput`| - | String | Tên biến dùng để lưu kết quả từ `stdout`. |
| `timeoutMs` | - | Number | Thời gian chờ tối đa (ms). |
| `failOnError`| - | Boolean | Nếu `true`, test sẽ dừng nếu script lỗi (exit code != 0). |

---

### `evalScript`
**Mô tả**: Thực thi mã JavaScript để tính toán và trả về giá trị cho biến.

**Ví dụ**:
```yaml
- evalScript: "Math.random() > 0.5"
```

---

### `httpRequest`
**Mô tả**: Gửi yêu cầu HTTP (REST API).

**Ví dụ**:
```yaml
- httpRequest:
    url: "https://api.example.com/login"
    method: "POST"
    headers:
      Content-Type: "application/json"
    body:
      username: "admin"
      password: "${pwd}"
    saveResponse:
      "$.token": "auth_token" # Lưu token từ JSON response vào biến
```

**Tham số**:
| Trường | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- |
| `url` | String | URL API cần gọi. |
| `method` | String | Phương thức: `GET`, `POST`, `PUT`, `DELETE`. |
| `headers` | Map | Các HTTP Headers. |
| `body` | Mixed | Nội dung request (JSON hoặc Yaml). |
| `saveResponse`| Map | Map giữa JSONPath và tên biến để lưu kết quả. |

---

### `dbQuery`
**Mô tả**: Thực hiện truy vấn vào cơ sở dữ liệu.

**Ví dụ**:
```yaml
- dbQuery:
    connection: "postgres://user@localhost:5432/db"
    query: "SELECT status FROM users WHERE id = ?"
    params: ["123"]
    save:
      "status": "user_status" # Lưu kết quả SQL vào biến
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `connection`| - | String | Connection string tới DB. |
| `query` | - | String | Câu lệnh SQL. |
| `params` | - | Array | Danh sách tham số cho SQL (`?`). |
| `save` | - | Map | Map kết quả cột vào tên biến. |
**Aliases**: `dbQuery`

---

## 📊 Reporting (Báo cáo)

### `exportReport`
**Mô tả**: Xuất báo cáo kết quả test ra file (HTML/JSON).

**Ví dụ**:
```yaml
- exportReport:
    path: "reports/daily_test.html"
    format: "html"
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `path` | - | String | - | Đường dẫn lưu file báo cáo. |
| `format` | - | String | `html` | Định dạng: `html`, `json`. |

---

## 📍 Location & GPS

### `mockLocation` / `gps`
**Mô tả**: Giả lập vị trí GPS của thiết bị.
**Aliases**: `mockLocation`, `gps`

**Ví dụ**:
```yaml
- gps:
    file: "path/to/route.gpx"
    speed: 60 # 60km/h
    loop: true
    startIndex: 0
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mặc định | Mô tả |
| :--- | :--- | :--- | :--- | :--- |
| `file` | - | String | - | Đường dẫn file chứa tọa độ (GPX, KML, JSON). |
| `speed` | - | Number | - | Tốc độ di chuyển (km/h). |
| `speedMode`| - | String | `linear` | Chế độ tốc độ: `linear` (cố định), `noise` (biến thiên). |
| `speedNoise`| - | Number | - | Độ biến thiên tốc độ khi dùng `noise`. |
| `loop` | - | Boolean | `false` | Tự động lặp lại route. |
| `startIndex`| - | Number | `0` | Chỉ số điểm bắt đầu trong file. |
| `intervalMs`| - | Number | `1000` | Tần suất cập nhật vị trí. |

---

### `mockLocationControl`
**Mô tả**: Điều khiển trạng thái giả lập GPS đang chạy.

**Ví dụ**:
```yaml
- mockLocationControl:
    speed: 100
    pause: true
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `speed` | - | Number | Tốc độ mới. |
| `pause` | - | Boolean | Tạm dừng. |
| `resume` | - | Boolean | Tiếp tục. |
| `speedMode`| - | String | Chế độ tốc độ mới. |

---

### `waitForLocation`
**Mô tả**: Chờ cho đến khi vị trí giả lập di chuyển đến tọa độ mục tiêu.

**Ví dụ**:
```yaml
- waitForLocation:
    lat: 10.7769
    lon: 106.7009
    tolerance: 10.0 # Bán kính 10m
```

**Tham số**:
| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `lat` | - | Number | Vĩ độ. |
| `lon` | - | Number | Kinh độ. |
| `tolerance` | - | Number | Độ lệch cho phép (mét). |

---

### `waitForMockCompletion`
**Mô tả**: Chờ cho đến khi route giả lập hoàn tất.

**Ví dụ**:
```yaml
- waitForMockCompletion: 60000 # Timeout 60s
```

---

## 📷 Media (Screenshot & Video)

### `takeScreenshot` / `screenshot`
**Mô tả**: Chụp ảnh màn hình hiện tại.

**Ví dụ**:
```yaml
- takeScreenshot: "screenshots/step_1.png"

- screenshot:
    path: "screenshots/error.png"
```

---

### `startRecording` / `stopRecording`
**Mô tả**: Quay phim màn hình thiết bị.

**Ví dụ**:
```yaml
- startRecording: "videos/test_run.mp4"

- stopRecording
```

---

### `startGifCapture` / `stopGifCapture`
**Mô tả**: Tự động chụp các khung hình để tạo ảnh GIF minh họa.

**Ví dụ**:
```yaml
- startGifCapture:
    interval: 500
    maxFrames: 50
```

**Tham số `startGifCapture`**:
- `interval`: Khoảng thời gian giữa các lần chụp (ms, mặc định 200).
- `maxFrames`: Số lượng ảnh tối đa (mặc định 150).
- `width`: Chiều rộng ảnh (tự động scale chiều cao).

**Tham số `stopGifCapture`**:
- `output`: File path đầu ra (.gif).
- `quality`: `low`, `medium`, `high`.

---

### `captureFrame` / `captureGifFrame`
**Mô tả**: Chụp một khung hình thủ công để đưa vào ảnh GIF.

**Ví dụ**:
```yaml
- captureFrame: "login_success"

- captureGifFrame:
    name: "error_state"
    crop: "0%,0%,100%,50%" # Cắt lấy nửa trên màn hình
```

**Tham số**:
- `name`: Tên định danh cho frame.
- `crop`: Vùng cắt ảnh `"left%,top%,width%,height%"`.

---

### `createGif` / `buildGif`
**Mô tả**: Tạo file GIF từ các frame đã chụp thủ công.

**Ví dụ**:
```yaml
- captureFrame: "step1"
- tap: "Next"
- captureFrame: "step2"
- buildGif:
    output: "result.gif"
    frames:
      - "step1"
      - name: "step2"
        delay: 1000 # Chờ 1s tại frame này
    quality: "high"
    loopGif: true
```

---

## 🌐 Web Specific & Deep Links

### `openLink` / `deepLink`
**Mô tả**: Mở một Deep Link hoặc URL tùy chỉnh.

**Ví dụ**:
```yaml
- openLink: "myapp://product/123"

- deepLink:
    url: "https://example.com/reset-password"
```

---

### `navigate`
**Mô tả**: Điều hướng trình duyệt tới một URL cụ thể.

**Ví dụ**:
```yaml
- navigate: "https://www.google.com"
```

---

### `click`
**Mô tả**: Click vào phần tử trên trình duyệt bằng CSS hoặc Text.

**Ví dụ**:
```yaml
- click:
    selector: ".nav-item"
    text: "Menu"
```

**Tham số**:
- `selector`: CSS Selector.
- `text`: Text nội dung.

---

### `type`
**Mô tả**: Nhập văn bản vào phần tử trên trình duyệt thông qua Selector.

**Ví dụ**:
```yaml
- type:
    selector: "#search-input"
    text: "lumi-tester"
```
