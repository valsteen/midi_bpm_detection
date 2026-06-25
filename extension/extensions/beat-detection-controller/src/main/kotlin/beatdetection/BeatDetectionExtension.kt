package beatdetection

import com.bitwig.extension.controller.ControllerExtension
import com.bitwig.extension.controller.api.ControllerHost
import com.bitwig.extension.controller.api.CursorDeviceFollowMode
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

        connection.setClientConnectCallback { remoteClient ->
            remoteClient.setReceiveCallback { bytes ->
                val bpm = TempoControllerFrame.readBpm(bytes)
                transport.tempo().value().raw = bpm.toDouble()
            }
        }

        cursor.addDirectParameterIdObserver { ids ->
            if (!ids.any { id -> id?.split("/")?.last() == inputPortParameter?.split("/")?.last() }) {
                cursorTrack.isPinned.set(false)
                cursor.isPinned.set(false)
            }
        }

        cursor.addDirectParameterNameObserver(MAX_DIRECT_PARAMETERS) { id, name ->
            if (name == DAW_PORT_PARAMETER_NAME) {
                inputPortParameter = id
                cursorTrack.isPinned.set(true)
                cursor.isPinned.set(true)
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
