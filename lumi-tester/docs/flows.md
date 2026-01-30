# 🔄 Cấu trúc và Luồng Test Flow

Tài liệu này giải thích cách `lumi-tester` tổ chức và thực thi các kịch bản test.

## 🏗️ Cấu trúc một Test Suite

Một kịch bản test có thể bao gồm 3 phần chính: **Setup**, **Main Flow**, và **Teardown**.

```mermaid
graph TD
    Start((Bắt đầu)) --> Discovery[Tìm thiết bị & Files test]
    Discovery --> Session[Khởi tạo Test Session]
    
    subgraph Execution [Vòng lặp thực thi]
        Session --> Setup{Có setup.yaml?}
        Setup -- Yes --> RunSetup[Thực thi Setup]
        RunSetup --> Main
        Setup -- No --> Main[Thực thi Main Test Flow]
        
        Main --> Teardown{Có teardown.yaml?}
        Teardown -- Yes --> RunTeardown[Thực thi Teardown]
        RunTeardown --> Next
        Teardown -- No --> Next{Còn file test?}
        
        Next -- Yes --> Setup
    end
    
    Next -- No --> Report[Tạo Báo cáo HTML/JSON]
    Report --> End((Kết thúc))
```

## 📋 Chi tiết các thành phần

### 1. Setup (`setup.yaml`)
Được chạy **trước mỗi file test**. Thường dùng để:
- Mở ứng dụng.
- Login (nếu cần cho mọi test).
- Cấp quyền (permissions).

### 2. Main Test Flow
Các file YAML chứa kịch bản test nghiệp vụ cụ thể.
- Ví dụ: `login_test.yaml`, `add_to_cart.yaml`.

### 3. Teardown (`teardown.yaml`)
Được chạy **sau mỗi file test** (ngay cả khi test thất bại). Dùng để:
- Đóng ứng dụng.
- Dọn dẹp dữ liệu test.
- Ngắt giả lập GPS (`stopMockLocation`).

## 🚀 Luồng xử lý Command

Mỗi dòng trong YAML được chuyển thành một Command. Dưới đây là luồng xử lý bên trong của một lệnh:

```mermaid
sequenceDiagram
    participant P as Parser (YAML)
    participant E as Executor
    participant D as Driver (Android)
    participant S as State (Variables)

    P->>E: Gửi Command (ví dụ: tap "Login")
    E->>D: Yêu cầu tìm Element ("Login")
    D-->>E: Trả về tọa độ/Trạng thái
    E->>D: Thực hiện thao tác (Click/Input)
    E->>S: Cập nhật biến số (nếu có)
    E->>D: Thực hiện thao tác (Click/Input)
    E->>S: Cập nhật biến số (nếu có)
    E-->>P: Trả về kết quả (Pass/Fail)
```

> **Lưu ý với iOS**: Luồng xử lý tương tự, nhưng sử dụng `idb` để tương tác với Simulator/Device. Một số lệnh như `eraseText` sẽ có hành vi khác (triple-tap + replace) để đảm bảo độ tin cậy.

## 🛠️ Xử lý khi Test Thất Bại

Khi một lệnh thất bại, `lumi-tester` thực hiện các bước sau để hỗ trợ debug:

1.  **Chụp ảnh màn hình lỗi**: Tên file có tiền tố `fail_`.
2.  **Dump UI Hierarchy**: Lưu cấu trúc XML của màn hình lúc lỗi.
3.  **Dump Logs**: Lấy logcat gần nhất từ thiết bị.
4.  **Teardown**: Vẫn thực thi phần teardown để trả thiết bị về trạng thái sạch.

---

## 💡 Mẹo cho Tester

- **Tính độc lập**: Mỗi file test nên độc lập, không phụ thuộc vào kết quả của file trước.
- **Dùng Sub-flows**: Có thể dùng lệnh `runFlow` để gọi các file YAML khác như một hàm, giúp tái sử dụng code.
---

## 📄 Cấu trúc File YAML

Một file test flow tiêu chuẩn của `lumi-tester` được chia làm 2 phần chính:

### 1. Header (Khai báo)
Chứa các thông tin cấu hình cho toàn bộ kịch bản test.
- `appId`: Package/Bundle ID.
- `platform`: `android`, `ios`, `web`.
- `vars` (alias `env`): Các biến dùng chung.
- `speed`: Tốc độ chạy (`turbo`, `fast`, `normal`, `safe`).
- `defaultTimeout`: Timeout mặc định cho mỗi bước.

### 2. Steps (Danh sách Lệnh)
Danh sách các hành động sẽ được thực hiện tuần tự. Phần này bắt đầu sau dấu phân cách `---`.
Mỗi bước có thể là một chuỗi (lệnh đơn giản) hoặc một map (lệnh kèm tham số).

```yaml
appId: com.example.app
platform: android
---
- open
- tap: "Login"
- inputText:
    id: "user_field"
    text: "admin"
```

> **Mẹo**: Bạn cũng có thể dùng định dạng map duy nhất với khóa `steps` hoặc `commands` nếu không muốn dùng dấu `---`.
