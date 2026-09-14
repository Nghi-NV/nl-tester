plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.lm.tester"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.lm.tester"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
        freeCompilerArgs += listOf(
            "-opt-in=kotlin.ExperimentalStdlibApi",
            "-opt-in=kotlin.RequiresOptIn"
        )
    }
}

dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.0")
    implementation("org.lsposed.hiddenapibypass:hiddenapibypass:4.3")
    // UiDevice.performMultiPointerGesture() - Google's own field-tested multi-touch
    // gesture primitive, used here instead of hand-rolled MotionEvent synthesis for
    // pinch/shove (see PinchShoveGestures.kt doc comment for why the hand-rolled version
    // wasn't enough on its own).
    implementation("androidx.test.uiautomator:uiautomator:2.3.0")
}
