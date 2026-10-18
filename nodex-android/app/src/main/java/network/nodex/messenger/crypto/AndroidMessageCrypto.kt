package network.nodex.messenger.crypto

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.bouncycastle.crypto.digests.SHA256Digest
import org.bouncycastle.crypto.generators.HKDFBytesGenerator
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.params.HKDFParameters
import org.bouncycastle.crypto.signers.Ed25519Signer
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import org.bouncycastle.crypto.params.KeyParameter
import org.bouncycastle.crypto.params.ParametersWithIV
import org.json.JSONArray
import org.json.JSONObject
import network.nodex.messenger.data.NodeXRepository
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import org.bouncycastle.math.ec.rfc7748.X25519

/** Common NodeX BIP39, Ed25519/X25519, envelope-v3 and at-rest crypto implementation. */
object AndroidMessageCrypto {
    private const val IDENTITY_ALIAS = "nodex_identity_v1"
    private const val STORE_ALIAS = "nodex_local_store_v1"
    private const val VERSION = 3

    fun inviteNodeId(invite: String): String? = runCatching {
        val encoded = invite.trim().removePrefix("nodex://invite/").substringBefore('?')
        val json = JSONObject(String(android.util.Base64.decode(encoded, android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING), Charsets.UTF_8))
        json.optString("user_id").takeIf { it.length == 40 }
    }.getOrNull()

    data class IdentityKeys(
        val nodeId: ByteArray,
        val edSeed: ByteArray,
        val edPublic: ByteArray,
        val xSecret: ByteArray,
        val xPublic: ByteArray
    )

    fun identityKeys(mnemonic: String): IdentityKeys {
        val words = mnemonic.trim().lowercase().split("\\s+".toRegex())
        require(MnemonicHelper.isValidMnemonic(mnemonic)) { "Invalid BIP-39 recovery phrase" }
        val spec = javax.crypto.spec.PBEKeySpec(words.joinToString(" ").toCharArray(), "mnemonic".toByteArray(Charsets.UTF_8), 2048, 512)
        val seed = javax.crypto.SecretKeyFactory.getInstance("PBKDF2WithHmacSHA512").generateSecret(spec).encoded.copyOfRange(0, 32)
        spec.clearPassword()
        fun derive(info: String): ByteArray = ByteArray(32).also { out ->
            HKDFBytesGenerator(SHA256Digest()).apply {
                init(HKDFParameters(seed, null, info.toByteArray(Charsets.UTF_8)))
                generateBytes(out, 0, out.size)
            }
        }
        val edSeed = derive("NodeX-Ed25519-Sign-v1")
        val ed = Ed25519PrivateKeyParameters(edSeed, 0)
        val xSecret = derive("NodeX-X25519-Static-v1")
        val xPublic = ByteArray(32)
        X25519.generatePublicKey(xSecret, 0, xPublic, 0)
        val sha1 = java.security.MessageDigest.getInstance("SHA-1").digest(ed.generatePublicKey().encoded)
        return IdentityKeys(sha1, edSeed, ed.generatePublicKey().encoded, xSecret, xPublic)
    }

    fun sealMessage(
        mnemonic: String,
        recipientIdHex: String,
        recipientXPublic: ByteArray,
        payload: JSONObject
    ): JSONObject {
        require(recipientXPublic.size == 32) { "Contact must have a verified 32-byte X25519 public key" }
        val recipientId = hexToBytes(recipientIdHex)
        require(recipientId.size == 20) { "Recipient ID must be 20 bytes" }
        val keys = identityKeys(mnemonic)
        require(keys.nodeId.contentEquals(hexToBytes(MnemonicHelper.deriveNodeId(mnemonic)))) { "Identity derivation mismatch" }

        val ephSecret = ByteArray(32).also(SecureRandom()::nextBytes)
        val ephPublic = ByteArray(32)
        X25519.generatePublicKey(ephSecret, 0, ephPublic, 0)
        val dhEphemeral = ByteArray(32)
        val dhStatic = ByteArray(32)
        X25519.calculateAgreement(ephSecret, 0, recipientXPublic, 0, dhEphemeral, 0)
        X25519.calculateAgreement(keys.xSecret, 0, recipientXPublic, 0, dhStatic, 0)
        require(dhEphemeral.any { it.toInt() != 0 } && dhStatic.any { it.toInt() != 0 }) { "Invalid X25519 public key" }

        val nonce = ByteArray(12).also(SecureRandom()::nextBytes)
        val ikm = dhEphemeral + dhStatic
        val cipherKey = ByteArray(32)
        HKDFBytesGenerator(SHA256Digest()).apply {
            init(HKDFParameters(ikm, nonce, "NodeX-ChaCha20Poly1305-PayloadKey-v2".toByteArray(Charsets.UTF_8)))
            generateBytes(cipherKey, 0, cipherKey.size)
        }
        val aead = ChaCha20Poly1305()
        aead.init(true, AEADParameters(KeyParameter(cipherKey), 128, nonce))
        val plain = payload.toString().toByteArray(Charsets.UTF_8)
        val ciphertext = ByteArray(aead.getOutputSize(plain.size))
        val written = aead.processBytes(plain, 0, plain.size, ciphertext, 0)
        val final = aead.doFinal(ciphertext, written)
        val sealed = ciphertext.copyOf(written + final)
        val timestamp = System.currentTimeMillis() / 1000
        val signed = envelopeSigningBytes(keys.nodeId, recipientId, keys.xPublic, ephPublic, nonce, timestamp, sealed)
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(signed, 0, signed.size)
        }
        val signature = signer.generateSignature()

        return JSONObject().apply {
            put("version", VERSION)
            put("sender_id", bytesJson(keys.nodeId))
            put("sender_ed25519_pub", bytesJson(keys.edPublic))
            put("sender_x25519_pub", bytesJson(keys.xPublic))
            put("ephemeral_x25519_pub", bytesJson(ephPublic))
            put("recipient_id", bytesJson(recipientId))
            put("nonce", bytesJson(nonce))
            put("ciphertext", bytesJson(sealed))
            put("signature", bytesJson(signature))
            put("timestamp", timestamp)
        }
    }

    fun openEnvelope(mnemonic: String, env: JSONObject): Pair<String, JSONObject> {
        require(env.optInt("version", 0) >= VERSION) { "Unsupported/legacy unverified envelope" }
        val senderId = env.bytes("sender_id")
        val edPublic = env.bytes("sender_ed25519_pub")
        val senderXPublic = env.bytes("sender_x25519_pub")
        val ephPublic = env.bytes("ephemeral_x25519_pub")
        val recipientId = env.bytes("recipient_id")
        val nonce = env.bytes("nonce")
        val ciphertext = env.bytes("ciphertext")
        val signature = env.bytes("signature")
        val keys = identityKeys(mnemonic)
        require(senderId.size == 20 && edPublic.size == 32 && senderXPublic.size == 32 && ephPublic.size == 32 &&
            recipientId.size == 20 && nonce.size == 12 && signature.size == 64 && ciphertext.size >= 16) { "Malformed envelope" }
        require(senderId.contentEquals(java.security.MessageDigest.getInstance("SHA-1").digest(edPublic))) { "Sender ID does not match signing key" }
        require(recipientId.contentEquals(keys.nodeId)) { "Envelope addressed to a different identity" }
        val timestamp = env.optLong("timestamp", 0)
        require(timestamp > 0 && kotlin.math.abs(System.currentTimeMillis() / 1000 - timestamp) <= 7 * 86400) { "Expired envelope" }
        val signingBytes = envelopeSigningBytes(senderId, recipientId, senderXPublic, ephPublic, nonce, timestamp, ciphertext)
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(edPublic, 0))
            update(signingBytes, 0, signingBytes.size)
        }
        require(verifier.verifySignature(signature)) { "Invalid sender signature" }

        val dhEphemeral = ByteArray(32)
        val dhStatic = ByteArray(32)
        X25519.calculateAgreement(keys.xSecret, 0, ephPublic, 0, dhEphemeral, 0)
        X25519.calculateAgreement(keys.xSecret, 0, senderXPublic, 0, dhStatic, 0)
        require(dhEphemeral.any { it.toInt() != 0 } && dhStatic.any { it.toInt() != 0 }) { "Invalid X25519 key" }
        val cipherKey = ByteArray(32)
        HKDFBytesGenerator(SHA256Digest()).apply {
            init(HKDFParameters(dhEphemeral + dhStatic, nonce, "NodeX-ChaCha20Poly1305-PayloadKey-v2".toByteArray(Charsets.UTF_8)))
            generateBytes(cipherKey, 0, cipherKey.size)
        }
        val aead = ChaCha20Poly1305()
        aead.init(false, AEADParameters(KeyParameter(cipherKey), 128, nonce))
        val plain = ByteArray(aead.getOutputSize(ciphertext.size))
        val written = aead.processBytes(ciphertext, 0, ciphertext.size, plain, 0)
        val final = aead.doFinal(plain, written)
        val payload = JSONObject(String(plain, 0, written + final, Charsets.UTF_8))
        return bytesHex(senderId) to payload
    }

    fun sealCallAudio(mnemonic: String, recipientXPublic: ByteArray, nonce: ByteArray, plaintext: ByteArray, senderId: ByteArray, recipientId: ByteArray, callId: String, seq: Long, sampleRate: Int): Pair<ByteArray, ByteArray> {
        require(recipientXPublic.size == 32 && nonce.size == 12)
        val keys = identityKeys(mnemonic)
        val ephSecret = ByteArray(32).also(SecureRandom()::nextBytes)
        val ephPublic = ByteArray(32)
        X25519.generatePublicKey(ephSecret, 0, ephPublic, 0)
        val dhEph = ByteArray(32); X25519.calculateAgreement(ephSecret, 0, recipientXPublic, 0, dhEph, 0)
        val dhStatic = ByteArray(32); X25519.calculateAgreement(keys.xSecret, 0, recipientXPublic, 0, dhStatic, 0)
        val aad = audioAad(senderId, recipientId, callId, seq, sampleRate)
        return encryptAudio(dhEph + dhStatic, nonce, ephPublic, plaintext, aad)
    }

    fun openCallAudio(mnemonic: String, senderXPublic: ByteArray, ephemeralPublic: ByteArray, nonce: ByteArray, ciphertext: ByteArray, senderId: ByteArray, recipientId: ByteArray, callId: String, seq: Long, sampleRate: Int): ByteArray {
        require(senderXPublic.size == 32 && ephemeralPublic.size == 32 && nonce.size == 12)
        val keys = identityKeys(mnemonic)
        val dhEph = ByteArray(32); X25519.calculateAgreement(keys.xSecret, 0, ephemeralPublic, 0, dhEph, 0)
        val dhStatic = ByteArray(32); X25519.calculateAgreement(keys.xSecret, 0, senderXPublic, 0, dhStatic, 0)
        val key = deriveAudioKey(dhEph + dhStatic, nonce)
        val aad = audioAad(senderId, recipientId, callId, seq, sampleRate)
        val cipher = ChaCha20Poly1305().apply { init(false, AEADParameters(KeyParameter(key), 128, nonce, aad)) }
        val out = ByteArray(cipher.getOutputSize(ciphertext.size))
        val n = cipher.processBytes(ciphertext, 0, ciphertext.size, out, 0)
        val f = cipher.doFinal(out, n)
        return out.copyOf(n + f)
    }

    fun createDeliveryReceipt(mnemonic: String, messageId: String): JSONObject {
        val keys = identityKeys(mnemonic)
        val timestamp = System.currentTimeMillis() / 1000
        val signed = "DELIVERY_RECEIPT_V1".toByteArray(Charsets.UTF_8) + messageId.toByteArray(Charsets.UTF_8) + java.nio.ByteBuffer.allocate(8).putLong(timestamp).array()
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(signed, 0, signed.size)
        }
        return JSONObject().apply {
            put("message_id", messageId); put("recipient_id", bytesHex(keys.nodeId)); put("timestamp", timestamp)
            put("signature", bytesJson(signer.generateSignature()))
        }
    }

    fun createPublicKeyBundle(mnemonic: String): JSONObject {
        val keys = identityKeys(mnemonic)
        val signed = "NodeX-contact-key-v1".toByteArray(Charsets.UTF_8) + keys.nodeId + keys.edPublic + keys.xPublic
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(signed, 0, signed.size)
        }
        return JSONObject().apply {
            put("id", bytesHex(keys.nodeId)); put("ed", bytesHex(keys.edPublic)); put("x", bytesHex(keys.xPublic))
            put("sig", android.util.Base64.encodeToString(signer.generateSignature(), android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING))
        }
    }

    fun createRustCompatibleInvite(mnemonic: String, displayName: String): String {
        val keys = identityKeys(mnemonic)
        val id = bytesHex(keys.nodeId)
        val ed = bytesHex(keys.edPublic)
        val x = bytesHex(keys.xPublic)
        val created = System.currentTimeMillis() / 1000
        val expires = created + 7 * 24 * 3600
        val signed = id.toByteArray(Charsets.UTF_8) + ed.toByteArray(Charsets.UTF_8) + x.toByteArray(Charsets.UTF_8) +
            java.nio.ByteBuffer.allocate(8).putLong(created).array() + java.nio.ByteBuffer.allocate(8).putLong(expires).array()
        val signer = Ed25519Signer().apply { init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0)); update(signed, 0, signed.size) }
        val payload = JSONObject().apply {
            put("user_id", id); put("ed25519_pub_hex", ed); put("x25519_pub_hex", x)
            put("display_name", displayName); put("fingerprint", id.chunked(4).joinToString(":"))
            put("endpoints", JSONArray()); put("created_at", created); put("expires_at", expires)
            put("signature_hex", bytesHex(signer.generateSignature()))
        }
        val encoded = android.util.Base64.encodeToString(payload.toString().toByteArray(Charsets.UTF_8), android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING)
        return "nodex://invite/$encoded"
    }

    private fun presenceSigningBytes(userId: String, endpoint: String, xPublicHex: String, online: Boolean, timestamp: Long): ByteArray =
        "NodeX-Presence-v1\n${userId.lowercase()}\n$endpoint\n${xPublicHex.lowercase()}\n$online\n$timestamp".toByteArray(Charsets.UTF_8)

    fun signPresence(keys: IdentityKeys, userId: String, endpoint: String, xPublicHex: String, online: Boolean, timestamp: Long): String {
        val bytes = presenceSigningBytes(userId, endpoint, xPublicHex, online, timestamp)
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(bytes, 0, bytes.size)
        }
        return bytesHex(signer.generateSignature())
    }

    fun verifyPresence(payload: JSONObject): Boolean = runCatching {
        val userId = payload.getString("user_id_hex").lowercase()
        val endpoint = payload.getString("socket_addr")
        val ed = hexToBytes(payload.getString("ed25519_pub_hex"))
        val xHex = payload.getString("x25519_pub_hex").lowercase()
        val online = payload.getBoolean("online")
        val timestamp = payload.getLong("timestamp")
        val signature = hexToBytes(payload.getString("signature_hex"))
        if (userId.length != 40 || ed.size != 32 || xHex.length != 64 || signature.size != 64 ||
            timestamp <= 0 || kotlin.math.abs(System.currentTimeMillis() / 1000 - timestamp) > 180 ||
            !java.security.MessageDigest.getInstance("SHA-1").digest(ed).contentEquals(hexToBytes(userId))) return false
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(ed, 0))
            val bytes = presenceSigningBytes(userId, endpoint, xHex, online, timestamp)
            update(bytes, 0, bytes.size)
        }
        verifier.verifySignature(signature)
    }.getOrDefault(false)

    fun verifyRustCompatibleInvite(payload: JSONObject): Boolean = runCatching {
        val id = payload.getString("user_id"); val ed = payload.getString("ed25519_pub_hex"); val x = payload.getString("x25519_pub_hex")
        val created = payload.getLong("created_at"); val expires = payload.getLong("expires_at")
        val now = System.currentTimeMillis() / 1000
        if (expires <= now || created > now + 300 || ed.length != 64 || x.length != 64) return false
        val signed = id.toByteArray(Charsets.UTF_8) + ed.toByteArray(Charsets.UTF_8) + x.toByteArray(Charsets.UTF_8) +
            java.nio.ByteBuffer.allocate(8).putLong(created).array() + java.nio.ByteBuffer.allocate(8).putLong(expires).array()
        val public = hexToBytes(ed); val signature = hexToBytes(payload.getString("signature_hex"))
        if (signature.size != 64 || !java.security.MessageDigest.getInstance("SHA-1").digest(public).contentEquals(hexToBytes(id))) return false
        val verifier = Ed25519Signer().apply { init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(public, 0)); update(signed, 0, signed.size) }
        verifier.verifySignature(signature)
    }.getOrDefault(false)

    fun verifyPublicKeyBundle(id: String, edPublic: ByteArray, xPublic: ByteArray, signature: ByteArray): Boolean = runCatching {
        if (edPublic.size != 32 || xPublic.size != 32 || signature.size != 64 ||
            !java.security.MessageDigest.getInstance("SHA-1").digest(edPublic).contentEquals(hexToBytes(id))) return false
        val signed = "NodeX-contact-key-v1".toByteArray(Charsets.UTF_8) + hexToBytes(id) + edPublic + xPublic
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(edPublic, 0))
            update(signed, 0, signed.size)
        }
        verifier.verifySignature(signature)
    }.getOrDefault(false)

    fun verifyDeliveryReceipt(edPublicKey: ByteArray, receipt: JSONObject, expectedRecipientId: String): Boolean = runCatching {
        val messageId = receipt.getString("message_id")
        val recipientId = receipt.getString("recipient_id")
        val timestamp = receipt.getLong("timestamp")
        if (!recipientId.equals(expectedRecipientId, true) || kotlin.math.abs(System.currentTimeMillis() / 1000 - timestamp) > 7 * 86400) return false
        val sig = receipt.bytes("signature")
        if (sig.size != 64) return false
        val signed = "DELIVERY_RECEIPT_V1".toByteArray(Charsets.UTF_8) + messageId.toByteArray(Charsets.UTF_8) + java.nio.ByteBuffer.allocate(8).putLong(timestamp).array()
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(edPublicKey, 0))
            update(signed, 0, signed.size)
        }
        verifier.verifySignature(sig)
    }.getOrDefault(false)

    private fun encryptAudio(ikm: ByteArray, nonce: ByteArray, ephemeralPublic: ByteArray, plaintext: ByteArray, aad: ByteArray): Pair<ByteArray, ByteArray> {
        val key = deriveAudioKey(ikm, nonce)
        val cipher = ChaCha20Poly1305().apply { init(true, AEADParameters(KeyParameter(key), 128, nonce, aad)) }
        val out = ByteArray(cipher.getOutputSize(plaintext.size))
        val n = cipher.processBytes(plaintext, 0, plaintext.size, out, 0)
        val f = cipher.doFinal(out, n)
        return ephemeralPublic to out.copyOf(n + f)
    }

    private fun audioAad(senderId: ByteArray, recipientId: ByteArray, callId: String, seq: Long, sampleRate: Int): ByteArray {
        val callBytes = callId.toByteArray(Charsets.UTF_8)
        require(senderId.size == 20 && recipientId.size == 20 && callBytes.size in 1..128)
        return "NodeX-call-audio-v1".toByteArray(Charsets.UTF_8) + senderId + recipientId + byteArrayOf(callBytes.size.toByte()) + callBytes +
            java.nio.ByteBuffer.allocate(8).putLong(seq).array() + java.nio.ByteBuffer.allocate(4).putInt(sampleRate).array()
    }

    private fun deriveAudioKey(ikm: ByteArray, nonce: ByteArray) = ByteArray(32).also { key ->
        HKDFBytesGenerator(SHA256Digest()).apply {
            init(HKDFParameters(ikm, nonce, "NodeX-call-audio-v1".toByteArray(Charsets.UTF_8)))
            generateBytes(key, 0, key.size)
        }
    }

    fun sealFile(context: Context, file: File, aad: ByteArray) {
        val key = storeKey()
        val plain = file.readBytes()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key)
        cipher.updateAAD(aad)
        val encrypted = cipher.doFinal(plain)
        val bytes = cipher.iv + encrypted
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeBytes(bytes)
        if (file.exists() && !file.delete()) throw IllegalStateException("Cannot replace ${file.name}")
        if (!tmp.renameTo(file)) throw IllegalStateException("Cannot commit ${file.name}")
    }

    fun openFile(context: Context, file: File, aad: ByteArray): ByteArray {
        val data = file.readBytes()
        require(data.size >= 12 + 16) { "Encrypted local data is truncated" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, storeKey(), GCMParameterSpec(128, data, 0, 12))
        cipher.updateAAD(aad)
        return cipher.doFinal(data, 12, data.size - 12)
    }

    fun isSealed(file: File): Boolean = file.exists() && file.readBytes().take(4).toByteArray().contentEquals(byteArrayOf('N'.code.toByte(), 'X'.code.toByte(), 'S'.code.toByte(), '1'.code.toByte()))

    fun sealJsonFile(context: Context, file: File, aad: ByteArray) {
        val plain = file.readBytes()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, storeKey())
        cipher.updateAAD(aad)
        val wrapped = byteArrayOf('N'.code.toByte(), 'X'.code.toByte(), 'S'.code.toByte(), '1'.code.toByte()) + cipher.iv + cipher.doFinal(plain)
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeBytes(wrapped)
        if (file.exists() && !file.delete()) throw IllegalStateException("Cannot replace ${file.name}")
        if (!tmp.renameTo(file)) throw IllegalStateException("Cannot commit ${file.name}")
    }

    fun openJsonFile(context: Context, file: File, aad: ByteArray): ByteArray {
        val data = file.readBytes()
        require(data.size >= 32 && data.copyOfRange(0, 4).contentEquals(byteArrayOf('N'.code.toByte(), 'X'.code.toByte(), 'S'.code.toByte(), '1'.code.toByte()))) { "Invalid encrypted local file" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, storeKey(), GCMParameterSpec(128, data, 4, 12))
        cipher.updateAAD(aad)
        return cipher.doFinal(data, 16, data.size - 16)
    }

    private fun storeKey(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(STORE_ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(STORE_ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true).build())
        return generator.generateKey()
    }

    fun sealAttachmentChunk(
        mnemonic: String,
        attachmentId: String,
        index: Int,
        recipientId: String,
        recipientXPublic: ByteArray,
        plaintext: ByteArray
    ): ByteArray {
        require(attachmentId.isNotBlank() && index >= 0 && recipientXPublic.size == 32 && plaintext.size <= 350)
        val keys = identityKeys(mnemonic)
        val recipientBytes = hexToBytes(recipientId)
        require(recipientBytes.size == 20)
        val nonce = ByteArray(12).also(SecureRandom()::nextBytes)
        val ephemeralSecret = ByteArray(32).also(SecureRandom()::nextBytes)
        val ephemeralPublic = ByteArray(32)
        X25519.generatePublicKey(ephemeralSecret, 0, ephemeralPublic, 0)
        val ephemeralDh = ByteArray(32)
        val staticDh = ByteArray(32)
        X25519.calculateAgreement(ephemeralSecret, 0, recipientXPublic, 0, ephemeralDh, 0)
        X25519.calculateAgreement(keys.xSecret, 0, recipientXPublic, 0, staticDh, 0)
        require(ephemeralDh.any { it.toInt() != 0 } && staticDh.any { it.toInt() != 0 })
        val context = attachmentChunkContext(attachmentId, index, keys.nodeId, recipientBytes)
        val key = ByteArray(32)
        HKDFBytesGenerator(SHA256Digest()).apply {
            init(HKDFParameters(ephemeralDh + staticDh, nonce, "NodeX-attachment-chunk-v1".toByteArray(Charsets.UTF_8)))
            generateBytes(key, 0, key.size)
        }
        val cipher = ChaCha20Poly1305().apply { init(true, AEADParameters(KeyParameter(key), 128, nonce, context)) }
        val ciphertext = ByteArray(cipher.getOutputSize(plaintext.size))
        val written = cipher.processBytes(plaintext, 0, plaintext.size, ciphertext, 0)
        val final = cipher.doFinal(ciphertext, written)
        val sealed = ciphertext.copyOf(written + final)
        val signatureInput = context + nonce + ephemeralPublic + sealed
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(signatureInput, 0, signatureInput.size)
        }
        return JSONObject().apply {
            put("version", 1); put("attachment_id", attachmentId); put("index", index)
            put("sender_id", bytesHex(keys.nodeId)); put("recipient_id", recipientId.lowercase())
            put("nonce", bytesHex(nonce)); put("ephemeral_x25519_pub", bytesHex(ephemeralPublic))
            put("ciphertext", bytesHex(sealed)); put("signature", bytesHex(signer.generateSignature()))
        }.toString().toByteArray(Charsets.UTF_8)
    }

    fun openAttachmentChunk(mnemonic: String, chunk: JSONObject, expectedPeerId: String): ByteArray {
        require(chunk.optInt("version", 0) == 1)
        val attachmentId = chunk.getString("attachment_id")
        val index = chunk.getInt("index")
        val senderId = hexToBytes(chunk.getString("sender_id"))
        val recipientId = hexToBytes(chunk.getString("recipient_id"))
        require(bytesHex(senderId).equals(expectedPeerId, true))
        val keys = identityKeys(mnemonic)
        require(recipientId.contentEquals(keys.nodeId))
        val nonce = hexToBytes(chunk.getString("nonce"))
        val ephemeralPublic = hexToBytes(chunk.getString("ephemeral_x25519_pub"))
        val ciphertext = hexToBytes(chunk.getString("ciphertext"))
        val signature = hexToBytes(chunk.getString("signature"))
        require(nonce.size == 12 && ephemeralPublic.size == 32 && signature.size == 64 && ciphertext.size in 16..366)
        val peerEd = NodeXRepository.getVerifiedPeerEd25519Key(expectedPeerId) ?: error("Sender identity is not verified")
        val signed = attachmentChunkContext(attachmentId, index, senderId, recipientId) + nonce + ephemeralPublic + ciphertext
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(peerEd, 0))
            update(signed, 0, signed.size)
        }
        require(verifier.verifySignature(signature)) { "Invalid attachment chunk signature" }
        val peerX = NodeXRepository.getVerifiedPeerX25519Key(expectedPeerId) ?: error("Sender encryption key is not verified")
        val ephemeralDh = ByteArray(32); X25519.calculateAgreement(keys.xSecret, 0, ephemeralPublic, 0, ephemeralDh, 0)
        val staticDh = ByteArray(32); X25519.calculateAgreement(keys.xSecret, 0, peerX, 0, staticDh, 0)
        require(ephemeralDh.any { it.toInt() != 0 } && staticDh.any { it.toInt() != 0 })
        val key = ByteArray(32)
        HKDFBytesGenerator(SHA256Digest()).apply {
            init(HKDFParameters(ephemeralDh + staticDh, nonce, "NodeX-attachment-chunk-v1".toByteArray(Charsets.UTF_8)))
            generateBytes(key, 0, key.size)
        }
        val cipher = ChaCha20Poly1305().apply { init(false, AEADParameters(KeyParameter(key), 128, nonce, attachmentChunkContext(attachmentId, index, senderId, recipientId))) }
        val plaintext = ByteArray(cipher.getOutputSize(ciphertext.size))
        val written = cipher.processBytes(ciphertext, 0, ciphertext.size, plaintext, 0)
        val final = cipher.doFinal(plaintext, written)
        return plaintext.copyOf(written + final)
    }

    fun createAttachmentAck(mnemonic: String, attachmentId: String, index: Int, recipientId: String): JSONObject {
        val keys = identityKeys(mnemonic)
        val recipient = hexToBytes(recipientId)
        val signed = attachmentAckBytes(attachmentId, index, keys.nodeId, recipient)
        val signer = Ed25519Signer().apply {
            init(true, Ed25519PrivateKeyParameters(keys.edSeed, 0))
            update(signed, 0, signed.size)
        }
        return JSONObject().apply {
            put("attachment_id", attachmentId); put("index", index)
            put("sender_id", bytesHex(keys.nodeId)); put("recipient_id", recipientId.lowercase())
            put("signature", bytesHex(signer.generateSignature()))
        }
    }

    fun verifyAttachmentAck(peerEd25519: ByteArray, ack: JSONObject, expectedPeerId: String, expectedLocalId: String): Boolean = runCatching {
        val sender = hexToBytes(ack.getString("sender_id"))
        val recipient = hexToBytes(ack.getString("recipient_id"))
        val signature = hexToBytes(ack.getString("signature"))
        require(bytesHex(sender).equals(expectedPeerId, true) && bytesHex(recipient).equals(expectedLocalId, true))
        require(signature.size == 64 && peerEd25519.size == 32)
        val signed = attachmentAckBytes(ack.getString("attachment_id"), ack.getInt("index"), sender, recipient)
        val verifier = Ed25519Signer().apply {
            init(false, org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(peerEd25519, 0))
            update(signed, 0, signed.size)
        }
        verifier.verifySignature(signature)
    }.getOrDefault(false)

    private fun attachmentAckBytes(attachmentId: String, index: Int, sender: ByteArray, recipient: ByteArray): ByteArray {
        require(attachmentId.toByteArray(Charsets.UTF_8).size in 1..128 && index >= 0 && sender.size == 20 && recipient.size == 20)
        val id = attachmentId.toByteArray(Charsets.UTF_8)
        return "NodeX-attachment-ack-v1".toByteArray(Charsets.UTF_8) + byteArrayOf(id.size.toByte()) + id +
            java.nio.ByteBuffer.allocate(4).putInt(index).array() + sender + recipient
    }

    private fun attachmentChunkContext(attachmentId: String, index: Int, senderId: ByteArray, recipientId: ByteArray): ByteArray {
        val attachment = attachmentId.toByteArray(Charsets.UTF_8)
        require(attachment.size in 1..128 && index >= 0 && senderId.size == 20 && recipientId.size == 20)
        return "NodeX-attachment-chunk-v1".toByteArray(Charsets.UTF_8) + byteArrayOf(attachment.size.toByte()) + attachment +
            java.nio.ByteBuffer.allocate(4).putInt(index).array() + senderId + recipientId
    }

    private fun envelopeSigningBytes(sender: ByteArray, recipient: ByteArray, senderX: ByteArray, eph: ByteArray, nonce: ByteArray, timestamp: Long, ciphertext: ByteArray): ByteArray =
        "NodeX-envelope-v3".toByteArray(Charsets.UTF_8) + byteArrayOf(VERSION.toByte()) + sender + recipient + senderX + eph + nonce +
            java.nio.ByteBuffer.allocate(8).putLong(timestamp).array() + ciphertext

    private fun JSONObject.bytes(key: String): ByteArray = when (val v = get(key)) {
        is JSONArray -> ByteArray(v.length()) { v.getInt(it).toByte() }
        is String -> hexToBytes(v)
        else -> error("Invalid $key")
    }

    private fun bytesJson(bytes: ByteArray) = JSONArray().also { a -> bytes.forEach { a.put(it.toInt() and 0xff) } }
    private fun hexToBytes(hex: String): ByteArray {
        require(hex.length % 2 == 0 && hex.all { it.digitToIntOrNull(16) != null }) { "Invalid hex" }
        return ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
    }
    private fun bytesHex(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it) }
}
