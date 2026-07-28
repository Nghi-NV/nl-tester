# Hướng dẫn Kiểm thử Phần cứng Tự động Đầy đủ (Hardware Automation User Guide)

Lumi Tester tích hợp module **Điều khiển Phần cứng Tự động (Hardware Automation)** phát triển bằng **100% Native Rust**, đáp ứng **100% tính năng** từ thư viện `hardware_control_services` (bao gồm điều khiển Servo, Relay, Cảm biến màu TCS34725/TCS3200, đo độ sáng Brightness, nhiệt độ màu CCT, phát hiện chớp tắt Blink, Hiệu chuẩn Calibration và Trạng thái An toàn System Safe).

---

## 🔌 1. Cấu hình Jig Phần cứng ở Header (`jig:`)

Bạn có thể khai báo kết nối với Jig phần cứng ngay tại phần **Header** của file YAML test (phía trên dấu `---`). 

Lumi Tester sẽ:
* **Tự động kết nối** với Jig phần cứng trước khi bắt đầu câu lệnh đầu tiên.
* **Tự động ngắt điện an toàn (`turnOffAll`) & ngắt kết nối (`disconnectJig`)** sau khi kịch bản kết thúc (kể cả khi Pass, Fail hay có lỗi).

### a. Cú pháp Ngắn gọn (Short-hand Syntax)
```yaml
platform: android
appId: com.lumi.lifenext
jig: "COM5"   # Cổng Serial kết nối Jig phần cứng (Windows: COM5, Linux/macOS: /dev/ttyUSB0)
---
- turnOn: 1
- clickButton: 1
- seeLedColor: "GREEN"
```

### b. Cú pháp Nâng cao cho CI/CD (Advanced CI/CD Config)
```yaml
platform: android
appId: com.lumi.lifenext

jig:
  port: "${JIG_PORT:-COM5}"     # Truyền biến môi trường CI/CD (mặc định COM5)
  baudrate: 115200             # Tốc độ Baud (Mặc định: 115200)
  autoPowerOff: true           # Tự động tắt toàn bộ rơ-le nguồn khi kết thúc test (Mặc định: true)
  timeoutMs: 4000              # Custom thời gian timeout chờ phản hồi từ vi điều khiển (ms)
---
- turnOn: 1
- clickButton: 1
- seeLedColor: "GREEN"
```

---

## 📖 2. Bảng Tra cứu Tập lệnh Ngôn ngữ Tự nhiên Đầy đủ (Full Matrix)

### A. Nhóm lệnh Điều khiển Nguồn Rơ-le (Relay Control)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `turnOn` | Bật nguồn cấp điện cho kênh chỉ định | `- turnOn: 1` |
| `turnOff` | Tắt nguồn cấp điện cho kênh chỉ định | `- turnOff: 1` |
| `powerCycle` | Hard Power Reboot (Tắt điện -> nghỉ -> Bật lại nguồn) | `- powerCycle: { channel: 1, offMs: 2000 }` |
| `turnOffAll` | Tắt toàn bộ nguồn điện các kênh rơ-le | `- turnOffAll` |

### B. Nhóm lệnh Điều khiển Động cơ Servo gạt nút (Servo Control)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `configureServo` | Cấu hình góc gạt (press/release angle) và thời gian gạt nút | `- configureServo: { channel: 1, pressAngle: 75, releaseAngle: 15, pressDurationMs: 400, holdDurationMs: 300 }` |
| `clickButton` | Bấm nút vật lý đơn | `- clickButton: 1` hoặc `- clickButton: { channel: 1, holdMs: 500 }` |
| `repeatClick` | Bấm nút lặp lại N lần (Nhấp đúp / Triple click) | `- repeatClick: { channel: 1, count: 3 }` |
| `holdButton` | Nhấn giữ nút vật lý (Pairing / Factory Reset) | `- holdButton: 1` |
| `releaseButton` | Nhả nút vật lý | `- releaseButton: 1` |
| `releaseAllButtons` | Nhả tất cả các nút servo | `- releaseAllButtons` |
| `startRepeatClick` | Bắt đầu chạy lặp bấm nút liên tục trên STM32 | `- startRepeatClick: { channel: 1, periodMs: 1500 }` |
| `stopRepeatClick` | Dừng lặp bấm nút liên tục | `- stopRepeatClick: 1` |

### C. Nhóm lệnh Cảm biến Màu & Độ sáng (Color & Blink Sensor)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `seeLedColor` | Kiểm tra màu đèn LED chỉ thị | `- seeLedColor: "GREEN"` hoặc `- seeLedColor: { channel: 1, expected: ["BLUE", "CYAN"] }` |
| `seeLedBlink` | Kiểm tra đèn LED chớp nháy / chớp tắt | `- seeLedBlink: 1` hoặc `- seeLedBlink: { channel: 1, timeoutMs: 8000 }` |
| `seeLedOff` | Chờ đèn LED tắt hẳn | `- seeLedOff: 1` |
| `setSensorLight` | Bật/Tắt đèn chiếu sáng hỗ trợ cảm biến màu | `- setSensorLight: "on"` hoặc `- setSensorLight: false` |
| `setBrightnessThresholds` | Cấu hình ngưỡng phần trăm độ sáng và khoảng thời gian chớp nháy | `- setBrightnessThresholds: { channel: 1, offBelowPercent: 30, onAbovePercent: 70, minPulseMs: 50, maxPulseMs: 1000 }` |
| `waitForBrightness` | Chờ độ sáng đạt khoảng phần trăm | `- waitForBrightness: { channel: 1, minPercent: 70, maxPercent: 100 }` |
| `waitForCct` | Chờ nhiệt độ màu Kelvin đạt khoảng CCT | `- waitForCct: { channel: 1, minKelvin: 2700, maxKelvin: 6500 }` |

### D. Nhóm lệnh Hiệu chuẩn (Calibration API)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `calibrateColor` | Hiệu chuẩn màu sắc mẫu (RED, GREEN, BLUE, YELLOW, CYAN, MAGENTA, PINK, WHITE) | `- calibrateColor: { channel: 1, color: "RED" }` |
| `calibrateBrightness` | Hiệu chuẩn điểm độ sáng tối (`dark`) hoặc sáng (`on`) | `- calibrateBrightness: { channel: 1, mode: "dark" }` |
| `addCctPoint` | Bổ sung điểm hiệu chuẩn CCT (Kelvin) | `- addCctPoint: { channel: 1, knownKelvin: 4000 }` |
| `saveCalibration` | Lưu dữ liệu hiệu chuẩn vào Flash vi điều khiển | `- saveCalibration` |
| `loadCalibration` | Nạp dữ liệu hiệu chuẩn từ Flash vào RAM | `- loadCalibration` |
| `resetCalibration` | Khôi phục hiệu chuẩn về mặc định nhà sản xuất | `- resetCalibration` |
| `eraseCalibration` | Xóa dữ liệu hiệu chuẩn trong Flash | `- eraseCalibration` |

### E. Nhóm lệnh Truy vấn & Đọc Trạng thái Linh kiện (Hardware State Query API)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `readServo` | Đọc trạng thái hoạt động hiện tại (RELEASED/PRESSED/CLICKING) và góc xoay của Servo | `- readServo: 1` |
| `readRelay` | Đọc trạng thái cấp nguồn ON/OFF của Rơ-le | `- readRelay: 1` |
| `readColor` | Đọc thông số RGBC, màu nhận diện ổn định và độ tin cậy của Cảm biến | `- readColor: 1` |
| `readSensorLight` | Đọc trạng thái BẬT/TẮT của Đèn chiếu sáng cảm biến (PB15) | `- readSensorLight` |

### F. Nhóm lệnh Hệ thống & Trạng thái An toàn (System Safety & Diagnostics)
| Câu lệnh YAML | Ý nghĩa tự nhiên | Cú pháp ví dụ |
| --- | --- | --- |
| `enterSafeState` | Kích hoạt trạng thái an toàn (Tắt rơ-le, nhả servo, tắt đèn cảm biến) | `- enterSafeState` |
| `systemDiagnostics` | Truy vấn nhật ký chẩn đoán hệ thống vi điều khiển | `- systemDiagnostics` |
| `connectJig` / `disconnectJig` | Kết nối / Ngắt kết nối Jig thủ công | `- connectJig: "COM5"` / `- disconnectJig` |

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
- configureServo:
    channel: 1
    pressAngle: 75
    releaseAngle: 15
    pressDurationMs: 400

# 2. Điều khiển nguồn & reboot
- turnOn: 1
- powerCycle:
    channel: 1
    offMs: 1000

# 3. Thao tác bấm nút
- clickButton: 1
- repeatClick:
    channel: 1
    count: 3
- releaseAllButtons

# 4. Cảm biến màu sắc & chớp tắt
- setSensorLight: "on"
- seeLedColor: "GREEN"
- seeLedBlink: 1
- seeLedOff: 1

# 5. Hiệu chuẩn phần cứng Flash
- calibrateColor:
    channel: 1
    color: "RED"
- saveCalibration

# 6. Safe state shutdown
- enterSafeState
- turnOffAll
```

---

## 🔍 4. Kết quả Verification

```bash
lumi-tester validate e2e/workspaces/lumi_life/hardware_full_features.yaml --json
```

**Output:** `"valid": true` (Tất cả 23 câu lệnh được phân tích cú pháp chuẩn xác).
