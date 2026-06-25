package beatdetection

import beatdetection.bootstrap.ExtensionIdentity
import beatdetection.bootstrap.requireValid
import com.bitwig.extension.api.PlatformType
import com.bitwig.extension.controller.AutoDetectionMidiPortNamesList
import com.bitwig.extension.controller.ControllerExtensionDefinition
import com.bitwig.extension.controller.api.ControllerHost
import java.util.UUID

/** Bitwig extension definition for the MIDI BPM Detection tempo bridge. */
public class BeatDetectionExtensionDefinition : ControllerExtensionDefinition() {
    private val identity =
        ExtensionIdentity(
            id = UUID.fromString("bb70b7dc-a900-46ea-8b50-611234df35e2"),
            name = "Beat Detection Bitwig Extension",
            author = "Midi BPM Detection",
            version = "0.1",
            hardwareVendor = "Midi BPM Detection",
            hardwareModel = "Beat Detection Bitwig Extension",
            requiredApiVersion = 2,
        ).requireValid()

    override fun getName(): String = identity.name

    override fun getAuthor(): String = identity.author

    override fun getVersion(): String = identity.version

    override fun getId(): UUID = identity.id

    override fun getHardwareVendor(): String = identity.hardwareVendor

    override fun getHardwareModel(): String = identity.hardwareModel

    override fun getRequiredAPIVersion(): Int = identity.requiredApiVersion

    override fun getNumMidiInPorts(): Int = 0

    override fun getNumMidiOutPorts(): Int = 0

    override fun listAutoDetectionMidiPortNames(
        _list: AutoDetectionMidiPortNamesList,
        _platformType: PlatformType,
    ): Unit = Unit

    override fun createInstance(host: ControllerHost): BeatDetectionExtension = BeatDetectionExtension(this, host)
}
