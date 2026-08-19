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
Để không phải lặp lại cấu hình cổng COM và các thông số `hwConfigureServo` trong từng file test, bạn có thể tạo một file profile dùng chung (ví dụ: `profiles/jig_switch_sample.yaml`):

```yaml
# profiles/jig_switch_sample.yaml
port: "${JIG_PORT:-COM5}"
nodeId: 1                      # Địa chỉ Node RS485 (Mặc định: 1)
wireFormat: "@{node} {command}\n" # Mẫu định dạng khung truyền (tùy biến linh hoạt nếu MCU đổi định dạng)
baudrate: 115200
autoPowerOff: true
timeoutMs: 4000
servos:
  - channel: 1
    pressAngle: 75
    releaseAngle: 15
    pressDurationMs: 400
  - channel: 2
    pressAngle: 72
    releaseAngle: 15
    pressDurationMs: 400
```

Trong tất cả các file test kịch bản, chỉ cần trỏ tới file profile:
```yaml
platform: android
appId: com.lumi.lifenext
jig: "profiles/jig_switch_sample.yaml"
---
- hwPowerOn: 1
- hwClick: 1        # Servo đã tự động được nạp đúng góc gạt từ profile
- hwSeeLed: "GREEN"
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

## 🔍 4. Kết quả Verification

```bash
lumi-tester validate e2e/workspaces/lumi_life/hardware_full_features.yaml --json
```

**Output:** `"valid": true` (Tất cả 23 câu lệnh được phân tích cú pháp chuẩn xác).
