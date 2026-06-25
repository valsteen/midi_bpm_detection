plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.spotless)
    alias(libs.plugins.detekt)
}

dependencies {
    compileOnly(libs.bitwig.extension.api)
}
