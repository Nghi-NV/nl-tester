# In-Car Infotainment Testing: Android Auto & Apple CarPlay (Kiểm thử Tự động Android Auto & Apple CarPlay)

Tài liệu hướng dẫn chuyên sâu về kiểm thử tự động hệ thống giải trí ô tô thông minh (In-Vehicle Infotainment - IVI) trên nền tảng **Android Auto** và **Apple CarPlay** bằng **Lumi Tester**.

---

## 🚗 1. Android Auto Automation Guide

### 1.1. Kiến trúc & Cơ chế hoạt động
Android Auto hoạt động theo cơ chế chiếu màn hình (Projection Protocol) từ điện thoại Android lên màn hình xe hơi (Head Unit):
- **Desktop Head Unit (DHU)**: Công cụ giả lập màn hình ô tô chính thức từ Google, giao tiếp với điện thoại qua cổng socket chuyển tiếp (ADB Port Forwarding `tcp:5277`).
- **Mô hình tương tác**: Android Auto trên DHU hoạt động như một giao diện đồ hoạ Canvas/Surface, hỗ trợ tương tác qua toạ độ chạm (`point`), cụm phím điều khiển cứng (D-pad), phím Media và tìm kiếm giọng nói.

### 1.2. Chuẩn bị môi trường (Prerequisites)
1. Cài đặt Android SDK Platform-Tools (`adb`) và Desktop Head Unit:
   ```bash
   # Tải DHU từ Android SDK Manager hoặc Android Studio:
   # SDK Tools -> Android Auto Desktop Head Unit emulator
   ```
2. Bật chế độ Nhà phát triển trên Android Auto của điện thoại:
   - Vào **Cài đặt -> Ứng dụng -> Android Auto**.
   - Cuộn xuống phần **Phiên bản**, nhấn liên tục 10 lần để mở Developer Settings.
   - Nhấn menu 3 chấm góc trên bên phải -> Chọn **Khởi động máy chủ đầu phát (Start head unit server)**.
3. Thiết lập chuyển tiếp cổng qua ADB:
   ```bash
   adb forward tcp:5277 tcp:5277
   ```
4. Kiểm tra môi trường với Lumi Tester:
   ```bash
   lumi-tester doctor --platform android_auto --json
   ```

### 1.3. Cú pháp Kịch bản YAML cho Android Auto
Header bắt buộc:
```yaml
platform: android_auto
appId: com.example.automotive.app
defaultTimeout: 15000
---
# Mở ứng dụng trên màn hình xe hơi
- launchApp
- wait: 2000

# 1. Chạm theo toạ độ phần trăm trên màn hình Head Unit
- tap:
    point: "50%,80%"

# 2. Điều khiển bằng các phím D-pad và Phím chức năng xe hơi
- press: navigation       # Phím Bản đồ dẫn đường
- wait: 1000
- press: search           # Phím Tìm kiếm
- press: play_pause       # Phím Tạm dừng/Phát nhạc
- press: media_next       # Phím Bài hát tiếp theo
- press: media_previous   # Phím Bài hát trước đó

# 3. Điều hướng Menu bằng D-pad
- press: DPAD_UP
- press: DPAD_DOWN
- press: DPAD_CENTER      # Chọn mục đang focus

# 4. Vuốt chuyển tab / danh sách
- swipeLeft
- swipeRight

# 5. Chụp ảnh màn hình làm bằng chứng kiểm thử
- screenshot: output/android_auto_dashboard.png

- stopApp
```

### 1.4. Bảng các phím điều khiển đặc thù trên Android Auto

| Tên phím (`press: <key>`) | Chức năng trên xe hơi |
| :--- | :--- |
| **`navigation`** | Mở ứng dụng Bản đồ dẫn đường mặc định (Google Maps / Waze) |
| **`search`** | Kích hoạt thanh tìm kiếm địa điểm / bài hát |
| **`play_pause`** | Bật / Tạm dừng phát nhạc |
| **`media_next`** | Chuyển bài hát kế tiếp |
| **`media_previous`** | Quay lại bài hát trước |
| **`DPAD_UP` / `DPAD_DOWN`** | Cuộn danh sách bài hát, danh bạ, địa điểm |
| **`DPAD_LEFT` / `DPAD_RIGHT`**| Chuyển danh mục / tab điều hướng |
| **`DPAD_CENTER` / `ENTER`** | Nhấn chọn mục đang được đánh dấu (Focus) |
| **`back`** | Quay lại màn hình trước |

---

## 🍏 2. Apple CarPlay Automation Guide

### 2.1. Kiến trúc & Cơ chế hoạt động
Apple CarPlay cho phép kết nối iPhone với màn hình xe hơi để hiển thị giao diện CarPlay chuyên dụng:
- **iOS Simulator CarPlay Display**: Xcode Simulator hỗ trợ mô phỏng màn hình ngoài CarPlay thông qua menu **I/O -> External Displays -> CarPlay**.
- **WebDriverAgent (WDA) Integration**: Lumi Tester giao tiếp với CarPlay thông qua cây Accessibility Hierarchy của màn hình phụ, cho phép tương tác bằng cả **Accessibility ID, Label, Text** và toạ độ `point`.

### 2.2. Chuẩn bị môi trường
1. Cài đặt Xcode & Command Line Tools:
   ```bash
   xcode-select --install
   ```
2. Khởi chạy máy ảo iOS Simulator:
   ```bash
   xcrun simctl boot "iPhone 16 Pro"
   open -a Simulator
   ```
3. Bật màn hình ngoài CarPlay trên Simulator:
   - Trên thanh menu của Simulator: chọn **I/O -> External Displays -> CarPlay**.
   - Một cửa sổ CarPlay độc lập sẽ xuất hiện song song với màn hình iPhone.
4. Kiểm tra với Lumi Tester:
   ```bash
   lumi-tester doctor --platform ios --json
   ```

### 2.3. Cú pháp Kịch bản YAML cho Apple CarPlay
Header kịch bản:
```yaml
platform: ios
appId: com.example.carplay.audioapp
defaultTimeout: 15000
---
# 1. Mở ứng dụng hỗ trợ CarPlay
- launchApp

# 2. Tương tác với các nút bấm trên giao diện CarPlay qua Accessibility Label / ID
- tap:
    text: "Now Playing|Đang phát"

# 3. Thao tác điều khiển nhạc
- tap:
    id: "play_button"
- wait: 1000
- tap:
    text: "Library|Thư viện"

# 4. Cuộn danh sách Playlist
- swipeUp
- tap:
    text: "Favorites"

# 5. Xác minh văn bản hiển thị trên màn hình xe
- see:
    text: "Track Title"

# 6. Chụp ảnh màn hình CarPlay
- screenshot: output/carplay_now_playing.png
```

---

## 📊 3. So sánh tính năng kiểm thử giữa Android Auto & Apple CarPlay

| Tính năng | Android Auto (`platform: android_auto`) | Apple CarPlay (`platform: ios`) |
| :--- | :--- | :--- |
| **Giao diện tương tác** | DHU (Desktop Head Unit) | Simulator External Display / Thiết bị thật |
| **Cây phân cấp UI (Hierarchy)**| Không có (Surface Graphics) | Có (Accessibility Element Tree qua WDA) |
| **Cơ chế chọn phần tử** | Toạ độ `point: "X%,Y%"` + Phím D-pad | `id`, `text`, `regex`, `accessibilityId`, `point` |
| **Xác minh nội dung** | Chụp ảnh (`screenshot`) / Nhận diện OCR | `see`, `notSee`, `waitUntilVisible`, `screenshot` |
| **Điều khiển phần cứng xe** | D-pad, Media keys, Rotary dial | Touch screen, Sidebar navigation, Siri audio |
| **Chạy kiểm thử Headless** | Hỗ trợ qua DHU command server | Hỗ trợ qua headless iOS Simulator |

---

## 💡 4. Best Practices khi viết Test cho xe hơi (Automotive Best Practices)

1. **Sử dụng `wait` sau khi chuyển màn hình**:
   - Giao diện xe hơi thường có độ trễ hiệu ứng chuyển cảnh (transition animations) an toàn, nên thêm `wait: 1000` đến `wait: 2000` sau các lệnh `launchApp` hoặc chuyển tab.
2. **Ưu tiên D-pad cho Android Auto**:
   - Khi kiểm thử trên Android Auto, việc dùng các phím `press: DPAD_DOWN` và `press: DPAD_CENTER` phản ánh chính xác 100% trải nghiệm của tài xế khi dùng núm xoay cứng (Rotary Controller) trên vô lăng hoặc bệ tỳ tay.
3. **Chụp ảnh bằng chứng trực quan (`--snapshot`)**:
   - Luôn kèm theo cờ `--snapshot` và `--report` khi chạy kiểm thử tự động để lưu lại toàn bộ ảnh chụp giao diện Dashboard, hỗ trợ đối chiếu với tiêu chuẩn thiết kế Apple MFi CarPlay và Android Auto Design Guidelines.
