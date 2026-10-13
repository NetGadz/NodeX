package network.nodex.messenger.bridge

object RustBridge {
    private var isNativeLoaded = false

    init {
        try {
            System.loadLibrary("nodex_core")
            isNativeLoaded = true
        } catch (e: UnsatisfiedLinkError) {
            // Running in standalone APK without compiled .so library
            isNativeLoaded = false
        }
    }

    fun isNativeReady(): Boolean = isNativeLoaded

    // Native JNI functions (defined in core-ffi)
    external fun startNode(ip: String, port: Int, bootstrap: String): Long
    external fun stopNode(handle: Long)
    external fun sendMessage(handle: Long, recipientId: String, text: String): Boolean
    external fun getInviteLink(handle: Long): String
}
