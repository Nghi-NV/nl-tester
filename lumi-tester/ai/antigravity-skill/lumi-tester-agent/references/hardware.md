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
```yaml
port: "${JIG_PORT:-COM5}"
baudrate: 115200
autoPowerOff: true
timeoutMs: 4000
servos:
  - channel: 1
    pressAngle: 75
    releaseAngle: 15
    pressDurationMs: 400
  - channel: 2
    pressAngle: 72
    releaseAngle: 15
    pressDurationMs: 400
```

## 2. Standardized Hardware Commands Matrix (`hw*`)

| Category | Commands | Common Usage |
| :--- | :--- | :--- |
| **Relay Power** | `hwPowerOn`, `hwPowerOff`, `hwPowerCycle`, `hwPowerOffAll` | `- hwPowerOn: 1`<br>`- hwPowerCycle: { channel: 1, delayMs: 2000 }` |
| **Servo Control** | `hwClick`, `hwPress`, `hwRelease`, `hwRotate`, `hwRepeatClick`, `hwStartRepeatClick`, `hwStopRepeatClick`, `hwReleaseAll`, `hwConfigureServo` | `- hwClick: 1`<br>`- hwRepeatClick: { channel: 1, count: 3, intervalMs: 300 }`<br>`- hwRotate: { channel: 1, angle: 90 }` |
| **Sensor & LED** | `hwSeeLed`, `hwSeeLedBlink`, `hwSeeLedOff`, `hwSensorLight`, `hwCalibrateColor`, `hwCalibrateBrightness`, `hwAddCctPoint`, `hwSaveCalibration`, `hwLoadCalibration`, `hwSetBrightnessThresholds` | `- hwSeeLedBlink: { channel: 1, color: "BLUE", count: 2, minPulseMs: 50, maxPulseMs: 800 }`<br>`- hwSeeLedOff: 1`<br>`- hwSensorLight: "on"` |
| **Diagnostics** | `hwReadServo`, `hwReadRelay`, `hwReadColor`, `hwReadSensorLight`, `hwDiagnostics`, `hwSafeState` | `- hwReadColor: 1`<br>`- hwSafeState` |

## 3. Fast CLI Discovery & Ping Utilities

```bash
# List all connected Serial / COM ports
lumi-tester jig ports
lumi-tester jig ports --json

# Ping Jig controller and check firmware / latency
lumi-tester jig ping COM5
lumi-tester jig ping profiles/jig_switch_sample.yaml
```

## 4. Hardware Safety Rules
- The executor automatically triggers `ctrl.enter_safe_state()` (release servos, turn off relays and sensor LEDs) and releases the serial port upon test completion or failure.
- Avoid raw coordinate taps when a physical button can be pressed via `hwClick`.
