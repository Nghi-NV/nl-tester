package dev.lm.tester.display

import android.os.Build
import android.os.IBinder
import java.lang.reflect.Method

object DisplayControl {
    private var setDisplayPowerModeMethod: Method? = null
    private var getBuiltInDisplayMethod: Method? = null

    // Constants from SurfaceControl
    const val POWER_MODE_OFF = 0
    const val POWER_MODE_NORMAL = 2

    fun setPowerMode(mode: Int): Boolean {
        try {
            val scClass = Class.forName("android.view.SurfaceControl")
            val setDisplayPowerMode = scClass.getMethod("setDisplayPowerMode", IBinder::class.java, Int::class.javaPrimitiveType)
            var success = false

            // Try 1: Physical Display IDs (Android 10+)
            if (Build.VERSION.SDK_INT >= 29) {
                try {
                    val getPhysicalDisplayIds = scClass.getMethod("getPhysicalDisplayIds")
                    val displayIds = getPhysicalDisplayIds.invoke(null) as LongArray?
                    if (displayIds != null) {
                        val getPhysicalDisplayToken = scClass.getMethod("getPhysicalDisplayToken", Long::class.javaPrimitiveType)
                        for (id in displayIds) {
                            val token = getPhysicalDisplayToken.invoke(null, id) as IBinder?
                            if (token != null) {
                                setDisplayPowerMode.invoke(null, token, mode)
                                android.util.Log.d("DisplayControl", "Method 1 (Phys $id): OK")
                                success = true
                            }
                        }
                    }
                } catch (e: Exception) {
                    android.util.Log.w("DisplayControl", "Method 1 Err: ${e.message}")
                }
            }

            // Try 2: getInternalDisplayToken (Android 10+)
            if (Build.VERSION.SDK_INT >= 29) {
                try {
                    val getInternalDisplayToken = scClass.getMethod("getInternalDisplayToken")
                    val token = getInternalDisplayToken.invoke(null) as IBinder?
                    if (token != null) {
                        setDisplayPowerMode.invoke(null, token, mode)
                        android.util.Log.d("DisplayControl", "Method 2 (Internal): OK")
                        success = true
                    }
                } catch (e: Exception) {
                    android.util.Log.w("DisplayControl", "Method 2 Err: ${e.message}")
                }
            }

            // Try 3: getBuiltInDisplay(0) (Legacy)
            try {
                val getBuiltInDisplay = scClass.getMethod("getBuiltInDisplay", Int::class.javaPrimitiveType)
                val token = getBuiltInDisplay.invoke(null, 0) as IBinder?
                if (token != null) {
                    setDisplayPowerMode.invoke(null, token, mode)
                    android.util.Log.d("DisplayControl", "Method 3 (BuiltIn 0): OK")
                    success = true
                }
            } catch (e: Exception) {
                // Ignore if not applicable
            }

            return success
        } catch (e: Exception) {
            android.util.Log.e("DisplayControl", "Failed to set power mode", e)
            return false
        }
    }
}
