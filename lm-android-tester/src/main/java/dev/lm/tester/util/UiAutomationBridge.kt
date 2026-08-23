package dev.lm.tester.util

import android.graphics.Rect
import android.os.Build
import android.os.HandlerThread
import android.os.Looper
import android.view.accessibility.AccessibilityNodeInfo
import java.util.ArrayDeque

/**
 * Keeps a single UiAutomation connection alive for the lifetime of the process instead of
 * spinning up a fresh `uiautomator dump` process (JVM + instrumentation cold start, ~2s on
 * real devices) on every hierarchy request. This is the same mechanism the `uiautomator`
 * CLI binary itself uses internally (frameworks/testing's UiAutomationShellWrapper):
 *
 *   val thread = HandlerThread(...); thread.start()
 *   val automation = UiAutomation(thread.looper, UiAutomationConnection())
 *   automation.connect()
 *
 * `android.app.UiAutomation` / `android.app.UiAutomationConnection` are `@hide` framework
 * classes not present in the public SDK stub, so they're reached via reflection. This works
 * here because this process runs under the `shell` UID (launched the same way as the
 * `uiautomator` binary itself via `adb shell app_process`), which already holds the
 * permissions `registerUiTestAutomationService` requires - no `am instrument` needed.
 */
object UiAutomationBridge {

    private const val CONNECT_FLAGS = 0 // UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES = 1; 0 = default flags

    private var handlerThread: HandlerThread? = null
    private var uiAutomation: Any? = null

    private var mConnect: java.lang.reflect.Method? = null
    private var mDisconnect: java.lang.reflect.Method? = null
    private var mGetRoot: java.lang.reflect.Method? = null
    private var mWaitForIdle: java.lang.reflect.Method? = null

    @Synchronized
    private fun ensureConnected(): Boolean {
        if (uiAutomation != null) return true

        return try {
            val thread = HandlerThread("LumiAgentUiAutomation")
            thread.start()

            val connectionClass = Class.forName("android.app.UiAutomationConnection")
            val connectionCtor = connectionClass.getDeclaredConstructor()
            connectionCtor.isAccessible = true
            val connection = connectionCtor.newInstance()

            val iConnectionClass = Class.forName("android.app.IUiAutomationConnection")
            val automationClass = Class.forName("android.app.UiAutomation")
            val automationCtor =
                automationClass.getDeclaredConstructor(Looper::class.java, iConnectionClass)
            automationCtor.isAccessible = true
            val automation = automationCtor.newInstance(thread.looper, connection)

            val connectMethod = automationClass.getMethod("connect", Int::class.javaPrimitiveType)
            connectMethod.invoke(automation, CONNECT_FLAGS)

            handlerThread = thread
            uiAutomation = automation
            mConnect = connectMethod
            mDisconnect = automationClass.getMethod("disconnect")
            mGetRoot = automationClass.getMethod("getRootInActiveWindow")
            mWaitForIdle = try {
                automationClass.getMethod(
                    "waitForIdle",
                    Long::class.javaPrimitiveType,
                    Long::class.javaPrimitiveType
                )
            } catch (_: Throwable) {
                null
            }
            true
        } catch (e: Throwable) {
            e.printStackTrace()
            teardown()
            false
        }
    }

    @Synchronized
    private fun teardown() {
        try {
            mDisconnect?.invoke(uiAutomation)
        } catch (_: Throwable) {
        }
        try {
            handlerThread?.quitSafely()
        } catch (_: Throwable) {
        }
        uiAutomation = null
        handlerThread = null
        mConnect = null
        mDisconnect = null
        mGetRoot = null
        mWaitForIdle = null
    }

    /**
     * Captures a screenshot via `UiAutomation.takeScreenshot()` and returns it as
     * base64-encoded PNG, or null on failure. `UiAutomation` itself is a *public* SDK
     * class (only its 2-arg constructor and `UiAutomationConnection` are hidden - see the
     * class doc above), so once connected, its ordinary public methods like
     * `takeScreenshot()` can be called directly with a simple cast, no reflection needed.
     *
     * This replaces `adb exec-out screencap` (spawns a fresh process + PNG-encodes
     * on-device + streams over the adb transport each call, ~350-450ms measured) with an
     * in-process capture over the already-open connection.
     */
    fun captureScreenshotPngBase64(): String? {
        if (!ensureConnected()) return null
        return try {
            val automation = uiAutomation as? android.app.UiAutomation ?: return null
            val bitmap = automation.takeScreenshot() ?: return null
            val stream = java.io.ByteArrayOutputStream()
            try {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, stream)
            } finally {
                bitmap.recycle()
            }
            android.util.Base64.encodeToString(stream.toByteArray(), android.util.Base64.NO_WRAP)
        } catch (e: Throwable) {
            e.printStackTrace()
            null
        }
    }

    /**
     * Reports whether the on-screen keyboard (IME) is currently showing, via
     * `UiAutomation.getWindows()` (public API) checking for a window of type
     * `TYPE_INPUT_METHOD` - the same signal `dumpsys input_method`'s `mInputShown` reports,
     * without an adb round-trip. Returns null (not false) if the check itself fails, so
     * callers can fall back to the adb-based check rather than assume "not visible" and
     * risk skipping the hide-keyboard action entirely.
     */
    fun isKeyboardVisible(): Boolean? {
        if (!ensureConnected()) return null
        return try {
            val automation = uiAutomation as? android.app.UiAutomation ?: return null
            val windows = automation.windows ?: return false
            windows.any { it.type == android.view.accessibility.AccessibilityWindowInfo.TYPE_INPUT_METHOD }
        } catch (e: Throwable) {
            e.printStackTrace()
            null
        }
    }

    /**
     * Sets the currently-focused input field's text directly via the standard
     * `AccessibilityNodeInfo.ACTION_SET_TEXT` action - fully public API, no reflection
     * needed. Unlike IME-based input (which requires switching to a Unicode-capable IME
     * like ADBKeyBoard, polling to confirm the switch, sending a broadcast, then switching
     * back - each step an adb round-trip), this sets the field's CharSequence value
     * directly over the already-open UiAutomation connection: no IME involved at all, and
     * fully Unicode-safe since Java Strings are UTF-16 (no KeyCharacterMap/ASCII
     * limitation). Matches how UiAutomator2/Espresso's "setText" works internally.
     *
     * Speed must not come at the cost of correctness: focus can lag slightly behind a
     * just-completed tap (more so on slower/loaded devices), so this polls briefly for a
     * focused field instead of giving up on a single miss, and - critically - re-reads the
     * field afterward to confirm the text actually landed before reporting success. A
     * `performAction` call that doesn't throw is not proof the app actually applied the
     * value (e.g. a custom TextWatcher could reject/transform it), so trusting the return
     * value alone isn't enough here; this matches the verify-after-write rigor the
     * ADBKeyBoard fallback path already has.
     *
     * Returns false (not an exception) if there's no focused input field, the action is
     * rejected, or the field's value doesn't match afterward, so the caller can fall back
     * to the IME-based path.
     */
    fun setFocusedText(text: String): Boolean {
        if (!ensureConnected()) return false

        var actionAttempted = false
        return try {
            val focused = findFocusedInputWithRetry() ?: return false
            // Password fields report masked text (dots, or nothing) through the
            // accessibility tree for privacy - Android doesn't expose the real characters
            // that way regardless of how they were entered. Comparing against the exact
            // plaintext would always fail here even when the value was set correctly, so
            // verify by length instead for these fields.
            val isPassword = focused.isPassword

            val args = android.os.Bundle()
            args.putCharSequence(
                AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                text
            )
            actionAttempted = true
            if (!focused.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)) {
                clearFocusedTextBestEffort()
                return false
            }

            val ok = verifyFocusedTextWithRetry(text, isPassword)
            if (!ok) {
                // Whatever ended up in the field (partial write, wrong node, a value the
                // app's TextWatcher transformed) is not what we asked for. Leaving it
                // there would let the ADBKeyBoard fallback path type on top of unknown
                // leftover content instead of into an empty field - clear it first so the
                // fallback starts from the same clean state it would have without this
                // fast path ever having run.
                clearFocusedTextBestEffort()
            }
            ok
        } catch (e: Throwable) {
            e.printStackTrace()
            if (actionAttempted) clearFocusedTextBestEffort()
            teardown()
            false
        }
    }

    /**
     * Best-effort: clear whatever text is in the currently-focused field. Used when a
     * `setFocusedText` attempt is being abandoned, so the IME-based fallback path (which
     * types into the field expecting it to be empty, exactly as it would be if this fast
     * path had never run) doesn't end up typing on top of a partial/wrong value. Never
     * throws - any failure here just means the fallback has to cope with whatever state
     * is left, same as if this fast path didn't exist.
     */
    private fun clearFocusedTextBestEffort() {
        try {
            @Suppress("UNCHECKED_CAST")
            val root = mGetRoot?.invoke(uiAutomation) as? AccessibilityNodeInfo ?: return
            val focused = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT) ?: return
            val args = android.os.Bundle()
            args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "")
            focused.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
        } catch (_: Throwable) {
            // Best-effort only.
        }
    }

    /**
     * Polls for a focused input field for up to ~2s before giving up.
     *
     * Root cause this covers (confirmed on-device, not a guess): this app is Flutter
     * (`dev.fluttercommunity.plus.share` / `io.flutter.plugins.*` present). Flutter
     * doesn't use native Android views for its widgets - it renders everything itself
     * and exposes an accessibility *semantics* tree through `io.flutter.embedding
     * .android.AccessibilityBridge`, built asynchronously and only on demand. Right
     * after a tap that navigates to a new screen, that semantics tree can legitimately
     * still be empty/unbuilt for a beat (observed directly: a hierarchy dump taken
     * immediately after such a tap showed zero focused nodes and no input widget at
     * all, with the Android *window* itself already focused and not animating per
     * `dumpsys window` - ruling out a plain view-layout or window-animation race).
     * `UiAutomation.waitForIdle()` tracks the accessibility *event* queue though, and
     * Flutter's semantics-tree build/update **does** fire real accessibility events
     * (that's how TalkBack stays in sync with it) - so calling it first here, before
     * polling, lets us resync with Flutter's own readiness signal instead of guessing
     * with a blind sleep loop alone. The old ADBKeyBoard path never hit this in
     * practice only because its own IME-switch overhead (~1s) accidentally gave the
     * semantics tree enough time to build; it was never actually protected against
     * this race. The polling loop after `waitForIdle` is a safety net for whatever
     * residual gap remains.
     */
    private fun findFocusedInputWithRetry(): AccessibilityNodeInfo? {
        try {
            mWaitForIdle?.invoke(uiAutomation, 100L, 1500L)
        } catch (_: Throwable) {
            // Best-effort resync; the poll loop below still covers correctness.
        }
        repeat(40) { attempt ->
            @Suppress("UNCHECKED_CAST")
            val root = mGetRoot?.invoke(uiAutomation) as? AccessibilityNodeInfo
            val focused = root?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            if (focused != null) return focused
            if (attempt < 39) Thread.sleep(50)
        }
        return null
    }

    /**
     * Re-reads the focused field (a fresh node reference - the one used to perform the
     * action may be stale after the mutation) and polls for up to ~200ms to confirm its
     * text matches what was just set. For password fields, the accessibility tree masks
     * the real characters, so this checks length instead of exact content - still a real
     * verification (catches "nothing was typed" or "wrong length"), just not exact-content
     * (which Android doesn't expose for password fields regardless of input method).
     */
    private fun verifyFocusedTextWithRetry(expected: String, isPassword: Boolean): Boolean {
        repeat(5) { attempt ->
            @Suppress("UNCHECKED_CAST")
            val root = mGetRoot?.invoke(uiAutomation) as? AccessibilityNodeInfo
            val current = root?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)?.text?.toString()
            val matches = if (isPassword) {
                current != null && current.length == expected.length
            } else {
                current == expected
            }
            if (matches) return true
            if (attempt < 4) Thread.sleep(40)
        }
        return false
    }

    /**
     * Dumps the current window hierarchy as XML in the same schema `uiautomator dump`
     * produces (flat `<node class=".." text=".." bounds="[l,t][r,b]" .../>` tags), which is
     * exactly what lumi-tester's `uiautomator::parse_hierarchy` already understands - no
     * changes needed on the Rust parsing side.
     *
     * Returns null on any failure so the caller can fall back to the slow shell-based dump.
     */
    fun dumpXml(): String? {
        if (!ensureConnected()) return null

        return try {
            try {
                mWaitForIdle?.invoke(uiAutomation, 200L, 2000L)
            } catch (_: Throwable) {
                // Idle wait is a best-effort optimization, not required for correctness.
            }

            @Suppress("UNCHECKED_CAST")
            val root = mGetRoot?.invoke(uiAutomation) as? AccessibilityNodeInfo
                ?: return null

            val sb = StringBuilder()
            sb.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?><hierarchy rotation=\"0\">")
            serializeNode(root, sb)
            sb.append("</hierarchy>")
            sb.toString()
        } catch (e: Throwable) {
            e.printStackTrace()
            // The connection may have gone stale (e.g. system_server restarted); drop it so
            // the next call attempts a fresh connect instead of repeatedly failing silently.
            teardown()
            null
        }
    }

    /**
     * Waits for the accessibility event stream to go quiet (no events for [idleMs]),
     * up to [timeoutMs] total - the same primitive `UiAutomator`/`UiDevice.waitForIdle()`
     * uses internally to detect that an animation/transition has settled. This is a
     * single in-process call over the already-open UiAutomation connection, replacing a
     * poll loop of `adb shell dumpsys window` round-trips.
     *
     * Returns false (not an exception) if the fast path isn't available, so the caller
     * can fall back to the shell-based poll.
     */
    fun waitForIdle(idleMs: Long, timeoutMs: Long): Boolean {
        if (!ensureConnected()) return false
        return try {
            mWaitForIdle?.invoke(uiAutomation, idleMs, timeoutMs)
            true
        } catch (e: java.lang.reflect.InvocationTargetException) {
            val cause = e.cause
            if (cause != null && cause.javaClass.simpleName.contains("Timeout")) {
                // Still busy after timeoutMs - same outcome as the old poll loop's
                // timeout, not a connection failure. Report success (we did wait).
                true
            } else {
                teardown()
                false
            }
        } catch (e: Throwable) {
            teardown()
            false
        }
    }

    /** Iterative (stack-based) tree walk - avoids recursion depth issues on deep view trees. */
    private fun serializeNode(root: AccessibilityNodeInfo, sb: StringBuilder) {
        data class Frame(val node: AccessibilityNodeInfo, val index: Int, val closing: Boolean)

        val stack = ArrayDeque<Frame>()
        stack.push(Frame(root, 0, closing = false))

        while (stack.isNotEmpty()) {
            val frame = stack.pop()
            if (frame.closing) {
                sb.append("</node>")
                continue
            }

            val node = frame.node
            val childCount = node.childCount
            sb.append("<node")
            appendAttr(sb, "index", frame.index.toString())
            appendAttr(sb, "text", node.text?.toString() ?: "")
            appendAttr(sb, "resource-id", node.viewIdResourceName ?: "")
            appendAttr(sb, "class", node.className?.toString() ?: "")
            appendAttr(sb, "package", node.packageName?.toString() ?: "")
            appendAttr(sb, "content-desc", node.contentDescription?.toString() ?: "")
            appendAttr(sb, "clickable", node.isClickable.toString())
            appendAttr(sb, "enabled", node.isEnabled.toString())
            appendAttr(sb, "focusable", node.isFocusable.toString())
            appendAttr(sb, "focused", node.isFocused.toString())
            appendAttr(sb, "scrollable", node.isScrollable.toString())
            appendAttr(sb, "password", node.isPassword.toString())
            appendAttr(sb, "hint", getHintTextSafe(node))

            val rect = Rect()
            node.getBoundsInScreen(rect)
            appendAttr(sb, "bounds", "[${rect.left},${rect.top}][${rect.right},${rect.bottom}]")

            if (childCount == 0) {
                sb.append("/>")
            } else {
                sb.append(">")
                stack.push(Frame(node, 0, closing = true))
                // Push children in reverse so they're popped/processed left-to-right.
                for (i in childCount - 1 downTo 0) {
                    val child = node.getChild(i) ?: continue
                    stack.push(Frame(child, i, closing = false))
                }
            }

            // recycle() is a deprecated no-op from API 33+ but still matters on older
            // devices to avoid leaking AccessibilityNodeInfo objects across many rapid
            // dumps in a long-running session. Safe here: all attributes were already
            // read above and any children were already fetched as independent instances.
            @Suppress("DEPRECATION")
            try {
                node.recycle()
            } catch (_: Throwable) {
            }
        }
    }

    private fun getHintTextSafe(node: AccessibilityNodeInfo): String {
        return try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                node.hintText?.toString() ?: ""
            } else {
                ""
            }
        } catch (_: Throwable) {
            ""
        }
    }

    private fun appendAttr(sb: StringBuilder, name: String, rawValue: String) {
        sb.append(' ').append(name).append("=\"").append(escapeXml(rawValue)).append('"')
    }

    private fun escapeXml(value: String): String {
        if (value.isEmpty()) return value
        val sb = StringBuilder(value.length)
        for (c in value) {
            when (c) {
                '&' -> sb.append("&amp;")
                '<' -> sb.append("&lt;")
                '>' -> sb.append("&gt;")
                '"' -> sb.append("&quot;")
                '\'' -> sb.append("&apos;")
                '\n' -> sb.append("&#10;")
                else -> sb.append(c)
            }
        }
        return sb.toString()
    }
}
