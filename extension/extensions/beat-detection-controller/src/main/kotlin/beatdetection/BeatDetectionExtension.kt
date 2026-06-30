package beatdetection

import com.bitwig.extension.controller.ControllerExtension
import com.bitwig.extension.controller.api.ControllerHost
import com.bitwig.extension.controller.api.CursorDeviceFollowMode
import com.bitwig.extension.controller.api.SettableEnumValue
import com.bitwig.extension.controller.api.Settings
import kotlin.random.Random

/** Runtime extension that receives plugin tempo messages and applies them to Bitwig transport. */
public class BeatDetectionExtension(
    definition: BeatDetectionExtensionDefinition,
    host: ControllerHost,
) : ControllerExtension(definition, host) {
    override fun init() {
        val transport = host.createTransport()
        val cursorTrack = host.createCursorTrack(TRACK_CURSOR_ID, TRACK_CURSOR_ID, 0, 0, true)
        cursorTrack.subscribe()

        val cursor =
            cursorTrack.createCursorDevice(
                TRACK_CURSOR_ID,
                TRACK_CURSOR_ID,
                0,
                CursorDeviceFollowMode.FOLLOW_SELECTION,
            )
        cursor.subscribe()

        var inputPortParameter: String? = null
        var port = Random.nextInt(MIN_DYNAMIC_PORT, MAX_PORT_EXCLUSIVE)
        val connection = host.createRemoteConnection("BPM Receiver", port)

        if (connection.port > PORT_NOT_BOUND) {
            port = connection.port
        }

        val statusSurface = createStatusSurface(host.getDocumentState())

        connection.setClientConnectCallback { remoteClient ->
            statusSurface.markBridgeConnected()
            remoteClient.setDisconnectCallback {
                statusSurface.markBridgeDisconnected()
            }
            remoteClient.setReceiveCallback { bytes ->
                val bpm = TempoControllerFrame.readBpm(bytes)
                statusSurface.markBpmReceived(bpm)
                transport.tempo().value().raw = bpm.toDouble()
            }
        }

        cursor.addDirectParameterIdObserver { ids ->
            if (trackedDawPortDisappeared(inputPortParameter, ids)) {
                inputPortParameter = null
                cursorTrack.isPinned.set(false)
                cursor.isPinned.set(false)
                statusSurface.markPluginWaiting()
            }
        }

        cursor.addDirectParameterNameObserver(MAX_DIRECT_PARAMETERS) { id, name ->
            if (name == DAW_PORT_PARAMETER_NAME) {
                inputPortParameter = id
                cursorTrack.isPinned.set(true)
                cursor.isPinned.set(true)
                statusSurface.markPluginFound()
                cursor.setDirectParameterValueNormalized(inputPortParameter, port, MAX_PORT_EXCLUSIVE)
            }
        }
    }

    override fun exit(): Unit = Unit

    override fun flush(): Unit = Unit

    private companion object {
        const val DAW_PORT_PARAMETER_NAME = "DAW Port"
        const val TRACK_CURSOR_ID = "track"
        const val MAX_DIRECT_PARAMETERS = 20
        const val MIN_DYNAMIC_PORT = 1024
        const val MAX_PORT_EXCLUSIVE = 65_536
        const val PORT_NOT_BOUND = -1
    }
}

private const val STATUS_SETTINGS_CATEGORY = "Midi BPM Detection"

private fun createStatusSurface(settings: Settings): ControllerStatusSurface =
    ControllerStatusSurface(
        status =
            settings.createStatusEnum(
                label = "Extension status",
                options =
                    arrayOf(
                        "Waiting for Plugin",
                        "Plugin found; waiting for connection",
                        "Plugin connected",
                    ),
                initialValue = "Waiting for Plugin",
            ),
    )

private fun Settings.createStatusEnum(
    label: String,
    options: Array<String>,
    initialValue: String,
): StatusControl {
    val setting =
        getEnumSetting(
            label,
            STATUS_SETTINGS_CATEGORY,
            options,
            initialValue,
        )

    return setting.asEnumStatusValue().statusControl()
}

private fun SettableEnumValue.asEnumStatusValue(): EnumStatusValue =
    object : EnumStatusValue {
        override fun set(value: String) {
            this@asEnumStatusValue.set(value)
        }
    }
