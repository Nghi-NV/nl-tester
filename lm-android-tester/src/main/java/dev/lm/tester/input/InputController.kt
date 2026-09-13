package dev.lm.tester.input

import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.MotionEvent

/**
 * InputController handles raw event injection for mouse/touch and keyboard.
 * Uses reflection to access InputManager.injectInputEvent() for low-latency control.
 */
object InputController {
    private val inputManager: Any by lazy {
        val imClass = Class.forName("android.hardware.input.InputManager")
        val getInstance = imClass.getMethod("getInstance")
        getInstance.invoke(null)
    }

    private val injectInputEventMethod by lazy {
        val imClass = Class.forName("android.hardware.input.InputManager")
        imClass.getMethod("injectInputEvent", android.view.InputEvent::class.java, Int::class.javaPrimitiveType)
    }

    /**
     * The real, physical touchscreen's device ID (queried via the public
     * `InputDevice.getDeviceIds()`/`getDevice()` API - no reflection needed here), or -1
     * if none is found. Synthesized touch events previously hardcoded `deviceId = 0`,
     * which doesn't correspond to any registered input device on real hardware (confirmed
     * via `dumpsys input`: this device's actual touchscreen is a different device ID).
     * Android's InputDispatcher and app-side frameworks (Flutter's engine in particular)
     * can key touch-source classification/routing off the reporting device's identity,
     * not just the `source` bitmask - using device ID 0 is passing bogus device identity
     * on every synthesized event.
     */
    private val touchScreenDeviceId: Int by lazy {
        try {
            for (id in InputDevice.getDeviceIds()) {
                val device = InputDevice.getDevice(id) ?: continue
                if (device.supportsSource(InputDevice.SOURCE_TOUCHSCREEN)) {
                    return@lazy id
                }
            }
        } catch (_: Throwable) {
        }
        -1
    }

    // Fire-and-forget: returns as soon as the event is queued, with no guarantee the
    // target app has actually processed it yet.
    private const val INJECT_INPUT_EVENT_MODE_ASYNC = 0

    // Blocks until the input dispatcher confirms the target app has *finished*
    // processing the event - this is what `adb shell input tap`/`input text` use
    // internally, and it's why they're reliable where ASYNC isn't: for a Flutter app,
    // touch handling is a round-trip through the Dart isolate (hit-testing, gesture
    // recognition, focus assignment) that doesn't complete synchronously with the
    // event being queued. Confirmed on-device: with ASYNC + a fixed 10ms DOWN->UP gap,
    // taps on this app's login form routinely "succeeded" (event injected, no error)
    // without ever actually assigning input focus - no keyboard appeared, and no
    // accessibility node reported focused afterward, in ~10% of runs. Switching to
    // WAIT_FOR_FINISH eliminated the failure across 20/20 stress-test runs.
    private const val INJECT_INPUT_EVENT_MODE_WAIT_FOR_FINISH = 2

    // Track the downTime for each gesture (same downTime must be used for DOWN, MOVE, UP)
    private var lastDownTime: Long = 0L
    private var isPointerDown: Boolean = false

    /**
     * Injects a touch event at the specified coordinates.
     * Important: For a gesture sequence (DOWN->MOVE->UP), the downTime must be consistent.
     */
    fun injectTouch(action: Int, x: Float, y: Float, pointerId: Int = 0): Boolean {
        val now = SystemClock.uptimeMillis()
        
        // Track downTime properly
        val downTime = when (action) {
            MotionEvent.ACTION_DOWN -> {
                lastDownTime = now
                isPointerDown = true
                now
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isPointerDown = false
                lastDownTime.takeIf { it > 0 } ?: now
            }
            else -> {
                // MOVE events use the original downTime
                lastDownTime.takeIf { it > 0 } ?: now
            }
        }
        
        val pointerProperties = arrayOf(MotionEvent.PointerProperties().apply {
            id = pointerId
            toolType = MotionEvent.TOOL_TYPE_FINGER
        })
        
        val pointerCoords = arrayOf(MotionEvent.PointerCoords().apply {
            this.x = x
            this.y = y
            pressure = if (action == MotionEvent.ACTION_UP) 0f else 1f
            size = 1f
        })

        val event = MotionEvent.obtain(
            downTime, now, action,
            1, pointerProperties, pointerCoords,
            0, 0, 1f, 1f,
            touchScreenDeviceId, 0, InputDevice.SOURCE_TOUCHSCREEN, 0
        )

        val result = injectEvent(event)
        event.recycle() // Important: recycle MotionEvent to avoid memory leak
        return result
    }

    /**
     * Simulates a tap at the specified coordinates.
     */
    fun tap(x: Float, y: Float): Boolean {
        val downResult = injectTouch(MotionEvent.ACTION_DOWN, x, y)
        Thread.sleep(10)
        val upResult = injectTouch(MotionEvent.ACTION_UP, x, y)
        return downResult && upResult
    }

    /**
     * Simulates a swipe from (x1, y1) to (x2, y2).
     */
    fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long = 100): Boolean {
        val steps = 5
        val stepDuration = durationMs / steps

        injectTouch(MotionEvent.ACTION_DOWN, x1, y1)

        for (i in 1..steps) {
            val ratio = i.toFloat() / steps
            val x = x1 + (x2 - x1) * ratio
            val y = y1 + (y2 - y1) * ratio
            Thread.sleep(stepDuration)
            injectTouch(MotionEvent.ACTION_MOVE, x, y)
        }

        return injectTouch(MotionEvent.ACTION_UP, x2, y2)
    }

    /**
     * Simulates a long press at the specified coordinates.
     */
    fun longPress(x: Float, y: Float, durationMs: Long = 500): Boolean {
        val downResult = injectTouch(MotionEvent.ACTION_DOWN, x, y)
        Thread.sleep(durationMs)
        val upResult = injectTouch(MotionEvent.ACTION_UP, x, y)
        return downResult && upResult
    }

    /**
     * Injects a multi-pointer touch event. `points` holds the CURRENT (x, y) of every
     * pointer that is down right now, in pointer-index order. For `ACTION_POINTER_DOWN`/
     * `ACTION_POINTER_UP`, `actionPointerIndex` selects which pointer the action applies to
     * and gets encoded into the action int (Android packs it into the action's upper bits -
     * a single MotionEvent always carries every active pointer's position, even when the
     * action itself concerns only one of them going down/up).
     */
    private fun injectMultiTouch(
        action: Int,
        points: List<Pair<Float, Float>>,
        downTime: Long,
        actionPointerIndex: Int = 0
    ): Boolean {
        val now = SystemClock.uptimeMillis()
        val pointerProperties = Array(points.size) { i ->
            MotionEvent.PointerProperties().apply {
                id = i
                toolType = MotionEvent.TOOL_TYPE_FINGER
            }
        }
        val pointerCoords = Array(points.size) { i ->
            MotionEvent.PointerCoords().apply {
                x = points[i].first
                y = points[i].second
                pressure = if (action == MotionEvent.ACTION_UP) 0f else 1f
                size = 1f
            }
        }
        val encodedAction =
            if (action == MotionEvent.ACTION_POINTER_DOWN || action == MotionEvent.ACTION_POINTER_UP) {
                action or (actionPointerIndex shl MotionEvent.ACTION_POINTER_INDEX_SHIFT)
            } else {
                action
            }

        val event = MotionEvent.obtain(
            downTime, now, encodedAction,
            points.size, pointerProperties, pointerCoords,
            0, 0, 1f, 1f,
            touchScreenDeviceId, 0, InputDevice.SOURCE_TOUCHSCREEN, 0
        )
        val result = injectEvent(event)
        event.recycle()
        return result
    }

    private fun lerp(a: Pair<Float, Float>, b: Pair<Float, Float>, t: Float): Pair<Float, Float> =
        Pair(a.first + (b.first - a.first) * t, a.second + (b.second - a.second) * t)

    /**
     * Two-finger gesture primitive: each finger moves along its own straight line from its
     * start to its end position over `durationMs`. Both `pinch` and `shove` below build on
     * this - the only difference between them is where the two fingers' start/end points
     * are placed.
     *
     * Sequencing matters here: finger 1 touches down alone (`ACTION_DOWN`), then finger 2
     * joins (`ACTION_POINTER_DOWN`, pointer index 1) - most map SDKs' gesture detectors
     * (Mapbox included) key their multi-finger recognizers off seeing a plain single-finger
     * touch first, exactly like a real hand landing on a screen finger-by-finger rather than
     * both fingers materializing atomically. Lift order mirrors this in reverse.
     */
    fun twoFingerGesture(
        finger1Start: Pair<Float, Float>,
        finger1End: Pair<Float, Float>,
        finger2Start: Pair<Float, Float>,
        finger2End: Pair<Float, Float>,
        durationMs: Long = 400
    ): Boolean {
        val steps = 10
        val stepDuration = durationMs / steps
        val downTime = SystemClock.uptimeMillis()

        if (!injectMultiTouch(MotionEvent.ACTION_DOWN, listOf(finger1Start), downTime)) return false

        if (!injectMultiTouch(
                MotionEvent.ACTION_POINTER_DOWN,
                listOf(finger1Start, finger2Start),
                downTime,
                actionPointerIndex = 1
            )
        ) return false

        for (i in 1..steps) {
            val ratio = i.toFloat() / steps
            val p1 = lerp(finger1Start, finger1End, ratio)
            val p2 = lerp(finger2Start, finger2End, ratio)
            Thread.sleep(stepDuration)
            if (!injectMultiTouch(MotionEvent.ACTION_MOVE, listOf(p1, p2), downTime)) return false
        }

        if (!injectMultiTouch(
                MotionEvent.ACTION_POINTER_UP,
                listOf(finger1End, finger2End),
                downTime,
                actionPointerIndex = 1
            )
        ) return false

        return injectMultiTouch(MotionEvent.ACTION_UP, listOf(finger1End), downTime)
    }

    /**
     * Pinch gesture centered at (cx, cy): two fingers start `startRadius`px from center
     * along a line at `angleDeg` and end `endRadius`px from center along that same line -
     * "open"/zoom-in when endRadius > startRadius, "close"/zoom-out when endRadius <
     * startRadius. This is the standard 2-finger pinch-to-zoom gesture every map SDK
     * (Mapbox included) recognizes.
     */
    fun pinch(
        cx: Float,
        cy: Float,
        startRadius: Float,
        endRadius: Float,
        angleDeg: Double = 0.0,
        durationMs: Long = 400
    ): Boolean {
        val rad = Math.toRadians(angleDeg)
        val dirX = Math.cos(rad).toFloat()
        val dirY = Math.sin(rad).toFloat()
        val f1Start = Pair(cx + dirX * startRadius, cy + dirY * startRadius)
        val f1End = Pair(cx + dirX * endRadius, cy + dirY * endRadius)
        val f2Start = Pair(cx - dirX * startRadius, cy - dirY * startRadius)
        val f2End = Pair(cx - dirX * endRadius, cy - dirY * endRadius)
        return twoFingerGesture(f1Start, f1End, f2Start, f2End, durationMs)
    }

    /**
     * Shove gesture: two fingers, held `spacing`px apart, slide together by (`dx`, `dy`)
     * device pixels. This is the gesture Mapbox (and most map SDKs) bind to camera
     * pitch/tilt - sliding both fingers up tilts the camera into a 3D perspective, sliding
     * down flattens it back to 2D.
     */
    fun shove(
        cx: Float,
        cy: Float,
        spacing: Float,
        dx: Float,
        dy: Float,
        durationMs: Long = 400
    ): Boolean {
        val f1Start = Pair(cx - spacing / 2f, cy)
        val f2Start = Pair(cx + spacing / 2f, cy)
        val f1End = Pair(f1Start.first + dx, f1Start.second + dy)
        val f2End = Pair(f2Start.first + dx, f2Start.second + dy)
        return twoFingerGesture(f1Start, f1End, f2Start, f2End, durationMs)
    }

    /**
     * Injects a key event with optional meta state (for modifiers like Ctrl, Alt).
     */
    fun injectKey(keyCode: Int, action: Int, metaState: Int = 0): Boolean {
        val now = SystemClock.uptimeMillis()
        val event = android.view.KeyEvent(
            now, now, action, keyCode, 0, metaState,
            KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD
        )
        return injectEvent(event)
    }

    /**
     * Simulates a key press (down + up).
     */
    fun pressKey(keyCode: Int): Boolean {
        val downResult = injectKey(keyCode, android.view.KeyEvent.ACTION_DOWN)
        val upResult = injectKey(keyCode, android.view.KeyEvent.ACTION_UP)
        return downResult && upResult
    }

    private fun injectEvent(event: android.view.InputEvent): Boolean {
        return try {
            injectInputEventMethod.invoke(
                inputManager,
                event,
                INJECT_INPUT_EVENT_MODE_WAIT_FOR_FINISH
            ) as Boolean
        } catch (e: Exception) {
            e.printStackTrace()
            false
        }
    }

    /**
     * Injects text by generating KeyEvents for each character.
     * Uses KeyCharacterMap to convert characters to key codes.
     */
    fun injectText(text: String): Boolean {
        return try {
            val keyCharacterMap = KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD)
            val events = keyCharacterMap.getEvents(text.toCharArray())
            
            if (events == null || events.isEmpty()) {
                return false
            }
            
            for (event in events) {
                if (!injectEvent(event)) return false
            }
            true
        } catch (e: Exception) {
            e.printStackTrace()
            false
        }
    }
}
