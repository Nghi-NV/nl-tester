# ✍️ Hướng dẫn Viết Test

Tài liệu này giúp bạn viết các kịch bản test hiệu quả và dễ bảo trì.

## 📄 File YAML cơ bản

Mỗi file test bắt đầu bằng phần khai báo (Header) và sau đó là danh sách các lệnh (Steps), phân cách bởi `---`.

```yaml
appId: com.example.app
name: "Test Đăng nhập"
---
- open: "com.example.app"
- tap: "Bắt đầu"
```

## 🔍 Cách tìm Elements (Selectors)

`lumi-tester` hỗ trợ nhiều cách để xác định element trên màn hình:

1.  **Theo Text**: Tìm văn bản hiển thị.
    ```yaml
    - tap: "Login"
    ```
2.  **Theo Resource ID**: ID định danh trong code (R.id.xxx).
    ```yaml
    - tap:
        id: "com.example:id/btn_login"
    ```
3.  **Theo Tọa độ**: Khi element không có ID hoặc Text. Hỗ trợ cả tọa độ tuyệt đối và phần trăm.
    ```yaml
    # Tọa độ tuyệt đối (pixels)
    - tap: 
        point: "500,1000"
    
    # Tọa độ phần trăm (responsive)
    - tap:
        point: "50%,80%"
    ```
4.  **Theo Regex**: Khớp văn bản theo khuôn mẫu. Hỗ trợ các cú pháp nâng cao:
    - `.` (bất kỳ ký tự nào), `*` (0 hoặc nhiều), `+` (1 hoặc nhiều).
    - `\d+` (số), `\d{4}` (4 chữ số).
    - `[0-9]` (khoảng ký tự), `(A|B)` (lựa chọn).
    
    ```yaml
    - see:
        regex: "Chào mừng .+"
    - see:
        regex: "OTP: \\d{6}"
    ```

## 📦 Biến số và Substitutions

Bạn có thể lưu dữ liệu và sử dụng lại bằng cách dùng biến.

```yaml
- setVar:
    name: "user_email"
    value: "tester@qora.vn"

- inputText:
    id: "email_field"
    text: "${user_email}"

# Nhập tiếng Việt có dấu hoặc ký tự đặc biệt
- inputText:
    text: "Mật khẩu @123"
    unicode: true
```

## 🔄 Xử lý Animations và Chờ đợi

Smartphone thường có độ trễ hoặc hiệu ứng chuyển cảnh. 
- Dùng `wait: 1000` (chờ cứng - không khuyến khích).
- Dùng `see: "Text"`: `lumi-tester` sẽ tự động chờ (default timeout) cho tới khi text xuất hiện.

## 🤝 Best Practices

1.  **Sử dụng `setup.yaml`**: Để reset trạng thái app trước mỗi test case.
2.  **Đặt tên file rõ ràng**: Ví dụ `01_login_success.yaml`, `02_login_fail.yaml`.
3.  **Hỗ trợ Accessibility**: Khuyên khích dev đặt `contentDescription` cho các icon/button không có text. `lumi-tester` có thể tìm theo mô tả này.
4.  **Hạn chế dùng tọa độ cứng**: App có thể chạy trên nhiều kích cỡ màn hình khác nhau. Hãy ưu tiên dùng Text hoặc ID. Nếu dùng tọa độ, hãy dùng percentage (`"50%,50%"`).

## 📁 Tổ chức thư mục

```text
tests/
├── setup.yaml          # Chạy trước mỗi test
├── teardown.yaml       # Chạy sau mỗi test
├── auth/               # Nhóm các test authentication
│   ├── login.yaml
│   └── signup.yaml
└── feature_x/          # Nhóm các test tính năng X
    └── feature_steps.yaml
```
