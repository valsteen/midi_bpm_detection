package beatdetection.bootstrap

import com.bitwig.extension.controller.api.ControllerHost

/** Minimal logging sink for bootstrap code that needs to report human-readable status messages. */
public fun interface BitwigLogger {
    /** Emits an informational message to the configured logging backend. */
    public fun info(message: String)
}

/** Adapts a Bitwig controller host into a logger that prefixes each line with the supplied label. */
public fun ControllerHost.asBitwigLogger(prefix: String): BitwigLogger {
    require(prefix.isNotBlank()) { "Log prefix must not be blank." }

    return BitwigLogger { message ->
        this.println("[$prefix] $message")
    }
}
