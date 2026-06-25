import org.jetbrains.kotlin.gradle.dsl.ExplicitApiMode
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.dsl.KotlinVersion

plugins {
    alias(libs.plugins.kotlin.jvm) apply false
    alias(libs.plugins.spotless) apply false
    alias(libs.plugins.detekt) apply false
}

allprojects {
    group = "beatdetection"
    version = "0.1.0"
}

val packageExtensions = tasks.register("packageExtensions") {
    group = "bitwig"
    description = "Packages all Bitwig extension outputs."
}

tasks.register("packageBitwigExtension") {
    group = "bitwig"
    description = "Packages the Beat Detection Bitwig extension."
    dependsOn(packageExtensions)
}

subprojects {
    plugins.withId("org.jetbrains.kotlin.jvm") {
        extensions.configure<org.jetbrains.kotlin.gradle.dsl.KotlinJvmProjectExtension>("kotlin") {
            jvmToolchain(17)
            explicitApi = ExplicitApiMode.Strict
            compilerOptions {
                allWarningsAsErrors.set(true)
                extraWarnings.set(true)
                progressiveMode.set(true)
                jvmTarget.set(JvmTarget.JVM_17)
                languageVersion.set(KotlinVersion.KOTLIN_2_4)
                apiVersion.set(KotlinVersion.KOTLIN_2_4)
            }
        }

        dependencies.add("testImplementation", libs.junit.jupiter)
        dependencies.add("testRuntimeOnly", libs.junit.platform.launcher)

        tasks.withType<Test>().configureEach {
            useJUnitPlatform()
        }
    }

    plugins.withId("com.diffplug.spotless") {
        extensions.configure<com.diffplug.gradle.spotless.SpotlessExtension>("spotless") {
            kotlin {
                ktlint()
                target("src/**/*.kt")
            }
            kotlinGradle {
                ktlint()
                target("*.gradle.kts")
            }
        }
    }

    plugins.withId("dev.detekt") {
        extensions.configure<dev.detekt.gradle.extensions.DetektExtension>("detekt") {
            toolVersion = libs.versions.detekt.get()
            config.setFrom(rootProject.files("config/detekt/detekt.yml"))
            buildUponDefaultConfig = true
            allRules = true
            ignoreFailures = false
            parallel = true
        }
    }

    if (path.startsWith(":extensions:")) {
        afterEvaluate {
            val packageTask =
                tasks.findByName("verifyBitwigExtensionArchiveContents")
                    ?: tasks.findByName("packageBitwigExtension")

            packageTask?.let { extensionPackageTask ->
                rootProject.tasks.named("packageExtensions") {
                    dependsOn(extensionPackageTask)
                }
            }
        }
    }
}
