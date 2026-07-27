pluginManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
}

plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        mavenCentral()
        maven("https://maven.bitwig.com")
    }
}

rootProject.name = "midi-bpm-detector-bitwig"

include(":libs:bitwig-bootstrap")
include(":extensions:beat-detection-controller")
