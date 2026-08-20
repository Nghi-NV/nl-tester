# Hướng dẫn Kiểm thử Phần cứng Tự động Đầy đủ (Hardware Automation User Guide)

Lumi Tester tích hợp module **Điều khiển Phần cứng Tự động (Hardware Automation)** phát triển bằng **100% Native Rust**, đáp ứng **100% tính năng** từ thư viện `hardware_control_services` (bao gồm điều khiển Servo, Relay, Cảm biến màu TCS34725/TCS3200, đo độ sáng Brightness, nhiệt độ màu CCT, phát hiện chớp tắt Blink, Hiệu chuẩn Calibration và Trạng thái An toàn System Safe).

Tất cả các câu lệnh phần cứng đều sử dụng tiền tố chuẩn hóa **`hw*`** để tránh xung đột với các lệnh UI/App (ví dụ: `click` là click giao diện, còn `hwClick` là bấm servo vật lý).

---

## 🔌 1. Cấu hình Jig Phần cứng ở Header (`jig:`)

Bạn có thể khai báo kết nối với Jig phần cứng ngay tại phần **Header** của file YAML test (phía trên dấu `---`). 

Lumi Tester sẽ:
* **Tự động kết nối** với Jig phần cứng trước khi bắt đầu câu lệnh đầu tiên.
* **Tự động ngắt điện an toàn (`hwPowerOffAll`) & ngắt kết nối (`hwDisconnect`)** sau khi kịch bản kết thúc (kể cả khi Pass, Fail hay có lỗi).

### a. Cú pháp Ngắn gọn (Short-hand Syntax)
```yaml
platform: android
appId: com.lumi.lifenext
jig:
  port: "${JIG_PORT:-COM5}"     # Truyền biến môi trường CI/CD (mặc định COM5)
  nodeId: 1                    # Địa chỉ Node RS485 (Mặc định: 1)
  baudrate: 115200             # Tốc độ Baud (Mặc định: 115200)
  autoPowerOff: true           # Tự động tắt toàn bộ rơ-le nguồn khi kết thúc test (Mặc định: true)
  timeoutMs: 4000              # Custom thời gian timeout chờ phản hồi từ vi điều khiển (ms)
---
- hwPowerOn: 1
- hwClick: 1
- hwSeeLed: "GREEN"
```

### b. Cú pháp Nâng cao cho CI/CD (Advanced CI/CD Config)
```yaml
platform: android
appId: com.lumi.lifenext

jig:
  port: "${JIG_PORT:-COM5}"     # Truyền biến môi trường CI/CD (mặc định COM5)
  nodeId: 1                    # Địa chỉ Node RS485 (Mặc định: 1)
  baudrate: 115200             # Tốc độ Baud (Mặc định: 115200)
  autoPowerOff: true           # Tự động tắt toàn bộ rơ-le nguồn khi kết thúc test (Mặc định: true)
  timeoutMs: 4000              # Custom thời gian timeout chờ phản hồi từ vi điều khiển (ms)
---
- hwPowerOn: 1
- hwClick: 1
- hwSeeLed: "GREEN"
```

### c. Cấu hình Jig & Servo Profile Dùng chung (Shared Reusable Profile)
Để không phải lặp lại cấu hình cổng COM, thông số `servos`, cùng bảng ánh xạ nút bấm `buttons` và rơ-le `relays` trong từng file test, bạn có thể tạo một file profile chuẩn dùng chung tại [`profiles/jig_config.yaml`](file:///Users/nghinguyen/Desktop/MyOpenSource/nl-tester/profiles/jig_config.yaml):

```yaml
# profiles/jig_config.yaml
port: "${JIG_PORT:-COM5}"
baudrate: 115200
nodeId: 1                          # Địa chỉ Node RS485 (Mặc định: 1)
wireFormat: "@{node} {command}\n"  # Mẫu định dạng khung truyền giao tiếp MCU
autoPowerOff: false                # Giữ nguồn bật sau khi test (hoặc true nếu muốn tự động ngắt relay)
timeoutMs: 4000

# 🎛️ Bảng ánh xạ Nút bấm thân thiện (Tách biệt độc lập Kênh Servo gạt và Kênh Cảm biến màu quang học)
buttons:
  NC1:
    servo: 5
    sensor: 5
  NC2:
    servo: 6
    sensor: 7
  NC3:
    servo: 7
    sensor: 6

# ⚡ Bảng ánh xạ Rơ-le nguồn (Hỗ trợ gọi bật/tắt nhóm nhiều kênh cùng lúc)
relays:
  mainPower: [3, 4]
  220V: [3, 4]

# ⚙️ Cấu hình góc gạt và thời gian cho từng Servo
servos:
  - channel: 5
    pressAngle: 75
    releaseAngle: 0
  - channel: 6
    pressAngle: 80
    releaseAngle: 0
  - channel: 7
    pressAngle: 75
    releaseAngle: 0
```

Trong tất cả các file test kịch bản, chỉ cần trỏ tới file profile và gọi tên nút thân thiện trực tiếp:
```yaml
platform: android
appId: com.lumi.lifenext
jig: "profiles/jig_config.yaml"
---
- hwPing: 1
- hwPowerOn: "220V"          # Bật đồng thời rơ-le 3 & 4
- hwReadColor: "NC3"         # Tự động đọc cảm biến kênh 6
- hwSeeLed:
    button: "NC3"            # Tự động đọc cảm biến kênh 6
    color: "BLUE"
- hwClick: "NC3"             # Tự động gạt servo kênh 7
- wait: 500
- hwSeeLed:
    button: "NC3"
    color: "RED"
```

### d. Công cụ CLI & VSCode Phát hiện Cổng COM & Ping Jig Nhanh (1-Click & CLI Tools)

#### 1. Bằng Dòng lệnh CLI:
* **Liệt kê toàn bộ cổng COM đang cắm vào máy:**
  ```bash
  lumi-tester jig ports
  # hoặc output JSON
  lumi-tester jig ports --json
  ```
* **Ping và kiểm tra kết nối với Jig phần cứng:**
  ```bash
  # Ping cổng COM trực tiếp (mặc định Node 1)
  lumi-tester jig ping COM5

  # Ping chỉ định Node ID cụ thể trên bus RS485 (ví dụ: Node 2)
  lumi-tester jig ping COM5 --node 2

  # Hoặc ping theo file profile YAML
  lumi-tester jig ping profiles/jig_switch_sample.yaml
  ```

#### 2. Thao tác trên VSCode Extension (`lumi-tester-vscode`):
* Nhấn `Cmd+Shift+P` (macOS) hoặc `Ctrl+Shift+P` (Windows) $\rightarrow$ chọn **`Lumi: Detect Hardware Jig Ports`**.
* VSCode hiển thị dropdown danh sách các cổng COM đang cắm $\rightarrow$ Chọn cổng để **Ping** hoặc **Chèn trực tiếp vào Header YAML** đang mở.
* Chọn **`Lumi: Ping Hardware Jig`** để kiểm tra phản hồi tức thì với thông báo popup.

---

## 📖 2. Bảng Tra cứu Tập lệnh Chuẩn hóa `hw*` (Full Matrix)

### A. Nhóm lệnh Điều khiển Nguồn Rơ-le (Relay Control)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwPowerOn` | Bật nguồn cấp điện cho kênh chỉ định | `- hwPowerOn: 1` |
| `hwPowerOff` | Tắt nguồn cấp điện cho kênh chỉ định | `- hwPowerOff: 1` |
| `hwPowerCycle` | Hard Power Reboot (Tắt điện -> nghỉ -> Bật lại nguồn) | `- hwPowerCycle: { channel: 1, offMs: 2000 }` |
| `hwPowerOffAll` | Tắt toàn bộ nguồn điện các kênh rơ-le | `- hwPowerOffAll` |

### B. Nhóm lệnh Điều khiển Động cơ Servo gạt nút (Servo Control)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwConfigureServo` | Cấu hình góc gạt (press/release angle) và thời gian gạt nút | `- hwConfigureServo: { channel: 1, pressAngle: 75, releaseAngle: 15, pressDurationMs: 400, holdDurationMs: 300 }` |
| `hwClick` | Bấm nút vật lý đơn | `- hwClick: 1` hoặc `- hwClick: { channel: 1, holdMs: 500 }` |
| `hwRepeatClick` | Bấm nút lặp lại N lần (Nhấp đúp / Triple click) | `- hwRepeatClick: { channel: 1, count: 3 }` |
| `hwPress` | Nhấn giữ nút vật lý (Pairing / Factory Reset) | `- hwPress: 1` |
| `hwRelease` | Nhả nút vật lý | `- hwRelease: 1` |
| `hwReleaseAll` | Nhả tất cả các nút servo | `- hwReleaseAll` |
| `hwStartRepeatClick` | Bắt đầu chạy lặp bấm nút liên tục trên STM32 | `- hwStartRepeatClick: { channel: 1, periodMs: 1500 }` |
| `hwStopRepeatClick` | Dừng lặp bấm nút liên tục | `- hwStopRepeatClick: 1` |

### C. Nhóm lệnh Cảm biến Màu & Độ sáng (Color & Blink Sensor)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwSeeLed` | Kiểm tra màu đèn LED chỉ thị | `- hwSeeLed: "GREEN"` hoặc `- hwSeeLed: { channel: 1, expected: ["BLUE", "CYAN"] }` |
| `hwSeeLedBlink` | Kiểm tra đèn LED chớp nháy / chớp tắt (hỗ trợ đếm số lần và màu) | `- hwSeeLedBlink: 1` hoặc `- hwSeeLedBlink: { channel: 1, color: "BLUE", count: 2, timeoutMs: 8000 }` |
| `hwSeeLedOff` | Chờ đèn LED tắt hẳn | `- hwSeeLedOff: 1` |
| `hwSensorLight` | Bật/Tắt đèn chiếu sáng hỗ trợ cảm biến màu | `- hwSensorLight: "on"` hoặc `- hwSensorLight: false` |
| `hwSetBrightnessThresholds` | Cấu hình ngưỡng phần trăm độ sáng và khoảng thời gian chớp nháy | `- hwSetBrightnessThresholds: { channel: 1, offBelowPercent: 30, onAbovePercent: 70, minPulseMs: 50, maxPulseMs: 1000 }` |
| `hwWaitForBrightness` | Chờ độ sáng đạt khoảng phần trăm | `- hwWaitForBrightness: { channel: 1, minPercent: 70, maxPercent: 100 }` |
| `hwWaitForCct` | Chờ nhiệt độ màu Kelvin đạt khoảng CCT | `- hwWaitForCct: { channel: 1, minKelvin: 2700, maxKelvin: 6500 }` |

### D. Nhóm lệnh Hiệu chuẩn (Calibration API)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwCalibrateColor` | Hiệu chuẩn màu sắc mẫu (RED, GREEN, BLUE, YELLOW, CYAN, MAGENTA, PINK, WHITE) | `- hwCalibrateColor: { channel: 1, color: "RED" }` |
| `hwCalibrateBrightness` | Hiệu chuẩn điểm độ sáng tối (`dark`) hoặc sáng (`on`) | `- hwCalibrateBrightness: { channel: 1, mode: "dark" }` |
| `hwAddCctPoint` | Bổ sung điểm hiệu chuẩn CCT (Kelvin) | `- hwAddCctPoint: { channel: 1, knownKelvin: 4000 }` |
| `hwSaveCalibration` | Lưu dữ liệu hiệu chuẩn vào Flash vi điều khiển | `- hwSaveCalibration` |
| `hwLoadCalibration` | Nạp dữ liệu hiệu chuẩn từ Flash vào RAM | `- hwLoadCalibration` |
| `hwResetCalibration` | Khôi phục hiệu chuẩn về mặc định nhà sản xuất | `- hwResetCalibration` |
| `hwEraseCalibration` | Xóa dữ liệu hiệu chuẩn trong Flash | `- hwEraseCalibration` |

### E. Nhóm lệnh Truy vấn & Đọc Trạng thái Linh kiện (Hardware State Query API)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwReadServo` | Đọc trạng thái hoạt động hiện tại (RELEASED/PRESSED/CLICKING) và góc xoay của Servo | `- hwReadServo: 1` |
| `hwReadRelay` | Đọc trạng thái cấp nguồn ON/OFF của Rơ-le | `- hwReadRelay: 1` |
| `hwReadColor` | Đọc thông số RGBC, màu nhận diện ổn định và độ tin cậy của Cảm biến | `- hwReadColor: 1` |
| `hwReadSensorLight` | Đọc trạng thái BẬT/TẮT của Đèn chiếu sáng cảm biến (PB15) | `- hwReadSensorLight: 1` |

### F. Nhóm lệnh Hệ thống & Trạng thái An toàn (System Safety & Diagnostics)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `hwSafeState` | Kích hoạt trạng thái an toàn (Tắt rơ-le, nhả servo, tắt đèn cảm biến) | `- hwSafeState` |
| `hwDiagnostics` | Truy vấn nhật ký chẩn đoán hệ thống vi điều khiển | `- hwDiagnostics` |
| `hwConnect` / `hwDisconnect` | Kết nối / Ngắt kết nối Jig thủ công | `- hwConnect: "COM5"` / `- hwDisconnect` |

---

## 🧪 3. Kịch bản Kiểm thử Tổng hợp Mẫu (`hardware_full_features.yaml`)

Xem chi tiết file kịch bản mẫu tại [hardware_full_features.yaml](file:///Users/nghinguyen/Desktop/MyOpenSource/nl-tester/lumi-tester/e2e/workspaces/lumi_life/hardware_full_features.yaml):

```yaml
platform: android
appId: com.lumi.lifenext
jig:
  port: "${JIG_PORT:-COM5}"
  baudrate: 115200

---
# 1. Cấu hình thông số servo
- hwConfigureServo:
    channel: 1
    pressAngle: 75
    releaseAngle: 15
    pressDurationMs: 400

# 2. Điều khiển nguồn & reboot
- hwPowerOn: 1
- hwPowerCycle:
    channel: 1
    offMs: 1000

# 3. Thao tác bấm nút
- hwClick: 1
- hwRepeatClick:
    channel: 1
    count: 3
- hwReleaseAll

# 4. Cảm biến màu sắc & chớp tắt
- hwSensorLight: "on"
- hwSeeLed: "GREEN"
# Nghiệp vụ: Kiểm tra nháy 2 lần màu BLUE xác nhận Lưu cấu hình thành công
- hwSeeLedBlink:
    channel: 1
    color: "BLUE"
    count: 2
    timeoutMs: 8000
- hwSeeLedOff: 1

# 5. Hiệu chuẩn phần cứng Flash
- hwCalibrateColor:
    channel: 1
    color: "RED"
- hwSaveCalibration

# 6. Safe state shutdown
- hwSafeState
- hwPowerOffAll
```

---

## 💡 4. Quy ước Kiểm tra Nháy LED Nghiệp vụ (LED Blink Verification Convention)

Theo thiết kế hệ thống từ thư viện `hardware_control_services` & `hardware_web`:
1. **Lưu Cấu Hình Nâng Cao Thành Công (Advanced Config SUCCESS)**:
   - Đèn LED nháy **2 lần màu XANH DƯƠNG (BLUE)**.
   - Code YAML:
     ```yaml
     - hwSeeLedBlink:
         channel: 1
         color: "BLUE"
         count: 2
         timeoutMs: 8000
     ```
2. **Lưu Cấu Hình Thất Bại (Advanced Config FAILURE)**:
   - Đèn LED nháy **2 lần màu ĐỎ (RED)**.
   - Code YAML:
     ```yaml
     - hwSeeLedBlink:
         channel: 1
         color: "RED"
         count: 2
         timeoutMs: 8000
     ```
3. **Vào Chế độ Ghép nối / Factory Reset (Pairing Mode)**:
   - Đèn LED nháy liên tục màu **HỒNG (PINK)** hoặc **BLUE**.
   - Code YAML:
     ```yaml
     - hwPress: 1
     - hwSeeLedBlink:
         channel: 1
         color: "PINK"
         timeoutMs: 10000
     - hwRelease: 1
     ```
4. **Bật/Tắt Đèn Rọi Cảm Biến Cân Bằng (Sensor Fill-Light)**:
   - Khi đo màu trong môi trường tối hoặc cần trợ sáng:
     ```yaml
     - hwSensorLight: "on"     # Bật đèn rọi LED trắng của cảm biến
     - hwSeeLed: "GREEN"
     - hwSensorLight: "off"    # Tắt đèn rọi
     ```

---

## 📚 5. Tuyển tập Kịch bản Mẫu Thực tế (Real-World Test Examples)

Dưới đây là các kịch bản test mẫu chuẩn hóa theo cấu trúc Jig Profile và điều khiển phần cứng thực tế (tương tự thư mục `my_testing/lumilife/servo`):

### Ví dụ 1: Nhấn Nút & Kiểm Chứng Chuyển Đổi Màu LED (Toggle Button: Blue -> Click -> Red)
Kiểm thử công tắc ban đầu ở trạng thái Tắt (LED màu Xanh / BLUE) $\rightarrow$ Nhấn nút NC3 $\rightarrow$ Chuyển sang Bật (LED màu Đỏ / RED):

```yaml
platform: android
appId: com.lumi.lumilife
defaultTimeout: 10000
jig: "profiles/jig_profile.yaml"
---
# 1. Ping kiểm tra kết nối STM32
- hwPing: 1

# 2. Kiểm chứng trạng thái ban đầu: Công tắc đang TẮT (LED sáng màu BLUE)
- hwReadColor: "NC3"
- hwSeeLed:
    button: "NC3"
    color: "BLUE"
    timeoutMs: 3000

# 3. Gạt servo nhấn nút NC3
- hwClick: "NC3"
- wait: 500

# 4. Kiểm chứng công tắc đã chuyển sang BẬT (LED đổi sang màu RED)
- hwReadColor: "NC3"
- hwSeeLed:
    button: "NC3"
    color: "RED"
    timeoutMs: 3000
```

---

### Ví dụ 2: Kiểm Chứng Mất Điện & Có Điện Trở Lại (Power Cycle & Safe State Verification)
Kiểm thử hành vi khi mất nguồn 220V $\rightarrow$ đèn tắt hoàn toàn (`hwSeeLedOff`) $\rightarrow$ cấp lại nguồn $\rightarrow$ đèn sáng trở lại:

```yaml
platform: android
appId: com.lumi.lumilife
defaultTimeout: 15000
jig: "profiles/jig_profile.yaml"
---
- hwPing: 1

# 1. Tắt nguồn 220V để xả điện
- hwPowerOff: "220V"
- wait: 2000

# 2. Bật nguồn 220V trở lại
- hwPowerOn: "220V"
- wait: 1500
- hwReadColor: "NC2"

# 3. Ngắt nguồn và kiểm chứng đèn LED tắt hoàn toàn
- hwPowerOff: "220V"
- wait: 2000
- hwSeeLedOff:
    button: "NC2"
    timeoutMs: 4000

# 4. Bật lại nguồn và an toàn
- hwPowerOn: "220V"
- wait: 1500
- hwReadColor: "NC2"
- hwSafeState
```

---

### Ví dụ 3: Nhấn Giữ Vào Chế Độ Pairing (Press & Hold for Reset / Pairing Blink)
Nhấn giữ nút trong 5 giây để đưa thiết bị vào chế độ Pairing, sau đó đợi sự kiện nháy LED màu Hồng (`PINK`):

```yaml
platform: android
appId: com.lumi.lumilife
jig: "profiles/jig_profile.yaml"
---
# 1. Nhấn đè nút NC1 (Pairing/Reset)
- hwPress: "NC1"
- wait: 5000
- hwRelease: "NC1"

# 2. Chờ đèn LED nháy báo hiệu đang ở chế độ Pairing
- hwSeeLedBlink:
    button: "NC1"
    color: "PINK"
    count: 3
    timeoutMs: 10000
```

---

### Ví dụ 4: Điều Khiển Trợ Sáng & Đọc Mẫu Màu Sắc Quang Học
Bật đèn LED trợ sáng của cảm biến (PB15) khi cần đo màu trong điều kiện thiếu sáng hoặc cân bằng quang học:

```yaml
platform: android
appId: com.lumi.lumilife
jig: "profiles/jig_profile.yaml"
---
# 1. Bật đèn trợ sáng cảm biến trên nút NC3
- hwSensorLight:
    button: "NC3"
    state: "on"

# 2. Đọc giá trị màu sắc quang học RGBC & Confidence
- hwReadColor: "NC3"

# 3. Tắt đèn trợ sáng sau khi đọc xong
- hwSensorLight:
    button: "NC3"
    state: "off"
```

---

### Ví dụ 5: Hiệu Chuẩn Màu Sắc & Ngưỡng Độ Sáng Flash MCU
Hiệu chuẩn mẫu màu thực tế trên thiết bị và lưu vào bộ nhớ Flash của bo mạch:

```yaml
platform: android
appId: com.lumi.lumilife
jig: "profiles/jig_profile.yaml"
---
# 1. Hiệu chuẩn màu ĐỎ (RED) và XANH DƯƠNG (BLUE) trên kênh cảm biến NC3
- hwCalibrateColor:
    button: "NC3"
    color: "RED"
- hwCalibrateColor:
    button: "NC3"
    color: "BLUE"

# 2. Cài đặt ngưỡng nhận diện độ sáng bật / tắt (%)
- hwSetBrightnessThresholds:
    button: "NC3"
    offBelowPercent: 25
    onAbovePercent: 65

# 3. Lưu toàn bộ cấu hình vào Flash MCU
- hwSaveCalibration
```

---

## 🔍 6. Kết quả Verification

```bash
lumi-tester validate e2e/workspaces/lumi_life/hardware_full_features.yaml --json
```

**Output:** `"valid": true` (Tất cả câu lệnh phần cứng được phân tích cú pháp chuẩn xác).
