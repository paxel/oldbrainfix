import java.util.Properties
import javax.inject.Inject

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Release signing: android/key.properties points at the upload keystore.
// The file is not in git; the release workflow writes it from secrets.
// There is no fallback to the debug key: a release build without it
// fails (see below), because an APK signed with the debug key would lock
// every F-Droid install to that key for good.
val keystoreProperties = Properties().apply {
    val f = rootProject.file("key.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}

android {
    namespace = "io.github.paxel.oldbrainfix"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "io.github.paxel.oldbrainfix"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        // Whisper for Transcripts is built for phones only; on any other
        // ABI there is no transcription and nothing else changes.
        externalNativeBuild {
            cmake {
                abiFilters += listOf("arm64-v8a", "armeabi-v7a")
            }
        }
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    signingConfigs {
        if (keystoreProperties.isNotEmpty()) {
            create("release") {
                storeFile = file(keystoreProperties.getProperty("storeFile"))
                // CI hands the password over as an environment variable, so
                // no shell or properties escaping can change it.
                val password = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                storePassword = password ?: keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = password ?: keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            if (keystoreProperties.isNotEmpty()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
}

// A release build without key.properties stops before anything is
// compiled, rather than turning out an unsigned or debug-signed APK.
// Everything else (debug builds, unit tests, the dependency report on
// the release classpath) still runs without the keystore.
gradle.taskGraph.whenReady {
    val release = allTasks.any { it.project == project && it.name == "preReleaseBuild" }
    if (release && keystoreProperties.isEmpty()) {
        throw GradleException(
            "Release build without android/key.properties: refusing to sign with the debug key.",
        )
    }
}

// A debug build is for a 64-bit phone or the emulator: Whisper and RustO
// are built for arm64-v8a only there, which halves the native build.
androidComponents {
    onVariants(selector().withBuildType("debug")) { variant ->
        variant.externalNativeBuild?.abiFilters?.set(setOf("arm64-v8a"))
    }
    onVariants { variant ->
        val ocr = rootProject.layout.projectDirectory.dir("ocr")
        val task = tasks.register<CargoNdk>(
            "cargoNdk" + variant.name.replaceFirstChar { it.uppercase() },
        ) {
            crate.set(ocr)
            sources.from(ocr.dir("src"), ocr.file("Cargo.toml"), ocr.file("Cargo.lock"))
            abis.set(
                if (variant.buildType == "debug") {
                    listOf("arm64-v8a")
                } else {
                    listOf("arm64-v8a", "armeabi-v7a")
                },
            )
            platform.set(flutter.minSdkVersion)
            ndk.set(androidComponents.sdkComponents.ndkDirectory.map { it.asFile.absolutePath })
            targetDir.set(layout.buildDirectory.dir("ocr/target"))
        }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(task, CargoNdk::outputDir)
    }
}

/**
 * RustO for Photo Texts (ADR 0007): the Rust library in android/ocr,
 * cross-built with cargo-ndk into liboldbrainfix_ocr.so per ABI. Always
 * optimised, in debug builds too: RustO is far too slow otherwise. At most
 * two compile jobs, so the build never takes the machine down. Up to
 * date, and not run, while the crate's sources and lock file stay the
 * same; cargo's own target directory keeps the rest incremental.
 */
abstract class CargoNdk @Inject constructor(
    private val exec: ExecOperations,
    private val files: FileSystemOperations,
) : DefaultTask() {
    @get:Internal
    abstract val crate: DirectoryProperty

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Input
    abstract val abis: ListProperty<String>

    @get:Input
    abstract val platform: Property<Int>

    @get:Input
    abstract val ndk: Property<String>

    @get:Internal
    abstract val targetDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val target = targetDir.get().asFile
        exec.exec {
            workingDir = crate.get().asFile
            environment("ANDROID_NDK_HOME", ndk.get())
            environment("CARGO_TARGET_DIR", target.absolutePath)
            commandLine(
                listOf("cargo", "ndk", "--platform", platform.get().toString()) +
                    abis.get().flatMap { listOf("--target", it) } +
                    listOf("build", "--release", "--locked", "-j", "2"),
            )
        }
        files.sync {
            for (abi in abis.get()) {
                from(File(target, "${TRIPLES.getValue(abi)}/release/liboldbrainfix_ocr.so")) {
                    into(abi)
                }
            }
            into(outputDir)
        }
    }

    companion object {
        /** The Rust target of each Android ABI built. */
        val TRIPLES = mapOf(
            "arm64-v8a" to "aarch64-linux-android",
            "armeabi-v7a" to "armv7-linux-androideabi",
        )
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

dependencies {
    // FileProvider for the camera app; already part of Flutter's own
    // dependencies, named here because the app uses it directly.
    implementation("androidx.core:core:1.13.1")
    // The pair code's QR read from a photo: zxing's pure-Java reader,
    // no Google service and no dependencies of its own.
    implementation("com.google.zxing:core:3.5.3")
    // The background sync's periodic work. WorkManager alone: no
    // Google Play services (not work-gcm).
    implementation("androidx.work:work-runtime:2.10.1")
    // The shared sync folder through the document tree the picker
    // grants (Storage Access Framework); AndroidX only.
    implementation("androidx.documentfile:documentfile:1.0.1")
    testImplementation("junit:junit:4.13.2")
    // The JSON the unit tests read stored values with: android.jar's
    // org.json is only a stub off the device.
    testImplementation("org.json:json:20250517")
}
