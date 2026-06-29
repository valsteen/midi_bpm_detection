package beatdetection

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Test
import kotlin.io.readText

internal class BeatDetectionExtensionMetadataTest {
    @Test
    fun metadataPointsToDefinition() {
        val resource =
            javaClass.classLoader
                .getResource("META-INF/services/com.bitwig.extension.ExtensionDefinition")

        assertNotNull(resource)
        assertEquals(
            "beatdetection.BeatDetectionExtensionDefinition",
            checkNotNull(resource).readText().trim(),
        )
    }

    @Test
    fun definitionNamesExtension() {
        val definition = BeatDetectionExtensionDefinition()

        assertEquals("Beat Detection Bitwig Extension", definition.name)
        assertEquals("Beat Detection Bitwig Extension", definition.hardwareModel)
        assertEquals(25, definition.requiredAPIVersion)
    }
}
