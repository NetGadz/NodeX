package network.nodex.messenger.data

enum class ChatTheme(
    val title: String,
    val bgHex: Long,
    val surfaceHex: Long,
    val primaryHex: Long,
    val bubbleOutHex: Long,
    val bubbleInHex: Long
) {
    GRAPHITE("Тёмный Графит", 0xFF080B12, 0xFF111725, 0xFF7C8CFF, 0xFF273B72, 0xFF151D2B),
    MIDNIGHT("Полночь (Navy)", 0xFF09101C, 0xFF111C2B, 0xFF8BB8FF, 0xFF1D3554, 0xFF172235),
    EMERALD("Изумруд (Nord)", 0xFF08120F, 0xFF101F19, 0xFF80E0B3, 0xFF164237, 0xFF14251E),
    AMETHYST("Аметист (Dark)", 0xFF100D18, 0xFF1A1526, 0xFFC2A0FF, 0xFF3A2A58, 0xFF211A2E)
}

enum class ChatType {
    DIRECT,
    GROUP,
    CHANNEL
}

data class NodeIdentity(
    val nodeId: String,
    val name: String,
    val bio: String = "На связи в NodeX",
    val mnemonic: String = "",
    val createdAt: Long = System.currentTimeMillis()
)

data class NodeContact(
    val userIdHex: String,
    val name: String,
    val bio: String = "",
    val endpointAddr: String = "",
    val isOnline: Boolean = false,
    val isSelf: Boolean = false,
    val unreadCount: Int = 0,
    val lastMessageText: String = "",
    val lastMessageTime: String = "",
    val chatType: ChatType = ChatType.DIRECT,
    val ownerIdHex: String = "",
    val description: String = "",
    val membersCount: Int = 1,
    val isChannel: Boolean = false,
    val isGroup: Boolean = false,
    val adminOnlyPosting: Boolean = false,
    val mutedMembers: List<String> = emptyList(),
    val bannedMembers: List<String> = emptyList(),
    val ed25519PublicKeyHex: String = "",
    val x25519PublicKeyHex: String = "",
    val isIdentityVerified: Boolean = false
)

data class ChatMessage(
    val id: String,
    val senderIdHex: String,
    val senderName: String = "",
    val text: String,
    val isIncoming: Boolean,
    val timestampStr: String,
    val isDelivered: Boolean = true,
    val isRead: Boolean = true,
    val isVoice: Boolean = false,
    val voiceDurationSecs: Int = 0,
    val waveform: List<Float> = emptyList(),
    val audioFilePath: String = "",
    val isImage: Boolean = false,
    val imageFilePath: String = "",
    val isFile: Boolean = false,
    val filePath: String = "",
    val fileName: String = "",
    val fileSizeFormatted: String = "",
    val attachmentId: String = "",
    val attachmentSizeBytes: Long = 0L,
    val attachmentMimeType: String = ""
) {
    val isOutgoing: Boolean get() = !isIncoming
}

data class CallState(
    val peerId: String,
    val peerName: String,
    val isIncoming: Boolean,
    val isConnected: Boolean = false,
    val isConnecting: Boolean = false,
    val durationSecs: Int = 0,
    val isMuted: Boolean = false,
    val isSpeakerOn: Boolean = false,
    val callId: String = ""
)

data class NetworkStatus(
    val isConnected: Boolean = false,
    val latencyMs: Long = 0,
    val bootstrapAddr: String = "31.77.71.122:8443",
    val peerCount: Int = 1,
    val is2HopOnionActive: Boolean = true,
    val isSecureTunnelActive: Boolean = true
)

data class DeviceSession(
    val id: String,
    val name: String,
    val appVersion: String,
    val ip: String,
    val isCurrent: Boolean = false
)

data class PushNotificationData(
    val id: String,
    val senderId: String,
    val senderName: String,
    val text: String,
    val timestamp: Long = System.currentTimeMillis()
)
