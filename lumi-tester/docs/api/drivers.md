# Tham chiếu Driver (Driver Reference)

## Driver Traits

Lớp trừu tượng cốt lõi được định nghĩa trong `src/driver/traits.rs`. Bất kỳ triển khai (implementation) nào cho nền tảng mới cũng phải thỏa mãn `Driver` trait này, đảm bảo tính nhất quán cho Runner.

Các phương thức chính bao gồm:
- `tap(selector: &Selector) -> Result<()>`
- `swipe(direction: SwipeDirection, ...) -> Result<()>`
- `input_text(text: &str, ...) -> Result<()>`
- `is_visible(selector: &Selector) -> Result<bool>`
- `get_ui_hierarchy() -> Result<String>`

## Android Driver (`src/driver/android`)

Driver Android sử dụng phương pháp lai (hybrid) để tối ưu hiệu năng và độ tương thích:

- **ADB (Android Debug Bridge)**: Dùng cho các lệnh shell, quản lý gói (cài/gỡ app), và gửi sự kiện input thô (key events).
- **UiAutomator**: Dùng riêng cho thao tác `dump` để lấy cây phân cấp UI dạng XML. Điều này cho phép tìm kiếm element phức tạp (ID, Text, XPath).
- **Chiến lược Input**: Trong khi lệnh `input` đơn giản dùng ADB key events, lệnh nhập liệu phức tạp có thể dùng ADB keyboard broadcast để đảm bảo độ tin cậy.

## iOS Driver (`src/driver/ios`)

Driver iOS quản lý giao tiếp với thiết bị Apple thông qua các giao thức chuẩn:

- **WebDriverAgent (WDA)**: Một ứng dụng phụ trợ (helper app) chạy trên thiết bị, mở một HTTP server. Lumi Tester giao tiếp với WDA để thực hiện các thao tác cảm ứng (touch action) và truy vấn dữ liệu Accessibility Audit (UI tree).
- **IDB (iOS Device Bridge)**: Được dùng để quản lý Simulator, cài đặt ứng dụng, và xem log hệ thống. Nó đóng vai trò thay thế mạnh mẽ cho `simctl` hoặc `libimobiledevice`.

## Web Driver (`src/driver/web`)

Driver Web hoạt động như một lớp wrapper bao quanh **Playwright**, ánh xạ các lệnh mobile sang thao tác web tương đương:

- **Selectors**: Chuyển đổi Lumi selectors sang Playwright Locators (ví dụ: `id: foo` -> `page.locator('#foo')`).
- **Thực thi**: Có thể chạy ở chế độ Headless (không giao diện) hoặc Headed.
- **Mạng**: Hỗ trợ chặn bắt (intercept) các request mạng để mock dữ liệu (thông qua API `route` của Playwright).
