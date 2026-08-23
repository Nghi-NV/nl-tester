package dev.lm.tester.util

import android.annotation.SuppressLint
import android.annotation.TargetApi
import android.content.AttributionSource
import android.content.ContentResolver
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import android.os.Process

/**
 * FakeContext
 * 
 * Impersonates com.android.shell to get proper permissions
 * for DisplayManager operations
 */
@SuppressLint("PrivateApi")
class FakeContext private constructor(baseContext: Context?) : ContextWrapper(baseContext) {
    
    companion object {
        const val PACKAGE_NAME = "com.android.shell"
        const val ROOT_UID = 0
        
        private val INSTANCE: FakeContext by lazy {
            // Ensure workarounds are applied BEFORE getting system context
            Workarounds.apply()
            FakeContext(Workarounds.getSystemContext())
        }
        
        @JvmStatic
        fun get(): FakeContext = INSTANCE
    }
    
    override fun getPackageName(): String = PACKAGE_NAME
    
    override fun getOpPackageName(): String = PACKAGE_NAME
    
    @TargetApi(31) // Android 12
    override fun getAttributionSource(): AttributionSource {
        val builder = AttributionSource.Builder(Process.SHELL_UID)
        builder.setPackageName(PACKAGE_NAME)
        return builder.build()
    }
    
    // @Override for Android 14
    @Suppress("unused")
    override fun getDeviceId(): Int = 0
    
    override fun getApplicationContext(): Context = this
    
    override fun createPackageContext(packageName: String?, flags: Int): Context = this
    
    @SuppressLint("DiscouragedPrivateApi")
    override fun getSystemService(name: String): Any? {
        val service = super.getSystemService(name)
        if (service == null) {
            return null
        }
        
        // Fix Samsung-specific services that require mContext
        // "semclipboard" is a Samsung-internal service
        if (name == Context.CLIPBOARD_SERVICE || name == "semclipboard" || name == Context.ACTIVITY_SERVICE) {
            try {
                val field = service.javaClass.getDeclaredField("mContext")
                field.isAccessible = true
                field.set(service, this)
            } catch (e: ReflectiveOperationException) {
                // Ignore if field doesn't exist
            }
        }
        
        return service
    }
}
