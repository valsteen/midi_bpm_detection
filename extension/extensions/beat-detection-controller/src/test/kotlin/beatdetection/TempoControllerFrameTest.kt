package beatdetection

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

internal class TempoControllerFrameTest {
    @Test
    fun readsBigEndianBpmPayload() {
        val payload = byteArrayOf(0x42, 0xF7.toByte(), 0x00, 0x00)

        assertEquals(123.5f, TempoControllerFrame.readBpm(payload))
    }
}
