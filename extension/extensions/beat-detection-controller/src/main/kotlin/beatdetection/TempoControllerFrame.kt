package beatdetection

import java.nio.ByteBuffer

internal object TempoControllerFrame {
    fun readBpm(payload: ByteArray): Float = ByteBuffer.wrap(payload).float
}
