package beatdetection

import com.bitwig.extension.controller.ControllerExtension
import com.bitwig.extension.controller.api.Application
import com.bitwig.extension.controller.api.ControllerHost
import com.bitwig.extension.controller.api.CursorDeviceFollowMode
import com.bitwig.extension.controller.api.SettableStringValue
import com.bitwig.extension.controller.api.Settings
import java.util.Locale
import kotlin.random.Random

/** Runtime extension that receives plugin tempo messages and applies them to Bitwig transport. */
public class BeatDetectionExtension(
    definition: BeatDetectionExtensionDefinition,
    host: ControllerHost,
) : ControllerExtension(definition, host) {
    override fun init() {
        val application = host.createApplication()
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

        val uiSurfaceProbe = UiSurfaceProbe.create(host, application, port)
        uiSurfaceProbe.logPanelActionCandidates()

        connection.setClientConnectCallback { remoteClient ->
            uiSurfaceProbe.markBridgeConnected()
            remoteClient.setReceiveCallback { bytes ->
                val bpm = TempoControllerFrame.readBpm(bytes)
                uiSurfaceProbe.markBpmReceived(bpm)
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
                uiSurfaceProbe.markPluginFound()
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
        const val PROBE_SETTINGS_CATEGORY = "Midi BPM Detection"
        const val PROBE_STATUS_TEXT_CHARS = 96
        const val PANEL_ACTION_CANDIDATE_LIMIT = 40
        const val NO_PANEL_ACTION_CANDIDATES = "No panel action candidates matched the probe keywords."
        const val PROBE_LOG_PREFIX = "[Midi BPM Detection UI probe]"
        val PANEL_ACTION_KEYWORDS =
            setOf(
                "monitor",
                "studio i/o",
                "studio io",
                "input",
                "output",
                "controller",
                "panel",
            )
    }

    private class UiSurfaceProbe(
        private val host: ControllerHost,
        private val application: Application,
        private val documentPresence: SettableStringValue,
        private val documentBridge: SettableStringValue,
        private val preferencesPresence: SettableStringValue,
        private val preferencesBridge: SettableStringValue,
    ) {
        fun markPluginFound() {
            writeStatus("Plugin selected; DAW Port written")
        }

        fun markBridgeConnected() {
            writeBridgeStatus("Bridge socket connected")
        }

        fun markBpmReceived(bpm: Float) {
            writeBridgeStatus("Received BPM ${"%.2f".format(Locale.ROOT, bpm)}")
        }

        fun logPanelActionCandidates() {
            val candidates =
                application
                    .getActions()
                    .filter { action -> action.matchesPanelProbe() }
                    .take(PANEL_ACTION_CANDIDATE_LIMIT)

            if (candidates.isEmpty()) {
                host.println("$PROBE_LOG_PREFIX $NO_PANEL_ACTION_CANDIDATES")
                return
            }

            candidates.forEach { action ->
                val category = action.category?.name.orEmpty()
                host.println(
                    "$PROBE_LOG_PREFIX action id='${action.id}' name='${action.name}' " +
                        "menu='${action.menuItemText}' category='$category'",
                )
            }
        }

        private fun writeStatus(status: String) {
            documentPresence.set(status)
            preferencesPresence.set(status)
        }

        private fun writeBridgeStatus(status: String) {
            documentBridge.set(status)
            preferencesBridge.set(status)
        }

        private fun com.bitwig.extension.controller.api.Action.matchesPanelProbe(): Boolean {
            val category = category?.name.orEmpty()
            val haystack =
                listOf(id, name, menuItemText, category)
                    .joinToString(separator = " ")
                    .lowercase(Locale.ROOT)

            return PANEL_ACTION_KEYWORDS.any { keyword -> keyword in haystack }
        }

        companion object {
            fun create(
                host: ControllerHost,
                application: Application,
                port: Int,
            ): UiSurfaceProbe {
                val initialPresence = "Loaded with API 25; waiting for selected plugin"
                val initialBridge = "Listening on localhost:$port"
                val documentState = host.getDocumentState()
                val preferences = host.getPreferences()

                val probe =
                    UiSurfaceProbe(
                        host = host,
                        application = application,
                        documentPresence = documentState.createProbeSetting("Document extension status", initialPresence),
                        documentBridge = documentState.createProbeSetting("Document bridge status", initialBridge),
                        preferencesPresence = preferences.createProbeSetting("Preferences extension status", initialPresence),
                        preferencesBridge = preferences.createProbeSetting("Preferences bridge status", initialBridge),
                    )

                preferences
                    .getSignalSetting("Log panel action candidates", PROBE_SETTINGS_CATEGORY, "Log")
                    .addSignalObserver { probe.logPanelActionCandidates() }

                return probe
            }

            private fun Settings.createProbeSetting(
                label: String,
                initialValue: String,
            ): SettableStringValue =
                getStringSetting(
                    label,
                    PROBE_SETTINGS_CATEGORY,
                    PROBE_STATUS_TEXT_CHARS,
                    initialValue,
                )
        }
    }
}
