package network.nodex.messenger.data

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.media.RingtoneManager
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import network.nodex.messenger.MainActivity
import network.nodex.messenger.R
import network.nodex.messenger.crypto.MnemonicHelper
import network.nodex.messenger.crypto.AndroidMessageCrypto
import network.nodex.messenger.webrtc.WebRtcCallManager
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64

object NodeXRepository {
    private const val IDENTITY_KEY_ALIAS = "nodex_identity_v1"
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var audioReceiveJob: Job? = null
    @Volatile private var messageStoreWriteSucceeded = true

    private var appContext: Context? = null
    private var identityFile: File? = null
    private var contactsFile: File? = null
    private var messagesFile: File? = null
    private var prefsFile: File? = null
    private var accountDirectory: File? = null
    private var migratedLegacyNodeId: String? = null

    private val _isOnboarded = MutableStateFlow(false)
    val isOnboarded: StateFlow<Boolean> = _isOnboarded.asStateFlow()

    private val _identity = MutableStateFlow(
        NodeIdentity(
            nodeId = "",
            name = "",
            bio = "NodeX Messenger",
            mnemonic = "",
            createdAt = System.currentTimeMillis()
        )
    )
    val identity: StateFlow<NodeIdentity> = _identity.asStateFlow()

    private val _contacts = MutableStateFlow<List<NodeContact>>(emptyList())
    val contacts: StateFlow<List<NodeContact>> = _contacts.asStateFlow()

    private val _messages = MutableStateFlow<Map<String, List<ChatMessage>>>(emptyMap())
    val messages: StateFlow<Map<String, List<ChatMessage>>> = _messages.asStateFlow()

    private val _currentTheme = MutableStateFlow(ChatTheme.GRAPHITE)
    val currentTheme: StateFlow<ChatTheme> = _currentTheme.asStateFlow()

    private val _customAccentHex = MutableStateFlow<String?>(null)
    val customAccentHex: StateFlow<String?> = _customAccentHex.asStateFlow()

    private val _chatFontSize = MutableStateFlow(14.5f)
    val chatFontSize: StateFlow<Float> = _chatFontSize.asStateFlow()

    private val _bubbleRounding = MutableStateFlow(16f)
    val bubbleRounding: StateFlow<Float> = _bubbleRounding.asStateFlow()

    private val _chatWallpaper = MutableStateFlow("solid")
    val chatWallpaper: StateFlow<String> = _chatWallpaper.asStateFlow()

    private val _soundEnabled = MutableStateFlow(true)
    val soundEnabled: StateFlow<Boolean> = _soundEnabled.asStateFlow()

    private val _vibrationEnabled = MutableStateFlow(true)
    val vibrationEnabled: StateFlow<Boolean> = _vibrationEnabled.asStateFlow()

    private val _pushNotificationsEnabled = MutableStateFlow(true)
    val pushNotificationsEnabled: StateFlow<Boolean> = _pushNotificationsEnabled.asStateFlow()

    private val _topPushNotification = MutableStateFlow<PushNotificationData?>(null)
    val topPushNotification: StateFlow<PushNotificationData?> = _topPushNotification.asStateFlow()

    private val _autoDownloadMedia = MutableStateFlow(true)
    val autoDownloadMedia: StateFlow<Boolean> = _autoDownloadMedia.asStateFlow()

    private val _autoDownloadVoice = MutableStateFlow(true)
    val autoDownloadVoice: StateFlow<Boolean> = _autoDownloadVoice.asStateFlow()

    private val _autoDeleteTimerSecs = MutableStateFlow(0L)
    val autoDeleteTimerSecs: StateFlow<Long> = _autoDeleteTimerSecs.asStateFlow()

    private val _p2pRelayEnabled = MutableStateFlow(true)
    val p2pRelayEnabled: StateFlow<Boolean> = _p2pRelayEnabled.asStateFlow()

    private val _appPasscode = MutableStateFlow<String?>(null)
    val appPasscode: StateFlow<String?> = _appPasscode.asStateFlow()

    private val _deviceSessions = MutableStateFlow<List<DeviceSession>>(emptyList())
    val deviceSessions: StateFlow<List<DeviceSession>> = _deviceSessions.asStateFlow()

    private val _networkStatus = MutableStateFlow(
        NetworkStatus(
            isConnected = false,
            latencyMs = 0,
            bootstrapAddr = "31.77.71.122:8443",
            peerCount = 1,
            is2HopOnionActive = false
        )
    )
    val networkStatus: StateFlow<NetworkStatus> = _networkStatus.asStateFlow()

    val myInviteLink: String
        get() = runCatching { AndroidMessageCrypto.createRustCompatibleInvite(_identity.value.mnemonic, _identity.value.name) }
            .getOrElse { "nodex://invite/${_identity.value.nodeId}" }

    fun initialize(context: Context) {
        if (appContext != null) return
        appContext = context.applicationContext

        val filesDir = context.filesDir
        identityFile = File(filesDir, "nodex_identity.json")
        prefsFile = File(filesDir, "nodex_prefs.json")

        loadIdentity()
        configureAccountStorage(_identity.value.nodeId, migrateLegacy = _identity.value.nodeId.isNotBlank())
        loadPreferences()
        loadDeviceSessions()
        loadContacts()
        loadMessages()
        startNetworkMonitor()
        if (_identity.value.nodeId.isNotBlank()) {
            P2PTransportEngine.start(context, _identity.value.nodeId)
            P2PTransportEngine.restorePendingMessages()
            audioReceiveJob?.cancel()
            audioReceiveJob = scope.launch {
                P2PTransportEngine.callAudioChunks.collect { frame ->
                    if (frame.size < 16) return@collect
                    val buffer = java.nio.ByteBuffer.wrap(frame).order(java.nio.ByteOrder.BIG_ENDIAN)
                    buffer.long // sequence; transport validates increasing order
                    val sampleRate = buffer.int
                    val size = buffer.int
                    if (sampleRate !in 8_000..48_000 || size < 0 || size != buffer.remaining() || size % 2 != 0) return@collect
                    val pcm = ShortArray(size / 2) { buffer.short }
                    WebRtcCallManager.playReceivedPcm(pcm, sampleRate)
                }
            }
        }
    }

    private fun loadPreferences() {
        val file = prefsFile ?: return
        if (!file.exists()) return
        try {
            val json = JSONObject(file.readText())
            val themeName = json.optString("theme", ChatTheme.GRAPHITE.name)
            _currentTheme.value = try { ChatTheme.valueOf(themeName) } catch (e: Exception) { ChatTheme.GRAPHITE }
            val accent = json.optString("custom_accent", "")
            _customAccentHex.value = if (accent.isNotBlank()) accent else null
            _chatFontSize.value = json.optDouble("chat_font_size", 14.5).toFloat()
            _bubbleRounding.value = json.optDouble("bubble_rounding", 16.0).toFloat()
            _chatWallpaper.value = json.optString("chat_wallpaper", "solid")
            _soundEnabled.value = json.optBoolean("sound", true)
            _vibrationEnabled.value = json.optBoolean("vibration", true)
            _pushNotificationsEnabled.value = json.optBoolean("push_notifications", true)
            _autoDownloadMedia.value = json.optBoolean("auto_media", true)
            _autoDownloadVoice.value = json.optBoolean("auto_voice", true)
            _autoDeleteTimerSecs.value = json.optLong("auto_delete_secs", 0L)
            _p2pRelayEnabled.value = json.optBoolean("p2p_relay", true)
            val pass = json.optString("app_passcode", "")
            _appPasscode.value = if (pass.isNotBlank()) pass else null
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun savePreferences() {
        val file = prefsFile ?: return
        try {
            val json = JSONObject()
            json.put("theme", _currentTheme.value.name)
            json.put("custom_accent", _customAccentHex.value ?: "")
            json.put("chat_font_size", _chatFontSize.value.toDouble())
            json.put("bubble_rounding", _bubbleRounding.value.toDouble())
            json.put("chat_wallpaper", _chatWallpaper.value)
            json.put("sound", _soundEnabled.value)
            json.put("vibration", _vibrationEnabled.value)
            json.put("push_notifications", _pushNotificationsEnabled.value)
            json.put("auto_media", _autoDownloadMedia.value)
            json.put("auto_voice", _autoDownloadVoice.value)
            json.put("auto_delete_secs", _autoDeleteTimerSecs.value)
            json.put("p2p_relay", _p2pRelayEnabled.value)
            json.put("app_passcode", _appPasscode.value ?: "")
            file.writeText(json.toString(2))
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun setTheme(theme: ChatTheme) {
        _currentTheme.value = theme
        savePreferences()
    }

    fun setCustomAccent(hex: String?) {
        _customAccentHex.value = hex
        savePreferences()
    }

    fun setChatFontSize(size: Float) {
        _chatFontSize.value = size
        savePreferences()
    }

    fun setBubbleRounding(rounding: Float) {
        _bubbleRounding.value = rounding
        savePreferences()
    }

    fun setChatWallpaper(type: String) {
        _chatWallpaper.value = type
        savePreferences()
    }

    fun setSoundEnabled(enabled: Boolean) {
        _soundEnabled.value = enabled
        savePreferences()
    }

    fun setVibrationEnabled(enabled: Boolean) {
        _vibrationEnabled.value = enabled
        savePreferences()
    }

    fun setPushNotificationsEnabled(enabled: Boolean) {
        _pushNotificationsEnabled.value = enabled
        savePreferences()
    }

    fun setAutoDownloadMedia(enabled: Boolean) {
        _autoDownloadMedia.value = enabled
        savePreferences()
    }

    fun setAutoDownloadVoice(enabled: Boolean) {
        _autoDownloadVoice.value = enabled
        savePreferences()
    }

    fun setAutoDeleteTimerSecs(secs: Long) {
        _autoDeleteTimerSecs.value = secs
        savePreferences()
    }

    fun setP2pRelayEnabled(enabled: Boolean) {
        _p2pRelayEnabled.value = enabled
        savePreferences()
    }

    fun setAppPasscode(pin: String?) {
        _appPasscode.value = pin
        savePreferences()
    }

    fun dismissTopPushNotification() {
        _topPushNotification.value = null
    }

    fun showTestPushNotification() {
        if (_pushNotificationsEnabled.value) {
            _topPushNotification.value = PushNotificationData(
                id = "test_${System.currentTimeMillis()}",
                senderId = "31777112284439999",
                senderName = "NodeX Network Bot",
                text = "Проверка уведомлений NodeX."
            )
            showSystemNotification(
                "31777112284439999",
                "NodeX Network Bot",
                "Проверка уведомлений NodeX."
            )
        }
        playNotificationChime()
    }

    fun loadDeviceSessions() {
        val file = File(appContext?.filesDir, "nodex_sessions.json")
        if (file.exists()) {
            try {
                val array = JSONArray(file.readText())
                val list = mutableListOf<DeviceSession>()
                for (i in 0 until array.length()) {
                    val obj = array.getJSONObject(i)
                    list.add(
                        DeviceSession(
                            id = obj.getString("id"),
                            name = obj.getString("name"),
                            appVersion = obj.optString("appVersion", "NodeX Mobile v2.0"),
                            ip = obj.optString("ip", "В сети"),
                            isCurrent = obj.optBoolean("isCurrent", false)
                        )
                    )
                }
                if (list.isNotEmpty()) {
                    _deviceSessions.value = list
                    return
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
        _deviceSessions.value = listOf(
            DeviceSession("cur", "Этот телефон (Android)", "NodeX Mobile v2.0", "В сети", true)
        )
        saveDeviceSessions()
    }

    fun saveDeviceSessions() {
        val file = File(appContext?.filesDir, "nodex_sessions.json")
        try {
            val array = JSONArray()
            for (sess in _deviceSessions.value) {
                val obj = JSONObject().apply {
                    put("id", sess.id)
                    put("name", sess.name)
                    put("appVersion", sess.appVersion)
                    put("ip", sess.ip)
                    put("isCurrent", sess.isCurrent)
                }
                array.put(obj)
            }
            file.writeText(array.toString(2))
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun addDeviceSession(session: DeviceSession) {
        _deviceSessions.value = _deviceSessions.value + session
        saveDeviceSessions()
    }

    fun resetDefaultSessions() {
        _deviceSessions.value = listOf(
            DeviceSession("cur", "Этот телефон (Android)", "NodeX Mobile v2.0", "В сети", true)
        )
        saveDeviceSessions()
    }

    fun removeDeviceSession(sessionId: String) {
        _deviceSessions.value = _deviceSessions.value.filter { it.id != sessionId || it.isCurrent }
        saveDeviceSessions()
    }

    fun terminateAllOtherSessions() {
        _deviceSessions.value = _deviceSessions.value.filter { it.isCurrent }
        saveDeviceSessions()
    }

    fun playNotificationChime() {
        val ctx = appContext ?: return
        if (!_soundEnabled.value) return
        try {
            val notificationUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
            val ringtone = RingtoneManager.getRingtone(ctx, notificationUri)
            ringtone?.play()
        } catch (e: Exception) {
            e.printStackTrace()
        }

        if (_vibrationEnabled.value) {
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    val vibratorManager = ctx.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager
                    vibratorManager?.defaultVibrator?.vibrate(VibrationEffect.createOneShot(80, VibrationEffect.DEFAULT_AMPLITUDE))
                } else {
                    @Suppress("DEPRECATION")
                    val vibrator = ctx.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
                    @Suppress("DEPRECATION")
                    vibrator?.vibrate(80)
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    private fun loadIdentity() {
        val file = identityFile ?: return
        if (file.exists()) {
            try {
                val identityBytes = if (AndroidMessageCrypto.isSealed(file)) {
                    AndroidMessageCrypto.openJsonFile(contextOrThrow(), file, "identity".toByteArray())
                } else file.readBytes()
                val json = JSONObject(String(identityBytes, Charsets.UTF_8))
                val nodeId = json.optString("nodeId", "")
                val name = json.optString("name", "Мой профиль")
                val bio = json.optString("bio", "На связи в NodeX")
                val encryptedMnemonic = json.optString("mnemonic_ciphertext", "")
                val mnemonic = if (encryptedMnemonic.isNotBlank()) {
                    decryptMnemonic(encryptedMnemonic, json.optString("mnemonic_iv", ""))
                } else {
                    // Migrate old installations on the next save; never keep writing
                    // a recovery phrase as plaintext to app storage.
                    json.optString("mnemonic", "")
                }
                val createdAt = json.optLong("createdAt", System.currentTimeMillis())
                if (mnemonic.isNotBlank()) {
                    val derivedNodeId = MnemonicHelper.deriveNodeId(mnemonic)
                    if (nodeId.isNotBlank() && !nodeId.equals(derivedNodeId, ignoreCase = true)) {
                        migratedLegacyNodeId = nodeId.lowercase()
                        _isOnboarded.value = false
                        _identity.value = NodeIdentity(derivedNodeId, name, bio, mnemonic, createdAt)
                        _isOnboarded.value = true
                        saveIdentity()
                        return
                    }
                    if (nodeId != derivedNodeId) {
                        _identity.value = NodeIdentity(derivedNodeId, name, bio, mnemonic, createdAt)
                        _isOnboarded.value = true
                        saveIdentity()
                        return
                    }
                }
                if (nodeId.isNotBlank()) {
                    _identity.value = NodeIdentity(
                        nodeId = nodeId,
                        name = name,
                        bio = bio,
                        mnemonic = mnemonic,
                        createdAt = createdAt
                    )
                    _isOnboarded.value = true
                    if (encryptedMnemonic.isBlank() && mnemonic.isNotBlank()) saveIdentity()
                    return
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
        _isOnboarded.value = false
    }

    fun createAccount(
        name: String,
        mnemonic: String
    ) {
        val cleanName = if (name.isNotBlank()) name.trim() else "Мой профиль"
        require(MnemonicHelper.isValidMnemonic(mnemonic)) { "Invalid BIP-39 recovery phrase" }
        val derivedId = MnemonicHelper.deriveNodeId(mnemonic)

        val newIdentity = NodeIdentity(
            nodeId = derivedId,
            name = cleanName,
            bio = "На связи в NodeX",
            mnemonic = mnemonic.trim(),
            createdAt = System.currentTimeMillis()
        )
        P2PTransportEngine.stop()
        _contacts.value = emptyList()
        _messages.value = emptyMap()
        _topPushNotification.value = null
        _identity.value = newIdentity
        configureAccountStorage(newIdentity.nodeId)
        saveIdentity()
        _isOnboarded.value = true
        loadContacts()
        loadMessages()
        appContext?.let { P2PTransportEngine.start(it, newIdentity.nodeId) }
    }

    fun restoreAccount(name: String, mnemonic: String): Boolean {
        if (!MnemonicHelper.isValidMnemonic(mnemonic)) return false
        createAccount(name, mnemonic)
        return true
    }

    fun logout() {
        if (_activeCall.value != null) endCall()
        P2PTransportEngine.stop()
        _isOnboarded.value = false
        _identity.value = NodeIdentity("", "", "", "", 0)
        _contacts.value = emptyList()
        _messages.value = emptyMap()
        _topPushNotification.value = null
        contactsFile = null
        messagesFile = null
        accountDirectory = null
        identityFile?.delete()
    }

    /**
     * Contacts and messages are account-scoped. Older versions stored these
     * files at the app root, which made a newly created identity see another
     * identity's history. The legacy files are copied once only for the
     * identity that was already signed in when this version first starts.
     */
    private fun configureAccountStorage(nodeId: String, migrateLegacy: Boolean = false) {
        val cleanId = nodeId.trim().lowercase()
        if (cleanId.length != 40 || cleanId.any { it.digitToIntOrNull(16) == null }) {
            contactsFile = null
            messagesFile = null
            accountDirectory = null
            return
        }
        val root = File(contextOrThrow().filesDir, "nodex_accounts")
        val directory = File(root, cleanId).apply { mkdirs() }
        val scopedContacts = File(directory, "contacts.json")
        val scopedMessages = File(directory, "messages.json")
        if (migrateLegacy) {
            val legacyContacts = File(contextOrThrow().filesDir, "nodex_contacts.json")
            val legacyMessages = File(contextOrThrow().filesDir, "nodex_messages.json")
            runCatching {
                // A legacy file is safe to adopt only when its own self-contact
                // proves it belonged to this exact identity. This keeps an
                // already-created new account from inheriting old history.
                val legacyBelongsToAccount = legacyContactsBelongTo(cleanId, legacyContacts)
                if (legacyBelongsToAccount && !scopedContacts.exists() && legacyContacts.exists()) legacyContacts.copyTo(scopedContacts)
                if (legacyBelongsToAccount && !scopedMessages.exists() && legacyMessages.exists()) legacyMessages.copyTo(scopedMessages)
            }.onFailure { android.util.Log.w("NodeX_Repo", "Legacy account data migration failed", it) }
        }
        accountDirectory = directory
        contactsFile = scopedContacts
        messagesFile = scopedMessages
    }

    private fun legacyContactsBelongTo(nodeId: String, legacyContacts: File): Boolean {
        if (!legacyContacts.exists()) return false
        return runCatching {
            val raw = if (AndroidMessageCrypto.isSealed(legacyContacts)) {
                AndroidMessageCrypto.openJsonFile(contextOrThrow(), legacyContacts, "contacts".toByteArray())
            } else legacyContacts.readBytes()
            val contacts = JSONArray(String(raw, Charsets.UTF_8))
            (0 until contacts.length()).any { index ->
                val item = contacts.getJSONObject(index)
                item.optBoolean("isSelf", false) && item.optString("userIdHex", "").equals(nodeId, ignoreCase = true)
            }
        }.getOrDefault(false)
    }

    fun updateProfile(name: String, bio: String) {
        _identity.value = _identity.value.copy(
            name = if (name.isNotBlank()) name else _identity.value.name,
            bio = bio
        )
        saveIdentity()
    }

    fun updateContactEndpoint(peerId: String, endpoint: String, online: Boolean = true) {
        var changed = false
        _contacts.value = _contacts.value.map {
            if (it.userIdHex.equals(peerId, ignoreCase = true)) {
                changed = true
                it.copy(endpointAddr = endpoint, isOnline = online)
            } else it
        }
        if (changed) {
            saveContacts()
        }
    }

    fun updateContactOnlineStatus(peerId: String, online: Boolean) {
        var changed = false
        _contacts.value = _contacts.value.map {
            if (it.userIdHex.equals(peerId, ignoreCase = true) && it.isOnline != online) {
                changed = true
                it.copy(isOnline = online)
            } else it
        }
        if (changed) saveContacts()
    }

    fun updateVerifiedContactKeys(
        peerId: String,
        ed25519PublicKey: ByteArray,
        x25519PublicKey: ByteArray,
        authenticatedRefresh: Boolean = false
    ) {
        if (ed25519PublicKey.size != 32 || x25519PublicKey.size != 32) return
        val id = peerId.trim().lowercase()
        val expectedId = java.security.MessageDigest.getInstance("SHA-1").digest(ed25519PublicKey)
            .joinToString("") { "%02x".format(it) }
        if (!expectedId.equals(id, ignoreCase = true)) return
        val existing = _contacts.value.firstOrNull { it.userIdHex.equals(id, ignoreCase = true) }
        if (existing != null && existing.isIdentityVerified) {
            if (existing.ed25519PublicKeyHex != ed25519PublicKey.toHex()) return
            if (existing.x25519PublicKeyHex != x25519PublicKey.toHex() && !authenticatedRefresh) return
        }
        if (existing == null) addContact(id, "Пользователь ${id.take(6)}")
        _contacts.value = _contacts.value.map {
            if (it.userIdHex.equals(id, ignoreCase = true)) it.copy(
                ed25519PublicKeyHex = ed25519PublicKey.toHex(),
                x25519PublicKeyHex = x25519PublicKey.toHex(),
                isIdentityVerified = true
            ) else it
        }
        saveContacts()
    }

    fun getVerifiedPeerX25519Key(peerId: String): ByteArray? {
        val contact = _contacts.value.firstOrNull { it.userIdHex.equals(peerId, ignoreCase = true) } ?: return null
        if (!contact.isIdentityVerified || contact.x25519PublicKeyHex.length != 64) return null
        return runCatching { contact.x25519PublicKeyHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray() }.getOrNull()
    }

    fun getVerifiedPeerEd25519Key(peerId: String): ByteArray? {
        val contact = _contacts.value.firstOrNull { it.userIdHex.equals(peerId, ignoreCase = true) } ?: return null
        if (!contact.isIdentityVerified || contact.ed25519PublicKeyHex.length != 64) return null
        return runCatching { contact.ed25519PublicKeyHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray() }.getOrNull()
    }

    fun markMessageDelivered(peerId: String, messageId: String) {
        // IDs received from envelopes are canonical lower-case hex, while a
        // contact can have been entered or imported with upper-case letters.
        // Resolve the existing conversation key case-insensitively so a valid
        // signed receipt cannot leave the bubble stuck on the clock icon.
        val conversationId = _messages.value.keys.firstOrNull { it.equals(peerId, ignoreCase = true) } ?: return
        val messages = _messages.value[conversationId] ?: return
        val updated = messages.map { if (it.id == messageId && !it.isDelivered) it.copy(isDelivered = true) else it }
        if (updated != messages) {
            _messages.value = _messages.value + (conversationId to updated)
            saveMessages()
        }
    }

    fun pendingOutgoingMessages(): List<Pair<String, ChatMessage>> = _messages.value.flatMap { (peer, list) ->
        list.filter { !it.isIncoming && !it.isDelivered && !it.text.startsWith("📞") && !it.text.startsWith("📲") }
            .map { peer to it }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    private fun saveIdentity() {
        val file = identityFile ?: return
        try {
            val json = JSONObject()
            json.put("nodeId", _identity.value.nodeId)
            json.put("name", _identity.value.name)
            json.put("bio", _identity.value.bio)
            val (iv, ciphertext) = encryptMnemonic(_identity.value.mnemonic)
            json.put("mnemonic_iv", iv)
            json.put("mnemonic_ciphertext", ciphertext)
            json.put("createdAt", _identity.value.createdAt)
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText(json.toString())
            AndroidMessageCrypto.sealJsonFile(contextOrThrow(), tmp, "identity".toByteArray())
            if (file.exists() && !file.delete()) error("Cannot replace identity store")
            if (!tmp.renameTo(file)) error("Cannot commit identity store")
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun identityKey(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(IDENTITY_KEY_ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(
                IDENTITY_KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            ).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build()
        )
        return generator.generateKey()
    }

    private fun encryptMnemonic(value: String): Pair<String, String> {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, identityKey())
        val encrypted = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(cipher.iv, Base64.NO_WRAP) to
            Base64.encodeToString(encrypted, Base64.NO_WRAP)
    }

    private fun decryptMnemonic(ciphertext: String, iv: String): String {
        require(ciphertext.isNotBlank() && iv.isNotBlank()) { "Stored recovery phrase is incomplete" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            identityKey(),
            GCMParameterSpec(128, Base64.decode(iv, Base64.NO_WRAP))
        )
        return String(Base64.decode(ciphertext, Base64.NO_WRAP).let(cipher::doFinal), Charsets.UTF_8)
    }

    private fun contextOrThrow(): Context = appContext ?: error("Repository is not initialized")

    private fun loadContacts() {
        val file = contactsFile ?: return
        if (!file.exists()) {
            val list = mutableListOf<NodeContact>()
            if (_identity.value.nodeId.isNotBlank()) {
                list.add(
                    NodeContact(
                        userIdHex = _identity.value.nodeId,
                        name = "${_identity.value.name} (Вы)",
                        bio = "Личные заметки и сообщения",
                        isOnline = false,
                        isSelf = true,
                        unreadCount = 0,
                        lastMessageText = "Личные заметки и сообщения",
                        lastMessageTime = ""
                    )
                )
            }
            _contacts.value = list
            return
        }
        try {
            val raw = if (AndroidMessageCrypto.isSealed(file)) AndroidMessageCrypto.openJsonFile(contextOrThrow(), file, "contacts".toByteArray()) else file.readBytes()
            val jsonArray = JSONArray(String(raw, Charsets.UTF_8))
            val list = mutableListOf<NodeContact>()
            for (i in 0 until jsonArray.length()) {
                val obj = jsonArray.getJSONObject(i)
                val storedUid = obj.getString("userIdHex")
                val legacyId = migratedLegacyNodeId
                val uid = if (legacyId != null && storedUid.equals(legacyId, true)) _identity.value.nodeId else storedUid
                val rawName = obj.getString("name")

                // Remove legacy test/corrupt ghost chats from previous runs or bootstrap leaks
                if (rawName == "📢 1" || rawName == "👥 1" || rawName.startsWith("Узел 4d457488") ||
                    rawName.startsWith("Узел 0000") || uid.length < 16 || uid.startsWith("00000000") ||
                    uid.contains(":") || uid.startsWith("31.77.71.122")) {
                    continue
                }

                val isSelf = obj.optBoolean("isSelf", false) || uid.equals(_identity.value.nodeId, ignoreCase = true)
                val chatTypeStr = obj.optString("chatType", "DIRECT")
                val chatType = try { ChatType.valueOf(chatTypeStr) } catch(e: Exception) { ChatType.DIRECT }
                val isChannel = obj.optBoolean("isChannel", chatType == ChatType.CHANNEL)
                val isGroup = obj.optBoolean("isGroup", chatType == ChatType.GROUP)
                val storedOwnerId = obj.optString("ownerIdHex", if (isChannel || isGroup) _identity.value.nodeId else "")
                val ownerIdHex = if (legacyId != null && storedOwnerId.equals(legacyId, true)) _identity.value.nodeId else storedOwnerId
                val description = obj.optString("description", "")
                val membersCount = obj.optInt("membersCount", 1)
                val adminOnlyPosting = obj.optBoolean("adminOnlyPosting", isChannel)

                val mutedArray = obj.optJSONArray("mutedMembers")
                val mutedList = mutableListOf<String>()
                if (mutedArray != null) {
                    for (m in 0 until mutedArray.length()) mutedList.add(mutedArray.getString(m))
                }

                val bannedArray = obj.optJSONArray("bannedMembers")
                val bannedList = mutableListOf<String>()
                if (bannedArray != null) {
                    for (b in 0 until bannedArray.length()) bannedList.add(bannedArray.getString(b))
                }

                list.add(
                    NodeContact(
                        userIdHex = uid,
                        name = if (isSelf && !rawName.contains("(Вы)")) "${rawName} (Вы)" else rawName,
                        bio = obj.optString("bio", if (isSelf) "Личные заметки и сообщения" else if (isChannel) "Канал" else if (isGroup) "Группа" else "Контакт"),
                        endpointAddr = obj.optString("endpointAddr", ""),
                        isOnline = if (isSelf || isChannel || isGroup) false else obj.optBoolean("isOnline", false),
                        isSelf = isSelf,
                        unreadCount = obj.optInt("unreadCount", 0),
                        lastMessageText = obj.optString("lastMessageText", ""),
                        lastMessageTime = obj.optString("lastMessageTime", ""),
                        chatType = chatType,
                        ownerIdHex = ownerIdHex,
                        description = description,
                        membersCount = membersCount,
                        isChannel = isChannel,
                        isGroup = isGroup,
                        adminOnlyPosting = adminOnlyPosting,
                        mutedMembers = mutedList,
                        bannedMembers = bannedList,
                        ed25519PublicKeyHex = obj.optString("ed25519PublicKeyHex", ""),
                        x25519PublicKeyHex = obj.optString("x25519PublicKeyHex", ""),
                        isIdentityVerified = obj.optBoolean("isIdentityVerified", false)
                    )
                )
            }
            if (_identity.value.nodeId.isNotBlank() && list.none { it.isSelf || it.userIdHex.equals(_identity.value.nodeId, ignoreCase = true) }) {
                list.add(0, NodeContact(
                    userIdHex = _identity.value.nodeId,
                    name = "${_identity.value.name} (Вы)",
                    bio = "Личные заметки и сообщения",
                    isOnline = false,
                    isSelf = true,
                    unreadCount = 0,
                    lastMessageText = "Личные заметки и сообщения",
                    lastMessageTime = ""
                ))
            }
            _contacts.value = list
            if (!AndroidMessageCrypto.isSealed(file) || migratedLegacyNodeId != null) saveContacts()
        } catch (e: Exception) {
            e.printStackTrace()
            _contacts.value = emptyList()
        }
    }

    private fun saveContacts() {
        val file = contactsFile ?: return
        try {
            val jsonArray = JSONArray()
            _contacts.value.forEach { c ->
                val obj = JSONObject()
                obj.put("userIdHex", c.userIdHex)
                obj.put("name", c.name)
                obj.put("bio", c.bio)
                obj.put("endpointAddr", c.endpointAddr)
                obj.put("isOnline", if (c.isSelf || c.isChannel || c.isGroup) false else c.isOnline)
                obj.put("isSelf", c.isSelf || c.userIdHex.equals(_identity.value.nodeId, ignoreCase = true))
                obj.put("unreadCount", c.unreadCount)
                obj.put("lastMessageText", c.lastMessageText)
                obj.put("lastMessageTime", c.lastMessageTime)
                obj.put("chatType", c.chatType.name)
                obj.put("ownerIdHex", c.ownerIdHex)
                obj.put("description", c.description)
                obj.put("membersCount", c.membersCount)
                obj.put("isChannel", c.isChannel)
                obj.put("isGroup", c.isGroup)
                obj.put("adminOnlyPosting", c.adminOnlyPosting)
                val mutedArr = JSONArray()
                c.mutedMembers.forEach { mutedArr.put(it) }
                obj.put("mutedMembers", mutedArr)
                val bannedArr = JSONArray()
                c.bannedMembers.forEach { bannedArr.put(it) }
                obj.put("bannedMembers", bannedArr)
                obj.put("ed25519PublicKeyHex", c.ed25519PublicKeyHex)
                obj.put("x25519PublicKeyHex", c.x25519PublicKeyHex)
                obj.put("isIdentityVerified", c.isIdentityVerified)
                jsonArray.put(obj)
            }
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText(jsonArray.toString())
            AndroidMessageCrypto.sealJsonFile(contextOrThrow(), tmp, "contacts".toByteArray())
            if (file.exists() && !file.delete()) error("Cannot replace contacts store")
            if (!tmp.renameTo(file)) error("Cannot commit contacts store")
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun loadMessages() {
        val file = messagesFile ?: return
        if (!file.exists()) {
            _messages.value = emptyMap()
            return
        }
        try {
            val raw = if (AndroidMessageCrypto.isSealed(file)) AndroidMessageCrypto.openJsonFile(contextOrThrow(), file, "messages".toByteArray()) else file.readBytes()
            val json = JSONObject(String(raw, Charsets.UTF_8))
            val map = mutableMapOf<String, List<ChatMessage>>()
            var identityMigrated = false
            val keys = json.keys()
            while (keys.hasNext()) {
                val storedContactId = keys.next()
                val legacyId = migratedLegacyNodeId
                val contactId = if (legacyId != null && storedContactId.equals(legacyId, true)) _identity.value.nodeId.also { identityMigrated = true } else storedContactId
                val array = json.getJSONArray(storedContactId)
                val msgList = mutableListOf<ChatMessage>()
                for (i in 0 until array.length()) {
                    val m = array.getJSONObject(i)
                    val waveformArray = m.optJSONArray("waveform")
                    val waveformList = mutableListOf<Float>()
                    if (waveformArray != null) {
                        for (w in 0 until waveformArray.length()) {
                            waveformList.add(waveformArray.getDouble(w).toFloat())
                        }
                    }

                    val isIncoming = m.getBoolean("isIncoming")
                    val messageText = m.getString("text")
                    val isLocalCallRecord = messageText.contains("вызов", ignoreCase = true)
                        || messageText.contains("звонок", ignoreCase = true)
                    val deliveryConfirmed = m.optBoolean("isDelivered", true)
                        && (isIncoming || isLocalCallRecord
                            || contactId.equals(_identity.value.nodeId, ignoreCase = true))
                    msgList.add(
                        ChatMessage(
                            id = m.getString("id"),
                            senderIdHex = m.getString("senderIdHex").let { if (legacyId != null && it.equals(legacyId, true)) _identity.value.nodeId.also { identityMigrated = true } else it },
                            text = messageText,
                            isIncoming = isIncoming,
                            timestampStr = m.getString("timestampStr"),
                            isDelivered = deliveryConfirmed,
                            isRead = m.optBoolean("isRead", true),
                            isVoice = m.optBoolean("isVoice", false),
                            voiceDurationSecs = m.optInt("voiceDurationSecs", 0),
                            waveform = waveformList,
                            audioFilePath = m.optString("audioFilePath", ""),
                            isImage = m.optBoolean("isImage", false),
                            imageFilePath = m.optString("imageFilePath", ""),
                            isFile = m.optBoolean("isFile", false),
                            filePath = m.optString("filePath", ""),
                            fileName = m.optString("fileName", ""),
                            fileSizeFormatted = m.optString("fileSizeFormatted", ""),
                            attachmentId = m.optString("attachmentId", ""),
                            attachmentSizeBytes = m.optLong("attachmentSizeBytes", 0L),
                            attachmentMimeType = m.optString("attachmentMimeType", "")
                        )
                    )
                }
                msgList.forEach { P2PTransportEngine.markMessageIdSeen(it.id) }
                map[contactId] = msgList
            }
            _messages.value = map
            if (!AndroidMessageCrypto.isSealed(file) || identityMigrated) saveMessages()
        } catch (e: Exception) {
            e.printStackTrace()
            _messages.value = emptyMap()
        }
    }

    private fun saveMessages(): Boolean {
        val file = messagesFile ?: return false
        try {
            val json = JSONObject()
            _messages.value.forEach { (contactId, list) ->
                val array = JSONArray()
                list.forEach { m ->
                    val obj = JSONObject()
                    obj.put("id", m.id)
                    obj.put("senderIdHex", m.senderIdHex)
                    obj.put("text", m.text)
                    obj.put("isIncoming", m.isIncoming)
                    obj.put("timestampStr", m.timestampStr)
                    obj.put("isDelivered", m.isDelivered)
                    obj.put("isRead", m.isRead)
                    obj.put("isVoice", m.isVoice)
                    obj.put("voiceDurationSecs", m.voiceDurationSecs)
                    obj.put("audioFilePath", m.audioFilePath)
                    obj.put("isImage", m.isImage)
                    obj.put("imageFilePath", m.imageFilePath)
                    obj.put("isFile", m.isFile)
                    obj.put("filePath", m.filePath)
                    obj.put("fileName", m.fileName)
                    obj.put("fileSizeFormatted", m.fileSizeFormatted)
                    obj.put("attachmentId", m.attachmentId)
                    obj.put("attachmentSizeBytes", m.attachmentSizeBytes)
                    obj.put("attachmentMimeType", m.attachmentMimeType)
                    if (m.waveform.isNotEmpty()) {
                        val waveArray = JSONArray()
                        m.waveform.forEach { waveArray.put(it.toDouble()) }
                        obj.put("waveform", waveArray)
                    }
                    array.put(obj)
                }
                json.put(contactId, array)
            }
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText(json.toString())
            if (file.exists() && !file.delete()) error("Cannot replace messages store")
            if (!tmp.renameTo(file)) error("Cannot commit messages store")
            AndroidMessageCrypto.sealJsonFile(contextOrThrow(), file, "messages".toByteArray())
            messageStoreWriteSucceeded = true
            return true
        } catch (e: Exception) {
            e.printStackTrace()
            messageStoreWriteSucceeded = false
            return false
        }
    }

    fun addContact(inviteOrId: String, name: String, endpointAddr: String = "") {
        val cleanInput = inviteOrId.trim()
        var parsedId = cleanInput
        var inviteEdKey = ""
        var inviteXKey = ""
        var verifiedBundle = false
        var invitedName = ""
        if (cleanInput.startsWith("nodex://invite/")) {
            val encoded = cleanInput.removePrefix("nodex://invite/").substringBefore('?')
            val invite = runCatching {
                JSONObject(String(android.util.Base64.decode(encoded, android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING), Charsets.UTF_8))
            }.getOrNull()
            if (invite != null) {
                if (!AndroidMessageCrypto.verifyRustCompatibleInvite(invite)) return
                parsedId = invite.getString("user_id")
                inviteEdKey = invite.getString("ed25519_pub_hex").lowercase()
                inviteXKey = invite.getString("x25519_pub_hex").lowercase()
                invitedName = invite.optString("display_name", "")
                verifiedBundle = true
            } else {
                parsedId = cleanInput.removePrefix("nodex://invite/").trim()
            }
        }
        if (parsedId.length != 40 || parsedId.any { it.digitToIntOrNull(16) == null }) return
        parsedId = parsedId.lowercase()

        val isSelf = parsedId.equals(_identity.value.nodeId, ignoreCase = true)
        val existing = _contacts.value.firstOrNull { it.userIdHex.equals(parsedId, ignoreCase = true) }
        if (existing != null) return

        val contactName = when {
            name.isNotBlank() -> name.trim()
            invitedName.isNotBlank() -> invitedName
            isSelf -> "${_identity.value.name} (Вы)"
            else -> "Пользователь ${parsedId.take(6)}"
        }

        val newContact = NodeContact(
            userIdHex = parsedId,
            name = contactName,
            bio = if (isSelf) "Личные заметки и сообщения" else "Контакт",
            endpointAddr = endpointAddr,
            isOnline = false,
            unreadCount = 0,
            lastMessageText = "Чат создан",
            lastMessageTime = getCurrentTimeStr(),
            ed25519PublicKeyHex = inviteEdKey,
            x25519PublicKeyHex = inviteXKey,
            isIdentityVerified = verifiedBundle
        )
        _contacts.value = listOf(newContact) + _contacts.value
        saveContacts()

        if (!isSelf) {
            P2PTransportEngine.queryPeerPresence(parsedId)
            if (endpointAddr.isNotBlank()) {
                P2PTransportEngine.recordContactEndpoint(parsedId, endpointAddr)
            }
        }
    }

    fun createChannel(title: String, description: String = ""): String {
        val cleanTitle = title.trim().ifBlank { "Новый канал" }
        val channelId = "channel_${System.currentTimeMillis()}_${(1000..9999).random()}"
        val timeStr = getCurrentTimeStr()
        val channel = NodeContact(
            userIdHex = channelId,
            name = cleanTitle,
            bio = description.ifBlank { "Публичный канал" },
            chatType = ChatType.CHANNEL,
            ownerIdHex = _identity.value.nodeId,
            description = description,
            membersCount = 1,
            isChannel = true,
            isGroup = false,
            adminOnlyPosting = true,
            unreadCount = 0,
            lastMessageText = "Канал создан",
            lastMessageTime = timeStr
        )
        val list = _contacts.value.toMutableList()
        val selfIdx = list.indexOfFirst { it.isSelf }
        if (selfIdx == 0) list.add(1, channel) else list.add(0, channel)
        _contacts.value = list
        saveContacts()
        return channelId
    }

    fun createGroup(title: String, description: String = ""): String {
        val cleanTitle = title.trim().ifBlank { "Новая группа" }
        val groupId = "group_${System.currentTimeMillis()}_${(1000..9999).random()}"
        val timeStr = getCurrentTimeStr()
        val group = NodeContact(
            userIdHex = groupId,
            name = cleanTitle,
            bio = description.ifBlank { "Группа участников" },
            chatType = ChatType.GROUP,
            ownerIdHex = _identity.value.nodeId,
            description = description,
            membersCount = 1,
            isChannel = false,
            isGroup = true,
            adminOnlyPosting = false,
            unreadCount = 0,
            lastMessageText = "Группа создана",
            lastMessageTime = timeStr
        )
        val list = _contacts.value.toMutableList()
        val selfIdx = list.indexOfFirst { it.isSelf }
        if (selfIdx == 0) list.add(1, group) else list.add(0, group)
        _contacts.value = list
        saveContacts()
        return groupId
    }

    fun updateGroupSettings(chatId: String, adminOnlyPosting: Boolean, title: String, description: String) {
        _contacts.value = _contacts.value.map { c ->
            if (c.userIdHex == chatId) {
                c.copy(
                    adminOnlyPosting = adminOnlyPosting,
                    name = title.ifBlank { c.name },
                    description = description,
                    bio = description.ifBlank { c.bio }
                )
            } else c
        }
        saveContacts()
    }

    fun toggleMuteMember(chatId: String, memberId: String) {
        _contacts.value = _contacts.value.map { c ->
            if (c.userIdHex == chatId) {
                val muted = c.mutedMembers.toMutableList()
                if (muted.contains(memberId)) muted.remove(memberId) else muted.add(memberId)
                c.copy(mutedMembers = muted)
            } else c
        }
        saveContacts()
    }

    fun toggleBanMember(chatId: String, memberId: String) {
        _contacts.value = _contacts.value.map { c ->
            if (c.userIdHex == chatId) {
                val banned = c.bannedMembers.toMutableList()
                if (banned.contains(memberId)) banned.remove(memberId) else banned.add(memberId)
                c.copy(bannedMembers = banned)
            } else c
        }
        saveContacts()
    }

    fun canUserPost(chatId: String): Boolean {
        val contact = _contacts.value.firstOrNull { it.userIdHex == chatId } ?: return true
        if (contact.isChannel) {
            return contact.ownerIdHex.isBlank() || contact.ownerIdHex.equals(_identity.value.nodeId, ignoreCase = true)
        }
        if (contact.isGroup) {
            if (contact.adminOnlyPosting && !contact.ownerIdHex.equals(_identity.value.nodeId, ignoreCase = true)) {
                return false
            }
            if (contact.bannedMembers.contains(_identity.value.nodeId) || contact.mutedMembers.contains(_identity.value.nodeId)) {
                return false
            }
        }
        return true
    }

    fun deleteContact(contactId: String) {
        _contacts.value = _contacts.value.filterNot { it.userIdHex == contactId }
        saveContacts()
        _messages.value = _messages.value - contactId
        saveMessages()
    }

    fun clearMessages(contactId: String) {
        _messages.value = _messages.value + (contactId to emptyList())
        saveMessages()
        _contacts.value = _contacts.value.map {
            if (it.userIdHex == contactId) it.copy(lastMessageText = "", lastMessageTime = "") else it
        }
        saveContacts()
    }

    fun clearAllData() {
        _contacts.value = emptyList()
        _messages.value = emptyMap()
        contactsFile?.delete()
        messagesFile?.delete()
    }

    fun sendMessage(contactId: String, text: String) {
        val cleanText = text.trim()
        if (cleanText.isEmpty()) return
        if (!canUserPost(contactId)) return

        val isSelf = contactId.equals(_identity.value.nodeId, ignoreCase = true)
        if (_contacts.value.none { it.userIdHex.equals(contactId, ignoreCase = true) }) {
            addContact(contactId, if (isSelf) "${_identity.value.name} (Вы)" else "Пользователь ${contactId.take(6)}")
        }

        val timeStr = getCurrentTimeStr()
        val msg = ChatMessage(
            id = P2PTransportEngine.newMessageId(_identity.value.nodeId),
            senderIdHex = _identity.value.nodeId,
            senderName = _identity.value.name,
            text = cleanText,
            isIncoming = false,
            timestampStr = timeStr,
            isDelivered = isSelf,
            isRead = isSelf
        )

        val currentList = _messages.value[contactId] ?: emptyList()
        _messages.value = _messages.value + (contactId to (currentList + msg))
        if (!saveMessages()) {
            _messages.value = _messages.value + (contactId to currentList)
            return
        }
        val contactSnapshot = _contacts.value

        _contacts.value = _contacts.value.map { contact ->
            if (contact.userIdHex.equals(contactId, ignoreCase = true)) {
                contact.copy(
                    lastMessageText = cleanText,
                    lastMessageTime = timeStr,
                    unreadCount = 0
                )
            } else {
                contact
            }
        }
        saveContacts()

        // Real P2P Network Dispatch across UDP mesh
        if (!isSelf) {
            if (!P2PTransportEngine.sendMessage(contactId, cleanText, _identity.value.nodeId, _identity.value.name, msg.id)) {
                // Keep the local message as pending. The transport will retry it
                // after presence/key discovery instead of making the message vanish.
                _messages.value = _messages.value + (contactId to (_messages.value[contactId].orEmpty().map {
                    if (it.id == msg.id) it.copy(isDelivered = false) else it
                }))
                _contacts.value = contactSnapshot.map { contact ->
                    if (contact.userIdHex.equals(contactId, ignoreCase = true)) {
                        contact.copy(lastMessageText = cleanText, lastMessageTime = timeStr)
                    } else contact
                }
                saveMessages()
                saveContacts()
            }
        }
    }

    private fun removeUnsentMessage(contactId: String, messageId: String) {
        val cleanPeer = contactId.trim().lowercase()
        val updated = (_messages.value[cleanPeer] ?: return).filterNot { it.id == messageId }
        _messages.value = _messages.value + (cleanPeer to updated)
        saveMessages()
    }

    fun markChatAsRead(contactId: String) {
        var changed = false
        _contacts.value = _contacts.value.map { c ->
            if (c.userIdHex.equals(contactId, ignoreCase = true) && c.unreadCount > 0) {
                changed = true
                c.copy(unreadCount = 0)
            } else c
        }
        if (changed) {
            saveContacts()
        }
    }

    fun receiveIncomingMessage(
        senderId: String,
        senderName: String,
        text: String,
        messageId: String = System.currentTimeMillis().toString(),
        isVoice: Boolean = false,
        voiceDurationSecs: Int = 0,
        audioFilePath: String = "",
        waveform: List<Float> = emptyList(),
        isImage: Boolean = false,
        imageFilePath: String = "",
        isFile: Boolean = false,
        filePath: String = "",
        fileName: String = "",
        fileSizeFormatted: String = "",
        attachmentId: String = "",
        attachmentSizeBytes: Long = 0L,
        attachmentMimeType: String = ""
    ): Boolean {
        val cleanSender = senderId.trim().lowercase()
        if (cleanSender.isBlank() || cleanSender.length < 16 || cleanSender.startsWith("00000000") ||
            cleanSender.startsWith("31.77.71.122") || cleanSender.contains(":")) {
            return false
        }

        // Filter out echo: if sender is ourselves, this is our own message bounced back
        val isSelf = cleanSender.equals(_identity.value.nodeId.trim().lowercase(), ignoreCase = true)
        if (isSelf) return false

        // Filter out call signals and protocol markers from becoming regular chat bubbles
        if (text.startsWith("[Call Signal") || text.contains("\"call_signal\"")) return false

        val timeStr = getCurrentTimeStr()

        val updatedList = _contacts.value.toMutableList()
        val existingIndex = updatedList.indexOfFirst { it.userIdHex.equals(cleanSender, ignoreCase = true) }

        val contactName = when {
            senderName.isNotBlank() && !senderName.startsWith("Peer") -> senderName
            existingIndex >= 0 && updatedList[existingIndex].name.isNotBlank() -> updatedList[existingIndex].name
            else -> "Узел ${senderId.take(8)}"
        }

        val preview = when {
            isVoice -> "🎤 Голосовое (${formatVoiceDuration(voiceDurationSecs)})"
            isImage -> "📷 Фотография"
            isFile -> "📁 ${fileName.ifBlank { "Документ" }}"
            else -> text
        }

        val updatedContact = if (existingIndex >= 0) {
            val old = updatedList.removeAt(existingIndex)
            old.copy(
                isOnline = true,
                unreadCount = old.unreadCount + 1,
                lastMessageText = preview,
                lastMessageTime = timeStr
            )
        } else {
            NodeContact(
                userIdHex = senderId,
                name = contactName,
                bio = "P2P контакт",
                isOnline = true,
                unreadCount = 1,
                lastMessageText = preview,
                lastMessageTime = timeStr
            )
        }

        val selfIdx = updatedList.indexOfFirst { it.isSelf }
        if (selfIdx == 0) {
            updatedList.add(1, updatedContact)
        } else {
            updatedList.add(0, updatedContact)
        }
        _contacts.value = updatedList
        saveContacts()

        val msg = ChatMessage(
            id = messageId,
            senderIdHex = senderId,
            text = text,
            isIncoming = true,
            timestampStr = timeStr,
            isDelivered = true,
            isRead = false,
            isVoice = isVoice,
            voiceDurationSecs = voiceDurationSecs,
            waveform = if (waveform.isNotEmpty()) waveform else generateDefaultWaveform(24),
            audioFilePath = audioFilePath,
            isImage = isImage,
            imageFilePath = imageFilePath,
            isFile = isFile,
            filePath = filePath,
            fileName = fileName,
            fileSizeFormatted = fileSizeFormatted,
            attachmentId = attachmentId,
            attachmentSizeBytes = attachmentSizeBytes,
            attachmentMimeType = attachmentMimeType
        )
        val currentList = _messages.value[senderId] ?: emptyList()
        if (currentList.any { it.id == messageId }) return true
        _messages.value = _messages.value + (senderId to (currentList + msg))
        if (!saveMessages()) return false

        // In-App Floating Top Push & System Notification
        if (_pushNotificationsEnabled.value) {
            _topPushNotification.value = PushNotificationData(
                id = msg.id,
                senderId = senderId,
                senderName = contactName,
                text = preview
            )
            showSystemNotification(senderId, contactName, preview)
        }

        // Audio notification for incoming messages (self messages are filtered out above)
        playNotificationChime()
        return true
    }

    fun hasPersistedMessage(peerId: String, messageId: String): Boolean = messageStoreWriteSucceeded &&
        _messages.value[peerId]?.any { it.id == messageId && it.isIncoming } == true

    fun completeIncomingAttachment(peerId: String, messageId: String, localPath: String) {
        val current = _messages.value[peerId] ?: return
        _messages.value = _messages.value + (peerId to current.map { message ->
            if (message.id != messageId) message else message.copy(
                imageFilePath = if (message.isImage) localPath else message.imageFilePath,
                filePath = if (message.isFile) localPath else message.filePath,
                fileSizeFormatted = message.attachmentSizeBytes.let { bytes -> if (bytes >= 1_048_576) "%.1f МБ".format(bytes / 1_048_576f) else "${bytes / 1024} КБ" }
            )
        })
        saveMessages()
    }

    private fun showSystemNotification(senderId: String, senderName: String, text: String) {
        val ctx = appContext ?: return
        try {
            val channelId = "nodex_messages"
            val notificationManager = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                val channel = NotificationChannel(
                    channelId,
                    "NodeX Сообщения",
                    NotificationManager.IMPORTANCE_HIGH
                ).apply {
                    description = "Уведомления о входящих P2P сообщениях"
                    enableLights(true)
                    enableVibration(true)
                }
                notificationManager.createNotificationChannel(channel)
            }

            val intent = Intent(ctx, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
                putExtra("open_chat_id", senderId)
            }
            val pendingIntent = PendingIntent.getActivity(
                ctx,
                senderId.hashCode(),
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            val builder = androidx.core.app.NotificationCompat.Builder(ctx, channelId)
                .setSmallIcon(R.drawable.ic_launcher_foreground)
                .setContentTitle(senderName)
                .setContentText(text)
                .setPriority(androidx.core.app.NotificationCompat.PRIORITY_HIGH)
                .setAutoCancel(true)
                .setContentIntent(pendingIntent)

            notificationManager.notify(senderId.hashCode(), builder.build())
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun sendVoiceMessage(
        contactId: String,
        durationSecs: Int,
        waveform: List<Float>,
        audioFilePath: String = ""
    ) {
        if (!canUserPost(contactId)) return
        val isSelf = contactId.equals(_identity.value.nodeId, ignoreCase = true)
        if (_contacts.value.none { it.userIdHex.equals(contactId, ignoreCase = true) }) {
            addContact(contactId, if (isSelf) "${_identity.value.name} (Вы)" else "Пользователь ${contactId.take(6)}")
        }

        val timeStr = getCurrentTimeStr()
        val msg = ChatMessage(
            id = P2PTransportEngine.newMessageId(_identity.value.nodeId),
            senderIdHex = _identity.value.nodeId,
            senderName = _identity.value.name,
            text = "Голосовое сообщение",
            isIncoming = false,
            timestampStr = timeStr,
            isDelivered = isSelf,
            isRead = isSelf,
            isVoice = true,
            voiceDurationSecs = durationSecs.coerceAtLeast(1),
            waveform = if (waveform.isNotEmpty()) waveform else generateDefaultWaveform(24),
            audioFilePath = audioFilePath
        )

        val currentList = _messages.value[contactId] ?: emptyList()
        _messages.value = _messages.value + (contactId to (currentList + msg))
        if (!saveMessages()) {
            _messages.value = _messages.value + (contactId to currentList)
            return
        }
        val contactSnapshot = _contacts.value

        val preview = "🎤 Голосовое (${formatVoiceDuration(durationSecs)})"
        _contacts.value = _contacts.value.map { contact ->
            if (contact.userIdHex == contactId) {
                contact.copy(
                    lastMessageText = preview,
                    lastMessageTime = timeStr,
                    unreadCount = 0
                )
            } else {
                contact
            }
        }
        saveContacts()

        if (!isSelf) {
            if (!P2PTransportEngine.sendVoiceMessage(contactId, durationSecs, _identity.value.nodeId, _identity.value.name, audioFilePath, msg.id)) {
                removeUnsentMessage(contactId, msg.id)
                _contacts.value = contactSnapshot
                saveContacts()
            }
        }
    }

    fun sendImageMessage(
        contactId: String,
        imageFilePath: String,
        caption: String = ""
    ) {
        if (!canUserPost(contactId)) return
        val isSelf = contactId.equals(_identity.value.nodeId, ignoreCase = true)
        if (_contacts.value.none { it.userIdHex.equals(contactId, ignoreCase = true) }) {
            addContact(contactId, if (isSelf) "${_identity.value.name} (Вы)" else "Пользователь ${contactId.take(6)}")
        }

        val timeStr = getCurrentTimeStr()
        val displayCaption = caption.ifBlank { "📷 Фотография" }
        val msg = ChatMessage(
            id = P2PTransportEngine.newMessageId(_identity.value.nodeId),
            senderIdHex = _identity.value.nodeId,
            senderName = _identity.value.name,
            text = displayCaption,
            isIncoming = false,
            timestampStr = timeStr,
            isDelivered = isSelf,
            isRead = isSelf,
            isImage = true,
            imageFilePath = imageFilePath
        )

        val currentList = _messages.value[contactId] ?: emptyList()
        _messages.value = _messages.value + (contactId to (currentList + msg))
        if (!saveMessages()) {
            _messages.value = _messages.value + (contactId to currentList)
            return
        }
        val contactSnapshot = _contacts.value

        _contacts.value = _contacts.value.map { contact ->
            if (contact.userIdHex.equals(contactId, ignoreCase = true)) {
                contact.copy(
                    lastMessageText = "📷 Фотография",
                    lastMessageTime = timeStr,
                    unreadCount = 0
                )
            } else contact
        }
        saveContacts()

        if (!isSelf) {
            if (!P2PTransportEngine.sendMediaMessage(
                contactId = contactId,
                mediaType = "image",
                filePath = imageFilePath,
                caption = displayCaption,
                myId = _identity.value.nodeId,
                myName = _identity.value.name,
                messageId = msg.id
            )) {
                removeUnsentMessage(contactId, msg.id)
                _contacts.value = contactSnapshot
                saveContacts()
            }
        }
    }

    fun sendFileMessage(
        contactId: String,
        filePath: String,
        fileName: String,
        fileSizeFormatted: String
    ) {
        if (!canUserPost(contactId)) return
        val isSelf = contactId.equals(_identity.value.nodeId, ignoreCase = true)
        if (_contacts.value.none { it.userIdHex.equals(contactId, ignoreCase = true) }) {
            addContact(contactId, if (isSelf) "${_identity.value.name} (Вы)" else "Пользователь ${contactId.take(6)}")
        }

        val timeStr = getCurrentTimeStr()
        val preview = "📁 $fileName"
        val msg = ChatMessage(
            id = P2PTransportEngine.newMessageId(_identity.value.nodeId),
            senderIdHex = _identity.value.nodeId,
            senderName = _identity.value.name,
            text = preview,
            isIncoming = false,
            timestampStr = timeStr,
            isDelivered = isSelf,
            isRead = isSelf,
            isFile = true,
            filePath = filePath,
            fileName = fileName,
            fileSizeFormatted = fileSizeFormatted
        )

        val currentList = _messages.value[contactId] ?: emptyList()
        _messages.value = _messages.value + (contactId to (currentList + msg))
        if (!saveMessages()) {
            _messages.value = _messages.value + (contactId to currentList)
            return
        }
        val contactSnapshot = _contacts.value

        _contacts.value = _contacts.value.map { contact ->
            if (contact.userIdHex.equals(contactId, ignoreCase = true)) {
                contact.copy(
                    lastMessageText = preview,
                    lastMessageTime = timeStr,
                    unreadCount = 0
                )
            } else contact
        }
        saveContacts()

        if (!isSelf) {
            if (!P2PTransportEngine.sendMediaMessage(
                contactId = contactId,
                mediaType = "file",
                filePath = filePath,
                caption = preview,
                fileName = fileName,
                fileSize = fileSizeFormatted,
                myId = _identity.value.nodeId,
                myName = _identity.value.name,
                messageId = msg.id
            )) {
                removeUnsentMessage(contactId, msg.id)
                _contacts.value = contactSnapshot
                saveContacts()
            }
        }
    }

    private val _activeCall = MutableStateFlow<CallState?>(null)
    val activeCall: StateFlow<CallState?> = _activeCall.asStateFlow()

    @Volatile
    var lastReceivedOfferSdp: String = ""
        private set

    fun startCall(peerId: String, peerName: String) {
        val cleanPeerId = peerId.trim().lowercase()
        val callId = "${System.currentTimeMillis()}"
        _activeCall.value = CallState(
            peerId = cleanPeerId,
            peerName = peerName,
            isIncoming = false,
            isConnected = false,
            isConnecting = true,
            durationSecs = 0,
            isMuted = false,
            isSpeakerOn = false,
            callId = callId
        )
        // Add outgoing call message to chat history (no notification for own action)
        addCallHistoryMessage(cleanPeerId, "📞 Исходящий вызов", isIncoming = false)
        val isSelf = cleanPeerId.equals(_identity.value.nodeId, ignoreCase = true)
        if (!isSelf) {
            // WebRtcCallManager owns call offers and sends the actual SDP. Sending
            // a placeholder Offer here races the real one and can make the peer
            // deduplicate away the SDP needed to establish media.
            appContext?.let { ctx ->
                WebRtcCallManager.startCall(ctx, cleanPeerId, callId)
            }
            P2PTransportEngine.triggerImmediatePoll()
        }
    }

    fun receiveIncomingCall(peerId: String, peerName: String, callId: String = "", offerSdp: String = "") {
        val cleanPeerId = peerId.trim().lowercase()
        if (offerSdp.isNotBlank()) {
            lastReceivedOfferSdp = offerSdp
        }
        _activeCall.value = CallState(
            peerId = cleanPeerId,
            peerName = peerName,
            isIncoming = true,
            isConnected = false,
            durationSecs = 0,
            isMuted = false,
            isSpeakerOn = false,
            callId = callId
        )
        // Add incoming call message to chat history
        addCallHistoryMessage(cleanPeerId, "📲 Входящий вызов", isIncoming = true)
        playNotificationChime()
    }

    fun refreshIncomingCallOffer(peerId: String, callId: String, offerSdp: String) {
        val current = _activeCall.value ?: return
        if (!current.isIncoming || current.callId != callId || !current.peerId.equals(peerId, true) || offerSdp.isBlank()) return
        lastReceivedOfferSdp = offerSdp
    }

    fun acceptCall() {
        val current = _activeCall.value ?: return
        val isSelf = current.peerId.equals(_identity.value.nodeId, ignoreCase = true)
        if (!isSelf) {
            val cid = current.callId.ifBlank { "call_${System.currentTimeMillis()}" }
            val offer = lastReceivedOfferSdp
            if (offer.isBlank()) {
                android.util.Log.w("NodeX_Call", "Cannot accept call $cid: offer SDP is missing")
                return
            }
            val context = appContext ?: return
            _activeCall.value = current.copy(isConnecting = true)
            WebRtcCallManager.acceptCall(context, current.peerId, cid, offer)
            P2PTransportEngine.triggerImmediatePoll()
        }
    }

    fun onCallAnswered(peerId: String) {
        val current = _activeCall.value ?: return
        if (current.peerId.equals(peerId.trim().lowercase(), ignoreCase = true) || peerId.isBlank()) {
            P2PTransportEngine.stopCallOfferRetransmission()
            if (!current.isConnected) {
                _activeCall.value = current.copy(isConnected = true, isConnecting = false, durationSecs = 0)
            }
        }
    }

    fun endCallFromRemote() {
        P2PTransportEngine.stopCallOfferRetransmission()
        WebRtcCallManager.endCall()
        val current = _activeCall.value
        if (current != null) {
            val callText = if (current.isConnected) {
                val mins = current.durationSecs / 60
                val secs = current.durationSecs % 60
                if (current.isIncoming) {
                    "📞 Входящий звонок • %d:%02d".format(mins, secs)
                } else {
                    "📞 Исходящий звонок • %d:%02d".format(mins, secs)
                }
            } else {
                if (current.isIncoming) {
                    "📵 Пропущенный вызов"
                } else {
                    "📵 Отменённый вызов"
                }
            }
            addCallHistoryMessage(current.peerId, callText, isIncoming = current.isIncoming)
        }
        _activeCall.value = null
    }

    fun endCall() {
        P2PTransportEngine.stopCallOfferRetransmission()
        WebRtcCallManager.endCall()
        val current = _activeCall.value
        if (current != null) {
            val isSelf = current.peerId.equals(_identity.value.nodeId, ignoreCase = true)
            if (!isSelf) {
                val cid = current.callId.ifBlank { "call_${System.currentTimeMillis()}" }
                P2PTransportEngine.sendCallSignal(current.peerId, "Hangup", cid, _identity.value.nodeId, "")
                P2PTransportEngine.triggerImmediatePoll()
            }
            // Add end/missed call message to chat history (no notification for own action)
            val callText = if (current.isConnected) {
                val mins = current.durationSecs / 60
                val secs = current.durationSecs % 60
                if (current.isIncoming) {
                    "📞 Входящий звонок • %d:%02d".format(mins, secs)
                } else {
                    "📞 Исходящий звонок • %d:%02d".format(mins, secs)
                }
            } else {
                if (current.isIncoming) {
                    "📵 Пропущенный вызов"
                } else {
                    "📵 Отменённый вызов"
                }
            }
            addCallHistoryMessage(current.peerId, callText, isIncoming = current.isIncoming)
        }
        _activeCall.value = null
    }

    /**
     * Adds a call history message to the chat. No notification is played for outgoing actions.
     */
    private fun addCallHistoryMessage(peerId: String, text: String, isIncoming: Boolean) {
        val cleanPeer = peerId.trim().lowercase()
        if (cleanPeer.isBlank() || cleanPeer.length < 16 || cleanPeer.startsWith("00000000") ||
            cleanPeer.startsWith("31.77.71.122") || cleanPeer.contains(":")) {
            return
        }
        if (cleanPeer.equals(_identity.value.nodeId.trim().lowercase(), ignoreCase = true)) return

        val timeStr = getCurrentTimeStr()
        val msg = ChatMessage(
            id = "call_${System.currentTimeMillis()}",
            senderIdHex = if (isIncoming) cleanPeer else _identity.value.nodeId,
            senderName = if (isIncoming) "" else _identity.value.name,
            text = text,
            isIncoming = isIncoming,
            timestampStr = timeStr,
            isDelivered = true,
            isRead = true
        )

        val currentList = _messages.value[cleanPeer] ?: emptyList()
        _messages.value = _messages.value + (cleanPeer to (currentList + msg))
        saveMessages()

        val updatedList = _contacts.value.toMutableList()
        val existingIndex = updatedList.indexOfFirst { it.userIdHex.equals(cleanPeer, ignoreCase = true) }
        val contactName = if (existingIndex >= 0 && updatedList[existingIndex].name.isNotBlank()) {
            updatedList[existingIndex].name
        } else {
            "Пользователь ${peerId.take(6)}"
        }

        val updatedContact = if (existingIndex >= 0) {
            val old = updatedList.removeAt(existingIndex)
            old.copy(
                lastMessageText = text,
                lastMessageTime = timeStr
            )
        } else {
            NodeContact(
                userIdHex = peerId,
                name = contactName,
                bio = "P2P контакт",
                isOnline = true,
                unreadCount = if (isIncoming) 1 else 0,
                lastMessageText = text,
                lastMessageTime = timeStr
            )
        }

        val selfIdx = updatedList.indexOfFirst { it.isSelf }
        if (selfIdx == 0) {
            updatedList.add(1, updatedContact)
        } else {
            updatedList.add(0, updatedContact)
        }
        _contacts.value = updatedList
        saveContacts()
    }

    fun toggleMute() {
        val current = _activeCall.value ?: return
        val newMuted = !current.isMuted
        _activeCall.value = current.copy(isMuted = newMuted)
        WebRtcCallManager.setMuted(newMuted)
    }

    fun toggleSpeaker() {
        val current = _activeCall.value ?: return
        val newSpeaker = !current.isSpeakerOn
        _activeCall.value = current.copy(isSpeakerOn = newSpeaker)
        appContext?.let { WebRtcCallManager.setSpeakerphone(it, newSpeaker) }
    }

    fun updateCallDuration(secs: Int) {
        val current = _activeCall.value ?: return
        _activeCall.value = current.copy(durationSecs = secs)
    }

    private fun generateDefaultWaveform(count: Int): List<Float> {
        val random = java.util.Random()
        return (0 until count).map {
            (0.2f + random.nextFloat() * 0.8f)
        }
    }

    fun formatVoiceDuration(seconds: Int): String {
        val m = seconds / 60
        val s = seconds % 60
        return "%d:%02d".format(m, s)
    }

    private fun getCurrentTimeStr(): String {
        return SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date())
    }

    private fun startNetworkMonitor() {
        scope.launch {
            while (isActive) {
                checkBootstrapPing()
                delay(10000)
            }
        }
    }

    private suspend fun checkBootstrapPing() = withContext(Dispatchers.IO) {
        val host = "31.77.71.122"
        val port = 8443
        if (_identity.value.nodeId.length != 40) {
            _networkStatus.value = NetworkStatus(
                isConnected = false,
                latencyMs = 0,
                bootstrapAddr = "$host:$port",
                peerCount = 0,
                is2HopOnionActive = false
            )
            return@withContext
        }
        var socket: DatagramSocket? = null
        try {
            val start = System.currentTimeMillis()
            val requestId = kotlin.random.Random.nextLong()
            socket = DatagramSocket()
            socket.soTimeout = 2500

            val pingData = java.nio.ByteBuffer.allocate(29).order(java.nio.ByteOrder.BIG_ENDIAN).apply {
                put(1) // MSG_PING
                putLong(requestId)
                put(decodeNodeId(_identity.value.nodeId))
            }.array()
            val address = InetAddress.getByName(host)
            val packet = DatagramPacket(pingData, pingData.size, address, port)
            socket.send(packet)
            val response = ByteArray(64)
            val reply = DatagramPacket(response, response.size)
            socket.receive(reply)
            val responseRequestId = if (reply.length >= 9) {
                java.nio.ByteBuffer.wrap(response, 1, 8).order(java.nio.ByteOrder.BIG_ENDIAN).long
            } else Long.MIN_VALUE
            if (reply.length < 29 || reply.address != address || reply.port != port
                || response[0] != 2.toByte() || responseRequestId != requestId
            ) error("Invalid bootstrap PONG")
            val rtt = System.currentTimeMillis() - start
            _networkStatus.value = NetworkStatus(
                isConnected = true,
                latencyMs = rtt,
                bootstrapAddr = "$host:$port",
                peerCount = 0,
                is2HopOnionActive = false
            )
        } catch (e: Exception) {
            _networkStatus.value = NetworkStatus(
                isConnected = false,
                latencyMs = 0,
                bootstrapAddr = "$host:$port",
                peerCount = 0,
                is2HopOnionActive = false
            )
        } finally {
            socket?.close()
        }
    }

    private fun decodeNodeId(hex: String): ByteArray {
        val clean = hex.trim()
        if (clean.length != 40 || clean.any { it.digitToIntOrNull(16) == null }) return ByteArray(20)
        return ByteArray(20) { index -> clean.substring(index * 2, index * 2 + 2).toInt(16).toByte() }
    }
}
