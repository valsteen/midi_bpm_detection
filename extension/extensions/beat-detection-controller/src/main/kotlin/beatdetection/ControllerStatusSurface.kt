package beatdetection

internal fun interface StatusControl {
    fun set(value: String)
}

internal interface EnumStatusValue {
    fun set(value: String)
}

internal fun EnumStatusValue.statusControl(): StatusControl = StatusControl { value -> set(value) }

internal class ControllerStatusSurface internal constructor(
    private val status: StatusControl,
) {
    private var pluginFound = false
    private var currentStatus: String? = null

    init {
        markPluginWaiting()
    }

    internal fun markPluginWaiting() {
        pluginFound = false
        updateStatus(WAITING_FOR_PLUGIN)
    }

    internal fun markPluginFound() {
        pluginFound = true
        updateStatus(PLUGIN_FOUND_WAITING_FOR_CONNECTION)
    }

    internal fun markBridgeConnected() {
        updateStatus(BRIDGE_CONNECTED)
    }

    internal fun markBridgeDisconnected() {
        if (pluginFound) {
            updateStatus(PLUGIN_FOUND_WAITING_FOR_CONNECTION)
        } else {
            updateStatus(WAITING_FOR_PLUGIN)
        }
    }

    internal fun markBpmReceived() {
        markBridgeConnected()
    }

    private fun updateStatus(value: String) {
        if (value == currentStatus) {
            return
        }

        currentStatus = value
        status.set(value)
    }

    private companion object {
        const val WAITING_FOR_PLUGIN = "Waiting for Plugin"
        const val PLUGIN_FOUND_WAITING_FOR_CONNECTION = "Plugin found; waiting for connection"
        const val BRIDGE_CONNECTED = "Plugin connected"
    }
}

internal fun trackedDawPortDisappeared(
    trackedParameterId: String?,
    directParameterIds: Array<out String?>,
): Boolean {
    if (trackedParameterId == null) {
        return false
    }

    val trackedParameterName = trackedParameterId.parameterName()
    return directParameterIds.none { id -> id?.parameterName() == trackedParameterName }
}

private fun String.parameterName(): String = split("/").last()
