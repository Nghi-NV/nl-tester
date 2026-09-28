# Lumi Tester CLI Commands & Options Master Reference (Cẩm nang Toàn diện về Lệnh & Tham số CLI)

Tài liệu tra cứu chi tiết và đầy đủ nhất về tất cả các câu lệnh (**Subcommands**), cờ tham số (**Options / Flags**), bí danh (**Aliases**), và các kịch bản sử dụng thực tế của **Lumi Tester CLI**.

---

## 📑 1. Bảng Tra cứu Nhanh Toàn bộ Subcommands

| Subcommand | Mục đích chính | Ví dụ điển hình |
| :--- | :--- | :--- |
| **`run`** | Thực thi một file test YAML hoặc toàn bộ thư mục test suite | `lumi-tester run tests/ --platform android --report --snapshot` |
| **`validate`** | Kiểm tra cú pháp, schema, selector hợp lệ (không cần kết nối máy) | `lumi-tester validate tests/ --json` |
| **`list`** | Khám phá cây test file và chỉ số index của từng câu lệnh trong luồng | `lumi-tester list tests/login.yaml --json` |
| **`doctor`** | Kiểm tra môi trường phụ thuộc (ADB, Xcode, Chrome, DHU, Drivers...) | `lumi-tester doctor --platform all --json` |
| **`devices`** | Liệt kê các thiết bị thật / máy ảo đang kết nối với máy tính | `lumi-tester devices --platform android` |
| **`inspect`** | Khởi chạy máy chủ Element Inspector trực quan trên nền Web | `lumi-tester inspect --platform android --port 9358` |
| **`record`** | Ghi lại thao tác thực tế của tester và tự động sinh file YAML | `lumi-tester record --platform android -o recorded.yaml` |
| **`studio`** | Mở ứng dụng Desktop Studio giao diện trực quan | `lumi-tester studio` |
| **`upgrade` / `update`** | Cập nhật CLI và VS Code Extension đa IDE lên bản mới nhất | `lumi-tester upgrade --all --force` |
| **`extension`** | Cài bản Lumi Tester IDE Extension mới nhất vào IDE được chọn | `lumi-tester extension install --ide antigravity` |
| **`version`** | Xem phiên bản hiện tại và đối chiếu với bản mới nhất trên GitHub | `lumi-tester version --json` |
| **`schema`** | Xuất JSON Schema chuẩn phục vụ autocompletion trong IDE | `lumi-tester schema --json` |
| **`report`** | Tái tạo báo cáo HTML Dashboard từ file kết quả JSON có sẵn | `lumi-tester report ./output/test-results.json` |
| **`shell`** | Terminal tương tác, hoặc `-c` để chạy thẳng 1-nhiều lệnh không cần file YAML | `lumi-tester shell -d <serial> -c 'tapOn: "Login"' --json` |
| **`requirements-coverage`** | Đối chiếu `requirements/index.yaml` với `cases.csv` để tìm requirement chưa có testcase thật | `lumi-tester requirements-coverage requirements/index.yaml --cases cases.csv --json` |
| **`system`** | Tự động cài đặt / cập nhật các driver và package hệ thống | `lumi-tester system install --all` |
| **`ai`** | Chọn client rồi cài AI Skill và MCP tương ứng | `lumi-tester ai install` |
| **`camera`** | Bộ công cụ kiểm thử thị giác & nhận diện trạng thái LED qua RTSP | `lumi-tester camera doctor` |
| **`jig`** | Kiểm tra cổng Serial và giao tiếp với mạch kiểm thử Jig phần cứng | `lumi-tester jig ping COM5 --node 1` |
| **`docs`** | Mở trang tài liệu web chính thức trên trình duyệt mặc định | `lumi-tester docs` |
| **`which`** (alias `where`) | Hiện đường dẫn binary đang chạy, binary trên PATH, và các thư mục AI skill/config | `lumi-tester which --json` |

---

## 🚀 2. Chi tiết Toàn bộ Tham số của lệnh `run` (`lumi-tester run`)

Lệnh `run` là lệnh quan trọng nhất, hỗ trợ đa dạng cờ điều khiển từ cấp độ kiểm thử đơn lẻ đến chạy song song hàng loạt trên CI/CD.

```bash
lumi-tester run <PATH> [OPTIONS]
```

### 📋 Bảng tổng hợp các cờ của lệnh `run`

| Cờ / Tham số | Viết tắt / Alias | Kiểu dữ liệu | Giá trị mặc định | Mô tả chi tiết |
| :--- | :--- | :--- | :--- | :--- |
| `<PATH>` | - | `Path` | *Bắt buộc* | Đường dẫn tới 1 file `.yaml` hoặc 1 thư mục chứa nhiều file test |
| **`--platform`** | `-p` | `String` | Tự nhận diện | Nền tảng mục tiêu: `android`, `android_auto`, `ios`, `web`, `macos`, `windows`, `all` |
| **`--device`** | `-d` | `Vec<String>` | `[]` (Tự chọn) | Serial Android (qua ADB) hoặc UDID iOS (qua simctl). Có thể truyền nhiều lần |
| **`--parallel`** | - | `Flag` | `false` | Chạy song song đồng thời test suite trên nhiều thiết bị cắm cùng lúc |
| **`--continue-on-failure`** | `--continue-on-error` | `Flag` | `false` | **Tiếp tục chạy các file test tiếp theo** trong thư mục dù có file test bị fail |
| **`--repeat`** | - | `u32` | `1` | Lặp lại toàn bộ kịch bản test **N lần liên tiếp** |
| **`--from-command-index`** | `--from-index`, `--start-from` | `usize` | `None` | **Chạy từ câu lệnh tại vị trí index N đến hết file** (Bỏ qua các bước trước đó) |
| **`--command-index`** | - | `usize` | `None` | **Chỉ chạy duy nhất 1 câu lệnh** tại vị trí index N |
| **`--command-name`** | - | `String` | `None` | Chỉ chạy câu lệnh đầu tiên có tên khớp với tên chỉ định |
| **`--tags`** | `-t` | `Vec<String>` | `None` | Lọc danh sách test theo nhãn tag (phân cách bằng dấu phẩy: `smoke,p0`) |
| **`--report`** | - | `Flag` | `false` | Tự động sinh báo cáo HTML Dashboard trực quan và file JSON kết quả |
| **`--snapshot`** | `-s` | `Flag` | `false` | Tự động chụp ảnh màn hình (screenshot) sau mỗi bước và khi có lỗi |
| **`--record`** | `-r` | `Flag` | `false` | Tự động quay video màn hình toàn bộ quá trình chạy test (`.mp4`) |
| **`--events-jsonl`** | - | `Flag` | `false` | Xuất luồng sự kiện JSON Lines chi tiết theo từng mili-giây (`output/events.jsonl`) |
| **`--data`** | - | `Path` | `None` | Đường dẫn file CSV hoặc JSON phục vụ kiểm thử theo dữ liệu (Data-Driven Testing) |
| **`--output`** | `-o` | `Path` | `./output` | Thư mục lưu trữ toàn bộ báo cáo, video, ảnh chụp và log |

---

### 💡 Hướng dẫn chi tiết từng nhóm chức năng của `run`

#### 2.1. Điều khiển Chạy Luồng & Xử lý Lỗi (`--continue-on-failure`, `--repeat`)

##### A. Tiếp tục chạy khi có test case bị lỗi (`--continue-on-failure`):
- Khi chạy một thư mục chứa 50 test case, nếu không có cờ này, mặc định khi test case số 3 fail thì toàn bộ chương trình sẽ dừng lại ngay lập tức.
- Thêm cờ `--continue-on-failure` giúp Lumi Tester **bỏ qua lỗi của test case hiện tại, ghi nhận fail vào báo cáo, và tiếp tục chạy từ test case số 4 đến 50**.
```bash
# Chạy toàn bộ thư mục regression, đảm bảo chạy hết 100% test case để có báo cáo đầy đủ
lumi-tester run tests/regression/ --platform android --continue-on-failure --report --output ./test-results
```

##### B. Lặp lại kịch bản nhiều lần (`--repeat <N>`):
- Dùng cho **Stress Testing**, kiểm tra độ ổn định của ứng dụng khi thao tác lặp đi lặp lại nhiều lần, phát hiện rò rỉ bộ nhớ (memory leak) hoặc lỗi deadlock ngẫu nhiên.
```bash
# Chạy lặp lại kịch bản thanh toán 20 lần liên tục
lumi-tester run tests/payment_flow.yaml --repeat 20 --platform ios --report
```

---

#### 2.2. Kỹ thuật Debug Siêu Nhanh (`--from-command-index`, `--command-index`)

Khi viết kịch bản gồm 30 bước, giả sử bước số 18 bị lỗi (do sai selector hoặc timing):

##### A. Chạy từ bước bị lỗi đến hết file (`--from-command-index <N>` / `--start-from <N>`):
- Bạn không cần chờ chạy lại từ bước 0 đến 17 mà có thể giữ nguyên màn hình hiện tại trên máy và chạy tiếp từ bước 18 đến 30:
```bash
# Tra cứu danh sách index các câu lệnh
lumi-tester list tests/complex_order.yaml --json

# Chạy tiếp tục từ bước index 18 đến hết kịch bản
lumi-tester run tests/complex_order.yaml --from-command-index 18 --platform android
```

##### B. Chạy cô lập duy nhất 1 câu lệnh (`--command-index <N>`):
- Dùng để tinh chỉnh và kiểm tra ngay một selector hoặc toạ độ mà không làm thay đổi trạng thái của các bước sau:
```bash
# Chạy duy nhất câu lệnh tại index 18
lumi-tester run tests/complex_order.yaml --command-index 18 --platform android --snapshot
```

---

#### 2.3. Chạy Song song Đa Thiết bị (`--device`, `--parallel`)

- **Chạy trên thiết bị cụ thể**:
```bash
# Chỉ định thiết bị qua Serial Android
lumi-tester run tests/smoke.yaml --device 11046644AC001185

# Chỉ định máy ảo iOS qua UDID
lumi-tester run tests/smoke.yaml --device 8B5C032F-1A40-4ED1-A0BD-38379B8C5311
```

- **Chạy song song trên nhiều thiết bị cùng lúc (`--parallel`)**:
```bash
# Chạy đồng thời trên cả 2 thiết bị Android cắm vào máy
lumi-tester run tests/smoke.yaml --device SERIAL_1 --device SERIAL_2 --parallel --report
```

---

#### 2.4. Lọc Test theo Tag (`--tags <tag1,tag2>`)

Phân loại kịch bản bằng thẻ `tags:` trong YAML header và chạy có chọn lọc:
```yaml
platform: android
tags:
  - smoke
  - authentication
---
- launchApp
```
Lệnh thực thi:
```bash
# Chỉ chạy các file có tag smoke
lumi-tester run tests/ -t smoke --platform android

# Chạy các test thỏa mãn cả tag authentication và p0
lumi-tester run tests/ --tags authentication,p0 --report
```

---

#### 2.5. Thu thập Bằng chứng Kiểm thử (`--report`, `--snapshot`, `--record`, `--events-jsonl`)

- **`--report`**: Tự động tạo:
  - `output/<device>/report.html`: Báo cáo chi tiết từng bước có ảnh chụp kèm theo.
  - `output/index.html`: Dashboard tổng quan toàn bộ lịch sử các phiên test.
  - `output/test-results.json`: Kết quả chi tiết dạng JSON phục vụ parse tự động.
  - `output/junit.xml`: File chuẩn JUnit để nạp vào Jenkins / GitLab CI / GitHub Actions Test Summary.
- **`--snapshot`** (`-s`): Chụp ảnh màn hình mọi lúc bước test thành công hoặc gặp sự cố.
- **`--record`** (`-r`): Ghi video màn hình toàn bộ bài test dạng `.mp4`.
- **`--events-jsonl`**: Xuất stream log dạng JSON Lines giúp IDE và AI Agent theo dõi real-time.

```bash
# Combo lệnh chuẩn đầy đủ nhất cho CI/CD pipeline
lumi-tester run tests/ --platform android --continue-on-failure --report --snapshot --record --events-jsonl --output ./artifacts
```

---

#### 2.6. Kiểm thử theo Dữ liệu (Data-Driven Testing với `--data`)

Thay vì viết vòng lặp thủ công hoặc lặp code trong test flow, bạn có thể truyền trực tiếp file dữ liệu ngoại vi (`.csv` hoặc `.json`):
```bash
# Thực thi test flow theo từng dòng của file CSV
lumi-tester run tests/login_flow.yaml --data users.csv --platform android --report

# Hoặc với file JSON (danh sách các object)
lumi-tester run tests/login_flow.yaml --data users.json --platform android
```

Kịch bản `login_flow.yaml` chỉ cần viết cho một trường hợp duy nhất và sử dụng biến `${tên_cột}` hoặc `${tên_thuộc_tính}`:
```yaml
- tap: "Email"
- inputText: "${username}"
- tap: "Password"
- inputText: "${password}"
- tap: "Login"
```
Lumi Tester sẽ tự động nạp từng bản ghi, truyền vào ngữ cảnh biến môi trường và chạy lần lượt từng lượt test.

---

## 🔍 3. Chi tiết Các Subcommand Khác

### 3.1. `validate` - Kiểm tra Cú pháp Kịch bản
Xác thực tính hợp lệ của toàn bộ file YAML mà không cần khởi động bất kỳ thiết bị hay trình duyệt nào:
```bash
# Kiểm tra 1 file
lumi-tester validate tests/login.yaml

# Kiểm tra cả thư mục và xuất kết quả JSON
lumi-tester validate tests/ --json
```

---

### 3.2. `doctor` - Kiểm tra Môi trường Tự động
Kiểm tra sức khỏe hệ thống, phiên bản driver và công cụ phụ thuộc của máy:
```bash
# Kiểm tra toàn bộ các platform
lumi-tester doctor --platform all

# Kiểm tra riêng cho Android Auto hoặc iOS
lumi-tester doctor --platform android_auto --json
lumi-tester doctor --platform ios --json
```

---

### 3.3. `upgrade` / `update` - Tự động Nâng cấp
Tự động tải bản mới nhất từ GitHub Releases, phân quyền, ký ad-hoc codesign và cập nhật Extension cho tất cả IDE phát hiện được (VS Code, Antigravity, Cursor, Windsurf, VSCodium):
```bash
# Kiểm tra xem có bản mới hay không mà không cài đặt
lumi-tester upgrade --check

# Nâng cấp cả CLI và tất cả IDE Extension
lumi-tester upgrade --all

# Bắt buộc cài đặt lại ngay cả khi đã ở bản mới nhất
lumi-tester upgrade --force

# Cài đặt một phiên bản cụ thể
lumi-tester upgrade --version v0.1.29
```

### 3.4. `extension install` - Cài IDE Extension mới nhất
Tải VSIX từ GitHub Extension Release mới nhất. Nếu bỏ `--ide`, CLI tự chọn IDE duy nhất được phát hiện hoặc hiện danh sách để bạn chọn khi có nhiều IDE:
```bash
# Chọn IDE trong danh sách được phát hiện
lumi-tester extension install

# Chỉ cài vào VS Code
lumi-tester extension install --ide vscode

# Chỉ cài vào Antigravity
lumi-tester extension install --ide antigravity

# Cài vào tất cả IDE được hỗ trợ đang có trên máy (chỉ khi yêu cầu rõ)
lumi-tester extension install --ide all

# Cài Extension và chọn AI client trong danh sách
lumi-tester extension install --ai

# Cài Extension và AI integration cho Codex
lumi-tester extension install --ai-client codex
```

Các giá trị `--ide` được hỗ trợ: `vscode`, `antigravity`, `cursor`, `windsurf`, `vscodium`, `vscode-insiders`, `all`.
AI client hỗ trợ: `codex`, `claude`, `antigravity`. Có thể chọn nhiều client bằng cách lặp lại `--ai-client` hoặc phân tách bằng dấu phẩy.

---

### 3.5. `inspect` - Trình soi Phần tử Giao diện Trực quan
Mở máy chủ Web Inspector giúp tester xem trực quan cây UI hierarchy, xem toạ độ, bounding box và nhận gợi ý selector chuẩn xác:
```bash
# Bật Inspector cho Android trên cổng 9333
lumi-tester inspect --platform android

# Bật Inspector cho macOS Desktop trên cổng 9358
lumi-tester inspect --platform macos --port 9358
```

Nếu cổng đang bị chiếm bởi 1 tiến trình `lumi-tester inspect` cũ còn sót lại (quên tắt, hoặc bị orphan sau khi cập nhật CLI lên bản mới) - triệu chứng thường gặp: chọn được thiết bị nhưng mọi lần chụp ảnh đều báo lỗi - lệnh này **tự phát hiện và dừng tiến trình cũ đó trước khi khởi động**, không cần tự tay tìm và kill port. Nếu cổng bị chiếm bởi 1 tiến trình khác không phải lumi-tester, lệnh sẽ không đụng vào và chỉ cảnh báo rõ ràng - khi đó cần tự giải phóng cổng hoặc dùng `--port` khác.

---

### 3.6. `record` - Ghi lại Thao tác Người dùng ra YAML
Tester thao tác trực tiếp trên màn hình điện thoại, CLI sẽ lắng nghe và tự động tạo ra file kịch bản YAML:
```bash
# Ghi lại thao tác trên app và lưu vào flow.yaml
lumi-tester record --platform android -o flows/recorded_flow.yaml --app com.example.app --include-waits --include-comments
```

---

### 3.7. `jig` - Điều khiển Mạch Kiểm thử Phần cứng
Dùng cho kiểm thử thiết bị IoT / Smart Home (như công tắc thông minh, relay, servo, cảm biến màu LED):
```bash
# Liệt kê danh sách các cổng COM / Serial đang kết nối
lumi-tester jig ports --json

# Kiểm tra kết nối tới Jig qua cổng COM5 với RS485 Node ID 1
lumi-tester jig ping COM5 --node 1 --baudrate 115200 --json
```

---

### 3.8. `camera` - Kiểm thử Thị giác & Trạng thái LED qua Camera RTSP
Dùng camera RTSP để đọc trạng thái nhấp nháy, đổi màu của đèn LED trên bo mạch phần cứng:
```bash
# Kiểm tra môi trường Camera & AI detect
lumi-tester camera doctor

# Chụp 1 khung hình từ luồng RTSP để căn chỉnh góc camera
lumi-tester camera snapshot --rtsp "rtsp://192.168.1.100:554/stream1" -o camera_view.jpg

# Mở giao diện Web để căn chỉnh vùng ROI và học màu LED
lumi-tester camera profile --rtsp "rtsp://192.168.1.100:554/stream1" --profile device_led_profile.json --port 9444

# Theo dõi trực tiếp trạng thái đèn LED đổi màu theo thời gian thực
lumi-tester camera check --profile device_led_profile.json --watch
```

---

### 3.9. `ai` - Cài đặt Skill cho AI Coding Agent
Cài đặt skill cho các AI client được chọn. Khi chạy trong terminal mà không có `--client`, CLI hiện danh sách Codex, Claude Code và Antigravity; nếu chỉ phát hiện một client thì nhấn Enter để chọn client đó. Chỉ client đã chọn mới được cài:
```bash
# Chọn client trong danh sách
lumi-tester ai install

# Chỉ cài Codex
lumi-tester ai install --client codex

# Cài Claude Code và Antigravity
lumi-tester ai install --client claude,antigravity
```

### 3.10. `docs` - Mở Tài liệu Web
Mở trang tài liệu chính thức (https://nghi-nv.github.io/nl-tester/) trên trình duyệt mặc định:
```bash
lumi-tester docs

# Chỉ in URL ra terminal thay vì mở trình duyệt (dùng cho môi trường headless/CI)
lumi-tester docs --print
```

### 3.11. `which` - Kiểm tra Binary & Thư mục Đang Dùng
Hiện đường dẫn binary đang thực thi, đường dẫn binary mà PATH sẽ resolve tới, và
các thư mục AI skill/config liên quan. Hữu ích khi một IDE/MCP client có vẻ
đang dùng một bản `lumi-tester` cũ (build khác với bản vừa cập nhật):
```bash
lumi-tester which
lumi-tester where          # alias
lumi-tester which --json   # đầu ra JSON máy đọc được
```
Nếu binary đang chạy khác với binary mà PATH resolve tới, lệnh sẽ cảnh báo
rõ ràng (`pathMismatch: true` trong JSON) - đây chính xác là nguyên nhân phổ
biến khiến một bản fix "không có tác dụng" trong IDE dù đã build lại CLI.

### 3.12. `shell` - Terminal Tương tác & Chạy Lệnh Trực Tiếp

Có 2 chế độ:

**Tương tác (không truyền `-c`)** - mở prompt, gõ từng lệnh (cú pháp y hệt 1
dòng trong file YAML), `exit`/`quit` để thoát:
```bash
lumi-tester shell --platform android --device <serial>
```

**Chạy thẳng, không cần file YAML (`-c`/`--command`, lặp lại được)** - dành
cho AI/script cần thực hiện 1 hành động nhanh (tap, chụp màn hình, pinch...)
mà không đáng để viết hẳn 1 file test. Mọi `-c` trong 1 lần gọi dùng chung
đúng 1 session thiết bị (chỉ kết nối 1 lần):
```bash
lumi-tester shell --device <serial> \
  -c 'tapOn: "Login"' \
  -c 'pinch: {direction: open, percent: 60}' \
  --json
```
`--json` in ra 1 dòng JSON `{"command","success","error"}` cho mỗi lệnh -
process thoát với exit code khác 0 nếu có lệnh nào fail, nên script/AI kiểm
tra qua exit code là đủ, không cần soi text. Chế độ này dùng chung đúng 1
bộ parser/executor với `run` - không có khái niệm "lệnh shell" tách biệt
với "lệnh trong file YAML".

Cả 2 chế độ hiện chỉ hỗ trợ `android`, `ios`, `macos`, `windows` (chưa hỗ
trợ `web`/`android_auto` - dùng `run`/`validate`/`list` cho 2 nền tảng đó).

### 3.13. `requirements-coverage` - Đối Chiếu SRS với Testcase Thật

Đọc `requirements/index.yaml` (định dạng mô tả trong AI skill's
`testcase-design.md`, mục "SRS Requirements Format"), gộp `requirements:`
từ mọi file trong `includes:`, rồi đối chiếu từng requirement id với
`cases.csv` - **không tin vào 1 trường status tự khai**, mà tính trực tiếp
từ việc file YAML testcase tương ứng có thật sự tồn tại trên đĩa hay không:

```bash
lumi-tester requirements-coverage requirements/index.yaml --cases cases.csv --json
```

Mỗi requirement rơi vào đúng 1 trong 4 trạng thái: `skipped` (có lý do
`skip:` ghi rõ), `covered` (có testcase thật + đủ tag `policy` yêu cầu theo
priority), `missing` (không có dòng `cases.csv` nào trỏ tới file YAML còn
tồn tại), hoặc `incomplete` (có testcase nhưng thiếu tag `policy` bắt buộc
cho priority đó, ví dụ priority `must` yêu cầu cả `smoke` và `negative`
nhưng testcase chỉ có tag `smoke`). Lệnh thoát khác 0 nếu còn `missing`
hoặc `incomplete` nào chưa `skip` - dùng được thẳng như 1 gate trong CI.

`--cases` là tuỳ chọn: bỏ qua thì mọi requirement chưa `skip` đều báo
`missing` (hữu ích để đếm nhanh số lượng chưa `skip` mà chưa cần `cases.csv`
đầy đủ).
