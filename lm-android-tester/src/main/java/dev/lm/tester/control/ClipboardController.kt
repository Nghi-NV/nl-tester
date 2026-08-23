package dev.lm.tester.control

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import dev.lm.tester.util.FakeContext
import android.os.Looper

/**
 * ClipboardController handles clipboard operations via ClipboardManager.
 * Uses FakeContext to access system services properly.
 */
object ClipboardController {
    
    private val clipboardManager: ClipboardManager? by lazy {
        try {
            if (Looper.myLooper() == null) {
                Looper.prepare()
            }
            val context = FakeContext.get()
            context.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
        } catch (e: Exception) {
            println("[CLIPBOARD] ERROR: ${e.message}")
            null
        }
    }

    fun getText(): String? {
        return try {
            val manager = clipboardManager ?: return null
            val clipData = manager.primaryClip
            if (clipData == null || clipData.itemCount == 0) return null
            clipData.getItemAt(0).text?.toString()
        } catch (e: Exception) {
            println("[CLIPBOARD] ERROR getText: ${e.message}")
            null
        }
    }

    fun setText(text: String): Boolean {
        return try {
            val manager = clipboardManager ?: return false
            // Avoid duplicate "Copied" toast
            try {
                val currentClip = manager.primaryClip
                if (currentClip != null && currentClip.itemCount > 0) {
                    if (currentClip.getItemAt(0).text?.toString() == text) return true
                }
            } catch (_: Exception) {}
            manager.setPrimaryClip(ClipData.newPlainText("text", text))
            true
        } catch (e: Exception) {
            println("[CLIPBOARD] ERROR setText: ${e.message}")
            false
        }
    }

    fun setTextAndPaste(text: String, paste: Boolean): Boolean {
        val success = setText(text)
        if (success && paste) {
            try {
                val ctrl = 113; val v = 50; val metaCtrl = 4096
                dev.lm.tester.input.InputController.injectKey(ctrl, android.view.KeyEvent.ACTION_DOWN)
                dev.lm.tester.input.InputController.injectKey(v, android.view.KeyEvent.ACTION_DOWN, metaCtrl)
                dev.lm.tester.input.InputController.injectKey(v, android.view.KeyEvent.ACTION_UP, metaCtrl)
                dev.lm.tester.input.InputController.injectKey(ctrl, android.view.KeyEvent.ACTION_UP)
            } catch (_: Exception) {}
        }
        return success
    }

    fun copyAndGetText(): String? {
        try {
            val ctrl = 113; val c = 31; val metaCtrl = 4096
            dev.lm.tester.input.InputController.injectKey(ctrl, android.view.KeyEvent.ACTION_DOWN)
            dev.lm.tester.input.InputController.injectKey(c, android.view.KeyEvent.ACTION_DOWN, metaCtrl)
            dev.lm.tester.input.InputController.injectKey(c, android.view.KeyEvent.ACTION_UP, metaCtrl)
            dev.lm.tester.input.InputController.injectKey(ctrl, android.view.KeyEvent.ACTION_UP)
            Thread.sleep(50)
        } catch (_: Exception) {}
        return getText()
    }
}
