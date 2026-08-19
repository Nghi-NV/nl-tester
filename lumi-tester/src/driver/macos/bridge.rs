use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Window position and dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Low-level macOS System and Application Bridge (similar to ADB for Android)
#[derive(Debug, Default, Clone)]
pub struct MacosBridge;

impl MacosBridge {
    pub fn new() -> Self {
        Self
    }

    /// Execute a Swift script via stdin
    pub fn run_swift(script: &str, args: &[&str]) -> Result<String> {
        let mut child = Command::new("swift")
            .arg("-")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn swift: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Swift script failed (code {:?}): stderr='{}' stdout='{}'",
                output.status.code(),
                stderr.trim(),
                stdout.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute an AppleScript snippet via osascript
    pub fn run_osascript(script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute osascript: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("osascript failed: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Find PID of running application matching bundle ID, path, or name
    pub fn find_app_pid(&self, app_target: &str) -> Result<Option<i32>> {
        const SCRIPT: &str = r#"
import AppKit
import Foundation

guard CommandLine.arguments.count > 1 else { exit(1) }
let appParam = CommandLine.arguments[1]

for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    let name = app.localizedName ?? ""
    if path.caseInsensitiveCompare(appParam) == .orderedSame || 
       bId.caseInsensitiveCompare(appParam) == .orderedSame || 
       name.caseInsensitiveCompare(appParam) == .orderedSame || 
       appParam.contains(name) {
        print(app.processIdentifier)
        exit(0)
    }
}
exit(1)
"#;
        match Self::run_swift(SCRIPT, &[app_target]) {
            Ok(out) => {
                if let Ok(pid) = out.trim().parse::<i32>() {
                    Ok(Some(pid))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Query current window position and size for an application
    pub fn get_window_bounds(&self, app_target: &str) -> Option<WindowBounds> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count > 1 else { exit(1) }
let appParam = CommandLine.arguments[1]

var targetApp: NSRunningApplication?
for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    let name = app.localizedName ?? ""
    if path.caseInsensitiveCompare(appParam) == .orderedSame || 
       bId.caseInsensitiveCompare(appParam) == .orderedSame || 
       name.caseInsensitiveCompare(appParam) == .orderedSame || 
       appParam.contains(name) {
        targetApp = app
        break
    }
}
guard let app = targetApp else { exit(1) }
let appElement = AXUIElementCreateApplication(app.processIdentifier)
var winVal: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
   let windows = winVal as? [AXUIElement], let win = windows.first {
    var posVal: CFTypeRef?
    var sizeVal: CFTypeRef?
    if AXUIElementCopyAttributeValue(win, kAXPositionAttribute as CFString, &posVal) == .success,
       AXUIElementCopyAttributeValue(win, kAXSizeAttribute as CFString, &sizeVal) == .success {
        var point = CGPoint.zero
        var size = CGSize.zero
        AXValueGetValue(posVal as! AXValue, .cgPoint, &point)
        AXValueGetValue(sizeVal as! AXValue, .cgSize, &size)
        print("\(point.x),\(point.y),\(size.width),\(size.height)")
        exit(0)
    }
}
exit(1)
"#;
        if let Ok(output) = Self::run_swift(SCRIPT, &[app_target]) {
            let parts: Vec<&str> = output.trim().split(',').collect();
            if parts.len() == 4 {
                if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    return Some(WindowBounds {
                        x,
                        y,
                        width: w,
                        height: h,
                    });
                }
            }
        }
        None
    }

    /// Query Window ID and bounds (win_id, x, y, width, height) via CoreGraphics window server
    pub fn get_app_window_info(app_target: &str) -> Option<(u32, f64, f64, f64, f64)> {
        const SCRIPT: &str = r#"
import AppKit
import CoreGraphics
import Foundation

guard CommandLine.arguments.count > 1 else { exit(1) }
let appParam = CommandLine.arguments[1]

var targetPid: pid_t = 0
for app in NSWorkspace.shared.runningApplications {
    let name = app.localizedName ?? ""
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    if path.caseInsensitiveCompare(appParam) == .orderedSame ||
       bId.caseInsensitiveCompare(appParam) == .orderedSame ||
       name.caseInsensitiveCompare(appParam) == .orderedSame ||
       appParam.contains(name) || name.contains(appParam) {
        targetPid = app.processIdentifier
        break
    }
}
guard targetPid > 0 else { exit(1) }

if let windowList = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] {
    for win in windowList {
        let pid = win[kCGWindowOwnerPID as String] as? pid_t ?? 0
        let layer = win[kCGWindowLayer as String] as? Int ?? -1
        let boundsDict = win[kCGWindowBounds as String] as? [String: Any] ?? [:]
        let x = boundsDict["X"] as? CGFloat ?? 0
        let y = boundsDict["Y"] as? CGFloat ?? 0
        let width = boundsDict["Width"] as? CGFloat ?? 0
        let height = boundsDict["Height"] as? CGFloat ?? 0
        let winId = win[kCGWindowNumber as String] as? CGWindowID ?? 0

        if pid == targetPid && layer == 0 && width > 50 && height > 50 {
            print("\(winId),\(Int(x)),\(Int(y)),\(Int(width)),\(Int(height))")
            exit(0)
        }
    }
}
exit(1)
"#;
        if let Ok(out) = Self::run_swift(SCRIPT, &[app_target]) {
            let parts: Vec<&str> = out.trim().split(',').collect();
            if parts.len() == 5 {
                if let (Ok(win_id), Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                    parts[4].parse::<f64>(),
                ) {
                    return Some((win_id, x, y, w, h));
                }
            }
        }
        None
    }

    /// Restore window position and size for an application
    pub fn set_window_bounds(&self, app_target: &str, bounds: WindowBounds) -> Result<()> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count >= 6 else { exit(1) }
let appParam = CommandLine.arguments[1]
let x = Double(CommandLine.arguments[2]) ?? 0
let y = Double(CommandLine.arguments[3]) ?? 0
let w = Double(CommandLine.arguments[4]) ?? 0
let h = Double(CommandLine.arguments[5]) ?? 0

var targetApp: NSRunningApplication?
for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    let name = app.localizedName ?? ""
    if path.caseInsensitiveCompare(appParam) == .orderedSame || 
       bId.caseInsensitiveCompare(appParam) == .orderedSame || 
       name.caseInsensitiveCompare(appParam) == .orderedSame || 
       appParam.contains(name) {
        targetApp = app
        break
    }
}
guard let app = targetApp else { exit(1) }
let appElement = AXUIElementCreateApplication(app.processIdentifier)
var winVal: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
   let windows = winVal as? [AXUIElement], let win = windows.first {
    var point = CGPoint(x: x, y: y)
    var size = CGSize(width: w, height: h)
    if let posVal = AXValueCreate(.cgPoint, &point),
       let sizeVal = AXValueCreate(.cgSize, &size) {
        _ = AXUIElementSetAttributeValue(win, kAXPositionAttribute as CFString, posVal)
        _ = AXUIElementSetAttributeValue(win, kAXSizeAttribute as CFString, sizeVal)
        exit(0)
    }
}
exit(1)
"#;
        let x_str = bounds.x.to_string();
        let y_str = bounds.y.to_string();
        let w_str = bounds.width.to_string();
        let h_str = bounds.height.to_string();
        let _ = Self::run_swift(SCRIPT, &[app_target, &x_str, &y_str, &w_str, &h_str]);
        Ok(())
    }

    /// Resize application window to specified width and height
    pub fn set_window_size(&self, app_target: &str, width: u32, height: u32) -> Result<()> {
        const SCRIPT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count >= 4 else { exit(1) }
let appParam = CommandLine.arguments[1]
let w = Double(CommandLine.arguments[2]) ?? 0
let h = Double(CommandLine.arguments[3]) ?? 0

var targetApp: NSRunningApplication?
for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    let name = app.localizedName ?? ""
    if path.caseInsensitiveCompare(appParam) == .orderedSame || 
       bId.caseInsensitiveCompare(appParam) == .orderedSame || 
       name.caseInsensitiveCompare(appParam) == .orderedSame || 
       appParam.contains(name) {
        targetApp = app
        break
    }
}
guard let app = targetApp else { exit(1) }
let appElement = AXUIElementCreateApplication(app.processIdentifier)
var winVal: CFTypeRef?
if AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &winVal) == .success,
   let windows = winVal as? [AXUIElement], let win = windows.first {
    var size = CGSize(width: w, height: h)
    if let sizeVal = AXValueCreate(.cgSize, &size) {
        let res = AXUIElementSetAttributeValue(win, kAXSizeAttribute as CFString, sizeVal)
        if res == .success {
            print("RESIZE_SUCCESS")
            exit(0)
        }
    }
}
exit(1)
"#;
        let w_str = width.to_string();
        let h_str = height.to_string();
        let _ = Self::run_swift(SCRIPT, &[app_target, &w_str, &h_str]);
        Ok(())
    }

    /// Launch an application in background without stealing active focus
    pub fn launch_app(&self, app_target: &str, saved_bounds: Option<WindowBounds>) -> Result<()> {
        let is_running = self.find_app_pid(app_target)?.is_some();
        if !is_running {
            if app_target.ends_with(".app") || app_target.starts_with('/') {
                let status = Command::new("open").arg("-g").arg(app_target).status()?;
                if !status.success() {
                    let _ = Command::new("open").arg(app_target).status();
                }
            } else {
                let status = Command::new("open")
                    .arg("-g")
                    .arg("-b")
                    .arg(app_target)
                    .status()?;
                if !status.success() {
                    let _ = Command::new("open").arg("-b").arg(app_target).status();
                }
            }
        }

        // Wait up to 3s for app process
        for _ in 0..15 {
            if self.find_app_pid(app_target)?.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        // Restore window bounds if previously captured
        if let Some(bounds) = saved_bounds {
            thread::sleep(Duration::from_millis(300));
            let _ = self.set_window_bounds(app_target, bounds);
        }

        Ok(())
    }

    /// Terminate an application
    pub fn stop_app(&self, app_target: &str) -> Result<()> {
        if app_target.ends_with(".app") || app_target.starts_with('/') {
            let app_name = Path::new(app_target)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(app_target);
            let _ = Command::new("killall").arg(app_name).status();
        } else {
            let _ = Self::run_osascript(&format!(
                "tell application id \"{}\" to quit",
                app_target
            ));
            let _ = Command::new("killall").arg(app_target).status();
        }
        Ok(())
    }

    /// Perform coordinate click with instant cursor restoration
    pub fn click_at(x: i32, y: i32, restore_cursor: bool) -> Result<()> {
        const SCRIPT: &str = r#"
import CoreGraphics
import Foundation

guard CommandLine.arguments.count >= 4 else { exit(1) }
let x = Double(CommandLine.arguments[1]) ?? 0
let y = Double(CommandLine.arguments[2]) ?? 0
let restore = CommandLine.arguments[3] == "true"

let clickPoint = CGPoint(x: x, y: y)
let origPos = CGEvent(source: nil)?.location ?? clickPoint
let src = CGEventSource(stateID: .hidSystemState)

CGWarpMouseCursorPosition(clickPoint)
Thread.sleep(forTimeInterval: 0.02)

if let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: clickPoint, mouseButton: .left),
   let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: clickPoint, mouseButton: .left) {
    down.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.05)
    up.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.03)
}
if restore {
    CGWarpMouseCursorPosition(origPos)
}
exit(0)
"#;
        let x_str = x.to_string();
        let y_str = y.to_string();
        let r_str = restore_cursor.to_string();
        Self::run_swift(SCRIPT, &[&x_str, &y_str, &r_str])?;
        Ok(())
    }

    /// Perform coordinate double click
    pub fn double_click_at(x: i32, y: i32) -> Result<()> {
        Self::click_at(x, y, true)?;
        thread::sleep(Duration::from_millis(100));
        Self::click_at(x, y, true)
    }

    /// Perform coordinate right click
    pub fn right_click_at(x: i32, y: i32) -> Result<()> {
        const SCRIPT: &str = r#"
import CoreGraphics
import Foundation

guard CommandLine.arguments.count >= 3 else { exit(1) }
let x = Double(CommandLine.arguments[1]) ?? 0
let y = Double(CommandLine.arguments[2]) ?? 0

let clickPoint = CGPoint(x: x, y: y)
let origPos = CGEvent(source: nil)?.location ?? clickPoint
let src = CGEventSource(stateID: .hidSystemState)

if let down = CGEvent(mouseEventSource: src, mouseType: .rightMouseDown, mouseCursorPosition: clickPoint, mouseButton: .right),
   let up = CGEvent(mouseEventSource: src, mouseType: .rightMouseUp, mouseCursorPosition: clickPoint, mouseButton: .right) {
    down.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.05)
    up.post(tap: .cghidEventTap)
}
CGWarpMouseCursorPosition(origPos)
exit(0)
"#;
        let x_str = x.to_string();
        let y_str = y.to_string();
        Self::run_swift(SCRIPT, &[&x_str, &y_str])?;
        Ok(())
    }

    /// Perform long press at coordinate
    pub fn long_press_at(x: i32, y: i32, duration_ms: u64) -> Result<()> {
        const SCRIPT: &str = r#"
import CoreGraphics
import Foundation

guard CommandLine.arguments.count >= 4 else { exit(1) }
let x = Double(CommandLine.arguments[1]) ?? 0
let y = Double(CommandLine.arguments[2]) ?? 0
let duration = Double(CommandLine.arguments[3]) ?? 1.0

let clickPoint = CGPoint(x: x, y: y)
let origPos = CGEvent(source: nil)?.location ?? clickPoint
let src = CGEventSource(stateID: .hidSystemState)

if let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: clickPoint, mouseButton: .left),
   let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: clickPoint, mouseButton: .left) {
    down.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: duration)
    up.post(tap: .cghidEventTap)
}
CGWarpMouseCursorPosition(origPos)
exit(0)
"#;
        let x_str = x.to_string();
        let y_str = y.to_string();
        let d_str = (duration_ms as f64 / 1000.0).to_string();
        Self::run_swift(SCRIPT, &[&x_str, &y_str, &d_str])?;
        Ok(())
    }

    /// Perform drag / swipe from one coordinate to another
    pub fn swipe(&self, from: (i32, i32), to: (i32, i32), duration_ms: u64) -> Result<()> {
        const SCRIPT: &str = r#"
import CoreGraphics
import Foundation

guard CommandLine.arguments.count >= 6 else { exit(1) }
let x1 = Double(CommandLine.arguments[1]) ?? 0
let y1 = Double(CommandLine.arguments[2]) ?? 0
let x2 = Double(CommandLine.arguments[3]) ?? 0
let y2 = Double(CommandLine.arguments[4]) ?? 0
let duration = Double(CommandLine.arguments[5]) ?? 0.3

let startPoint = CGPoint(x: x1, y: y1)
let endPoint = CGPoint(x: x2, y: y2)
let origPos = CGEvent(source: nil)?.location ?? startPoint
let src = CGEventSource(stateID: .hidSystemState)

if let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: startPoint, mouseButton: .left) {
    down.post(tap: .cghidEventTap)
    let steps = 20
    let stepDuration = duration / Double(steps)
    for i in 1...steps {
        let t = Double(i) / Double(steps)
        let cx = x1 + (x2 - x1) * t
        let cy = y1 + (y2 - y1) * t
        let cp = CGPoint(x: cx, y: cy)
        if let drag = CGEvent(mouseEventSource: src, mouseType: .leftMouseDragged, mouseCursorPosition: cp, mouseButton: .left) {
            drag.post(tap: .cghidEventTap)
        }
        Thread.sleep(forTimeInterval: stepDuration)
    }
    if let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: endPoint, mouseButton: .left) {
        up.post(tap: .cghidEventTap)
    }
}
CGWarpMouseCursorPosition(origPos)
exit(0)
"#;
        let x1 = from.0.to_string();
        let y1 = from.1.to_string();
        let x2 = to.0.to_string();
        let y2 = to.1.to_string();
        let d = (duration_ms as f64 / 1000.0).to_string();
        Self::run_swift(SCRIPT, &[&x1, &y1, &x2, &y2, &d])?;
        Ok(())
    }

    /// Dispatch unicode text keystrokes directly to process PID
    pub fn post_key_events(&self, pid: i32, text: &str) -> Result<()> {
        const SCRIPT: &str = r#"
import CoreGraphics
import Foundation

guard CommandLine.arguments.count >= 3 else { exit(1) }
let pid = pid_t(CommandLine.arguments[1]) ?? 0
let text = CommandLine.arguments[2]

let src = CGEventSource(stateID: .hidSystemState)
for char in text.utf16 {
    var code = char
    if let down = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: true),
       let up = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: false) {
        down.keyboardSetUnicodeString(stringLength: 1, unicodeString: &code)
        up.keyboardSetUnicodeString(stringLength: 1, unicodeString: &code)
        down.postToPid(pid)
        up.postToPid(pid)
        Thread.sleep(forTimeInterval: 0.01)
    }
}
exit(0)
"#;
        let pid_str = pid.to_string();
        Self::run_swift(SCRIPT, &[&pid_str, text])?;
        Ok(())
    }

    /// Dispatch a shortcut / key code
    pub fn post_key(&self, key_name: &str) -> Result<()> {
        match key_name.to_lowercase().as_str() {
            "enter" | "return" => Self::run_osascript("tell application \"System Events\" to key code 36")?,
            "tab" => Self::run_osascript("tell application \"System Events\" to key code 48")?,
            "escape" | "esc" => Self::run_osascript("tell application \"System Events\" to key code 53")?,
            "space" => Self::run_osascript("tell application \"System Events\" to key code 49")?,
            "delete" | "backspace" => Self::run_osascript("tell application \"System Events\" to key code 51")?,
            "up" => Self::run_osascript("tell application \"System Events\" to key code 126")?,
            "down" => Self::run_osascript("tell application \"System Events\" to key code 125")?,
            "left" => Self::run_osascript("tell application \"System Events\" to key code 123")?,
            "right" => Self::run_osascript("tell application \"System Events\" to key code 124")?,
            other => Self::run_osascript(&format!(
                "tell application \"System Events\" to keystroke \"{}\"",
                other.replace('"', "\\\"")
            ))?,
        };
        Ok(())
    }

    /// Capture screenshot of window or entire screen
    pub fn capture_screenshot(&self, path: &str) -> Result<()> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let status = Command::new("screencapture")
            .arg("-x")
            .arg(path)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to run screencapture: {}", e))?;

        if !status.success() {
            anyhow::bail!("screencapture failed with status {:?}", status);
        }
        Ok(())
    }

    /// List running and installed macOS applications with search support
    pub fn list_running_apps() -> Result<Vec<String>> {
        const SCRIPT: &str = r#"
import AppKit
import Foundation

var seen = Set<String>()
var apps: [String] = []

// 1. Running GUI Apps first
for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let name = app.localizedName ?? ""
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    if !path.isEmpty && !seen.contains(path) {
        seen.insert(path)
        if !bId.isEmpty {
            apps.append("\(name) [Running] | \(bId) | \(path)")
        } else {
            apps.append("\(name) [Running] | \(path) | \(path)")
        }
    }
}

// 2. Installed Apps in standard directories
let dirs = [
    "/Applications",
    "/System/Applications",
    "/System/Applications/Utilities",
    "\(NSHomeDirectory())/Applications"
]

let fm = FileManager.default
for dir in dirs {
    guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { continue }
    for item in contents where item.hasSuffix(".app") {
        let fullPath = (dir as NSString).appendingPathComponent(item)
        if !seen.contains(fullPath) {
            seen.insert(fullPath)
            let bundle = Bundle(path: fullPath)
            let bId = bundle?.bundleIdentifier ?? ""
            let name = (item as NSString).deletingPathExtension
            if !bId.isEmpty {
                apps.append("\(name) | \(bId) | \(fullPath)")
            } else {
                apps.append("\(name) | \(fullPath) | \(fullPath)")
            }
        }
    }
}

for app in apps {
    print(app)
}
"#;
        let out = Self::run_swift(SCRIPT, &[])?;
        let apps: Vec<String> = out
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(apps)
    }

    /// Get main display resolution (width, height)
    pub fn get_main_display_size() -> Result<(u32, u32)> {
        const SCRIPT: &str = r#"
import CoreGraphics
let bounds = CGDisplayBounds(CGMainDisplayID())
print("\(Int(bounds.width))x\(Int(bounds.height))")
"#;
        let out = Self::run_swift(SCRIPT, &[])?;
        let trimmed = out.trim();
        if let Some((w, h)) = trimmed.split_once('x') {
            if let (Ok(width), Ok(height)) = (w.parse::<u32>(), h.parse::<u32>()) {
                return Ok((width, height));
            }
        }
        Ok((1920, 1080))
    }

    /// Open URL in default browser
    pub fn open_url(&self, url: &str) -> Result<()> {
        Command::new("open")
            .arg(url)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to open URL: {}", e))?;
        Ok(())
    }

    /// Extract 32x32 PNG icon for a macOS application path
    pub fn get_app_icon_png(app_path: &str) -> Option<Vec<u8>> {
        const SCRIPT: &str = r#"
import AppKit
import Foundation

guard CommandLine.arguments.count > 1 else { exit(1) }
let path = CommandLine.arguments[1]

let icon = NSWorkspace.shared.icon(forFile: path)
let targetSize = NSSize(width: 32, height: 32)
let newImg = NSImage(size: targetSize)
newImg.lockFocus()
icon.draw(in: NSRect(origin: .zero, size: targetSize), from: .zero, operation: .copy, fraction: 1.0)
newImg.unlockFocus()

if let tiff = newImg.tiffRepresentation,
   let rep = NSBitmapImageRep(data: tiff),
   let png = rep.representation(using: .png, properties: [:]) {
    FileHandle.standardOutput.write(png)
    exit(0)
}
exit(1)
"#;
        let output = Command::new("swift")
            .arg("-e")
            .arg(SCRIPT)
            .arg(app_path)
            .output()
            .ok()?;
        if output.status.success() && !output.stdout.is_empty() {
            Some(output.stdout)
        } else {
            None
        }
    }
}
