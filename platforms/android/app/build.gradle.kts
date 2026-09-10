plugins {
    id("com.android.application")
}

android {
    namespace = "org.nvdarust.screenreader"
    compileSdk = 37

    defaultConfig {
        applicationId = "org.nvdarust.screenreader"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "android.test.InstrumentationTestRunner"
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
}
