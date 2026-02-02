# Hướng dẫn Cài đặt & Thiết lập

## Yêu cầu tiên quyết (Prerequisites)

Trước khi chạy Lumi Tester, hãy đảm bảo môi trường của bạn đã được cấu hình cho các nền tảng mục tiêu.

### Chung
- **Rust Toolchain**: Cài đặt qua [rustup.rs](https://rustup.rs/).
- **GitHub CLI** (Tùy chọn): Để cài đặt script nhanh hơn.

### Cài đặt cho Android
1. **ADB**: Cài đặt Android Platform Tools.
   ```bash
   brew install android-platform-tools  # macOS
   sudo apt install adb                 # Linux
   ```
2. **Thiết bị**: Bật "Developer Options" (Tùy chọn nhà phát triển) & "USB Debugging" trên điện thoại.

### Cài đặt cho iOS (chỉ macOS)
1. **Xcode**: Cài đặt từ Mac App Store.
2. **Xcode Command Line Tools**:
   ```bash
   xcode-select --install
   ```
3. **idb-companion**: Cần thiết để tương tác với Simulator/Device.
   ```bash
   brew tap facebook/fb
   brew install idb-companion
   ```

## Cài đặt Lumi Tester

### Từ mã nguồn (Source)
1. Clone repository:
   ```bash
   git clone https://github.com/Nghi-NV/nl-tester.git
   cd nl-tester/lumi-tester
   ```
2. Build dự án:
   ```bash
   cargo build --release
   ```
3. Chạy thử:
   ```bash
   ./target/release/lumi-tester --help
   ```

### Sử dụng Script cài đặt tự động
Sử dụng script được cung cấp để tự động thiết lập các dependencies:

```bash
./scripts/install.sh
```

## Khắc phục sự cố (Troubleshooting)

- **Permissions**: Trên macOS, bạn có thể cần cấp quyền cho file binary trong System Settings -> Privacy & Security.
- **Không tìm thấy thiết bị**: Đảm bảo `adb devices` hoặc `xcrun XCTRunner` nhìn thấy thiết bị của bạn.
