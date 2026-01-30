# ✍️ Hướng dẫn Viết Test

Tài liệu này giúp bạn hiểu rõ cấu trúc file kịch bản test và cách tổ chức một test flow hiệu quả.

## 📄 Cấu trúc File YAML

`lumi-tester` chấp nhận hai định dạng file để phù hợp với nhu cầu đơn giản hoặc phức tạp.

### 1. Định dạng Phân tách (Header --- Steps)
Đây là định dạng khuyến nghị cho các test thực tế. Sử dụng dấu `---` để tách biệt phần khai báo cấu hình và danh sách các lệnh thực thi.

```yaml
appId: com.example.app
platform: android
tags:
  - smoke
  - regression
---
- launchApp
- tap: "Login"
```

### 2. Định dạng Map (Single Block)
Phù hợp khi bạn muốn định nghĩa toàn bộ test trong một cấu trúc map duy nhất, hoặc khi Test Flow được lồng vào một hệ thống khác.

```yaml
appId: com.example.app
steps: # Hoặc 'commands'
  - open: "com.example.app"
  - tap: "Login"
```

---

## 📋 Các trường Header (Khai báo)

Phần Header nằm phía trên dấu `---`. Nếu không có dấu `---`, các trường này có thể khai báo cùng cấp với `steps`.

| Trường | Alias | Kiểu dữ liệu | Mô tả |
| :--- | :--- | :--- | :--- |
| `appId` | - | String | Package name (Android) hoặc Bundle ID (iOS). |
| `url` | - | String | URL khởi tạo (Web). |
| `platform` | - | String | `android`, `ios`, `web`. |
| `env` | `vars`, `var`| Map | Định nghĩa biến môi trường (Key-Value) hoặc load từ file (`file: path`). |
| `data` | - | String | Path tới file dữ liệu (CSV/JSON). |
| `defaultTimeout` | - | Number | Thời gian chờ mặc định (ms) cho các lệnh. |
| `tags` | - | Array | Danh sách nhãn phân loại test. |
| `speed` | - | String | Tốc độ: `turbo`, `fast`, `normal`, `safe`. |
| `browser` | - | String | (Web) `Chrome`, `Firefox`, `Webkit`. |
| `closeWhenFinish`| - | Boolean | Tự động đóng app khi kết thúc. |
| `steps` | `commands` | Array | Danh sách các lệnh (Dùng trong định dạng Map). |

---

## 🔍 Cách tìm Elements (Selectors)

`lumi-tester` hỗ trợ nhiều cách để xác định element trên màn hình:

1.  **Theo Text**: Tìm văn bản hiển thị (case-insensitive).
    ```yaml
    - tap: "Login"
    ```
2.  **Theo Resource ID**: ID định danh trong code. (Alias: `id`)
    ```yaml
    - tap:
        id: "btn_login"
    ```
3.  **Theo Tọa độ**: Phù hợp khi element không có định danh. (Alias: `point`)
    ```yaml
    - tap:
        point: "50%,80%"
    ```
4.  **Theo Regex**: Tìm theo biểu mẫu của chữ. (Alias: `regex`)
    ```yaml
    - see:
        regex: "OTP: \\d{6}"
    ```
5.  **Theo Vị trí tương đối**: (Aliases: `rightOf`, `leftOf`, `above`, `below`)
    ```yaml
    - tap:
        rightOf: "Username"
        type: "EditText"
    ```
6.  **Theo Mô tả (Accessibility)**: (Aliases: `desc`, `contentDesc`, `accessibilityId`)
    ```yaml
    - tap:
        desc: "Nút Lưu"
    ```

### 🧱 Tìm hiểu về `type` (Element Type)
Trường `type` giúp chỉ định loại thành phần:
- **Android**: `Button`, `EditText`, `TextView`, `ImageView`, `CheckBox`, `Switch`.
- **iOS**: `Button`, `TextField`, `SecureTextField`, `StaticText`, `Image`, `Cell`.
- **Web**: `input`, `button`, `a`, `span`, `div`, `p`.

---

## 📦 Biến số và Substitutions

Sử dụng `${variable_name}` để truy xuất biến.
```yaml
vars:
  username: "test_user"
---
- write: "${username}"
```

---

## 🤝 Best Practices

1.  **Sử dụng `setup.yaml` & `teardown.yaml`**: Để tái sử dụng code login/logout.
2.  **Tránh Tọa độ Cứng**: Luôn ưu tiên Text, ID. Nếu dùng tọa độ, hãy dùng percentage.
3.  **Sâu chuỗi sub-flows**: Dùng `runFlow` để module hóa kịch bản.

## 📁 Tổ chức thư mục

```text
tests/
├── setup.yaml
├── data/
├── common/             # Sub-flows (Login.yaml)
└── scenarios/          # Test chính
```
