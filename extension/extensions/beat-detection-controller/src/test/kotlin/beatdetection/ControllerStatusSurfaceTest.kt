package beatdetection

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

internal class ControllerStatusSurfaceTest {
    @Test
    fun reportsPluginAndBridgeLifecycle() {
        val status = RecordingStatusControl()
        val surface =
            ControllerStatusSurface(
                status = status,
            )

        assertEquals("Waiting for Plugin", status.value)

        surface.markPluginFound()
        assertEquals("Plugin found; waiting for connection", status.value)

        surface.markBridgeConnected()
        surface.markBpmReceived(126.456f)
        assertEquals("Plugin connected", status.value)

        surface.markBridgeDisconnected()
        assertEquals("Plugin found; waiting for connection", status.value)

        surface.markPluginWaiting()
        assertEquals("Waiting for Plugin", status.value)
    }

    @Test
    fun doesNotRewriteUnchangedConnectedStatus() {
        val status = RecordingStatusControl()
        val surface =
            ControllerStatusSurface(
                status = status,
            )

        surface.markBridgeConnected()
        surface.markBpmReceived(126.456f)
        surface.markBpmReceived(127.0f)

        assertEquals(listOf("Waiting for Plugin", "Plugin connected"), status.values)
    }

    @Test
    fun identifiesWhenTrackedDawPortDisappears() {
        assertFalse(
            trackedDawPortDisappeared(
                trackedParameterId = "track/device/daw-port",
                directParameterIds = arrayOf("other/device/daw-port"),
            ),
        )

        assertTrue(
            trackedDawPortDisappeared(
                trackedParameterId = "track/device/daw-port",
                directParameterIds = arrayOf("track/device/send-tempo", "track/device/output"),
            ),
        )

        assertFalse(
            trackedDawPortDisappeared(
                trackedParameterId = null,
                directParameterIds = arrayOf("track/device/send-tempo"),
            ),
        )
    }

    @Test
    fun doesNotImmediatelyRestoreUserSelectedEnumValue() {
        val enum = RecordingEnumStatusValue()
        val control = enum.statusControl()

        control.set("Connected")
        enum.simulateUserSelection("Listening")

        assertEquals(listOf("Connected", "Listening"), enum.values)
    }

    private class RecordingStatusControl : StatusControl {
        val values = mutableListOf<String>()
        var value: String = ""
            private set

        override fun set(value: String) {
            values += value
            this.value = value
        }
    }

    private class RecordingEnumStatusValue : EnumStatusValue {
        val values = mutableListOf<String>()

        override fun set(value: String) {
            values += value
        }

        fun simulateUserSelection(value: String) {
            values += value
        }
    }
}
