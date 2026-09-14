package dev.lm.tester.input

import android.app.Instrumentation
import android.app.UiAutomation
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.UiObject2
import dev.lm.tester.util.UiAutomationBridge
import kotlin.math.max
import kotlin.math.roundToInt

/**
 * Pinch via `UiObject2.pinchOpen()`/`pinchClose()` - Google's own field-tested multi-touch
 * gesture implementation - instead of the hand-rolled `InputController.twoFingerGesture()`
 * MotionEvent synthesis.
 *
 * Why this exists: the hand-rolled version was verified live against GOFA's Mapbox map -
 * Android's own Pointer Location developer overlay confirmed the OS genuinely tracks 2
 * correctly-positioned pointers during the gesture (`P: 2/2`, both crosshairs at the
 * expected coordinates) - yet Mapbox's gesture detector never reacted: no zoom, across
 * three different fix attempts (a realistic inter-event delay, finer per-step motion
 * granularity, and an explicit stationary "baseline hold" right after the second finger
 * lands). A real finger pinch on the same screen zoomed normally, so the gap is
 * specifically in the synthesized event stream, not a GOFA-side restriction.
 *
 * `UiDevice.performMultiPointerGesture` (the arbitrary-coordinates multi-touch primitive
 * from the old `android.support.test.uiautomator` support library) does NOT exist in the
 * current `androidx.test.uiautomator:2.3.0` public API (confirmed via `javap` against the
 * actual jar - it was dropped in the AndroidX migration). The only multi-touch gesture
 * still exposed publicly is `UiObject2.pinchOpen(percent, speed)`/`pinchClose(...)`, which
 * operates relative to a *UiObject2's* bounds rather than an arbitrary point. This class
 * fakes an object with exactly the bounds we want (the depth-0 root node, sized/margined so
 * its center lands on the requested pinch center) so the public, tested pinch gesture can
 * still be driven at an arbitrary screen point.
 *
 * There is no `UiObject2` equivalent for a generic 2-finger "shove" (both fingers sliding
 * together) - only pinch open/close are exposed - so `shove` has no UiObject2-based path
 * here and callers must keep using [InputController.shove] for that gesture; it carries the
 * same "verified success at the OS level, unconfirmed at the Mapbox level" caveat as before.
 *
 * `UiDevice.getInstance(Instrumentation)` normally requires a real `Instrumentation` (i.e.
 * running as an instrumented test via `am instrument`), which this agent does not use - it
 * runs via `adb shell app_process` under the `shell` UID (see `UiAutomationBridge`'s class
 * doc). `UiDevice` only ever calls `instrumentation.getUiAutomation()` internally, so a
 * minimal [Instrumentation] subclass overriding just that one method to return the
 * [UiAutomation] connection [UiAutomationBridge] already holds is enough to satisfy it,
 * without needing the full instrumented-test execution model.
 */
object PinchShoveGestures {

    private class BridgedInstrumentation(private val automation: UiAutomation) : Instrumentation() {
        override fun getUiAutomation(): UiAutomation = automation
    }

    @Volatile
    private var cachedDevice: UiDevice? = null

    private fun device(): UiDevice? {
        cachedDevice?.let { return it }
        val automation = UiAutomationBridge.getAutomation() ?: return null
        return try {
            UiDevice.getInstance(BridgedInstrumentation(automation)).also { cachedDevice = it }
        } catch (e: Throwable) {
            e.printStackTrace()
            null
        }
    }

    /**
     * The depth-0 accessibility root, its bounds narrowed via gesture margins so the
     * *center* of what remains sits at (cx, cy) and the remaining half-extent is
     * `halfSpan` in every direction - i.e. a `halfSpan*2` square centered exactly where
     * the caller wants the pinch to happen, clipped to the real screen bounds.
     */
    private fun boundedRootAt(dev: UiDevice, cx: Float, cy: Float, halfSpan: Float): UiObject2? {
        val root = try {
            dev.findObject(By.depth(0))
        } catch (e: Throwable) {
            e.printStackTrace()
            null
        } ?: return null

        val full = root.visibleBounds
        val left = (cx - halfSpan).coerceIn(full.left.toFloat(), full.right.toFloat())
        val top = (cy - halfSpan).coerceIn(full.top.toFloat(), full.bottom.toFloat())
        val right = (cx + halfSpan).coerceIn(full.left.toFloat(), full.right.toFloat())
        val bottom = (cy + halfSpan).coerceIn(full.top.toFloat(), full.bottom.toFloat())

        root.setGestureMargins(
            (left - full.left).roundToInt().coerceAtLeast(0),
            (top - full.top).roundToInt().coerceAtLeast(0),
            (full.right - right).roundToInt().coerceAtLeast(0),
            (full.bottom - bottom).roundToInt().coerceAtLeast(0)
        )
        return root
    }

    /**
     * Pinch centered at (cx, cy). `startRadius`/`endRadius` behave like the hand-rolled
     * version's: opening (zoom in) has `endRadius > startRadius`. The margined root's
     * half-span is set to `max(startRadius, endRadius)` and `pinchOpen`/`pinchClose` is
     * called at 100% of that span, so the fingers travel across (approximately) the same
     * pixel range the caller asked for either way.
     */
    fun pinch(
        cx: Float,
        cy: Float,
        startRadius: Float,
        endRadius: Float,
        durationMs: Long
    ): Boolean {
        val dev = device() ?: return false
        val halfSpan = max(startRadius, endRadius).coerceAtLeast(50f)
        val obj = boundedRootAt(dev, cx, cy, halfSpan) ?: return false
        // percent/speed is UiObject2's own gesture parameterization (not pixels), speed in
        // pixels/second - derived from how far the fingers travel and the requested
        // duration so a longer `durationMs` still produces a slower, longer gesture.
        val speed = max(500, ((halfSpan * 2f) / (durationMs.coerceAtLeast(100) / 1000f)).roundToInt())
        return try {
            if (endRadius >= startRadius) {
                obj.pinchOpen(1.0f, speed)
            } else {
                obj.pinchClose(1.0f, speed)
            }
            true
        } catch (e: Throwable) {
            e.printStackTrace()
            false
        }
    }
}
