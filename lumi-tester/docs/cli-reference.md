# Lumi Tester CLI Commands & Options Reference (Tổng hợp lệnh & Tham số CLI)

Tài liệu hướng dẫn tra cứu đầy đủ tất cả các câu lệnh (subcommands), cờ tham số (options/flags), và cách sử dụng thực tế của **Lumi Tester CLI** phục vụ kiểm thử tự động trên **Android, Android Auto, iOS, Web, macOS, và Windows**.

---

## 📑 Bảng tra cứu nhanh các câu lệnh chính (Subcommands)

| Câu lệnh | Mục đích chính | Ví dụ điển hình |
| :--- | :--- | :--- |
| **`run`** | Thực thi một hoặc nhiều file kịch bản test YAML | `lumi-tester run tests/smoke.yaml --platform android --report` |
| **`validate`** | Kiểm tra cú pháp, selector, schema YAML (không cần mở máy/trình duyệt) | `lumi-tester validate tests/ --json` |
| **`list`** | Liệt kê các test file và danh sách index của từng câu lệnh trong luồng | `lumi-tester list tests/login.yaml --json` |
| **`doctor`** | Kiểm tra môi trường, công cụ phụ thuộc (ADB, Xcode, Chrome, DHU, etc.) | `lumi-tester doctor --platform all --json` |
| **`devices`** | Liệt kê danh sách thiết bị/máy ảo đang kết nối | `lumi-tester devices --platform android` |
| **`inspect`** | Khởi chạy máy chủ Element Inspector để soi UI hierarchy trực tiếp | `lumi-tester inspect --platform android --port 9358` |
| **`record`** | Ghi lại thao tác thực tế trên màn hình và tự động sinh file YAML | `lumi-tester record --platform android -o recorded.yaml` |
| **`studio`** | Khởi chạy giao diện Desktop Studio trực quan | `lumi-tester studio` |
| **`upgrade`** | Cập nhật Lumi Tester CLI và VS Code Extension lên bản mới nhất | `lumi-tester upgrade --all` |
| **`schema`** | Xuất JSON Schema chuẩn của kịch bản YAML | `lumi-tester schema --json` |
| **`camera`** | Chạy kiểm thử thị giác & nhận diện trạng thái đèn LED qua Camera RTSP | `lumi-tester camera run flow.yaml --rtsp rtsp://...` |
| **`jig`** | Điều khiển mạch kiểm thử phần cứng Jig qua cổng Serial/COM | `lumi-tester jig ping COM5 --node 1` |
| **`system`** | Tự động tải và cài đặt các driver/công cụ hệ thống còn thiếu | `lumi-tester system install --all` |
| **`ai`** | Cài đặt các skill tích hợp cho AI Agent (Codex, Antigravity, Claude, Cursor) | `lumi-tester ai install` |

---

## ⚙️ Chi tiết các cờ và tham số của lệnh `run` (Execution Options)

Lệnh `run` là lệnh trọng tâm của Lumi Tester. Dưới đây là chi tiết các cờ tham số hỗ trợ:

### 1. Điều khiển độ ổn định & Vòng lặp test
- **`--repeat <N>` / `-r <N>`**:
  - *Ý nghĩa*: Lặp lại toàn bộ kịch bản test **N lần liên tiếp**.
  - *Ứng dụng*: Chạy Stress Testing, kiểm tra độ ổn định (flaky test detection), phát hiện rò rỉ bộ nhớ (memory leak) hoặc nghẽn tài nguyên sau nhiều chu kỳ thao tác.
  - *Ví dụ*:
    ```bash
    # Chạy lặp lại kịch bản 10 lần liên tục
    lumi-tester run tests/payment.yaml --repeat 10 --platform android
    ```
- **`--retry <N>`**:
  - *Ý nghĩa*: Tự động thử lại kịch bản tối đa **N lần** nếu bước nào đó bị fail giữa chừng.
  - *Ứng dụng*: Giúp giảm thiểu việc test bị fail do mạng chập chờn, animation trễ ngẫu nhiên hoặc pop-up bất ngờ.
  - *Ví dụ*:
    ```bash
    # Tự động thử lại tối đa 3 lần nếu có lỗi xảy ra
    lumi-tester run tests/checkout.yaml --retry 3 --platform ios
    ```

### 2. Định tuyến Nền tảng & Thiết bị mục tiêu
- **`--platform <platform>` / `-p <platform>`**:
  - *Giá trị hợp lệ*: `android`, `android_auto`, `ios`, `web`, `macos`, `windows`, `all`.
  - *Ý nghĩa*: Chỉ định nền tảng cần thực thi kịch bản. Nếu kịch bản đã có header `platform: ...`, cờ này có thể dùng để ghi đè hoặc xác thực thiết bị tương ứng.
  - *Ví dụ*:
    ```bash
    lumi-tester run test.yaml --platform android_auto
    ```
- **`--device <serial_or_udid>` / `-d <id>`**:
  - *Ý nghĩa*: Chỉ định rõ Serial thiết bị Android (qua `adb devices`), Simulator UDID của iOS (qua `xcrun simctl list`), hoặc địa chỉ IP/Port khi có nhiều thiết bị cắm vào máy.
  - *Ví dụ*:
    ```bash
    lumi-tester run test.yaml --device 11046644AC001185
    lumi-tester run test.yaml --device 8B5C032F-1A40-4ED1-A0BD-38379B8C5311
    ```

### 3. Tối ưu thời gian & Debug đơn lẻ
- **`--command-index <N>`**:
  - *Ý nghĩa*: Chỉ thực thi **duy nhất 1 câu lệnh** tại vị trí index `N` (đánh số từ `0`) trong file kịch bản.
  - *Ứng dụng*: Tiết kiệm thời gian debug - khi một câu lệnh phức tạp bị lỗi (ví dụ bước thứ 5), bạn không cần chạy lại từ bước 1 đến 4 mà có thể sửa YAML và chạy lại riêng bước đó.
  - *Ví dụ*:
    ```bash
    # Tra cứu index câu lệnh
    lumi-tester list tests/login.yaml --json
    # Chạy lại riêng câu lệnh index số 3
    lumi-tester run tests/login.yaml --command-index 3 --report
    ```
- **`--timeout <ms>`**:
  - *Ý nghĩa*: Ghi đè thời gian chờ tối đa (mặc định tính bằng mili-giây) cho việc tìm kiếm element hoặc thực thi lệnh.
  - *Ví dụ*:
    ```bash
    lumi-tester run tests/heavy_load.yaml --timeout 20000
    ```

### 4. Báo cáo & Bằng chứng kiểm thử (Reports & Artifacts)
- **`--report`**:
  - *Ý nghĩa*: Tự động sinh báo cáo HTML Dashboard trực quan (`output/<device>/report.html`), báo cáo lịch sử (`output/index.html`), và file kết quả chuẩn JSON (`test-results.json`), JUnit XML (`junit.xml`).
- **`--snapshot`**:
  - *Ý nghĩa*: Tự động chụp ảnh màn hình (screenshot) sau mỗi câu lệnh thành công và khi xảy ra lỗi để làm bằng chứng kiểm thử trực quan.
- **`--events-jsonl`**:
  - *Ý nghĩa*: Bắn ra luồng sự kiện chi tiết theo từng mili-giây dạng JSON Lines (`output/events.jsonl`), giúp IDE extension hoặc AI Agent theo dõi trạng thái runtime trực tiếp.
- **`--output <path>` / `-o <path>`**:
  - *Ý nghĩa*: Tuỳ chỉnh thư mục lưu trữ toàn bộ báo cáo, ảnh chụp và log (mặc định là `./output`).
  - *Ví dụ*:
    ```bash
    lumi-tester run tests/ --report --snapshot --events-jsonl --output ./test-results/build_102
    ```

### 5. Nâng cao: Môi trường & Web headless
- **`--env-file <path>` / `-e <path>`**:
  - *Ý nghĩa*: Nạp các biến môi trường từ file `.env` (ví dụ thông tin tài khoản, API key, URL test).
  - *Ví dụ*:
    ```bash
    lumi-tester run tests/auth.yaml -e .env.staging
    ```
- **`--headless`**:
  - *Ý nghĩa*: Chạy trình duyệt Web ở chế độ nền không mở cửa sổ (Headless mode), tối ưu cho server CI/CD (GitHub Actions, GitLab CI, Jenkins).
  - *Ví dụ*:
    ```bash
    lumi-tester run tests/web_smoke.yaml --platform web --headless --report
    ```

---

## 🛠️ Hướng dẫn sử dụng các Subcommand phụ trợ

### 1. Kiểm tra môi trường (`doctor`)
Kiểm tra xem máy tính của bạn đã cài đặt đầy đủ công cụ cần thiết cho từng nền tảng hay chưa:
```bash
# Kiểm tra toàn bộ nền tảng
lumi-tester doctor --platform all

# Kiểm tra riêng cho Android Auto hoặc iOS
lumi-tester doctor --platform android_auto --json
lumi-tester doctor --platform ios --json
```

### 2. Kiểm tra tính hợp lệ của kịch bản (`validate`)
Xác thực toàn bộ cú pháp YAML, các trường selector, tham số lệnh mà không cần kết nối thiết bị thật:
```bash
lumi-tester validate tests/ --json
```

### 3. Tự động nâng cấp (`upgrade`)
Nâng cấp CLI và Extension đa IDE (VS Code, Antigravity, Cursor, Windsurf, VSCodium) tự động kèm thanh tiến trình trực quan:
```bash
# Kiểm tra phiên bản mới
lumi-tester upgrade --check

# Cập nhật toàn bộ (CLI + IDE Extension)
lumi-tester upgrade --all
```

### 4. Máy chủ Soi Element Inspector (`inspect`)
Bật máy chủ Inspector để hiển thị cây UI Hierarchy, bounding box và gợi ý selector:
```bash
lumi-tester inspect --platform android --port 9358
lumi-tester inspect --platform macos --port 9358
```
