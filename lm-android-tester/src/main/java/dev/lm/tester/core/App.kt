package dev.lm.tester.core

import dev.lm.tester.network.CommandServer
import android.os.Build
import android.util.Log
import org.lsposed.hiddenapibypass.HiddenApiBypass

/**
 * Entry point for the lumi-tester Android agent, launched via
 * `adb shell app_process ... dev.lm.tester.core.App` (no install required, same
 * technique as `uiautomator`/scrcpy - runs under the `shell` UID). Only runs a single
 * command socket (see [CommandServer]) exposing fast in-process UI-hierarchy dumps
 * (persistent UiAutomation connection), tap/swipe/key/text input, and mock location -
 * everything lumi-tester's Android driver needs. No screen/audio mirroring: this is a
 * dedicated automation-speed agent, not a general-purpose mirroring tool.
 */
object App {
    const val COMMAND_PORT = 7899

    @JvmStatic
    fun main(args: Array<String>) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            HiddenApiBypass.addHiddenApiExemptions("L")
        }

        // Initialize Looper and Workarounds on the startup thread
        // to avoid crashes when FakeContext is first accessed from a background thread.
        try {
            if (android.os.Looper.myLooper() == null) {
                android.os.Looper.prepareMainLooper()
            }
        } catch (_: Exception) {}

        dev.lm.tester.util.Workarounds.apply()
        dev.lm.tester.util.FakeContext.get()

        try {
            Log.d("LM_AGENT", "App starting up...")

            val commandServer = CommandServer(COMMAND_PORT)
            commandServer.start()

            Log.d("LM_AGENT", "App.main joining")
            Thread.currentThread().join()
            Log.d("LM_AGENT", "App.main joined (normal exit)")
        } catch (e: Exception) {
            Log.e("LM_AGENT", "App fatal error", e)
            System.exit(1)
        }
    }
}
