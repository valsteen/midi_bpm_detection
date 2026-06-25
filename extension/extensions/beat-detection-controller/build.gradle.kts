import java.util.Properties
import java.util.zip.ZipFile

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.spotless)
    alias(libs.plugins.detekt)
}

dependencies {
    implementation(project(":libs:bitwig-bootstrap"))
    compileOnly(libs.bitwig.extension.api)
    testImplementation(libs.bitwig.extension.api)
}

tasks.jar {
    archiveBaseName.set("beat-detection-controller")
    archiveExtension.set("bwextension")
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    from(
        configurations.runtimeClasspath.map { runtimeClasspath ->
            runtimeClasspath.map { dependency ->
                if (dependency.isDirectory) {
                    dependency
                } else {
                    zipTree(dependency)
                }
            }
        },
    )
    manifest {
        attributes["Main-Class"] = "beatdetection.BeatDetectionExtensionDefinition"
    }
}

val bitwigExtensionArchiveName = "BeatDetectionExtension.bwextension"
val bitwigExtensionOutputDirectory = layout.buildDirectory.dir("bitwig-extension")
val bitwigExtensionArchiveFile = layout.buildDirectory.file("bitwig-extension/$bitwigExtensionArchiveName")

val packageBitwigExtension =
    tasks.register<Sync>("packageBitwigExtension") {
        group = "bitwig"
        description = "Packages the Beat Detection Bitwig controller extension."
        dependsOn(tasks.jar)
        from(tasks.jar)
        into(bitwigExtensionOutputDirectory)
        rename { bitwigExtensionArchiveName }
        inputs.property("bitwigExtensionArchiveName", bitwigExtensionArchiveName)
        outputs.file(bitwigExtensionArchiveFile)
    }

val verifyBitwigExtensionArchiveContents =
    tasks.register("verifyBitwigExtensionArchiveContents") {
        group = "verification"
        description = "Verifies that the packaged extension archive contains runtime classes."
        dependsOn(packageBitwigExtension)

        inputs.file(bitwigExtensionArchiveFile)

        doLast {
            val requiredEntries =
                listOf(
                    "beatdetection/BeatDetectionExtension.class",
                    "beatdetection/BeatDetectionExtensionDefinition.class",
                    "beatdetection/TempoControllerFrame.class",
                    "beatdetection/bootstrap/ExtensionIdentity.class",
                )
            val forbiddenEntriesPrefix = "com/bitwig/extension/"

            ZipFile(bitwigExtensionArchiveFile.get().asFile).use { archive ->
                val archiveEntries =
                    archive
                        .entries()
                        .asSequence()
                        .map { it.name }
                        .toSet()

                requiredEntries.forEach { entry ->
                    check(entry in archiveEntries) {
                        "Packaged Bitwig extension is missing required runtime entry: $entry"
                    }
                }

                check(archiveEntries.none { it.startsWith(forbiddenEntriesPrefix) }) {
                    "Packaged Bitwig extension must not bundle Bitwig API classes."
                }
            }
        }
    }

val localBitwigExtensionsDirectory =
    providers.provider {
        val propertiesFile =
            rootProject.layout.projectDirectory
                .file("gradle-local.properties")
                .asFile
        if (!propertiesFile.isFile) {
            null
        } else {
            val properties =
                Properties().apply {
                    propertiesFile.inputStream().use(::load)
                }
            properties.getProperty("bitwigExtensionsDir")?.takeIf { it.isNotBlank() }
        }
    }

val bitwigExtensionsDirectory =
    providers
        .gradleProperty("bitwigExtensionsDir")
        .orElse(providers.environmentVariable("BITWIG_EXTENSIONS_DIR"))
        .orElse(localBitwigExtensionsDirectory)
        .orElse(providers.systemProperty("user.home").map { "$it/Documents/Bitwig Studio/Extensions" })

tasks.register("printBitwigExtensionInstallDirectory") {
    group = "bitwig"
    description = "Prints the resolved local Bitwig extension install directory."

    doLast {
        println(bitwigExtensionsDirectory.get())
    }
}

tasks.register<Copy>("installBitwigExtension") {
    group = "bitwig"
    description = "Installs the Beat Detection Bitwig controller extension into the user Bitwig extensions directory."
    dependsOn(tasks.test)
    dependsOn(verifyBitwigExtensionArchiveContents)
    from(bitwigExtensionOutputDirectory)
    into(bitwigExtensionsDirectory.map { file(it) })

    doFirst {
        println("Installing Bitwig extension into: ${bitwigExtensionsDirectory.get()}")
    }
}
