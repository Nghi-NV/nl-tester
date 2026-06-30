# Camera Hardware Testing

Tài liệu này mô tả cách dùng camera để đóng vòng kiểm thử phần cứng:

```text
actor điều khiển -> thiết bị vật lý phản ứng -> camera đọc LED -> test verify
```

Ví dụ: app test, người test, script ngoài hoặc robot arm bật đèn, sau đó camera
kiểm tra LED của công tắc thật đã đổi từ `OFF` sang `ON`. Camera profile chỉ mô
tả cách nhìn thiết bị trong khung hình và cách đọc LED; không chứa home, room,
account, tên nhà, hay logic điều khiển app.

## Nguyên Tắc

- Profile là visual model của thiết bị: hình học, vùng LED, nhãn vùng, trạng thái
  màu/pattern có thể đọc.
- Hành động điều khiển là actor riêng: app flow, người test, script lab, hoặc
  robot. Camera flow chỉ đọc precondition, chờ trạng thái/pattern, và lưu bằng
  chứng.
- RTSP/camera source là runtime config. Không commit credential camera vào profile
  hoặc test YAML.
- Dùng target ổn định dạng `deviceId.regionId` trong test, ví dụ
  `switch_4gang_wall.button_1`. `button_1`, `status`, `wifi_led` chỉ unique
  bên trong một device. Label tiếng Việt như `Nút 1` chỉ để hiển thị trong UI
  calibration/report.
- Với thiết bị nhiều màu, học trạng thái theo từng region, không dùng một bảng màu
  global cho mọi LED.

## Luồng Tạo Profile Tốt Nhất

Tester không nên phải nhập tọa độ, HSV, threshold hoặc timeout thủ công. UI
calibration nên dẫn theo wizard sau.

### 1. Chọn loại thiết bị

Chọn template ban đầu:

- `switch_1`
- `switch_2`
- `switch_3`
- `switch_4`
- `switch_8`
- `switch_10`
- `sensor`
- `home_controller`
- `custom`

Template chỉ dùng để sinh layout ROI ban đầu. Nó không quyết định thiết bị thuộc
nhà/phòng nào trong app.

### 2. Canh thiết bị trong ảnh

Tester click 4 góc thiết bị theo thứ tự top-left, top-right, bottom-right,
bottom-left. UI nắn phối cảnh thành ảnh phẳng, rồi overlay các vùng LED/nút lên
ảnh đã warp.

Sau khi canh, UI nên chạy drift check ngắn:

```text
Camera: online
Frame: updating
FPS: 25
Device alignment: OK
Brightness baseline: OK
```

Nếu camera hoặc thiết bị lệch quá nhiều, fail sớm bằng lỗi cấu hình thay vì để
actor bên ngoài chạy tiếp rồi fail mơ hồ.

### 3. Tự tìm LED và gán nhãn vùng

Với switch, UI tự sinh id ổn định:

```text
button_1
button_2
button_3
button_4
status
```

Sau khi chọn layout, bấm `Tự tìm LED trong ảnh`. Detector sẽ quét toàn bộ vùng
button search để tìm blob sáng/màu nổi bật, kể cả khi LED nằm ở góc nút. Kết quả
nên là ROI nhỏ quanh LED với `mask: "ellipse"`. Nếu UI báo không tìm thấy LED,
hãy bật LED, chỉnh lại camera/ánh sáng, hoặc kéo ROI thủ công.

Mỗi region nên có id và label:

```jsonc
{
  "id": "button_1",
  "label": "Nút 1",
  "kind": "led",
  "roi": [80, 80, 36, 36],
  "mask": "ellipse"
}
```

Quy ước:

- `id`: dùng trong YAML test, ổn định, không phụ thuộc ngôn ngữ.
- `deviceId`: namespace của thiết bị trong lab; kết hợp với `id` thành target
  test đầy đủ như `switch_4gang_wall.button_1`.
- `label`: hiển thị trong calibration UI/report.
- `kind`: `led`, `button_led`, `status_led`, hoặc loại chuyên biệt hơn nếu cần.
- `mask`: ưu tiên `ellipse` cho LED tròn/nhỏ; rectangle chỉ nên dùng khi vùng sáng
  thật sự là hình chữ nhật.

### 4. Học trạng thái

UI nên có 3 cách học chính.

#### Học ON/OFF cho switch

Tester dùng app hoặc thao tác vật lý để đưa thiết bị vào trạng thái mong muốn:

1. Bật các nút cần học.
2. Chọn đúng region, ví dụ `switch_4gang_wall · Nút 1`.
3. Bấm `Vùng đang chọn bật → Học ON`.
4. Tắt region đó.
5. Bấm `Vùng đang chọn tắt → Học OFF`.
6. Lặp lại cho từng region cần model riêng, rồi bấm `Verify profile`.

`OFF` không nên được hiểu đơn giản là một màu HSV. Nên coi `OFF` là trạng thái
không có blob sáng vượt baseline của region.

#### Học LED nhiều màu

Với `status`, `wifi_led`, home controller hoặc sensor có nhiều màu:

1. Chọn region.
2. Chọn hoặc nhập state: `RED`, `GREEN`, `PINK`, `YELLOW`, `WHITE`, `BLUE`, `OFF`.
3. Đưa thiết bị vào state đó.
4. Bấm `Học trạng thái hiện tại`.
5. UI báo confidence và margin với state gần nhất.

Ví dụ UI nên hiển thị:

```text
status.PINK learned: confidence 94%, margin vs RED 31%
status.YELLOW learned: confidence 88%, margin vs WHITE 12% - cần học lại hoặc giảm sáng nền
```

Nếu hai màu quá gần nhau, profile nên báo `AMBIGUOUS` thay vì đoán bừa.

#### Học pattern nhấp nháy

Các trạng thái reset/pairing thường là temporal pattern, không phải state tĩnh.
Ví dụ: LED hồng nháy 3 lần trong khoảng 800ms, mỗi pulse tối đa 250ms.

Pattern nên được mô tả riêng:

```jsonc
{
  "id": "reset_pink_3",
  "region": "status",
  "type": "blink",
  "color": "PINK",
  "count": 3,
  "withinMs": 800,
  "pulseMaxMs": 250
}
```

Để đọc pattern nhanh, engine cần frame timestamp và camera latency đủ thấp. Poll
state chậm theo kiểu `waitDeviceState` không đủ tin cậy cho blink ngắn.

### 5. Verify trước khi lưu

Trước khi lưu profile, wizard nên chạy self-test 5-10 giây:

```text
button_1   ON       96%
button_2   OFF      91%
status     PINK     88%
alignment  OK
```

Nếu state dao động, UI phải hướng dẫn cụ thể:

```text
button_1 dao động ON/UNKNOWN.
Gợi ý: thu nhỏ ROI, đổi mask sang ellipse, hoặc học lại ON dưới ánh sáng hiện tại.
```

Nếu region có `allowedStates`, verify nên báo state nào chưa có model học được,
ví dụ `button_1 thiếu OFF`, để tester biết cần đưa actor bên ngoài về state đó
và học bổ sung trước khi dùng state trong test.

## Dùng Trong YAML

Khai báo camera ở header để runner giữ kết nối ấm trong suốt flow.

```yaml
appId: com.lumi.life
vars:
  CAMERA_PROFILE: "profiles/switch_4_wall_left.json"
  TARGET_DEVICE: "switch_4_wall_left"
  STATE_ON: "ON"
  STATE_OFF: "OFF"
camera:
  rtsp: "${CAMERA_RTSP}"
  profile: "${CAMERA_PROFILE}"
  transport: "tcp"
---
- assertDeviceState: { button: "${TARGET_DEVICE}.button_1", expect: "${STATE_OFF}" }
- wait: 5000 # Trong khoảng này app, người test, hoặc robot điều khiển thiết bị.
- waitDeviceState: { button: "${TARGET_DEVICE}.button_1", expect: "${STATE_ON}" }
- getDeviceState: { saveAs: "switchState" }
```

Camera flow không nên biết home, room, account, hay selector điều khiển trong app.
Phần điều khiển có thể đến từ app test riêng, người test, script khác, hoặc robot
arm. Trách nhiệm của camera flow là đọc precondition, chờ trạng thái/pattern, và
lưu bằng chứng đủ rõ khi pass/fail.

`rtsp: "${CAMERA_RTSP}"` là best practice để không commit credential.
Runner resolve biến trong camera header (`rtsp`, `profile`, `transport`) và trong
camera commands (`button`, `expect`, `from`, `to`, `camera`, `saveAs`) trước khi
thực thi.

Khi có nhiều camera:

```yaml
appId: com.lumi.life
vars:
  WALL_SWITCH_DEVICE: "switch_4_wall_left"
  HOME_CONTROLLER_DEVICE: "home_controller"
cameras:
  wall_switch:
    rtsp: "${CAMERA_RTSP}"
    profile: "profiles/switch_4_wall_left.json"
  home_controller:
    rtsp: "${HOME_CONTROLLER_CAMERA_RTSP}"
    profile: "profiles/home_controller.json"
---
- assertDeviceTransition:
    camera: home_controller
    button: "${HOME_CONTROLLER_DEVICE}.wifi_led"
    from: "OFF"
    to: "GREEN"
- getDeviceState:
    camera: wall_switch
    saveAs: "switchState"
```

Lưu ý bảo mật: RTSP URL thường chứa username/password. Runner và docs không nên
in URL đầy đủ ra report/log. Nếu cần debug, chỉ hiển thị URL đã redact như
`rtsp://***@10.0.0.5:554/live`.

## Lệnh Hiện Có

| Lệnh | Mục đích | Tham số chính |
|---|---|---|
| `assertDeviceState` | Đọc trạng thái tức thời. Hợp với precondition hoặc no-cross-talk. | `button`/`led`/`region`, `expect`, `camera?` |
| `waitDeviceState` | Chờ LED đạt state mong muốn. Hợp với app -> thiết bị có độ trễ. | `button`/`led`/`region`, `expect`, `timeoutMs?`, `stableFrames?`, `camera?` |
| `assertDeviceTransition` | Bắt buộc bắt đầu ở `from`, rồi chờ sang `to`. Giảm false pass khi LED đã ở target từ trước. | `from`, `to`, `timeoutMs?`, `stableFrames?`, `camera?` |
| `waitLedPattern` / `assertDevicePattern` | Đọc pattern nhấp nháy bằng timeline frame mới. Hợp reset/pairing blink ngắn. | `button`/`led`/`region`, `expect`, `count`, `withinMs`, `pulseMaxMs?`, `sampleMs?` |
| `getDeviceState` | Đọc toàn bộ region vào biến JSON. | `saveAs`, `camera?` |

Khuyến nghị UX: không bắt tester nhập `timeoutMs` và `stableFrames` ở mọi lệnh.
Default nên đến từ engine/profile. Chỉ override khi case thật sự đặc biệt.

## Profile Đề Xuất

Profile nên tối giản nhưng đủ để mở rộng. Không nên chứa home/room/app account.

```jsonc
{
  "version": 2,
  "name": "switch_4_wall_left",
  "model": "switch_4",
  "geometry": {
    "method": "corners",
    "corners": [[120, 80], [520, 82], [518, 500], [118, 496]],
    "warp": [500, 500]
  },
  "alignment": {
    "maxDriftPx": 8,
    "requireBeforeRun": true,
    "anchors": []
  },
  "regions": [
    {
      "id": "button_1",
      "label": "Nút 1",
      "kind": "button_led",
      "roi": [80, 80, 36, 36],
      "mask": "ellipse",
      "expectedCenter": [98, 98],
      "maxCenterDrift": 6,
      "allowedStates": ["ON", "OFF"]
    },
    {
      "id": "status",
      "label": "LED trạng thái",
      "kind": "status_led",
      "roi": [230, 30, 28, 28],
      "mask": "ellipse",
      "expectedCenter": [244, 44],
      "maxCenterDrift": 6,
      "allowedStates": ["RED", "GREEN", "PINK", "YELLOW", "WHITE", "OFF"]
    }
  ],
  "stateModels": {
    "button_1.ON": { "type": "color", "samples": [] },
    "button_1.OFF": { "type": "dark", "baseline": {} },
    "status.PINK": { "type": "color", "samples": [] },
    "status.WHITE": { "type": "white", "samples": [] }
  },
  "patterns": [
    {
      "id": "reset_pink_3",
      "region": "status",
      "type": "blink",
      "color": "PINK",
      "count": 3,
      "withinMs": 800,
      "sampleMs": 20,
      "pulseMinMs": 40,
      "pulseMaxMs": 250
    }
  ]
}
```

Nếu cần tương thích với schema hiện tại, UI có thể export thêm `buttons` và
`states`. Pattern nháy hiện là runtime assertion bằng `waitLedPattern`; UI
calibration chủ yếu học state màu và vùng ROI. Với reset/pairing blink nhanh,
khai báo rõ `sampleMs`, `pulseMinMs`, `pulseMaxMs`; không giả định camera đọc ổn
xung ngắn hơn cadence frame thực tế.

## Nhiều Màu Và LED Gần Nhau

Các lỗi thường gặp:

- LED trắng có saturation thấp nên dễ bị nhầm với nền sáng.
- LED hồng/đỏ gần nhau nên HSV range có thể overlap.
- LED gần nhau làm ROI của nút này ăn sáng từ nút bên cạnh.
- Mặt nhựa/kính của thiết bị tạo phản xạ màu.
- Camera auto exposure/white balance làm màu thay đổi theo thời gian.

Detector nên quyết định theo nhiều tín hiệu, không chỉ pixel ratio:

```text
blob có đủ sáng không
blob có nằm gần expectedCenter không
màu nào thắng
state thắng cách state thứ hai bao xa
region có bị nhiễu từ neighbor không
camera/profile có bị lệch không
```

Kết quả nên có nhiều trạng thái hơn `MATCH`/`UNKNOWN`:

| Kết quả | Ý nghĩa |
|---|---|
| `MATCH` | Đủ confidence, đúng màu, đúng vị trí. |
| `UNKNOWN` | Không thấy LED hoặc tín hiệu quá yếu. |
| `AMBIGUOUS` | Có tín hiệu nhưng hai state quá gần nhau. |
| `MISALIGNED` | Blob sáng lệch khỏi vị trí LED kỳ vọng hoặc profile không còn khớp. |

## Sai Lệch Camera/Thiết Bị

Lab thật sẽ có camera lệch, thiết bị bị chạm, robot che một phần khung hình, hoặc
ánh sáng phòng thay đổi. Profile cần chống drift theo 3 mức.

### Drift check

Trước suite hoặc trước flow quan trọng, kiểm tra:

```text
camera online
frame không đứng hình
device outline còn khớp
ROI không lệch quá maxDriftPx
brightness baseline không đổi quá ngưỡng
```

Nếu fail, trả lỗi cấu hình như `PROFILE_MISALIGNED` hoặc `CAMERA_STALE_FRAME`.

### Auto realignment

Nếu có thể, dùng anchor point hoặc marker vật lý như ArUco/AprilTag đặt cạnh thiết
bị. Camera đọc marker để tính lại transform, sau đó ROI tự dịch theo thiết bị.

```jsonc
{
  "alignment": {
    "method": "marker",
    "markerType": "aruco",
    "maxDriftPx": 8
  }
}
```

### Recalibration nhanh

Khi auto realignment fail, UI nên mở profile cũ, highlight vị trí lệch và cho
tester kéo lại 4 góc/ROI. Không bắt tạo profile từ đầu.

## Evidence Khi Fail

Lỗi phần cứng phải có bằng chứng đủ để phân biệt actor điều khiển không làm đúng,
thiết bị không phản ứng, camera đọc sai, hay môi trường lab không ổn.

Khi `assertDeviceState`, `waitDeviceState` hoặc `assertDeviceTransition` fail,
report nên lưu:

- frame camera gốc
- frame warped có overlay ROI/state
- crop của LED liên quan
- state/confidence/second-best/margin
- timeline 1-3 giây trước và sau lỗi
- timestamp/latency camera
- metadata actor bên ngoài nếu test runner cung cấp, ví dụ screenshot app hoặc
  robot command id. Đây là thông tin tùy chọn, không thuộc camera profile.

Ví dụ thông tin lỗi mong muốn:

```text
waitDeviceState failed
region: button_1
expected: ON
last: UNKNOWN
confidence: 0.18
second_best: OFF 0.15
camera_latency_ms: 420
evidence: output/camera/button_1_failure_001/
```

## Health Check Trước Suite

Trước khi chạy test E2E dài, nên chạy health check phần cứng:

```text
camera connected
frame updating
profile readable
alignment OK
all required regions visible
known states can be classified
RTSP latency acceptable
```

Nếu sau này có robot arm, health check bổ sung:

```text
robot connected
robot homed
safe pose OK
press points calibrated
emergency stop available
```

Robot arm nên là actor riêng của test runner. Camera chỉ verify kết quả, không
trộn logic điều khiển robot vào profile camera.

## Command Phụ Trợ

```bash
# Mở UI hiệu chỉnh profile
lumi-tester camera calibrate \
  --rtsp "$CAMERA_RTSP" \
  --profile profiles/switch_4_wall_left.json

# Chụp một frame để canh góc camera
lumi-tester camera snapshot \
  --rtsp "$CAMERA_RTSP" \
  --output output/aim.jpg

# Đọc trạng thái một lần
lumi-tester camera detect \
  --rtsp "$CAMERA_RTSP" \
  --profile profiles/switch_4_wall_left.json

# Theo dõi state thay đổi khi debug
lumi-tester camera detect \
  --rtsp "$CAMERA_RTSP" \
  --profile profiles/switch_4_wall_left.json \
  --watch
```

## Checklist Cho Profile Chất Lượng

- Region dùng id ổn định, không phụ thuộc ngôn ngữ.
- Mỗi region chỉ cho phép các state hợp lệ của chính region đó.
- `OFF` được học bằng baseline/dark model.
- LED gần nhau dùng ROI nhỏ, mask ellipse và expected center.
- State nhiều màu có confidence và margin đủ xa nhau.
- Profile có drift/alignment threshold.
- Không lưu RTSP credential trong profile hoặc YAML commit vào git.
- Verify profile chạy ổn 5-10 giây trước khi dùng trong suite.
- Khi fail có ảnh/crop/timeline đủ để debug.
