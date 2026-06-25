package beatdetection.bootstrap

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test
import java.util.UUID

internal class ExtensionIdentityTest {
    @Test
    fun validIdentityRoundTrips() {
        val identity =
            ExtensionIdentity(
                id = UUID.fromString("bb70b7dc-a900-46ea-8b50-611234df35e2"),
                name = "Beat Detection Bitwig Extension",
                author = "Midi BPM Detection",
                version = "0.1",
                hardwareVendor = "Midi BPM Detection",
                hardwareModel = "Beat Detection Bitwig Extension",
                requiredApiVersion = 2,
            )

        assertEquals(identity, identity.requireValid())
    }

    @Test
    fun blankNameIsRejected() {
        val identity =
            ExtensionIdentity(
                id = UUID.fromString("bb70b7dc-a900-46ea-8b50-611234df35e2"),
                name = " ",
                author = "Midi BPM Detection",
                version = "0.1",
                hardwareVendor = "Midi BPM Detection",
                hardwareModel = "Beat Detection Bitwig Extension",
                requiredApiVersion = 2,
            )

        assertThrows(IllegalArgumentException::class.java) {
            identity.requireValid()
        }
    }
}
