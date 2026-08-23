package dev.lm.tester.util

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import android.os.Debug
import org.json.JSONObject
import java.io.RandomAccessFile

/**
 * PerformanceMonitor collects real-time device performance metrics.
 */
object PerformanceMonitor {
    private var lastCpuTime = 0L
    private var lastAppCpuTime = 0L

    /**
     * Returns current performance stats as JSON.
     */
    fun getStats(): JSONObject {
        return JSONObject().apply {
            put("cpu", getCpuUsage())
            put("memory", getMemoryInfo())
            put("device", getDeviceInfo())
            put("timestamp", System.currentTimeMillis())
        }
    }

    private fun getCpuUsage(): JSONObject {
        return try {
            val reader = RandomAccessFile("/proc/stat", "r")
            val line = reader.readLine()
            reader.close()

            val parts = line.split("\\s+".toRegex())
            val user = parts[1].toLong()
            val nice = parts[2].toLong()
            val system = parts[3].toLong()
            val idle = parts[4].toLong()

            val total = user + nice + system + idle
            val used = user + nice + system

            JSONObject().apply {
                put("total", total)
                put("used", used)
                put("percentage", if (total > 0) (used * 100 / total) else 0)
            }
        } catch (e: Exception) {
            JSONObject().put("error", e.message)
        }
    }

    private fun getMemoryInfo(): JSONObject {
        val runtime = Runtime.getRuntime()
        val usedMemory = runtime.totalMemory() - runtime.freeMemory()
        val maxMemory = runtime.maxMemory()

        return JSONObject().apply {
            put("used", usedMemory)
            put("max", maxMemory)
            put("percentage", if (maxMemory > 0) (usedMemory * 100 / maxMemory) else 0)
            put("nativeHeap", Debug.getNativeHeapAllocatedSize())
        }
    }

    private fun getDeviceInfo(): JSONObject {
        return JSONObject().apply {
            put("manufacturer", Build.MANUFACTURER)
            put("model", Build.MODEL)
            put("sdk", Build.VERSION.SDK_INT)
            put("release", Build.VERSION.RELEASE)
        }
    }
}
