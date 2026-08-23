package dev.lm.tester.network

import android.view.KeyEvent
import android.view.MotionEvent
import dev.lm.tester.control.ClipboardController
import dev.lm.tester.input.InputController
import dev.lm.tester.input.TouchScaler
import dev.lm.tester.util.PerformanceMonitor
import dev.lm.tester.util.ViewHierarchyDumper
import org.json.JSONObject
import java.io.InputStream

/**
 * CommandHandler processes incoming commands from the host.
 * Uses a simple JSON protocol for command dispatch.
 */
object CommandHandler {

    fun handleCommand(reader: java.io.BufferedReader): String? {
        val line = try {
            reader.readLine()
        } catch (e: Exception) {
            null
        } ?: return null
        
        return try {
            val json = JSONObject(line)
            val cmd = json.optString("cmd", "")
            
            when (cmd) {
                // Real-time touch event
                "touch" -> {
                    val action = when(json.getString("action")) {
                        "down" -> MotionEvent.ACTION_DOWN
                        "move" -> MotionEvent.ACTION_MOVE
                        else -> MotionEvent.ACTION_UP
                    }
                    val rawX = json.getDouble("x").toFloat()
                    val rawY = json.getDouble("y").toFloat()
                    val (x, y) = TouchScaler.transform(rawX, rawY)
                    val success = InputController.injectTouch(action, x, y)
                    """{"cmd": "touch", "success": $success}"""
                }
                // Keycode injection (down/up)
                "keycode" -> {
                    val action = if (json.getString("action") == "down") 
                        KeyEvent.ACTION_DOWN else KeyEvent.ACTION_UP
                    val keyCode = json.getInt("keyCode")
                    val metaState = json.optInt("metaState", 0)
                    val success = InputController.injectKey(keyCode, action, metaState)
                    """{"cmd": "keycode", "success": $success}"""
                }
                // Text injection: use KeyCharacterMap
                // Run on separate thread to avoid blocking command handler
                "text" -> {
                    val text = json.getString("text")
                    Thread {
                        InputController.injectText(text)
                    }.start()
                    """{"cmd": "text", "success": true}"""
                }
                // Set the focused input field's text directly via ACTION_SET_TEXT - no IME
                // switching, fully Unicode-safe. Synchronous (unlike "text" above) so the
                // caller gets a reliable success/failure result to decide on fallback.
                "set_text" -> {
                    val text = json.getString("text")
                    val success = dev.lm.tester.util.UiAutomationBridge.setFocusedText(text)
                    """{"cmd": "set_text", "success": $success}"""
                }
                // Clipboard operations
                "set_clipboard" -> {
                    val text = json.getString("text")
                    val paste = json.optBoolean("paste", false)
                    Thread {
                        ClipboardController.setTextAndPaste(text, paste)
                    }.start()
                    """{"cmd": "set_clipboard", "success": true}"""
                }
                "get_clipboard" -> {
                    val copy = json.optBoolean("copy", false)
                    val text = if (copy) {
                        ClipboardController.copyAndGetText() ?: ""
                    } else {
                        ClipboardController.getText() ?: ""
                    }
                    val escaped = text.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
                    """{"cmd": "get_clipboard", "text": "$escaped"}"""
                }
                // Legacy commands (kept for compatibility)
                "tap" -> {
                    val rawX = json.getDouble("x").toFloat()
                    val rawY = json.getDouble("y").toFloat()
                    val (x, y) = TouchScaler.transform(rawX, rawY)
                    val success = InputController.tap(x, y)
                    """{"cmd": "tap", "success": $success}"""
                }
                "swipe" -> {
                    val rawX1 = json.getDouble("x1").toFloat()
                    val rawY1 = json.getDouble("y1").toFloat()
                    val rawX2 = json.getDouble("x2").toFloat()
                    val rawY2 = json.getDouble("y2").toFloat()
                    val (x1, y1) = TouchScaler.transform(rawX1, rawY1)
                    val (x2, y2) = TouchScaler.transform(rawX2, rawY2)
                    val duration = json.optLong("duration", 300)
                    val success = InputController.swipe(x1, y1, x2, y2, duration)
                    """{"cmd": "swipe", "success": $success}"""
                }
                "long_press" -> {
                    val rawX = json.getDouble("x").toFloat()
                    val rawY = json.getDouble("y").toFloat()
                    val (x, y) = TouchScaler.transform(rawX, rawY)
                    val duration = json.optLong("duration", 500)
                    val success = InputController.longPress(x, y, duration)
                    """{"cmd": "long_press", "success": $success}"""
                }
                "key" -> {
                    val keyCode = json.getInt("keyCode")
                    val success = InputController.pressKey(keyCode)
                    """{"cmd": "key", "success": $success}"""
                }
                "hierarchy" -> {
                    // Fast path: reuse a persistent UiAutomation connection (no per-call
                    // process spawn). Falls back to the slow `uiautomator dump` shell-out
                    // if the fast path fails for any reason (e.g. first call, or the
                    // connection went stale after a system_server restart).
                    val fastXml = dev.lm.tester.util.UiAutomationBridge.dumpXml()
                    val hierarchy = if (fastXml != null) {
                        org.json.JSONObject().apply {
                            put("format", "xml")
                            put("data", fastXml)
                        }.toString()
                    } else {
                        ViewHierarchyDumper.dump()
                    }
                    """{"cmd": "hierarchy", "data": $hierarchy}"""
                }
                "wait_idle" -> {
                    // Waits for the accessibility event stream to settle (same primitive
                    // UiAutomator uses internally) instead of polling `dumpsys window` over
                    // adb. `success:false` tells the caller to fall back to that poll.
                    val idleMs = json.optLong("idleMs", 100L)
                    val timeoutMs = json.optLong("timeoutMs", 200L)
                    val success = dev.lm.tester.util.UiAutomationBridge.waitForIdle(idleMs, timeoutMs)
                    """{"cmd": "wait_idle", "success": $success}"""
                }
                "screenshot" -> {
                    // Fast path: UiAutomation.takeScreenshot() over the already-open
                    // connection instead of spawning `adb exec-out screencap`.
                    val png = dev.lm.tester.util.UiAutomationBridge.captureScreenshotPngBase64()
                    if (png != null) {
                        """{"cmd": "screenshot", "success": true, "data": "$png"}"""
                    } else {
                        """{"cmd": "screenshot", "success": false}"""
                    }
                }
                "is_keyboard_visible" -> {
                    val visible = dev.lm.tester.util.UiAutomationBridge.isKeyboardVisible()
                    if (visible != null) {
                        """{"cmd": "is_keyboard_visible", "success": true, "visible": $visible}"""
                    } else {
                        """{"cmd": "is_keyboard_visible", "success": false}"""
                    }
                }
                "erase_text" -> {
                    // ACTION_SET_TEXT fully replaces a field's content, so clearing it is
                    // just setting it to empty - reuses the same retry/verify-hardened
                    // path as "set_text" rather than a separate implementation.
                    val success = dev.lm.tester.util.UiAutomationBridge.setFocusedText("")
                    """{"cmd": "erase_text", "success": $success}"""
                }
                "stats" -> {
                    val stats = PerformanceMonitor.getStats()
                    """{"cmd": "stats", "data": $stats}"""
                }
                "set_screen_power_mode" -> {
                    val mode = json.getInt("mode") // 0=OFF, 2=NORMAL
                    val success = dev.lm.tester.display.DisplayControl.setPowerMode(mode)
                    """{"cmd": "set_screen_power_mode", "success": $success}"""
                }
                "start_mock_location" -> {
                    dev.lm.tester.input.LocationController.startMocking()
                    """{"cmd": "start_mock_location", "success": true}"""
                }
                "stop_mock_location" -> {
                    dev.lm.tester.input.LocationController.stopMocking()
                    """{"cmd": "stop_mock_location", "success": true}"""
                }
                "set_location" -> {
                    val lat = json.getDouble("lat")
                    val lon = json.getDouble("lon")
                    val alt = json.optDouble("alt", 0.0)
                    val bearing = json.optDouble("bearing", 0.0).toFloat()
                    val speed = json.optDouble("speed", 0.0).toFloat()
                    dev.lm.tester.input.LocationController.updateLocation(lat, lon, alt, bearing, speed)
                    """{"cmd": "set_location", "success": true}"""
                }
                else -> """{"error": "Unknown command: $cmd"}"""
            }
        } catch (e: Exception) {
            """{"error": "${e.message}"}"""
        }
    }
}
