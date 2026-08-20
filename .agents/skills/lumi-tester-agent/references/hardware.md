# Hardware Automation Reference

Guide for automating physical IoT / Smart devices using Lumi Tester hardware control commands (`hw*`), shared Jig profiles, and color/blink sensors.

## 1. Shared Jig Profiles in YAML Header

Declare reusable Jig and Servo configurations in the YAML header:

```yaml
platform: android
appId: com.lumi.lifenext
jig: "profiles/jig_switch_sample.yaml"
---
- hwPowerOn: 1
- hwClick: 1
- hwSeeLedBlink:
    channel: 1
    color: "BLUE"
    count: 2
```

### Profile Structure (`profiles/jig_switch_sample.yaml`):
```text
port: "${JIG_PORT:-COM5}"
nodeId: 1
wireFormat: "@{node} {command}\n"
baudrate: 115200
autoPowerOff: false
timeoutMs: 4000

# Map button names to decoupled Servo and Optical Sensor channels:
buttons:
  NC1:
    servo: 5
    sensor: 5
  NC2:
    servo: 6
    sensor: 7
  NC3:
    servo: 7
    sensor: 6

# Map friendly power groups to multiple relay channels:
relays:
  mainPower: [3, 4]
  220V: [3, 4]

servos:
  - channel: 5
    pressAngle: 75
    releaseAngle: 0
  - channel: 6
    pressAngle: 80
    releaseAngle: 0
  - channel: 7
    pressAngle: 75
    releaseAngle: 0
```

## 2. Standardized Hardware Commands Matrix (`hw*`)

| Category | Commands | Common Usage |
| :--- | :--- | :--- |
| **Relay Power** | `hwPowerOn`, `hwPowerOff`, `hwPowerCycle`, `hwPowerOffAll` | `- hwPowerOn: "220V"`<br>`- hwPowerCycle: { channel: 1, offMs: 2000 }` |
| **Servo Control** | `hwClick`, `hwPress`, `hwRelease`, `hwRotate`, `hwRepeatClick`, `hwStartRepeatClick`, `hwStopRepeatClick`, `hwReleaseAll`, `hwConfigureServo` | `- hwClick: "NC3"`<br>`- hwRepeatClick: { channel: 1, count: 3, pressMs: 200, releaseMs: 200 }`<br>`- hwRotate: { channel: 1, angle: 90 }` |
| **Sensor & LED** | `hwSeeLed`, `hwSeeLedBlink`, `hwSeeLedOff`, `hwSensorLight`, `hwCalibrateColor`, `hwCalibrateBrightness`, `hwAddCctPoint`, `hwSaveCalibration`, `hwLoadCalibration`, `hwSetBrightnessThresholds` | `- hwSeeLed: { button: "NC3", color: "RED", timeoutMs: 3000 }`<br>`- hwSeeLedBlink: { channel: 1, color: "BLUE", count: 2, minPulseMs: 50, maxPulseMs: 800 }`<br>`- hwSeeLedOff: 1`<br>`- hwSensorLight: "on"` |
| **Diagnostics** | `hwReadServo`, `hwReadRelay`, `hwReadColor`, `hwReadSensorLight`, `hwDiagnostics`, `hwSafeState` | `- hwReadColor: "NC3"`<br>`- hwSafeState` |

## 3. Fast CLI Discovery & Ping Utilities

```bash
# List all connected Serial / COM ports
lumi-tester jig ports
lumi-tester jig ports --json

# Ping Jig controller and check firmware / latency (default node 1)
lumi-tester jig ping COM5
lumi-tester jig ping COM5 --node 2
lumi-tester jig ping profiles/jig_switch_sample.yaml
```

## 4. Hardware Safety Rules
- The executor automatically triggers `ctrl.enter_safe_state()` (release servos, turn off relays and sensor LEDs) and releases the serial port upon test completion or failure.
- Avoid raw coordinate taps when a physical button can be pressed via `hwClick`.
