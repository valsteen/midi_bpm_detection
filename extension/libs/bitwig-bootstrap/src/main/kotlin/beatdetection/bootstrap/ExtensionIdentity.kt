package beatdetection.bootstrap

import java.util.UUID

/**
 * Stable Bitwig extension metadata used during bootstrap and packaging.
 *
 * @property id Extension UUID expected by Bitwig.
 * @property name Human-readable extension name.
 * @property author Extension author shown by Bitwig.
 * @property version Extension version string.
 * @property hardwareVendor Hardware vendor name reported to Bitwig.
 * @property hardwareModel Hardware model name reported to Bitwig.
 * @property requiredApiVersion Minimum Bitwig controller API version required by the extension.
 */
public data class ExtensionIdentity(
    public val id: UUID,
    public val name: String,
    public val author: String,
    public val version: String,
    public val hardwareVendor: String,
    public val hardwareModel: String,
    public val requiredApiVersion: Int,
)

/** Validates required metadata fields and returns this identity when they are usable. */
public fun ExtensionIdentity.requireValid(): ExtensionIdentity {
    require(name.isNotBlank()) { "Extension name must not be blank." }
    require(author.isNotBlank()) { "Extension author must not be blank." }
    require(version.isNotBlank()) { "Extension version must not be blank." }
    require(hardwareVendor.isNotBlank()) { "Hardware vendor must not be blank." }
    require(hardwareModel.isNotBlank()) { "Hardware model must not be blank." }
    require(requiredApiVersion > 0) { "Required API version must be positive." }

    return this
}
