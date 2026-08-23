package dev.lm.tester.input

import android.content.Context
import android.location.Location
import android.location.LocationManager
import android.location.provider.ProviderProperties
import android.os.Build
import android.os.SystemClock
import android.util.Log
import dev.lm.tester.util.FakeContext

object LocationController {
    private val locationManager: LocationManager by lazy {
        FakeContext.get().getSystemService(Context.LOCATION_SERVICE) as LocationManager
    }
    
    private val providers = listOf(
        LocationManager.GPS_PROVIDER,
        LocationManager.NETWORK_PROVIDER,
        LocationManager.FUSED_PROVIDER
    )
    
    private var isMocking = false
    private var lastReRegisterTime = 0L
    
    // Samsung removes test providers after ~60s, re-register every 30s to stay ahead
    private const val RE_REGISTER_INTERVAL_MS = 30_000L

    fun startMocking() {
        if (isMocking) return
        

        registerProviders()
        isMocking = true
        lastReRegisterTime = System.currentTimeMillis()

    }

    /**
     * Register/re-register all test providers.
     * Samsung devices remove test providers after ~60s, so this needs to be called periodically.
     */
    private fun registerProviders() {
        providers.forEach { provider ->
            try {
                locationManager.removeTestProvider(provider)
            } catch (e: Exception) {
                // Expected if not exists
            }

            try {
                locationManager.addTestProvider(
                    provider,
                    false, // requiresNetwork
                    false, // requiresSatellite
                    false, // requiresCell
                    false, // hasMonetaryCost
                    true,  // supportsAltitude
                    true,  // supportsSpeed
                    true,  // supportsBearing
                    ProviderProperties.POWER_USAGE_LOW,
                    ProviderProperties.ACCURACY_FINE
                )
            } catch (e: IllegalArgumentException) {

            } catch (e: SecurityException) {
                Log.d("NL_MIRROR", "Security exception adding $provider: ${e.message}")
            }
            
            try {
                locationManager.setTestProviderEnabled(provider, true)
            } catch (e: Exception) {
                Log.d("NL_MIRROR", "Failed to enable $provider: ${e.message}")
            }
        }
    }

    fun stopMocking() {
        if (!isMocking) return
        
        providers.forEach { provider ->
            try {
                locationManager.removeTestProvider(provider)
            } catch (e: Exception) {
                // Ignore
            }
        }
        isMocking = false
    }

    fun updateLocation(lat: Double, lon: Double, alt: Double = 0.0, bearing: Float = 0.0f, speed: Float = 0.0f) {
        if (!isMocking) {
            startMocking()
        }

        // Samsung workaround: periodically re-register providers before Samsung removes them
        val now = System.currentTimeMillis()
        if (now - lastReRegisterTime > RE_REGISTER_INTERVAL_MS) {

            registerProviders()
            lastReRegisterTime = now
        }

        providers.forEach { provider ->
            val loc = Location(provider)
            loc.latitude = lat
            loc.longitude = lon
            loc.altitude = alt
            loc.bearing = bearing
            loc.speed = speed
            loc.accuracy = 1.0f  // Very high accuracy to override other sources
            loc.time = System.currentTimeMillis()
            
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR1) {
                loc.elapsedRealtimeNanos = SystemClock.elapsedRealtimeNanos()
            }
            
            // Set additional accuracy fields for Android O+
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                loc.verticalAccuracyMeters = 1.0f
                loc.speedAccuracyMetersPerSecond = 0.1f
                loc.bearingAccuracyDegrees = 1.0f
            }

            try {
                 locationManager.setTestProviderLocation(provider, loc)
            } catch (e: Exception) {
                 // Samsung may have removed the test provider - try to re-register and retry

                 try {
                     locationManager.removeTestProvider(provider)
                 } catch (_: Exception) {}
                 try {
                     locationManager.addTestProvider(
                         provider,
                         false, false, false, false,
                         true, true, true,
                         ProviderProperties.POWER_USAGE_LOW,
                         ProviderProperties.ACCURACY_FINE
                     )
                     locationManager.setTestProviderEnabled(provider, true)
                     locationManager.setTestProviderLocation(provider, loc)

                     lastReRegisterTime = System.currentTimeMillis()
                 } catch (retryEx: Exception) {
                     Log.d("NL_MIRROR", "Retry failed for $provider: ${retryEx.message}")
                 }
            }
        }
    }
}

