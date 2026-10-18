package network.nodex.messenger.data

import android.content.Context
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Collections
import java.util.Random
import java.util.concurrent.ConcurrentHashMap
import network.nodex.messenger.crypto.AndroidMessageCrypto

/**
 * Android UDP transport prototype for NodeX Messenger.
 * It speaks the current Kademlia-compatible packet format and provides direct
 * peer sends plus bootstrap mailbox polling with authenticated encrypted envelopes.
 */
object P2PTransportEngine {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var socket: DatagramSocket? = null
    @Volatile private var tcpServerSocket: java.net.ServerSocket? = null
    @Volatile private var isRunning = false
    private var localNodeIdHex: String = ""
    private var appContext: Context? = null

    private var receiveJob: Job? = null
    private var tcpJob: Job? = null
    private var pollJob: Job? = null
    private var deliveryJob: Job? = null

    fun stop() {
        publishPresence(online = false)
        isRunning = false
        receiveJob?.cancel(); receiveJob = null
        tcpJob?.cancel(); tcpJob = null
        pollJob?.cancel(); pollJob = null
        deliveryJob?.cancel(); deliveryJob = null
        try { tcpServerSocket?.close() } catch (_: Exception) {}
        tcpServerSocket = null
        try { socket?.close() } catch (_: Exception) {}
        socket = null
        activeOfferParams = null
        activeOfferSdp = ""
        contactEndpoints.clear()
        verifiedContactKeys.clear()
        lastCallAudioSequence.clear()
        remoteCallOfferHashes.clear()
        pendingPresenceQueries.clear()
        remoteCallOfferHashes.clear()
        pendingDeliveries.clear()
        activeAssemblies.clear()
        incomingAttachments.clear()
        attachmentAcks.values.forEach { it.cancel() }
        attachmentAcks.clear()
        localNodeIdHex = ""
        myPublicEndpoint = ""
    }

    @Volatile
    private var activeOfferParams: Triple<String, String, String>? = null // (contactId, callId, myId)
    @Volatile
    private var activeOfferSdp: String = ""

    // NodeX Wire Protocol Constants matching core-ffi
    private const val MSG_PING: Byte = 1
    private const val MSG_PONG: Byte = 2
    private const val MSG_STORE: Byte = 3
    private const val MSG_STORE_ACK: Byte = 4
    private const val MSG_FIND_VALUE: Byte = 7
    private const val MSG_FIND_VALUE_RESP: Byte = 8

    private const val BOOTSTRAP_HOST = "31.77.71.122"
    private const val BOOTSTRAP_PORT = 8443
    private const val DEFAULT_PORT = 8444

    // Deduplication cache for incoming messages and call events (LRU-capped at 3000)
    private const val MAX_SEEN_IDS = 3000
    private val seenMessageIds: MutableSet<String> = Collections.synchronizedSet(
        object : LinkedHashSet<String>() {
            override fun add(element: String): Boolean {
                val added = super.add(element)
                if (size > MAX_SEEN_IDS) {
                    val it = iterator()
                    it.next()
                    it.remove()
                }
                return added
            }
        }
    )

    // Direct endpoints cache: userIdHex -> "ip:port"
    private val contactEndpoints = ConcurrentHashMap<String, String>()
    private val verifiedContactKeys = ConcurrentHashMap<String, Pair<ByteArray, ByteArray>>()
    private val lastCallAudioSequence = ConcurrentHashMap<String, Long>()
    private val remoteCallOfferHashes = ConcurrentHashMap<String, String>()
    private val endedCallIds = java.util.Collections.newSetFromMap(ConcurrentHashMap<String, Boolean>())

    private fun rememberEndedCall(peerId: String, callId: String) {
        val key = "${peerId.trim().lowercase()}:$callId"
        if (key.endsWith(":")) return
        endedCallIds.add(key)
        while (endedCallIds.size > 256) endedCallIds.remove(endedCallIds.first())
        appContext?.getSharedPreferences("nodex_call_state", Context.MODE_PRIVATE)?.edit()
            ?.putStringSet("ended", endedCallIds.toSet())?.apply()
    }

    private fun isCallEnded(peerId: String, callId: String): Boolean {
        val key = "${peerId.trim().lowercase()}:$callId"
        val prefs = appContext?.getSharedPreferences("nodex_call_state", Context.MODE_PRIVATE)
        if (endedCallIds.isEmpty()) prefs?.getStringSet("ended", emptySet())?.let(endedCallIds::addAll)
        return key in endedCallIds
    }
    private val pendingPresenceQueries = ConcurrentHashMap<Long, String>()
    private data class PendingDelivery(val contactId: String, val packet: ByteArray, val createdAtMs: Long, @Volatile var nextAttemptMs: Long, @Volatile var attempts: Int = 0)
    private val pendingDeliveries = ConcurrentHashMap<String, PendingDelivery>()
    private const val ATTACHMENT_CHUNK_BYTES = 350
    private const val MAX_ATTACHMENT_BYTES = 20L * 1024 * 1024
    private data class IncomingAttachment(val senderId: String, val messageId: String, val manifest: JSONObject, val partFiles: Array<File?>, val createdAt: Long = System.currentTimeMillis())
    private val incomingAttachments = ConcurrentHashMap<String, IncomingAttachment>()
    private val attachmentAcks = ConcurrentHashMap<String, CompletableDeferred<Unit>>()

    // Delivery state must belong to one identity. A shared queue can otherwise
    // replay another account's messages after sign-out/sign-in on one device.
    private fun queueFile() = appContext?.filesDir?.let { filesDir ->
        val nodeId = localNodeIdHex.trim().lowercase()
        if (nodeId.length == 40 && nodeId.all { it.digitToIntOrNull(16) != null }) {
            File(File(filesDir, "nodex_accounts").apply { mkdirs() }, "$nodeId.delivery_queue.json")
        } else null
    }
    private fun persistQueue() {
        try {
            val file = queueFile() ?: return
            val rows = JSONArray()
            pendingDeliveries.entries.sortedBy { it.key }.forEach { (key, item) -> rows.put(JSONObject().apply {
                put("key", key); put("peer", item.contactId)
                put("packet", android.util.Base64.encodeToString(item.packet, android.util.Base64.NO_WRAP))
                put("created", item.createdAtMs); put("next", item.nextAttemptMs); put("attempts", item.attempts)
            }) }
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText(rows.toString())
            AndroidMessageCrypto.sealJsonFile(appContext!!, tmp, "delivery_queue".toByteArray())
            if (file.exists() && !file.delete()) error("Cannot replace delivery queue")
            if (!tmp.renameTo(file)) error("Cannot commit delivery queue")
        } catch (e: Exception) { android.util.Log.w("NodeX_P2P", "Delivery queue save failed", e) }
    }
    private fun restoreQueue() {
        try {
            val file = queueFile() ?: return
            if (!file.exists()) return
            val raw = if (AndroidMessageCrypto.isSealed(file)) AndroidMessageCrypto.openJsonFile(appContext!!, file, "delivery_queue".toByteArray()) else file.readBytes()
            val rows = JSONArray(String(raw, Charsets.UTF_8))
            for (i in 0 until rows.length()) {
                val row = rows.getJSONObject(i)
                if (pendingDeliveries.size >= 100) break
                val packet = android.util.Base64.decode(row.getString("packet"), android.util.Base64.NO_WRAP)
                val created = row.getLong("created")
                if (packet.size > 64 * 1024 || System.currentTimeMillis() - created > 7L * 24 * 60 * 60 * 1000) continue
                val key = row.getString("key")
                val peer = row.getString("peer")
                val separator = key.indexOf(':')
                if (separator != 40 || key.substring(0, separator) != peer || peer.any { it.digitToIntOrNull(16) == null }) continue
                if (pendingDeliveries.values.sumOf { it.packet.size.toLong() } + packet.size > 8L * 1024 * 1024) break
                pendingDeliveries[key] = PendingDelivery(peer, packet, created, row.getLong("next"), row.optInt("attempts"))
            }
        } catch (e: Exception) { android.util.Log.w("NodeX_P2P", "Delivery queue restore failed", e) }
    }

    private fun registerPendingDelivery(key: String, contactId: String, packet: ByteArray): Boolean {
        if (pendingDeliveries.containsKey(key)) return true
        if (pendingDeliveries.size >= 100 || pendingDeliveries.values.sumOf { it.packet.size.toLong() } + packet.size > 8L * 1024 * 1024) {
            android.util.Log.w("NodeX_P2P", "Delivery outbox is full; refusing another pending message")
            return false
        }
        val item = PendingDelivery(contactId, packet, System.currentTimeMillis(), System.currentTimeMillis())
        pendingDeliveries[key] = item
        persistQueue()
        if (queueFile()?.let { AndroidMessageCrypto.isSealed(it) } != true) {
            pendingDeliveries.remove(key, item)
            return false
        }
        return true
    }
    private var myPublicEndpoint: String = ""
    private val stunTransactionId = ByteArray(12).also { Random().nextBytes(it) }

    // Stream of real-time P2P call audio chunks (PCM 16kHz mono)
    private val _callAudioChunks = MutableSharedFlow<ByteArray>(extraBufferCapacity = 128)
    val callAudioChunks = _callAudioChunks.asSharedFlow()

    // NXCK Binary Chunk Assembler for Kademlia WAN Fragmentation Protocol
    private class ChunkAssembly(val totalCount: Int, val createdAt: Long = System.currentTimeMillis()) {
        val parts = mutableMapOf<Int, ByteArray>()
    }
    private val activeAssemblies = ConcurrentHashMap<String, ChunkAssembly>()

    private fun processIncomingChunk(data: ByteArray, senderKey: String): ByteArray? {
        if (data.size < 28) return null
        val msgIdBytes = data.copyOfRange(4, 20)
        val msgIdHex = bytesToHex(msgIdBytes)
        val chunkIdx = ByteBuffer.wrap(data, 20, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
        val chunkCount = ByteBuffer.wrap(data, 22, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
        if (chunkCount !in 1..256 || chunkIdx !in 0 until chunkCount || data.size > 528) return null
        val expectedCrc = ByteBuffer.wrap(data, 24, 4).order(ByteOrder.BIG_ENDIAN).int.toLong() and 0xFFFFFFFFL

        val payload = data.copyOfRange(28, data.size)
        val crc32 = java.util.zip.CRC32()
        crc32.update(payload)
        if (crc32.value != expectedCrc) {
            println("[P2P] CRC mismatch for chunk $chunkIdx of $msgIdHex")
            return null
        }

        val now = System.currentTimeMillis()
        activeAssemblies.entries.removeIf { now - it.value.createdAt > 30000L }

        val assemblyKey = "$senderKey:$msgIdHex"
        if (!activeAssemblies.containsKey(assemblyKey) && activeAssemblies.size >= 128) return null
        val assembly = activeAssemblies.computeIfAbsent(assemblyKey) { ChunkAssembly(chunkCount) }
        synchronized(assembly) {
            if (assembly.totalCount != chunkCount) {
                activeAssemblies.remove(assemblyKey, assembly)
                return null
            }
            assembly.parts[chunkIdx] = payload
            if (assembly.parts.size == assembly.totalCount) {
                activeAssemblies.remove(assemblyKey, assembly)
                val totalLen = assembly.parts.values.sumOf { it.size }
                if (totalLen > 128 * 1024) return null
                val full = ByteArray(totalLen)
                var cursor = 0
                for (i in 0 until assembly.totalCount) {
                    val p = assembly.parts[i] ?: return null
                    System.arraycopy(p, 0, full, cursor, p.size)
                    cursor += p.size
                }
                println("[P2P] Successfully reassembled $msgIdHex (${assembly.totalCount} chunks, $totalLen bytes)")
                return full
            }
        }
        return null
    }

    private fun splitIntoNxckChunks(data: ByteArray): List<ByteArray> {
        val chunkSize = 500
        val chunkCount = (data.size + chunkSize - 1) / chunkSize
        val msgId = ByteArray(16).also { Random().nextBytes(it) }
        val outList = mutableListOf<ByteArray>()

        for (idx in 0 until chunkCount) {
            val start = idx * chunkSize
            val end = (start + chunkSize).coerceAtMost(data.size)
            val slice = data.copyOfRange(start, end)

            val crc32 = java.util.zip.CRC32()
            crc32.update(slice)
            val crc = crc32.value.toInt()

            val chunkBuf = ByteBuffer.allocate(28 + slice.size).order(ByteOrder.BIG_ENDIAN)
            chunkBuf.put(0x4E.toByte()) // N
            chunkBuf.put(0x58.toByte()) // X
            chunkBuf.put(0x43.toByte()) // C
            chunkBuf.put(0x4B.toByte()) // K
            chunkBuf.put(msgId)
            chunkBuf.putShort(idx.toShort())
            chunkBuf.putShort(chunkCount.toShort())
            chunkBuf.putInt(crc)
            chunkBuf.put(slice)
            outList.add(chunkBuf.array())
        }
        return outList
    }

    fun start(context: Context, nodeIdHex: String) {
        appContext = context.applicationContext
        localNodeIdHex = nodeIdHex.trim().lowercase()
        isRunning = true
        restoreQueue()
        if (localNodeIdHex.isNotBlank()) {
            publishPresence()
        }

        // Initialize UDP DatagramSocket
        if (socket == null || socket!!.isClosed) {
            socket = try {
                DatagramSocket(DEFAULT_PORT).apply { broadcast = true }
            } catch (e: Exception) {
                try {
                    DatagramSocket().apply { broadcast = true }
                } catch (e2: Exception) {
                    null
                }
            }
            println("[P2P] Socket bound to port ${socket?.localPort} for node $localNodeIdHex")
        }

        // 1. Packet Receiver Coroutine
        if (receiveJob == null || !receiveJob!!.isActive) {
            receiveJob = scope.launch {
                val receiveBuffer = ByteArray(65536)
                while (isActive && isRunning) {
                    try {
                        val currentSocket = socket
                        if (currentSocket == null || currentSocket.isClosed) {
                            delay(100)
                            continue
                        }
                        val packet = DatagramPacket(receiveBuffer, receiveBuffer.size)
                        currentSocket.receive(packet)

                        val data = packet.data.copyOfRange(packet.offset, packet.offset + packet.length)
                        val senderAddress = packet.address
                        val senderPort = packet.port

                        handleIncomingPacket(data, senderAddress, senderPort)
                    } catch (e: Exception) {
                        if (!isRunning) break
                        delay(10)
                    }
                }
            }
        }

        // 2. TCP Fast Bridge Server for direct host/emulator delivery
        if (tcpJob == null || !tcpJob!!.isActive) {
            tcpJob = scope.launch {
                var localTcpServer: java.net.ServerSocket? = null
                try {
                    val tcpServer = java.net.ServerSocket(DEFAULT_PORT)
                    localTcpServer = tcpServer
                    tcpServerSocket = tcpServer
                    while (isActive && isRunning) {
                        try {
                            val client = tcpServer.accept()
                            scope.launch {
                                handleTcpClient(client)
                            }
                        } catch (e: Exception) {
                            if (!isRunning) break
                            delay(50)
                        }
                    }
                } catch (e: Exception) {
                    println("[P2P] TCP Server note: ${e.message}")
                } finally {
                    try { localTcpServer?.close() } catch (e: Exception) {}
                    if (tcpServerSocket === localTcpServer) tcpServerSocket = null
                }
            }
        }

        // 3. Periodic STUN Query, Bootstrap Keep-Alive, Peer Hole Punching & High-Speed Mailbox/Call Polling
        if (pollJob == null || !pollJob!!.isActive) {
            pollJob = scope.launch {
                var loopCount = 0
                while (isActive && isRunning) {
                    try {
                        // Keep-alive ping every ~6 seconds (loopCount % 20)
                        if (loopCount % 20 == 0) {
                            sendKeepAlivePing()
                        }

                        // Refresh STUN & Presence every ~15 seconds (loopCount % 50)
                        if (loopCount % 50 == 0) {
                            sendStunBindingRequest()
                            publishPresence()
                        }

                        // Continuous high-speed polling of Real-Time Call Signals and Mailbox messages
                        pollCallSignalsFromBootstrap()
                        pollMailboxFromBootstrap()
                    } catch (e: Exception) {
                        // ignore network glitches
                    }
                    loopCount++
                    // Keep mailbox delivery responsive without busy-spinning.
                    val baseDelay = if (NodeXRepository.activeCall.value != null) 750L else 1_500L
                    delay(baseDelay + kotlin.random.Random.nextLong(0, 501))
                }
            }
        }

        if (deliveryJob == null || !deliveryJob!!.isActive) {
            deliveryJob = scope.launch {
                while (isActive && isRunning) {
                    val now = System.currentTimeMillis()
                    pendingDeliveries.entries.toList().forEach { (id, pending) ->
                        if (now - pending.createdAtMs > 7L * 24 * 60 * 60 * 1000) {
                            pendingDeliveries.remove(id, pending)
                            persistQueue()
                        }
                        else if (now >= pending.nextAttemptMs) {
                            dispatchToCandidateEndpoints(pending.packet, pending.contactId)
                            pending.attempts += 1
                            pending.nextAttemptMs = now + (2_000L shl pending.attempts.coerceAtMost(5))
                            persistQueue()
                        }
                    }
                    delay(1000)
                }
            }
        }
    }

    fun triggerImmediatePoll() {
        scope.launch {
            pollCallSignalsFromBootstrap()
            pollMailboxFromBootstrap()
        }
    }

    fun retransmitOfferIfDialing() {
        val params = activeOfferParams ?: return
        val current = NodeXRepository.activeCall.value ?: return
        if (!current.isConnected && !current.isIncoming) {
            sendCallSignal(params.first, "Offer", params.second, params.third, activeOfferSdp)
        } else {
            activeOfferParams = null
            activeOfferSdp = ""
        }
    }

    fun stopCallOfferRetransmission() {
        activeOfferParams = null
        activeOfferSdp = ""
    }

    fun recordContactEndpoint(contactId: String, endpoint: String) {
        val cleanId = contactId.trim().lowercase()
        if (cleanId.isNotBlank() && endpoint.isNotBlank()) {
            contactEndpoints[cleanId] = endpoint
            sendDirectPingTo(endpoint)
        }
    }

    fun markMessageIdSeen(msgId: String) {
        if (msgId.isNotBlank()) {
            seenMessageIds.add(msgId)
        }
    }

    private fun sendStunBindingRequest() {
        try {
            val req = ByteArray(20)
            req[0] = 0x00
            req[1] = 0x01 // Binding Request
            req[2] = 0x00
            req[3] = 0x00 // Length 0
            // Magic Cookie 0x2112A442
            req[4] = 0x21
            req[5] = 0x12
            req[6] = 0xA4.toByte()
            req[7] = 0x42
            System.arraycopy(stunTransactionId, 0, req, 8, 12)

            val stunServers = listOf(
                Pair("stun.l.google.com", 19302),
                Pair("stun1.l.google.com", 19302)
            )
            for ((host, port) in stunServers) {
                try {
                    val addr = InetAddress.getByName(host)
                    socket?.send(DatagramPacket(req, req.size, addr, port))
                } catch (e: Exception) {}
            }
        } catch (e: Exception) {}
    }

    private fun parseStunResponse(data: ByteArray) {
        if (data.size < 20) return
        val msgType = ByteBuffer.wrap(data, 0, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
        if (msgType != 0x0101) return
        val magic = ByteBuffer.wrap(data, 4, 4).order(ByteOrder.BIG_ENDIAN).int
        if (magic != 0x2112A442) return

        val msgLen = ByteBuffer.wrap(data, 2, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
        var offset = 20
        val end = (20 + msgLen).coerceAtMost(data.size)

        while (offset + 4 <= end) {
            val attrType = ByteBuffer.wrap(data, offset, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
            val attrLen = ByteBuffer.wrap(data, offset + 2, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
            offset += 4
            if (offset + attrLen > end) break

            if (attrType == 0x0020) { // XOR-MAPPED-ADDRESS
                if (attrLen >= 8) {
                    val family = data[offset + 1].toInt()
                    if (family == 0x01) { // IPv4
                        val xorPort = ByteBuffer.wrap(data, offset + 2, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
                        val port = xorPort xor 0x2112
                        val b0 = (data[offset + 4].toInt() xor 0x21) and 0xFF
                        val b1 = (data[offset + 5].toInt() xor 0x12) and 0xFF
                        val b2 = (data[offset + 6].toInt() xor 0xA4) and 0xFF
                        val b3 = (data[offset + 7].toInt() xor 0x42) and 0xFF
                        myPublicEndpoint = "$b0.$b1.$b2.$b3:$port"
                        publishPresence()
                        return
                    }
                }
            } else if (attrType == 0x0001) { // MAPPED-ADDRESS
                if (attrLen >= 8) {
                    val family = data[offset + 1].toInt()
                    if (family == 0x01) {
                        val port = ByteBuffer.wrap(data, offset + 2, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xFFFF
                        val b0 = data[offset + 4].toInt() and 0xFF
                        val b1 = data[offset + 5].toInt() and 0xFF
                        val b2 = data[offset + 6].toInt() and 0xFF
                        val b3 = data[offset + 7].toInt() and 0xFF
                        myPublicEndpoint = "$b0.$b1.$b2.$b3:$port"
                        publishPresence()
                        return
                    }
                }
            }
            offset += attrLen
            val rem = attrLen % 4
            if (rem > 0) offset += (4 - rem)
        }
    }

    fun publishPresence(online: Boolean = true) {
        if (localNodeIdHex.isBlank()) return
        val send = {
            try {
                val cleanLocalId = localNodeIdHex.trim().lowercase()
                val nowSecs = System.currentTimeMillis() / 1000
                val identityKeys = AndroidMessageCrypto.identityKeys(NodeXRepository.identity.value.mnemonic)
                val presenceObj = JSONObject().apply {
                    put("user_id_hex", cleanLocalId)
                    put("socket_addr", myPublicEndpoint.ifBlank { "0.0.0.0:${socket?.localPort ?: DEFAULT_PORT}" })
                    put("ed25519_pub_hex", bytesToHex(identityKeys.edPublic))
                    put("x25519_pub_hex", bytesToHex(identityKeys.xPublic))
                    put("online", online)
                    put("timestamp", nowSecs)
                }
                presenceObj.put(
                    "signature_hex",
                    AndroidMessageCrypto.signPresence(
                        identityKeys,
                        cleanLocalId,
                        presenceObj.getString("socket_addr"),
                        bytesToHex(identityKeys.xPublic),
                        online,
                        nowSecs
                    )
                )
                val valueBytes = presenceObj.toString().toByteArray(Charsets.UTF_8)
                val presenceKeyBytes = presenceKey(cleanLocalId)

                val packet = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanLocalId,
                    keyBytes = presenceKeyBytes,
                    valueBytes = valueBytes
                )
                val addr = InetAddress.getByName(BOOTSTRAP_HOST)
                socket?.send(DatagramPacket(packet, packet.size, addr, BOOTSTRAP_PORT))
            } catch (e: Exception) {}
        }
        if (online) scope.launch { send() } else send()
    }

    fun queryPeerPresence(peerIdHex: String) {
        val cleanPeerId = peerIdHex.trim().lowercase()
        if (cleanPeerId.isBlank() || localNodeIdHex.isBlank()) return
        scope.launch {
            try {
                val presenceKeyBytes = presenceKey(cleanPeerId)
                val reqId = Random().nextLong()
                if (pendingPresenceQueries.size >= 1024) pendingPresenceQueries.clear()
                pendingPresenceQueries[reqId] = cleanPeerId
                val buf = ByteBuffer.allocate(49).order(ByteOrder.BIG_ENDIAN).apply {
                    put(MSG_FIND_VALUE)
                    putLong(reqId)
                    put(hexToBytes(localNodeIdHex, 20))
                    put(presenceKeyBytes)
                }.array()
                val addr = InetAddress.getByName(BOOTSTRAP_HOST)
                socket?.send(DatagramPacket(buf, buf.size, addr, BOOTSTRAP_PORT))
            } catch (e: Exception) {}
        }
    }

    private fun sendKeepAlivePing() {
        val pingPkt = ByteBuffer.allocate(29).order(ByteOrder.BIG_ENDIAN).apply {
            put(MSG_PING)
            putLong(Random().nextLong())
            put(hexToBytes(localNodeIdHex, 20))
        }.array()

        val targets = mutableListOf(
            Pair(BOOTSTRAP_HOST, BOOTSTRAP_PORT),
            Pair("255.255.255.255", DEFAULT_PORT),
            Pair("10.0.2.2", DEFAULT_PORT)
        )

        // Active bidirectional UDP NAT hole punching to all peer endpoints
        for ((_, ep) in contactEndpoints) {
            val parts = ep.split(":")
            if (parts.size == 2) {
                val host = parts[0]
                val port = parts[1].toIntOrNull()
                if (port != null && host.isNotBlank() && host != "0.0.0.0" && host != "127.0.0.1") {
                    targets.add(Pair(host, port))
                }
            }
        }

        for ((host, port) in targets) {
            try {
                val addr = InetAddress.getByName(host)
                socket?.send(DatagramPacket(pingPkt, pingPkt.size, addr, port))
            } catch (e: Exception) {}
        }
    }

    private fun sendDirectPingTo(endpoint: String) {
        try {
            val parts = endpoint.split(":")
            if (parts.size == 2) {
                val host = parts[0]
                val port = parts[1].toIntOrNull()
                if (port != null && host.isNotBlank() && host != "0.0.0.0") {
                    val pingPkt = ByteBuffer.allocate(29).order(ByteOrder.BIG_ENDIAN).apply {
                        put(MSG_PING)
                        putLong(Random().nextLong())
                        put(hexToBytes(localNodeIdHex, 20))
                    }.array()
                    val addr = InetAddress.getByName(host)
                    socket?.send(DatagramPacket(pingPkt, pingPkt.size, addr, port))
                }
            }
        } catch (e: Exception) {}
    }

    /**
     * Poll incoming messages deposited on the Bootstrap Kademlia node using MSG_FIND_VALUE.
     */
    private fun pollMailboxFromBootstrap() {
        if (localNodeIdHex.isBlank()) return
        val cleanLocalId = localNodeIdHex.trim().lowercase()
        val nowSecs = System.currentTimeMillis() / 1000

        // 1. Permanent primary mailbox key (deterministic, clock-independent)
        sendFindValue(mailboxKey(cleanLocalId))

        // 2. Day-bucket fallback mailbox key
        sendFindValue(deriveMailboxKeyId(cleanLocalId, nowSecs))
    }

    /**
     * Poll incoming high-priority call signals (Offer, Answer, Hangup) from Bootstrap DHT.
     */
    private fun pollCallSignalsFromBootstrap() {
        if (localNodeIdHex.isBlank()) return
        val cleanLocalId = localNodeIdHex.trim().lowercase()
        sendFindValue(callKey(cleanLocalId))
    }

    private fun sendFindValue(keyBytes: ByteArray) {
        try {
            val reqId = Random().nextLong()
            val buf = ByteBuffer.allocate(49).order(ByteOrder.BIG_ENDIAN).apply {
                put(MSG_FIND_VALUE)
                putLong(reqId)
                put(hexToBytes(localNodeIdHex, 20))
                put(keyBytes)
            }.array()

            val addr = InetAddress.getByName(BOOTSTRAP_HOST)
            val dp = DatagramPacket(buf, buf.size, addr, BOOTSTRAP_PORT)
            socket?.send(dp)
        } catch (e: Exception) {}
    }

    private fun handleIncomingPacket(data: ByteArray, senderAddress: InetAddress, senderPort: Int) {
        if (data.isEmpty()) return

        if (data.size >= 8 && data[0] == 0x4E.toByte() && data[1] == 0x58.toByte() && data[2] == 0x41.toByte() && data[3] == 0x41.toByte()) {
            val length = ByteBuffer.wrap(data, 4, 4).order(ByteOrder.BIG_ENDIAN).int
            if (length in 2..1024 && data.size == 8 + length) {
                val ack = runCatching { JSONObject(String(data, 8, length, Charsets.UTF_8)) }.getOrNull()
                if (ack != null) {
                    val peerId = ack.optString("sender_id", "").lowercase()
                    val peerEd = NodeXRepository.getVerifiedPeerEd25519Key(peerId)
                    if (peerEd != null && AndroidMessageCrypto.verifyAttachmentAck(peerEd, ack, peerId, localNodeIdHex)) {
                        attachmentAcks["${ack.optString("attachment_id")}:${ack.optInt("index", -1)}"]?.complete(Unit)
                    }
                }
            }
            return
        }

        if (data.size >= 4 && data[0] == 0x4E.toByte() && data[1] == 0x58.toByte() && data[2] == 0x41.toByte() && data[3] == 0x54.toByte()) {
            handleAttachmentDatagram(data, senderAddress, senderPort)
            return
        }

        if (data.size >= 4 && data.copyOfRange(0, 4).contentEquals(byteArrayOf(0x4E, 0x58, 0x43, 0x41))) {
            handleCallAudioDatagram(data, senderAddress, senderPort)
            return
        }

        // 0. Binary UDP Chunks (NXCK) reassembly matching nodex-kademlia WAN chunking protocol
        if (data.size >= 28 && data[0] == 0x4E.toByte() && data[1] == 0x58.toByte() && data[2] == 0x43.toByte() && data[3] == 0x4B.toByte()) {
            val completeBytes = processIncomingChunk(data, "${senderAddress.hostAddress}:$senderPort")
            if (completeBytes != null) {
                handleIncomingPacket(completeBytes, senderAddress, senderPort)
            }
            return
        }

        // 1. Check for STUN response (starts with 0x01, 0x01, magic 0x2112A442)
        if (data.size >= 20 && data[0] == 0x01.toByte() && data[1] == 0x01.toByte() &&
            data[4] == 0x21.toByte() && data[5] == 0x12.toByte() && data[6] == 0xA4.toByte() && data[7] == 0x42.toByte()) {
            parseStunResponse(data)
            return
        }

        when (val msgType = data[0]) {
            MSG_PING -> {
                if (data.size >= 9) {
                    val reqId = ByteBuffer.wrap(data, 1, 8).order(ByteOrder.BIG_ENDIAN).long
                    sendPong(reqId, senderAddress, senderPort)
                }
            }

            MSG_PONG -> {
                android.util.Log.d("NodeX_P2P", "Received PONG from ${senderAddress.hostAddress}:$senderPort")
            }

            MSG_STORE -> {
                if (data.size >= 53) {
                    val reqId = ByteBuffer.wrap(data, 1, 8).order(ByteOrder.BIG_ENDIAN).long
                    val senderIdBytes = data.copyOfRange(9, 29)
                    val senderHex = bytesToHex(senderIdBytes)

                    if (senderHex.equals(localNodeIdHex, ignoreCase = true)) return

                    val valueLen = ByteBuffer.wrap(data, 49, 4).order(ByteOrder.BIG_ENDIAN).int.coerceAtLeast(0)
                    android.util.Log.d("NodeX_P2P", "Received MSG_STORE from $senderHex at ${senderAddress.hostAddress}:$senderPort (valLen=$valueLen)")
                    if (data.size >= 53 + valueLen) {
                        val valueBytes = data.copyOfRange(53, 53 + valueLen)
                        processStorePayload(valueBytes, senderHex, "${senderAddress.hostAddress}:$senderPort")
                        sendStoreAck(reqId, senderAddress, senderPort)
                    }
                }
            }

            MSG_FIND_VALUE_RESP -> {
                if (data.size >= 30) {
                    val reqId = ByteBuffer.wrap(data, 1, 8).order(ByteOrder.BIG_ENDIAN).long
                    val senderIdBytes = data.copyOfRange(9, 29)
                    val senderHex = bytesToHex(senderIdBytes)
                    val hasValue = data[29].toInt() != 0

                    if (!hasValue) {
                        pendingPresenceQueries.remove(reqId)?.let { peerId ->
                            NodeXRepository.updateContactOnlineStatus(peerId, false)
                        }
                    }

                    if (hasValue && data.size >= 34) {
                        val valueLen = ByteBuffer.wrap(data, 30, 4).order(ByteOrder.BIG_ENDIAN).int.coerceAtLeast(0)
                        android.util.Log.d("NodeX_P2P", "Received MSG_FIND_VALUE_RESP with value! (len=$valueLen)")
                        if (data.size >= 34 + valueLen) {
                            val valueBytes = data.copyOfRange(34, 34 + valueLen)
                            val jsonStr = String(valueBytes, Charsets.UTF_8).trim()

                            // Check if this is a Presence Card lookup response
                            if (jsonStr.startsWith("{") && jsonStr.contains("\"socket_addr\"")) {
                                try {
                                    val obj = JSONObject(jsonStr)
                                    val peerUid = obj.optString("user_id_hex", "").trim().lowercase()
                                    val sAddr = obj.optString("socket_addr", "")
                                    val online = obj.optBoolean("online", true)
                                    val timestamp = obj.optLong("timestamp", 0L)
                                    val fresh = timestamp > 0L && kotlin.math.abs(System.currentTimeMillis() / 1000 - timestamp) <= 180L
                                    val edHex = obj.optString("ed25519_pub_hex", "").trim().lowercase()
                                    val xHex = obj.optString("x25519_pub_hex", "").trim().lowercase()
                                    val authenticatedPresence = AndroidMessageCrypto.verifyPresence(obj)
                                    if (authenticatedPresence && peerUid.length == 40 && edHex.length == 64 && xHex.length == 64) {
                                        val ed = hexToBytes(edHex, 32)
                                        val x = hexToBytes(xHex, 32)
                                        val derived = MessageDigest.getInstance("SHA-1").digest(ed)
                                        if (bytesToHex(derived).equals(peerUid, ignoreCase = true)) {
                                            val previousX = NodeXRepository.getVerifiedPeerX25519Key(peerUid)
                                            NodeXRepository.updateVerifiedContactKeys(peerUid, ed, x, authenticatedRefresh = true)
                                            pendingPresenceQueries.entries.removeIf { it.value == peerUid }
                                            if (previousX != null && !previousX.contentEquals(x)) {
                                                reencryptPendingMessagesForPeer(peerUid)
                                            }
                                        }
                                    }
                                    if (authenticatedPresence && peerUid.isNotBlank() && sAddr.isNotBlank() && !sAddr.startsWith("0.0.0.0")) {
                                        contactEndpoints[peerUid] = sAddr
                                        NodeXRepository.updateContactEndpoint(peerUid, sAddr, online && fresh)
                                        sendDirectPingTo(sAddr)
                                    } else if (peerUid.isNotBlank()) {
                                        pendingPresenceQueries.entries.removeIf { it.value == peerUid }
                                        NodeXRepository.updateContactOnlineStatus(peerUid, false)
                                    }
                                } catch (e: Exception) {}
                            } else {
                                // Mailbox message envelopes are validated by the v3 envelope handler.
                                processStorePayload(valueBytes, senderHex, "${senderAddress.hostAddress}:$senderPort")
                            }
                        }
                    }
                }
            }

        }

    }

    private fun handleCallAudioDatagram(data: ByteArray, senderAddress: InetAddress, senderPort: Int) {
        val minimumSize = 4 + 20 + 20 + 1 + 1 + 8 + 4 + 12 + 32 + 2 + 16
        if (data.size < minimumSize) return
        val senderHex = bytesToHex(data.copyOfRange(4, 24))
        val targetHex = bytesToHex(data.copyOfRange(24, 44))
        if (targetHex != localNodeIdHex || senderHex == localNodeIdHex) return
        val idLength = data[44].toInt() and 0xff
        if (idLength !in 1..128) return
        var cursor = 45
        if (data.size < cursor + idLength + 8 + 4 + 12 + 32 + 2 + 16) return
        val callId = String(data, cursor, idLength, Charsets.UTF_8); cursor += idLength
        val seq = ByteBuffer.wrap(data, cursor, 8).order(ByteOrder.BIG_ENDIAN).long; cursor += 8
        val sampleRate = ByteBuffer.wrap(data, cursor, 4).order(ByteOrder.BIG_ENDIAN).int; cursor += 4
        if (sampleRate !in 8_000..48_000) return
        val nonce = data.copyOfRange(cursor, cursor + 12); cursor += 12
        val ephemeralPublic = data.copyOfRange(cursor, cursor + 32); cursor += 32
        val ciphertextLength = ByteBuffer.wrap(data, cursor, 2).order(ByteOrder.BIG_ENDIAN).short.toInt() and 0xffff; cursor += 2
        if (ciphertextLength !in 16..1100 || data.size != cursor + ciphertextLength) return
        val peerKey = NodeXRepository.getVerifiedPeerX25519Key(senderHex) ?: return
        val plaintext = try {
            AndroidMessageCrypto.openCallAudio(
                NodeXRepository.identity.value.mnemonic, peerKey, ephemeralPublic, nonce, data.copyOfRange(cursor, data.size),
                data.copyOfRange(4, 24), data.copyOfRange(24, 44), callId, seq, sampleRate
            )
        } catch (_: Exception) { return }
        if (plaintext.size % 2 != 0) return
        val active = NodeXRepository.activeCall.value ?: return
        if (!active.isConnected || active.peerId != senderHex || active.callId != callId) return
        contactEndpoints[senderHex] = "${senderAddress.hostAddress}:$senderPort"
        NodeXRepository.updateContactEndpoint(senderHex, "${senderAddress.hostAddress}:$senderPort")
        val streamKey = "$senderHex:$callId"
        val previous = lastCallAudioSequence[streamKey]
        if (previous != null && seq <= previous) return
        if (lastCallAudioSequence.size >= 4096 && previous == null) return
        lastCallAudioSequence[streamKey] = seq
        _callAudioChunks.tryEmit(ByteBuffer.allocate(8 + 4 + 4 + plaintext.size).order(ByteOrder.BIG_ENDIAN)
            .putLong(seq).putInt(sampleRate).putInt(plaintext.size).put(plaintext).array())
    }

    private fun sendPong(reqId: Long, address: InetAddress, port: Int) {
        try {
            val buf = ByteBuffer.allocate(29).order(ByteOrder.BIG_ENDIAN).apply {
                put(MSG_PONG)
                putLong(reqId)
                put(hexToBytes(localNodeIdHex, 20))
            }.array()
            val packet = DatagramPacket(buf, buf.size, address, port)
            socket?.send(packet)
        } catch (e: Exception) {}
    }

    private fun sendStoreAck(reqId: Long, address: InetAddress, port: Int) {
        try {
            val buf = ByteBuffer.allocate(30).order(ByteOrder.BIG_ENDIAN).apply {
                put(MSG_STORE_ACK)
                putLong(reqId)
                put(hexToBytes(localNodeIdHex, 20))
                put(0.toByte())
            }.array()
            val packet = DatagramPacket(buf, buf.size, address, port)
            socket?.send(packet)
        } catch (e: Exception) {}
    }

    private fun processStorePayload(valueBytes: ByteArray, senderHex: String, senderEndpoint: String) {
        try {
            val jsonStr = String(valueBytes, Charsets.UTF_8).trim()
            if (jsonStr.isEmpty() || jsonStr.contains("\"drained\":true")) return
            // Mailbox values are normally arrays, but older bootstrap nodes
            // may return a single envelope. Accept both wire forms.
            val entries = if (jsonStr.startsWith("[")) {
                JSONArray(jsonStr)
            } else {
                JSONArray().put(JSONObject(jsonStr))
            }
            if (entries.length() == 0) return
            for (i in 0 until entries.length()) {
                val envelope = entries.optJSONObject(i) ?: continue
                if (!envelope.has("ciphertext") || envelope.optInt("version", 0) < 3) continue
            }
            for (i in 0 until entries.length()) {
                val envelope = entries.getJSONObject(i)
                // Do not reject a mailbox envelope only because the locally
                // cached contact key is old. openEnvelope below verifies the
                // signature and derives the sender id from the envelope key;
                // that authenticated path is the source of truth after an
                // account reinstall or identity refresh.
                handleEnvelopeObject(envelope, senderHex, senderEndpoint)
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun handleEnvelopeObject(env: JSONObject, fallbackSenderHex: String, senderEndpoint: String) {
        if (env.optInt("version", 0) < 3) return
        val actualSenderHex = if (env.has("sender_id")) {
            val sId = env.get("sender_id")
            if (sId is JSONArray) {
                val bytes = ByteArray(sId.length()) { sId.getInt(it).toByte() }
                bytesToHex(bytes)
            } else sId.toString().trim().lowercase()
        } else {
            val cleanFallback = fallbackSenderHex.trim().lowercase()
            if (cleanFallback.length >= 16 && !cleanFallback.startsWith("00000000") && !senderEndpoint.startsWith(BOOTSTRAP_HOST)) {
                cleanFallback
            } else ""
        }

        if (actualSenderHex.isBlank() || actualSenderHex.length < 16 || actualSenderHex.startsWith("00000000")) return
        if (actualSenderHex.equals(localNodeIdHex, ignoreCase = true)) return

        try {
            val (verifiedSenderId, payload) = AndroidMessageCrypto.openEnvelope(NodeXRepository.identity.value.mnemonic, env)
            if (!verifiedSenderId.equals(actualSenderHex, ignoreCase = true)) return
            // Presence becomes online only after an authenticated envelope
            // proves the claimed sender id; raw UDP header ids are spoofable.
            if (senderEndpoint.isNotBlank() && !senderEndpoint.startsWith(BOOTSTRAP_HOST) &&
                !senderEndpoint.startsWith("0.0.0.0") && !senderEndpoint.startsWith("127.0.0.1") &&
                !senderEndpoint.startsWith("10.0.2.2")) {
                contactEndpoints[verifiedSenderId] = senderEndpoint
                NodeXRepository.updateContactEndpoint(verifiedSenderId, senderEndpoint, online = true)
            }
            val edKey = env.getBytes("sender_ed25519_pub")
            val xKey = env.getBytes("sender_x25519_pub")
            // The envelope signature and the SHA-1(ed25519 public key) binding
            // have already been verified by openEnvelope. Refresh stale cached
            // contact keys instead of silently dropping otherwise valid mail.
            verifiedContactKeys[verifiedSenderId] = edKey to xKey
            NodeXRepository.updateVerifiedContactKeys(verifiedSenderId, edKey, xKey, authenticatedRefresh = true)
            handleDirectPayload(payload, verifiedSenderId, senderEndpoint)
            val messageId = payload.optString("id", "")
            val isChatMessage = !payload.has("call_signal") && !payload.has("delivery_receipt") && !payload.has("attachment_manifest")
            if (isChatMessage) {
                if (messageId.isBlank() || !NodeXRepository.hasPersistedMessage(verifiedSenderId, messageId)) return
                sendDeliveryReceipt(verifiedSenderId, messageId)
            }
        } catch (e: Exception) {
            android.util.Log.w("NodeX_P2P", "Rejected invalid or undecryptable envelope", e)
        }
    }

    private fun handleDirectPayload(payload: JSONObject, senderHex: String, senderEndpoint: String) {
        val cleanSenderHex = senderHex.trim().lowercase()
        if (cleanSenderHex.isBlank() || cleanSenderHex.length < 16 || cleanSenderHex.startsWith("00000000")) return
        if (cleanSenderHex.equals(localNodeIdHex, ignoreCase = true)) return

        val senderName = "Узел ${cleanSenderHex.take(6)}"
        val msgId = payload.optString("id", "")

        if (payload.has("delivery_receipt")) {
            val receipt = payload.getJSONObject("delivery_receipt")
            val messageId = receipt.optString("message_id", "")
            val publicKey = NodeXRepository.getVerifiedPeerEd25519Key(cleanSenderHex) ?: return
            if (messageId.isBlank() || !AndroidMessageCrypto.verifyDeliveryReceipt(publicKey, receipt, cleanSenderHex)) return
            val key = "$cleanSenderHex:$messageId"
            // A receipt is authoritative even if the app restarted and the
            // in-memory delivery queue no longer contains this message.
            pendingDeliveries.remove(key)
            persistQueue()
            NodeXRepository.markMessageDelivered(cleanSenderHex, messageId)
            return
        }

        // 1. High-Priority Call Signaling
        if (payload.has("call_signal")) {
            val sig = payload.getJSONObject("call_signal")
            val signalType = sig.optString("signal_type", "")
            val callId = sig.optString("call_id", "")
            val callerId = sig.optString("caller_id_hex", cleanSenderHex).ifBlank { cleanSenderHex }.trim().lowercase()
            val remoteEndpoint = sig.optString("endpoint", "")
            val sigTimestamp = sig.optLong("timestamp", 0L)
            val nowSecs = System.currentTimeMillis() / 1000

            val calleeId = sig.optString("callee_id_hex", "").trim().lowercase()
            if (callerId != cleanSenderHex || calleeId != localNodeIdHex || callId.isBlank()) return

            // Call control messages expire quickly. Also reject future timestamps,
            // which otherwise survive a one-sided age check.
            // DHT mailbox delivery can legitimately take longer than a direct
            // UDP hop. Keep call control messages valid long enough for a
            // cold-started phone or emulator to poll and answer.
            if (sigTimestamp <= 0 || kotlin.math.abs(nowSecs - sigTimestamp) > 45) {
                return
            }

            // DHT mailbox values can be replayed after a call was already
            // canceled. A signed end signal permanently tombstones that call
            // id on this account so an old Offer cannot reopen its UI.
            if (signalType == "Hangup" || signalType == "Reject") {
                rememberEndedCall(callerId, callId)
            } else if (signalType == "Offer" && isCallEnded(callerId, callId)) {
                return
            }

            val currentCall = NodeXRepository.activeCall.value
            if (signalType != "Offer" && (currentCall == null
                    || currentCall.peerId != callerId
                    || currentCall.callId != callId)) return

            android.util.Log.d("NodeX_P2P", "Processing CallSignal: $signalType for call $callId from $callerId")

            if (remoteEndpoint.isNotBlank() && !remoteEndpoint.startsWith("0.0.0.0") && !remoteEndpoint.startsWith(BOOTSTRAP_HOST)) {
                contactEndpoints[callerId] = remoteEndpoint
                NodeXRepository.updateContactEndpoint(callerId, remoteEndpoint)
                sendDirectPingTo(remoteEndpoint)
            }

            val callData = sig.optString("data", "")

            when (signalType) {
                "Offer" -> {
                    val offerKey = "$callerId:$callId"
                    val offerHash = bytesToHex(MessageDigest.getInstance("SHA-256").digest(callData.toByteArray(Charsets.UTF_8)))
                    val previousHash = remoteCallOfferHashes.put(offerKey, offerHash)
                    val sameCall = NodeXRepository.activeCall.value?.let {
                        it.peerId.equals(callerId, true) && it.callId == callId
                    } == true
                    if (previousHash == null && !sameCall) {
                        android.util.Log.d("NodeX_P2P", "Incoming call offer received from $callerId")
                        NodeXRepository.receiveIncomingCall(callerId, "Пользователь ${callerId.take(6)}", callId, callData)
                    } else if (previousHash != offerHash) {
                        android.util.Log.d("NodeX_P2P", "Updated call offer received for call $callId")
                        NodeXRepository.refreshIncomingCallOffer(callerId, callId, callData)
                        network.nodex.messenger.webrtc.WebRtcCallManager.onRemoteOfferReceived(callData)
                    }
                }
                "Answer" -> {
                    val dedupeKey = "call_${callId}_${callerId}_Answer"
                    seenMessageIds.add(dedupeKey)
                    android.util.Log.d("NodeX_P2P", "Call Answer received from $callerId")
                    if (callData.isNotBlank()) {
                        network.nodex.messenger.webrtc.WebRtcCallManager.onAnswerReceived(callData)
                    }
                }
                "IceCandidate" -> {
                    network.nodex.messenger.webrtc.WebRtcCallManager.onRemoteIceCandidate(callData)
                }
                "Connected" -> {
                    val dedupeKey = "call_${callId}_${callerId}_Connected"
                    seenMessageIds.add(dedupeKey)
                    android.util.Log.d("NodeX_P2P", "Call Connected signal received from $callerId")
                    if (NodeXRepository.activeCall.value?.isConnected == true) {
                        NodeXRepository.onCallAnswered(callerId)
                    }
                }
                "Hangup", "Reject" -> {
                    val dedupeKey = "call_${callId}_${callerId}_$signalType"
                    if (seenMessageIds.add(dedupeKey)) {
                        android.util.Log.d("NodeX_P2P", "Call $signalType signal received from $callerId - ending call")
                        network.nodex.messenger.webrtc.WebRtcCallManager.endCall()
                        NodeXRepository.endCallFromRemote()
                    }
                }
            }
            return
        }

        if (payload.has("attachment_manifest")) {
            runCatching {
                val manifest = payload.getJSONObject("attachment_manifest")
                val id = manifest.getString("attachment_id")
                val size = manifest.getLong("file_size")
                val count = manifest.getInt("chunk_count")
                val chunkSize = manifest.getInt("chunk_size")
                require(manifest.optInt("version") == 1 && size in 1..MAX_ATTACHMENT_BYTES && chunkSize == ATTACHMENT_CHUNK_BYTES)
                require(count == ((size + chunkSize - 1) / chunkSize).toInt() && count in 1..100_000)
                require(manifest.getString("sha256").matches(Regex("[0-9a-f]{64}")))
                require(incomingAttachments.size < 4 && incomingAttachments.values.sumOf { it.manifest.optLong("file_size") } <= 40L * 1024 * 1024 - size)
                if (incomingAttachments.containsKey(id)) return
                val transfer = IncomingAttachment(cleanSenderHex, msgId, JSONObject(manifest.toString()), arrayOfNulls(count))
                incomingAttachments.putIfAbsent(id, transfer)
                NodeXRepository.receiveIncomingMessage(
                    senderId = cleanSenderHex, senderName = senderName,
                    text = payload.optString("text", manifest.optString("file_name")), messageId = msgId,
                    isImage = manifest.optString("media_type") == "image", isFile = manifest.optString("media_type") != "image",
                    fileName = manifest.optString("file_name"), fileSizeFormatted = "Получение…",
                    attachmentId = id, attachmentSizeBytes = size, attachmentMimeType = manifest.optString("mime_type")
                )
            }.onFailure { android.util.Log.w("NodeX_P2P", "Rejected invalid attachment manifest", it) }
            return
        }

        // Deduplicate chat messages. Keep the id out of the in-memory set until
        // persistence succeeds, otherwise a transient JSON/storage error would
        // permanently drop the message and the sender would stay on a clock.
        val effMsgId = msgId.ifBlank { "${cleanSenderHex}_${payload.optLong("timestamp", 0L)}_${payload.optString("text").hashCode()}" }
        if (seenMessageIds.contains(effMsgId) || NodeXRepository.hasPersistedMessage(cleanSenderHex, effMsgId)) {
            return
        }

        // 2. Voice Note with Real AAC Audio Bytes
        if (payload.has("voice_note") && !payload.isNull("voice_note")) {
            val vn = payload.getJSONObject("voice_note")
            val duration = vn.optInt("duration_seconds", 3)
            val audioBase64 = vn.optString("audio_base64", "")
            var localAudioPath = ""

            if (audioBase64.isNotBlank()) {
                try {
                    val bytes = android.util.Base64.decode(audioBase64, android.util.Base64.NO_WRAP)
                    val ctx = appContext
                    if (ctx != null && bytes.isNotEmpty()) {
                        val audioDir = File(ctx.filesDir, "audio_notes").apply { mkdirs() }
                        val rxFile = File(audioDir, "rx_${System.currentTimeMillis()}_${cleanSenderHex.take(6)}.m4a")
                        rxFile.writeBytes(bytes)
                        if (rxFile.exists() && rxFile.length() > 50L) {
                            localAudioPath = rxFile.absolutePath
                        }
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }

            if (NodeXRepository.receiveIncomingMessage(
                senderId = cleanSenderHex,
                senderName = senderName,
                text = "🎤 Голосовое сообщение",
                messageId = effMsgId,
                isVoice = true,
                voiceDurationSecs = duration,
                audioFilePath = localAudioPath
            )) seenMessageIds.add(effMsgId)
            return
        }

        // 3. Photo Image Attachment
        if (payload.has("image_data") && !payload.isNull("image_data")) {
            val imgData = payload.getJSONObject("image_data")
            val base64 = imgData.optString("image_base64", "")
            var localImgPath = ""
            if (base64.isNotBlank()) {
                try {
                    val bytes = android.util.Base64.decode(base64, android.util.Base64.NO_WRAP)
                    val ctx = appContext
                    if (ctx != null && bytes.isNotEmpty()) {
                        val imgDir = File(ctx.filesDir, "media_images").apply { mkdirs() }
                        val rxFile = File(imgDir, "rx_${System.currentTimeMillis()}_${cleanSenderHex.take(6)}.jpg")
                        rxFile.writeBytes(bytes)
                        if (rxFile.exists() && rxFile.length() > 50L) {
                            localImgPath = rxFile.absolutePath
                        }
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }

            if (NodeXRepository.receiveIncomingMessage(
                senderId = cleanSenderHex,
                senderName = senderName,
                text = payload.optString("text", "📷 Фотография"),
                messageId = effMsgId,
                isImage = true,
                imageFilePath = localImgPath
            )) seenMessageIds.add(effMsgId)
            return
        }

        // 4. Document File Attachment
        if (payload.has("file_data") && !payload.isNull("file_data")) {
            val fData = payload.getJSONObject("file_data")
            val base64 = fData.optString("file_base64", "")
            val rawName = fData.optString("file_name", "document")
            val fSize = fData.optString("file_size", "")
            var localFilePath = ""
            if (base64.isNotBlank()) {
                try {
                    val bytes = android.util.Base64.decode(base64, android.util.Base64.NO_WRAP)
                    val ctx = appContext
                    if (ctx != null && bytes.isNotEmpty()) {
                        val fileDir = File(ctx.filesDir, "media_files").apply { mkdirs() }
                        val safeName = rawName.replace(Regex("[^a-zA-Z0-9._-]"), "_")
                        val rxFile = File(fileDir, "rx_${System.currentTimeMillis()}_$safeName")
                        rxFile.writeBytes(bytes)
                        if (rxFile.exists() && rxFile.length() > 0L) {
                            localFilePath = rxFile.absolutePath
                        }
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }

            if (NodeXRepository.receiveIncomingMessage(
                senderId = cleanSenderHex,
                senderName = senderName,
                text = payload.optString("text", "📁 $rawName"),
                messageId = effMsgId,
                isFile = true,
                filePath = localFilePath,
                fileName = rawName,
                fileSizeFormatted = fSize
            )) seenMessageIds.add(effMsgId)
            return
        }

        // 5. Regular Text Message
        val text = payload.optString("text", "")
        if (text.isNotBlank() && !text.startsWith("[Call Signal")) {
            android.util.Log.d("NodeX_P2P", "Received text message ${effMsgId.take(18)} from peer ${cleanSenderHex.take(8)} (${text.length} chars)")
            if (NodeXRepository.receiveIncomingMessage(
                senderId = cleanSenderHex,
                senderName = senderName,
                text = text,
                messageId = effMsgId
            )) seenMessageIds.add(effMsgId)
        }
    }

    /**
     * Send chat text message to peer across direct UDP endpoint and Bootstrap DHT mailbox.
     */
    private suspend fun awaitVerifiedPeerKey(peerId: String): ByteArray? {
        repeat(12) {
            val key = NodeXRepository.getVerifiedPeerX25519Key(peerId)
                ?: verifiedContactKeys[peerId]?.second
            if (key != null) return key
            queryPeerPresence(peerId)
            delay(250)
        }
        return NodeXRepository.getVerifiedPeerX25519Key(peerId)
            ?: verifiedContactKeys[peerId]?.second
    }

    fun sendMessage(contactId: String, text: String, myId: String, myName: String, messageId: String = newMessageId(myId)): Boolean {
        val cleanPeerId = contactId.trim().lowercase()
        if (pendingDeliveries.size >= 100 || pendingDeliveries.values.sumOf { it.packet.size.toLong() } > 8L * 1024 * 1024 - 64 * 1024) return false
        if (text.toByteArray(Charsets.UTF_8).size > 40 * 1024) return false
        val queued = java.util.concurrent.CountDownLatch(1)
        var accepted = false
        scope.launch {
            try {
                val cleanContactId = contactId.trim().lowercase()
                val cleanMyId = myId.trim().lowercase()
                android.util.Log.d("NodeX_P2P", "Queueing message ${messageId.take(18)} for peer ${cleanContactId.take(8)} (${text.length} chars)")
                queryPeerPresence(cleanContactId)
                val nowSecs = System.currentTimeMillis() / 1000
                val msgId = messageId
                seenMessageIds.add(msgId)

                val payloadObj = JSONObject().apply {
                    put("id", msgId)
                    put("text", text)
                    put("timestamp", nowSecs)
                }

                val peerKeys = awaitVerifiedPeerKey(cleanContactId)
                    ?: error("Контакт не ответил ключевым presence-пакетом")
                val envelopeObj = AndroidMessageCrypto.sealMessage(NodeXRepository.identity.value.mnemonic, cleanContactId, peerKeys, payloadObj)

                val envelopeBytes = JSONArray().put(envelopeObj).toString().toByteArray(Charsets.UTF_8)

                // 1. Primary permanent mailbox key
                val primaryMbxKey = mailboxKey(cleanContactId)
                val primaryPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = primaryMbxKey,
                    valueBytes = envelopeBytes
                )

                if (envelopeBytes.size > 60 * 1024) error("Encrypted message is too large for current mailbox transport")
                accepted = registerPendingDelivery("$cleanContactId:$msgId", cleanContactId, primaryPacket)
                queued.countDown()
                if (!accepted) return@launch

                // 2. Fallback daily mailbox key
                val dailyMbxKey = deriveMailboxKeyId(cleanContactId, nowSecs)
                val dailyPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = dailyMbxKey,
                    valueBytes = envelopeBytes
                )

                // 3. Direct packet
                val directPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = hexToBytes(cleanContactId, 20),
                    valueBytes = envelopeBytes
                )

                dispatchToCandidateEndpoints(primaryPacket, cleanContactId)
                dispatchToCandidateEndpoints(dailyPacket, cleanContactId)
                dispatchToCandidateEndpoints(directPacket, cleanContactId)
                triggerImmediatePoll()
            } catch (e: Exception) {
                e.printStackTrace()
                queued.countDown()
            }
        }
        queued.await(5, java.util.concurrent.TimeUnit.SECONDS)
        return accepted
    }

    fun newMessageId(senderId: String): String = "${System.currentTimeMillis()}_${senderId.trim().lowercase().take(12)}_${java.util.UUID.randomUUID()}"

    fun retryPendingMessage(peerId: String, messageId: String, text: String) {
        val id = "$peerId:$messageId"
        if (pendingDeliveries.containsKey(id)) return
        sendMessage(peerId, text, localNodeIdHex, "", messageId)
    }

    private fun reencryptPendingMessagesForPeer(peerId: String) {
        val cleanPeer = peerId.trim().lowercase()
        pendingDeliveries.entries.toList().forEach { (key, pending) ->
            if (pending.contactId.equals(cleanPeer, true)) pendingDeliveries.remove(key, pending)
        }
        persistQueue()
        scope.launch {
            NodeXRepository.pendingOutgoingMessages()
                .filter { (peer, message) -> peer.equals(cleanPeer, true) && !message.isVoice && !message.isImage && !message.isFile }
                .forEach { (peer, message) -> retryPendingMessage(peer, message.id, message.text) }
        }
    }

    fun restorePendingMessages() {
        restoreQueue()
        NodeXRepository.pendingOutgoingMessages().forEach { (peerId, message) ->
            if (!pendingDeliveries.containsKey("$peerId:${message.id}") && !message.isVoice && !message.isImage && !message.isFile) {
                retryPendingMessage(peerId, message.id, message.text)
            }
        }
    }

    /**
     * Send Voice Message with compressed AAC audio payload across direct UDP endpoint and Bootstrap relay.
     */
    fun sendVoiceMessage(contactId: String, durationSecs: Int, myId: String, myName: String, audioFilePath: String = "", messageId: String = newMessageId(myId)): Boolean {
        val cleanPeerId = contactId.trim().lowercase()
        if ((NodeXRepository.getVerifiedPeerX25519Key(cleanPeerId) ?: verifiedContactKeys[cleanPeerId]?.second) == null) return false
        if (pendingDeliveries.size >= 100 || pendingDeliveries.values.sumOf { it.packet.size.toLong() } > 8L * 1024 * 1024 - 64 * 1024) return false
        if (audioFilePath.isNotBlank() && runCatching { File(audioFilePath).length() > 35 * 1024L }.getOrDefault(true)) return false
        val queued = java.util.concurrent.CountDownLatch(1)
        var accepted = false
        scope.launch {
            try {
                val cleanContactId = contactId.trim().lowercase()
                val cleanMyId = myId.trim().lowercase()
                queryPeerPresence(cleanContactId)
                val nowSecs = System.currentTimeMillis() / 1000
                val msgId = messageId
                seenMessageIds.add(msgId)

                val audioBytes = if (audioFilePath.isNotBlank()) {
                    try {
                        val f = File(audioFilePath)
                        if (f.exists() && f.length() > 50L) f.readBytes() else null
                    } catch (e: Exception) {
                        null
                    }
                } else null

                val audioBase64 = if (audioBytes != null && audioBytes.isNotEmpty()) {
                    android.util.Base64.encodeToString(audioBytes, android.util.Base64.NO_WRAP)
                } else ""

                val vnObj = JSONObject().apply {
                    put("duration_seconds", durationSecs)
                    put("codec", "AAC_M4A")
                    put("audio_base64", audioBase64)
                }

                val payloadObj = JSONObject().apply {
                    put("id", msgId)
                    put("text", "🎤 Voice message (${durationSecs}s)")
                    put("voice_note", vnObj)
                    put("timestamp", nowSecs)
                }

                val peerKeys = NodeXRepository.getVerifiedPeerX25519Key(cleanContactId)
                    ?: verifiedContactKeys[cleanContactId]?.second
                    ?: error("Contact has no previously verified encryption key; authenticate the contact first")
                val envelopeObj = AndroidMessageCrypto.sealMessage(NodeXRepository.identity.value.mnemonic, cleanContactId, peerKeys, payloadObj)

                val envelopeBytes = JSONArray().put(envelopeObj).toString().toByteArray(Charsets.UTF_8)

                val primaryMbxKey = mailboxKey(cleanContactId)
                val primaryPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = primaryMbxKey,
                    valueBytes = envelopeBytes
                )

                val dailyMbxKey = deriveMailboxKeyId(cleanContactId, nowSecs)
                val dailyPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = dailyMbxKey,
                    valueBytes = envelopeBytes
                )

                val directPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = hexToBytes(cleanContactId, 20),
                    valueBytes = envelopeBytes
                )

                if (envelopeBytes.size > 60 * 1024) error("Voice message is too large for current mailbox transport")
                accepted = registerPendingDelivery("$cleanContactId:$msgId", cleanContactId, primaryPacket)
                queued.countDown()
                if (!accepted) return@launch
                dispatchToCandidateEndpoints(primaryPacket, cleanContactId)
                dispatchToCandidateEndpoints(dailyPacket, cleanContactId)
                dispatchToCandidateEndpoints(directPacket, cleanContactId)
                triggerImmediatePoll()
            } catch (e: Exception) {
                e.printStackTrace()
                queued.countDown()
            }
        }
        queued.await(5, java.util.concurrent.TimeUnit.SECONDS)
        return accepted
    }

    fun sendMediaMessage(
        contactId: String,
        mediaType: String,
        filePath: String,
        caption: String = "",
        fileName: String = "",
        fileSize: String = "",
        myId: String,
        myName: String,
        messageId: String = newMessageId(myId)
    ): Boolean {
        val peer = contactId.trim().lowercase()
        val file = File(filePath)
        if (mediaType !in setOf("image", "file") || !file.isFile || file.length() !in 1..MAX_ATTACHMENT_BYTES) return false
        if (NodeXRepository.getVerifiedPeerX25519Key(peer) == null) return false
        return sendChunkedAttachment(peer, mediaType, file, caption, fileName, messageId)
    }

    private fun handleAttachmentDatagram(data: ByteArray, address: InetAddress, port: Int) {
        if (data.size < 12 || data.size > 1400) return
        val index = ByteBuffer.wrap(data, 4, 4).order(ByteOrder.BIG_ENDIAN).int
        val length = ByteBuffer.wrap(data, 8, 4).order(ByteOrder.BIG_ENDIAN).int
        if (length !in 2..(ATTACHMENT_CHUNK_BYTES * 2 + 1000) || data.size != 12 + length) return
        val chunk = runCatching { JSONObject(String(data, 12, length, Charsets.UTF_8)) }.getOrNull() ?: return
        if (chunk.optInt("index", -1) != index) return
        val sender = chunk.optString("sender_id", "").lowercase()
        val id = chunk.optString("attachment_id", "")
        val transfer = incomingAttachments[id] ?: return
        if (transfer.senderId != sender || index !in transfer.partFiles.indices || System.currentTimeMillis() - transfer.createdAt > 24L * 60 * 60 * 1000) return
        val plain = runCatching { AndroidMessageCrypto.openAttachmentChunk(NodeXRepository.identity.value.mnemonic, chunk, sender) }.getOrNull() ?: return
        synchronized(transfer) {
            if (transfer.partFiles[index] == null) {
                val dir = File(appContext?.filesDir, "attachment_parts").apply { mkdirs() }
                val part = File(dir, "$id-$index.part")
                runCatching {
                    part.writeBytes(plain)
                    AndroidMessageCrypto.sealFile(appContext!!, part, "attachment:$id:$index".toByteArray())
                }.getOrElse { part.delete(); return }
                transfer.partFiles[index] = part
            }
        }
        val ackPayload = AndroidMessageCrypto.createAttachmentAck(NodeXRepository.identity.value.mnemonic, id, index, sender)
            .toString().toByteArray(Charsets.UTF_8)
        val ack = ByteBuffer.allocate(8 + ackPayload.size).order(ByteOrder.BIG_ENDIAN)
            .put(byteArrayOf(0x4E, 0x58, 0x41, 0x41)).putInt(ackPayload.size).put(ackPayload).array()
        socket?.send(DatagramPacket(ack, ack.size, address, port))
        if (transfer.partFiles.all { it?.exists() == true }) finishIncomingAttachment(id, transfer)
    }

    private fun finishIncomingAttachment(id: String, transfer: IncomingAttachment) {
        if (!incomingAttachments.remove(id, transfer)) return
        scope.launch {
            runCatching {
                val manifest = transfer.manifest
                val image = manifest.optString("media_type") == "image"
                val dir = File(appContext?.filesDir, if (image) "media_images" else "media_files").apply { mkdirs() }
                val safeName = manifest.optString("file_name", "attachment.bin").replace(Regex("[^a-zA-Z0-9._-]"), "_").take(160)
                val output = File(dir, "${id}_$safeName")
                val digest = MessageDigest.getInstance("SHA-256")
                output.outputStream().buffered().use { stream ->
                    transfer.partFiles.forEachIndexed { index, part ->
                        val bytes = AndroidMessageCrypto.openFile(appContext!!, part!!, "attachment:$id:$index".toByteArray())
                        digest.update(bytes); stream.write(bytes)
                    }
                }
                val actualHash = digest.digest().joinToString("") { "%02x".format(it) }
                if (output.length() != manifest.optLong("file_size") || actualHash != manifest.optString("sha256")) {
                    output.delete(); error("Attachment size or integrity check failed")
                }
                transfer.partFiles.forEach { it?.delete() }
                NodeXRepository.completeIncomingAttachment(transfer.senderId, transfer.messageId, output.absolutePath)
                sendDeliveryReceipt(transfer.senderId, transfer.messageId)
            }.onFailure { android.util.Log.e("NodeX_P2P", "Attachment integrity/assembly failed", it) }
        }
    }

    private fun sendChunkedAttachment(contactId: String, mediaType: String, file: File, caption: String, fileName: String, messageId: String): Boolean {
        if (file.length() !in 1..MAX_ATTACHMENT_BYTES) return false
        val peer = contactId.trim().lowercase()
        if (NodeXRepository.getVerifiedPeerX25519Key(peer) == null) return false
        val done = java.util.concurrent.CountDownLatch(1)
        var accepted = false
        scope.launch {
            try {
                val identity = NodeXRepository.identity.value
                val peerX = NodeXRepository.getVerifiedPeerX25519Key(peer) ?: error("Missing verified peer key")
                queryPeerPresence(peer)
                delay(600)
                val id = java.util.UUID.randomUUID().toString()
                val hash = MessageDigest.getInstance("SHA-256")
                file.inputStream().buffered().use { input ->
                    val buffer = ByteArray(32 * 1024)
                    while (true) { val count = input.read(buffer); if (count < 0) break; hash.update(buffer, 0, count) }
                }
                val chunkCount = ((file.length() + ATTACHMENT_CHUNK_BYTES - 1) / ATTACHMENT_CHUNK_BYTES).toInt()
                val manifest = JSONObject().apply {
                    put("version", 1); put("attachment_id", id); put("media_type", mediaType)
                    put("file_name", fileName.ifBlank { file.name }.take(200)); put("file_size", file.length())
                    put("mime_type", if (mediaType == "image") "image/jpeg" else "application/octet-stream")
                    put("chunk_size", ATTACHMENT_CHUNK_BYTES); put("chunk_count", chunkCount)
                    put("sha256", hash.digest().joinToString("") { "%02x".format(it) })
                }
                val payload = JSONObject().put("id", messageId).put("text", caption).put("attachment_manifest", manifest)
                val env = AndroidMessageCrypto.sealMessage(identity.mnemonic, peer, peerX, payload)
                val bytes = JSONArray().put(env).toString().toByteArray(Charsets.UTF_8)
                val packet = buildStorePacketWithKeyBytes(Random().nextLong(), identity.nodeId, hexToBytes(peer, 20), bytes)
                val queued = registerPendingDelivery("$peer:$messageId", peer, packet)
                if (!queued) error("Delivery queue full")
                dispatchToCandidateEndpoints(packet, peer)
                accepted = true
                done.countDown()
                delay(350)
                file.inputStream().buffered().use { input ->
                    val buffer = ByteArray(ATTACHMENT_CHUNK_BYTES)
                    var index = 0
                    val window = ArrayList<Pair<Int, ByteArray>>(32)
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) {
                            if (window.isNotEmpty() && !sendAttachmentWindow(id, peer, window)) error("Attachment chunk acknowledgement timed out")
                            break
                        }
                        if (count == 0) continue
                        val chunk = AndroidMessageCrypto.sealAttachmentChunk(identity.mnemonic, id, index, peer, peerX, buffer.copyOf(count))
                        val datagram = ByteBuffer.allocate(12 + chunk.size).order(ByteOrder.BIG_ENDIAN)
                            .put(byteArrayOf(0x4E, 0x58, 0x41, 0x54)).putInt(index).putInt(chunk.size).put(chunk).array()
                        window.add(index to datagram)
                        index++
                        if (window.size == 32) {
                            if (!sendAttachmentWindow(id, peer, window)) error("Attachment chunk acknowledgement timed out")
                            window.clear()
                        }
                    }
                }
            } catch (error: Exception) {
                android.util.Log.e("NodeX_P2P", "Attachment transfer could not be queued", error)
            } finally { done.countDown() }
        }
        done.await(3, java.util.concurrent.TimeUnit.SECONDS)
        return accepted
    }

    private suspend fun sendAttachmentWindow(attachmentId: String, peer: String, packets: List<Pair<Int, ByteArray>>): Boolean {
        val pending = packets.associate { (index, _) ->
            val key = "$attachmentId:$index"
            key to CompletableDeferred<Unit>().also { attachmentAcks[key] = it }
        }
        try {
            repeat(4) {
                packets.forEach { (index, packet) ->
                    if (pending["$attachmentId:$index"]?.isCompleted != true) dispatchSinglePacket(packet, peer, directPeerOnly = true)
                }
                delay(250)
                if (pending.values.all { it.isCompleted }) return true
            }
            return pending.values.all { it.isCompleted }
        } finally {
            pending.keys.forEach(attachmentAcks::remove)
        }
    }

    /**
     * Send Call Signaling (Offer, Answer, Hangup, IceCandidate) to peer with rapid multi-target UDP burst.
     */
    fun sendCallSignal(contactId: String, signalType: String, callId: String, myId: String, sdpOrCandidate: String = "") {
        val cleanContactId = contactId.trim().lowercase()
        val cleanMyId = myId.trim().lowercase()
        if (signalType == "Offer") {
            activeOfferParams = Triple(cleanContactId, callId, cleanMyId)
            activeOfferSdp = sdpOrCandidate
        } else if (signalType == "Hangup" || signalType == "Reject") {
            activeOfferParams = null
            activeOfferSdp = ""
            rememberEndedCall(cleanContactId, callId)
        }

        scope.launch {
            try {
                queryPeerPresence(cleanContactId)
                val nowSecs = System.currentTimeMillis() / 1000

                val sigObj = JSONObject().apply {
                    put("call_id", callId)
                    put("caller_id_hex", cleanMyId)
                    put("callee_id_hex", cleanContactId)
                    put("recipient_id_hex", cleanContactId)
                    put("signal_type", signalType)
                    put("endpoint", myPublicEndpoint.ifBlank { "0.0.0.0:${socket?.localPort ?: DEFAULT_PORT}" })
                    if (sdpOrCandidate.isNotBlank()) {
                        put("data", sdpOrCandidate)
                    }
                    put("timestamp", nowSecs)
                }

                val payloadObj = JSONObject().apply {
                    put("id", "call_${callId}_${nowSecs}_${signalType}_${Random().nextInt(1000)}")
                    put("text", "[Call Signal $signalType]")
                    put("call_signal", sigObj)
                    put("timestamp", nowSecs)
                }

                val peerKey = awaitVerifiedPeerKey(cleanContactId)
                    ?: error("Контакт не ответил ключевым presence-пакетом")
                val envelopeObj = AndroidMessageCrypto.sealMessage(NodeXRepository.identity.value.mnemonic, cleanContactId, peerKey, payloadObj)

                val envelopeBytes = JSONArray().put(envelopeObj).toString().toByteArray(Charsets.UTF_8)

                // 1. Dedicated Real-Time Call Signal Key
                val callSignalKey = callKey(cleanContactId)
                val callPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = callSignalKey,
                    valueBytes = envelopeBytes
                )

                // 2. Primary Mailbox Key
                val primaryMbxKey = mailboxKey(cleanContactId)
                val mbxPacket = buildStorePacketWithKeyBytes(
                    reqId = Random().nextLong(),
                    senderIdHex = cleanMyId,
                    keyBytes = primaryMbxKey,
                    valueBytes = envelopeBytes
                )

                // Retry unanswered offers on a timer. A single send per key and
                // endpoint avoids the old burst of up to 30 redundant datagrams.
                dispatchToCandidateEndpoints(callPacket, cleanContactId)
                dispatchToCandidateEndpoints(mbxPacket, cleanContactId)
                triggerImmediatePoll()
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    /**
     * Stream live speech PCM audio chunk directly to peer during active call.
     */
    fun sendCallAudio(contactId: String, pcmData: ByteArray, seq: Long) {
        scope.launch {
            try {
                val cleanContactId = contactId.trim().lowercase()
                val senderBytes = hexToBytes(localNodeIdHex, 20)
                val targetBytes = hexToBytes(cleanContactId, 20)
                if (pcmData.isEmpty() || pcmData.size > 900 || pcmData.size % 2 != 0) return@launch
                val peerXKey = NodeXRepository.getVerifiedPeerX25519Key(cleanContactId) ?: return@launch
                val nonce = ByteArray(12).also(SecureRandom()::nextBytes)
                val callId = NodeXRepository.activeCall.value?.callId ?: return@launch
                val sampleRate = 16_000
                val (ephemeralPublic, ciphertext) = AndroidMessageCrypto.sealCallAudio(
                    NodeXRepository.identity.value.mnemonic, peerXKey, nonce, pcmData, senderBytes, targetBytes, callId, seq, sampleRate
                )
                val callBytes = callId.toByteArray(Charsets.UTF_8)
                if (callBytes.isEmpty() || callBytes.size > 128) return@launch
                val buf = ByteBuffer.allocate(4 + 20 + 20 + 1 + callBytes.size + 8 + 4 + 12 + 32 + 2 + ciphertext.size).order(ByteOrder.BIG_ENDIAN)
                buf.put(byteArrayOf(0x4E, 0x58, 0x43, 0x41))
                buf.put(senderBytes).put(targetBytes).put(callBytes.size.toByte()).put(callBytes).putLong(seq).putInt(sampleRate)
                buf.put(nonce).put(ephemeralPublic).putShort(ciphertext.size.toShort()).put(ciphertext)

                val packetBytes = buf.array()
                val directEndpoint = contactEndpoints[cleanContactId] ?: NodeXRepository.contacts.value
                    .firstOrNull { it.userIdHex.equals(cleanContactId, true) }?.endpointAddr
                if (directEndpoint.isNullOrBlank()) {
                    queryPeerPresence(cleanContactId)
                    return@launch
                }
                contactEndpoints[cleanContactId] = directEndpoint
                dispatchSinglePacket(packetBytes, cleanContactId, directPeerOnly = true)
            } catch (e: Exception) {}
        }
    }

    private fun dispatchToCandidateEndpoints(packetBytes: ByteArray, targetContactId: String = "") {
        if (packetBytes.size > 1000) {
            val chunks = splitIntoNxckChunks(packetBytes)
            for (chunk in chunks) {
                dispatchSinglePacket(chunk, targetContactId)
                try { Thread.sleep(1) } catch (e: Exception) {}
            }
        } else {
            dispatchSinglePacket(packetBytes, targetContactId)
        }
    }

    private fun dispatchSinglePacket(
        packetBytes: ByteArray,
        targetContactId: String = "",
        directPeerOnly: Boolean = false
    ) {
        val targets = mutableListOf<Pair<String, Int>>()
        if (!directPeerOnly) targets.add(Pair(BOOTSTRAP_HOST, BOOTSTRAP_PORT))

        // Add verified peer direct public endpoint as top priority
        val directEp = if (targetContactId.isNotBlank()) {
            contactEndpoints[targetContactId.trim().lowercase()]
                ?: NodeXRepository.contacts.value.firstOrNull { it.userIdHex.equals(targetContactId, true) }?.endpointAddr
        } else null
        if (!directEp.isNullOrBlank()) {
            val parts = directEp.split(":")
            if (parts.size == 2) {
                val host = parts[0]
                val port = parts[1].toIntOrNull()
                if (port != null && host.isNotBlank() && host != "0.0.0.0" && host != BOOTSTRAP_HOST) {
                    targets.add(0, Pair(host, port))
                }
            }
        }

        for ((host, port) in targets) {
            try {
                val addr = InetAddress.getByName(host)
                val dp = DatagramPacket(packetBytes, packetBytes.size, addr, port)
                socket?.send(dp)
            } catch (e: Exception) {}
        }
    }

    private fun handleTcpClient(client: java.net.Socket) {
        try {
            client.soTimeout = 5000
            val input = client.getInputStream()
            val output = client.getOutputStream()

            val buf = ByteArray(65536)
            var totalRead = 0
            while (totalRead < buf.size) {
                val r = input.read(buf, totalRead, buf.size - totalRead)
                if (r < 0) break
                totalRead += r
                if (totalRead >= 53 && buf[0] == MSG_STORE) {
                    val valueLen = ByteBuffer.wrap(buf, 49, 4).order(ByteOrder.BIG_ENDIAN).int.coerceAtLeast(0)
                    if (totalRead >= 53 + valueLen) {
                        break
                    }
                }
            }

            if (totalRead > 0) {
                val data = buf.copyOfRange(0, totalRead)
                handleIncomingPacket(data, client.inetAddress, client.port)
                output.write(byteArrayOf(MSG_STORE_ACK, 0))
                output.flush()
            }
        } catch (e: Exception) {
            // ignore
        } finally {
            try { client.close() } catch (e: Exception) {}
        }
    }

    private fun sendDeliveryReceipt(recipientId: String, messageId: String) {
        try {
            val receiptPayload = JSONObject().apply {
                put("id", "receipt_${messageId}_${localNodeIdHex.take(8)}")
                put("delivery_receipt", AndroidMessageCrypto.createDeliveryReceipt(NodeXRepository.identity.value.mnemonic, messageId))
                put("timestamp", System.currentTimeMillis() / 1000)
            }
            val key = NodeXRepository.getVerifiedPeerX25519Key(recipientId)
                ?: verifiedContactKeys[recipientId]?.second ?: return
            val receipt = AndroidMessageCrypto.sealMessage(NodeXRepository.identity.value.mnemonic, recipientId, key, receiptPayload)
            val data = JSONArray().put(receipt).toString().toByteArray(Charsets.UTF_8)
            val permanentPacket = buildStorePacketWithKeyBytes(Random().nextLong(), localNodeIdHex, mailboxKey(recipientId), data)
            val dailyPacket = buildStorePacketWithKeyBytes(Random().nextLong(), localNodeIdHex, deriveMailboxKeyId(recipientId, System.currentTimeMillis() / 1000), data)
            dispatchToCandidateEndpoints(permanentPacket, recipientId)
            dispatchToCandidateEndpoints(dailyPacket, recipientId)
            triggerImmediatePoll()
        } catch (e: Exception) {
            android.util.Log.w("NodeX_P2P", "Could not send authenticated delivery receipt", e)
        }
    }

    private fun JSONObject.getBytes(key: String): ByteArray {
        val value = get(key)
        return when (value) {
            is JSONArray -> ByteArray(value.length()) { value.getInt(it).toByte() }
            is String -> hexToBytes(value, 32)
            else -> error("Invalid byte array: $key")
        }
    }

    fun mailboxKey(nodeId: String): ByteArray = sha1("mailbox_" + nodeId.trim().lowercase())
    fun callKey(nodeId: String): ByteArray = sha1("call_signal_" + nodeId.trim().lowercase())
    fun presenceKey(nodeId: String): ByteArray = sha1("presence_" + nodeId.trim().lowercase())

    private fun deriveMailboxKeyId(userIdHex: String, timestampSecs: Long): ByteArray {
        return try {
            val clean = userIdHex.trim().lowercase()
            val dayBucket = timestampSecs / 86400
            val mac = javax.crypto.Mac.getInstance("HmacSHA256")
            mac.init(javax.crypto.spec.SecretKeySpec(clean.toByteArray(Charsets.UTF_8), "HmacSHA256"))
            val bucketBytes = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN).putLong(dayBucket).array()
            val res = mac.doFinal(bucketBytes)
            val hex = res.take(16).joinToString("") { "%02x".format(it) }
            val mailboxKey = "mbx_$hex"
            sha1(mailboxKey)
        } catch (e: Exception) {
            hexToBytes(userIdHex.trim().lowercase(), 20)
        }
    }

    private fun sha1(str: String): ByteArray {
        val md = MessageDigest.getInstance("SHA-1")
        return md.digest(str.toByteArray(Charsets.UTF_8))
    }

    private fun buildStorePacketWithKeyBytes(reqId: Long, senderIdHex: String, keyBytes: ByteArray, valueBytes: ByteArray): ByteArray {
        val senderIdBytes = hexToBytes(senderIdHex.trim().lowercase(), 20)
        val fixedKeyBytes = ByteArray(20)
        System.arraycopy(keyBytes, 0, fixedKeyBytes, 0, keyBytes.size.coerceAtMost(20))

        val buf = ByteBuffer.allocate(1 + 8 + 20 + 20 + 4 + valueBytes.size).order(ByteOrder.BIG_ENDIAN)
        buf.put(MSG_STORE)
        buf.putLong(reqId)
        buf.put(senderIdBytes)
        buf.put(fixedKeyBytes)
        buf.putInt(valueBytes.size)
        buf.put(valueBytes)
        return buf.array()
    }

    private fun toJsonByteArray(bytes: ByteArray): JSONArray {
        val arr = JSONArray()
        for (b in bytes) {
            arr.put(b.toInt() and 0xFF)
        }
        return arr
    }

    private fun derive32Bytes(id: String): ByteArray {
        val md = MessageDigest.getInstance("SHA-256")
        return md.digest(id.trim().lowercase().toByteArray(Charsets.UTF_8))
    }

    private fun hexToBytes(hex: String, targetLen: Int = 20): ByteArray {
        val clean = hex.trim().lowercase()
        val out = ByteArray(targetLen)
        val bytes = try {
            clean.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        } catch (e: Exception) {
            clean.toByteArray(Charsets.UTF_8)
        }
        val copyLen = bytes.size.coerceAtMost(targetLen)
        System.arraycopy(bytes, 0, out, 0, copyLen)
        return out
    }

    private fun bytesToHex(bytes: ByteArray): String {
        return bytes.joinToString("") { "%02x".format(it) }
    }
}
