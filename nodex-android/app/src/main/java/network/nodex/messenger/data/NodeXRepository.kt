package network.nodex.messenger.data

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

object NodeXRepository {
    private val _networkStatus = MutableStateFlow(
        NetworkStatus(
            isConnected = true,
            peerCount = 14,
            bootstrapAddr = "31.77.71.122:8443",
            is2HopOnionActive = true,
            natType = "Full Cone (STUN Hole-Punching)"
        )
    )
    val networkStatus: StateFlow<NetworkStatus> = _networkStatus.asStateFlow()

    private val _contacts = MutableStateFlow(
        listOf(
            NodeContact(
                userIdHex = "a1c9e88d2f10b740529e",
                name = "Алиса (Kademlia Peer)",
                bio = "Zero metadata • Pure P2P",
                lastMessageText = "Слушай голосовое, отправил через 2-Hop Onion!",
                lastMessageTime = "22:45",
                unreadCount = 1,
                isOnline = true
            ),
            NodeContact(
                userIdHex = "b4f2c019d38aa72e09b1",
                name = "Боб (Relay Node)",
                bio = "Genesis Validator",
                lastMessageText = "Пакет доставлен через STUN Fast-Path (45ms)",
                lastMessageTime = "21:30",
                unreadCount = 0,
                isOnline = true
            ),
            NodeContact(
                userIdHex = "c791d200e47ff311a884",
                name = "Кибер-Стеганография",
                bio = "LSB Carrier Channel",
                lastMessageText = "Инвайт-паспорт успешно извлечен из PNG",
                lastMessageTime = "Вчера",
                unreadCount = 0,
                isOnline = false
            )
        )
    )
    val contacts: StateFlow<List<NodeContact>> = _contacts.asStateFlow()

    private val _messages = MutableStateFlow<Map<String, List<ChatMessage>>>(
        mapOf(
            "a1c9e88d2f10b740529e" to listOf(
                ChatMessage(
                    id = "1",
                    senderIdHex = "a1c9e88d2f10b740529e",
                    text = "Привет! Как там подключение к нашему новому бутстрапу 31.77.71.122?",
                    isIncoming = true,
                    timestampStr = "22:42"
                ),
                ChatMessage(
                    id = "2",
                    senderIdHex = "self",
                    text = "Всё работает идеально! STUN Hole-Punching пробивает роутер за 45ms напрямую ⚡",
                    isIncoming = false,
                    timestampStr = "22:43"
                ),
                ChatMessage(
                    id = "3",
                    senderIdHex = "a1c9e88d2f10b740529e",
                    text = "Отлично. Голосовые сообщения тоже работают без серверов?",
                    isIncoming = true,
                    timestampStr = "22:44"
                ),
                ChatMessage(
                    id = "4",
                    senderIdHex = "self",
                    text = "",
                    isIncoming = false,
                    timestampStr = "22:45",
                    isVoice = true,
                    voiceDurationSecs = 14,
                    waveform = listOf(0.3f, 0.6f, 0.9f, 0.4f, 0.8f, 0.6f, 0.3f, 0.7f, 0.9f, 0.5f, 0.3f)
                )
            )
        )
    )
    val messages: StateFlow<Map<String, List<ChatMessage>>> = _messages.asStateFlow()

    val myNodeId = "4798658e62111f203a10a351f091845c7190ddde"
    val myInviteLink = "nodex://invite/4798658e62111f203a10a351f091845c7190ddde"

    fun addContact(inviteOrId: String, name: String) {
        val newContact = NodeContact(
            userIdHex = inviteOrId.take(16),
            name = name,
            bio = "Verified NodeX Contact",
            lastMessageText = "Чат создан",
            lastMessageTime = "Только что",
            unreadCount = 0,
            isOnline = true
        )
        _contacts.value = listOf(newContact) + _contacts.value
    }

    fun sendMessage(contactId: String, text: String) {
        val currentList = _messages.value[contactId] ?: emptyList()
        val newMsg = ChatMessage(
            id = System.currentTimeMillis().toString(),
            senderIdHex = "self",
            text = text,
            isIncoming = false,
            timestampStr = "Только что"
        )
        _messages.value = _messages.value + (contactId to (currentList + newMsg))
    }
}
