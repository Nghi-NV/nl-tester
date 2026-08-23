package dev.lm.tester.util

import android.annotation.SuppressLint
import android.app.Application
import android.app.Instrumentation
import android.content.Context
import android.content.pm.ApplicationInfo
import android.os.Build
import java.lang.reflect.Constructor
import java.lang.reflect.Field

@SuppressLint("PrivateApi", "DiscouragedPrivateApi")
object Workarounds {
    
    private val ACTIVITY_THREAD_CLASS: Class<*>
    private val ACTIVITY_THREAD: Any
    
    init {
        try {
            // Only prepare if not already done, and catch if it throws
            try {
                if (android.os.Looper.myLooper() == null) {
                    android.os.Looper.prepareMainLooper()
                }
            } catch (_: Exception) {}
            
            ACTIVITY_THREAD_CLASS = Class.forName("android.app.ActivityThread")
            val activityThreadConstructor = ACTIVITY_THREAD_CLASS.getDeclaredConstructor()
            activityThreadConstructor.isAccessible = true
            ACTIVITY_THREAD = activityThreadConstructor.newInstance()
            val sCurrentActivityThreadField = ACTIVITY_THREAD_CLASS.getDeclaredField("sCurrentActivityThread")
            sCurrentActivityThreadField.isAccessible = true
            sCurrentActivityThreadField.set(null, ACTIVITY_THREAD)
            val mSystemThreadField = ACTIVITY_THREAD_CLASS.getDeclaredField("mSystemThread")
            mSystemThreadField.isAccessible = true
            mSystemThreadField.setBoolean(ACTIVITY_THREAD, true)
        } catch (e: Exception) {
            throw AssertionError(e)
        }
    }
    
    private var applied = false
    
    fun apply() {
        if (applied) return
        applied = true
        if (Build.VERSION.SDK_INT >= 31) { fillConfigurationController() }
        fillAppInfo()
        fillAppContext()
    }
    
    private fun fillAppInfo() {
        try {
            val appBindDataClass = Class.forName("android.app.ActivityThread\$AppBindData")
            val appBindDataConstructor = appBindDataClass.getDeclaredConstructor()
            appBindDataConstructor.isAccessible = true
            val appBindData = appBindDataConstructor.newInstance()
            val applicationInfo = ApplicationInfo()
            applicationInfo.packageName = FakeContext.PACKAGE_NAME
            val appInfoField = appBindDataClass.getDeclaredField("appInfo")
            appInfoField.isAccessible = true
            appInfoField.set(appBindData, applicationInfo)
            val mBoundApplicationField = ACTIVITY_THREAD_CLASS.getDeclaredField("mBoundApplication")
            mBoundApplicationField.isAccessible = true
            mBoundApplicationField.set(ACTIVITY_THREAD, appBindData)
        } catch (_: Throwable) {}
    }
    
    private fun fillAppContext() {
        try {
            val app = Instrumentation.newApplication(Application::class.java, FakeContext.get())
            val mInitialApplicationField = ACTIVITY_THREAD_CLASS.getDeclaredField("mInitialApplication")
            mInitialApplicationField.isAccessible = true
            mInitialApplicationField.set(ACTIVITY_THREAD, app)
        } catch (_: Throwable) {}
    }
    
    private fun fillConfigurationController() {
        try {
            val configurationControllerClass = Class.forName("android.app.ConfigurationController")
            val activityThreadInternalClass = Class.forName("android.app.ActivityThreadInternal")
            val configurationControllerConstructor = configurationControllerClass.getDeclaredConstructor(activityThreadInternalClass)
            configurationControllerConstructor.isAccessible = true
            val configurationController = configurationControllerConstructor.newInstance(ACTIVITY_THREAD)
            val configurationControllerField = ACTIVITY_THREAD_CLASS.getDeclaredField("mConfigurationController")
            configurationControllerField.isAccessible = true
            configurationControllerField.set(ACTIVITY_THREAD, configurationController)
        } catch (_: Throwable) {}
    }
    
    fun getSystemContext(): Context? {
        return try {
            val getSystemContextMethod = ACTIVITY_THREAD_CLASS.getDeclaredMethod("getSystemContext")
            getSystemContextMethod.invoke(ACTIVITY_THREAD) as Context
        } catch (_: Throwable) { null }
    }
}
