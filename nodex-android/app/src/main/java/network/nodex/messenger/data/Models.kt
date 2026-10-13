package network.nodex.messenger.data

data class NodeContact(
    val userIdHex: String,
    val name: String,
    val bio: String = "",
    val lastSeenAddr: String = "",
    val isOnline: Boolean = true,
    val unreadCount: Int = 0,
    val lastMessageText: String = "",
    val lastMessageTime: String = ""
)

data class ChatMessage(
    val id: String,
    val senderIdHex: String,
    val text: String,
    val isIncoming: Boolean,
    val timestampStr: String,
    val isDelivered: Boolean = true,
    val isRead: Boolean = true,
    val isVoice: Boolean = false,
    val voiceDurationSecs: Int = 0,
    val waveform: List<Float> = emptyList()
)

data class NetworkStatus(
    val isConnected: Boolean = true,
    val peerCount: Int = 14,
    val bootstrapAddr: String = "31.77.71.122:8443",
    val is2HopOnionActive: Boolean = true,
    val natType: String = "Full Cone (P2P Direct)"
)
