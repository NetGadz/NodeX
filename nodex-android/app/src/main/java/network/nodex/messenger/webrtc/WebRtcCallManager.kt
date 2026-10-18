package network.nodex.messenger.webrtc

import android.content.Context
import android.media.AudioManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import kotlinx.coroutines.*
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.data.P2PTransportEngine
import org.json.JSONObject
import org.webrtc.*
import org.webrtc.audio.JavaAudioDeviceModule

/**
 * Production-ready WebRTC Audio Engine for NodeX Messenger.
 * Uses official Google WebRTC native C++ core with Opus codec (16-32 kbps adaptive),
 * hardware Acoustic Echo Cancellation (AEC), Noise Suppression (NS), Automatic Gain Control (AGC),
 * and DTLS-SRTP mandatory end-to-end encryption.
 * Signaling (SDP Offer/Answer, ICE Candidates) is exchanged via NodeX P2P DHT encrypted mesh.
 */
object WebRtcCallManager {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var peerConnectionFactory: PeerConnectionFactory? = null
    private var peerConnection: PeerConnection? = null
    private var localAudioSource: AudioSource? = null
    private var localAudioTrack: org.webrtc.AudioTrack? = null
    private var isInitialized = false

    private var currentPeerId: String = ""
    private var currentCallId: String = ""
    private var isCaller: Boolean = false
    private val pendingIceCandidates = mutableListOf<IceCandidate>()
    @Volatile private var compatibilityAudioTrack: AudioTrack? = null

    fun playReceivedPcm(samples: ShortArray, sampleRate: Int) {
        if (samples.isEmpty() || sampleRate !in 8_000..48_000) return
        // The NXCA bridge is used only for the Rust client. Android-to-Android
        // calls continue to use WebRTC's device, AEC and jitter handling.
        synchronized(this) {
            val track = compatibilityAudioTrack?.takeIf { it.state == AudioTrack.STATE_INITIALIZED && it.sampleRate == sampleRate }
                ?: run {
                    compatibilityAudioTrack?.runCatching { pause(); flush(); release() }
                    val minBytes = AudioTrack.getMinBufferSize(sampleRate, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_16BIT)
                    if (minBytes <= 0) return
                    AudioTrack.Builder()
                        .setAudioAttributes(AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                            .build())
                        .setAudioFormat(AudioFormat.Builder()
                            .setSampleRate(sampleRate)
                            .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .build())
                        .setBufferSizeInBytes((minBytes * 2).coerceAtMost(sampleRate * 2))
                        .setTransferMode(AudioTrack.MODE_STREAM)
                        .build().also { compatibilityAudioTrack = it }
                }
            if (track.playState != AudioTrack.PLAYSTATE_PLAYING) track.play()
            track.write(samples, 0, samples.size, AudioTrack.WRITE_NON_BLOCKING)
        }
    }

    private fun releaseCompatibilityAudio() {
        synchronized(this) {
            compatibilityAudioTrack?.runCatching { pause(); flush(); release() }
            compatibilityAudioTrack = null
        }
    }

    private val iceServers = listOf(
        PeerConnection.IceServer.builder("stun:stun.l.google.com:19302").createIceServer(),
        PeerConnection.IceServer.builder("stun:stun1.l.google.com:19302").createIceServer(),
        PeerConnection.IceServer.builder("stun:stun2.l.google.com:19302").createIceServer(),
        PeerConnection.IceServer.builder("stun:stun.cloudflare.com:3478").createIceServer()
    )

    fun initialize(context: Context) {
        if (isInitialized) return
        val appContext = context.applicationContext
        try {
            PeerConnectionFactory.initialize(
                PeerConnectionFactory.InitializationOptions.builder(appContext)
                    .setEnableInternalTracer(false)
                    .createInitializationOptions()
            )

            val audioDeviceModule = JavaAudioDeviceModule.builder(appContext)
                .setUseHardwareAcousticEchoCanceler(true)
                .setUseHardwareNoiseSuppressor(true)
                .createAudioDeviceModule()

            val options = PeerConnectionFactory.Options()
            peerConnectionFactory = PeerConnectionFactory.builder()
                .setOptions(options)
                .setAudioDeviceModule(audioDeviceModule)
                .createPeerConnectionFactory()

            isInitialized = true
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    /**
     * Start outgoing WebRTC call as caller.
     */
    fun startCall(context: Context, peerId: String, callId: String) {
        initialize(context)
        currentPeerId = peerId.trim().lowercase()
        currentCallId = callId
        isCaller = true
        pendingIceCandidates.clear()

        setupPeerConnection(context)

        val mediaConstraints = MediaConstraints().apply {
            mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"))
            mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveVideo", "false"))
        }

        peerConnection?.createOffer(object : SimpleSdpObserver() {
            override fun onCreateSuccess(desc: SessionDescription?) {
                if (desc == null) return
                peerConnection?.setLocalDescription(object : SimpleSdpObserver() {
                    override fun onSetSuccess() {
                        val myNodeId = NodeXRepository.identity.value.nodeId
                        P2PTransportEngine.sendCallSignal(
                            contactId = currentPeerId,
                            signalType = "Offer",
                            callId = currentCallId,
                            myId = myNodeId,
                            sdpOrCandidate = desc.description
                        )
                    }
                }, desc)
            }
        }, mediaConstraints)
    }

    /**
     * Accept incoming WebRTC call as callee after receiving Offer.
     */
    fun acceptCall(context: Context, peerId: String, callId: String, offerSdp: String) {
        initialize(context)
        currentPeerId = peerId.trim().lowercase()
        currentCallId = callId
        isCaller = false
        pendingIceCandidates.clear()

        setupPeerConnection(context)

        val remoteDesc = SessionDescription(SessionDescription.Type.OFFER, offerSdp)
        peerConnection?.setRemoteDescription(object : SimpleSdpObserver() {
            override fun onSetSuccess() {
                drainPendingCandidates()
                val mediaConstraints = MediaConstraints().apply {
                    mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"))
                    mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveVideo", "false"))
                }
                peerConnection?.createAnswer(object : SimpleSdpObserver() {
                    override fun onCreateSuccess(desc: SessionDescription?) {
                        if (desc == null) return
                        peerConnection?.setLocalDescription(object : SimpleSdpObserver() {
                            override fun onSetSuccess() {
                                val myNodeId = NodeXRepository.identity.value.nodeId
                                P2PTransportEngine.sendCallSignal(
                                    contactId = currentPeerId,
                                    signalType = "Answer",
                                    callId = currentCallId,
                                    myId = myNodeId,
                                    sdpOrCandidate = desc.description
                                )
                            }
                        }, desc)
                    }
                }, mediaConstraints)
            }
        }, remoteDesc)
    }

    /**
     * Called on caller device when callee returns SDP Answer.
     */
    fun onRemoteOfferReceived(offerSdp: String) {
        val connection = peerConnection ?: return
        if (isCaller || offerSdp.isBlank() || currentPeerId.isBlank() || currentCallId.isBlank()) return
        val remoteDesc = SessionDescription(SessionDescription.Type.OFFER, offerSdp)
        connection.setRemoteDescription(object : SimpleSdpObserver() {
            override fun onSetSuccess() {
                drainPendingCandidates()
                val constraints = MediaConstraints().apply {
                    mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"))
                    mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveVideo", "false"))
                }
                connection.createAnswer(object : SimpleSdpObserver() {
                    override fun onCreateSuccess(desc: SessionDescription?) {
                        if (desc == null) return
                        connection.setLocalDescription(object : SimpleSdpObserver() {
                            override fun onSetSuccess() {
                                P2PTransportEngine.sendCallSignal(
                                    contactId = currentPeerId,
                                    signalType = "Answer",
                                    callId = currentCallId,
                                    myId = NodeXRepository.identity.value.nodeId,
                                    sdpOrCandidate = desc.description
                                )
                            }
                        }, desc)
                    }
                }, constraints)
            }
        }, remoteDesc)
    }

    fun onAnswerReceived(answerSdp: String) {
        val remoteDesc = SessionDescription(SessionDescription.Type.ANSWER, answerSdp)
        peerConnection?.setRemoteDescription(object : SimpleSdpObserver() {
            override fun onSetSuccess() {
                drainPendingCandidates()
            }
        }, remoteDesc)
    }

    /**
     * Called when ICE candidate arrives from peer via DHT.
     */
    fun onRemoteIceCandidate(candidateJsonStr: String) {
        try {
            val json = JSONObject(candidateJsonStr)
            val sdpMid = json.optString("sdpMid", "")
            val sdpMLineIndex = json.optInt("sdpMLineIndex", 0)
            val sdp = json.optString("sdp", "")
            if (sdp.isNotBlank()) {
                val candidate = IceCandidate(sdpMid, sdpMLineIndex, sdp)
                if (peerConnection?.remoteDescription != null) {
                    peerConnection?.addIceCandidate(candidate)
                } else {
                    pendingIceCandidates.add(candidate)
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun drainPendingCandidates() {
        for (cand in pendingIceCandidates) {
            peerConnection?.addIceCandidate(cand)
        }
        pendingIceCandidates.clear()
    }

    private fun setupPeerConnection(context: Context) {
        try {
            peerConnection?.close()
            peerConnection?.dispose()
            peerConnection = null
        } catch (e: Exception) {}

        val rtcConfig = PeerConnection.RTCConfiguration(iceServers).apply {
            // Keep TCP candidates available for restrictive mobile networks.
            // Disabling them made calls wait for an unreachable UDP path.
            tcpCandidatePolicy = PeerConnection.TcpCandidatePolicy.ENABLED
            bundlePolicy = PeerConnection.BundlePolicy.MAXBUNDLE
            rtcpMuxPolicy = PeerConnection.RtcpMuxPolicy.REQUIRE
            continualGatheringPolicy = PeerConnection.ContinualGatheringPolicy.GATHER_CONTINUALLY
            keyType = PeerConnection.KeyType.ECDSA
            sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN
        }

        val observer = object : SimplePeerConnectionObserver() {
            override fun onIceCandidate(candidate: IceCandidate?) {
                if (candidate == null) return
                try {
                    val candObj = JSONObject().apply {
                        put("sdpMid", candidate.sdpMid)
                        put("sdpMLineIndex", candidate.sdpMLineIndex)
                        put("sdp", candidate.sdp)
                    }
                    val myNodeId = NodeXRepository.identity.value.nodeId
                    P2PTransportEngine.sendCallSignal(
                        contactId = currentPeerId,
                        signalType = "IceCandidate",
                        callId = currentCallId,
                        myId = myNodeId,
                        sdpOrCandidate = candObj.toString()
                    )
                } catch (e: Exception) {}
            }

            override fun onIceConnectionChange(state: PeerConnection.IceConnectionState?) {
                if (state == PeerConnection.IceConnectionState.CONNECTED || state == PeerConnection.IceConnectionState.COMPLETED) {
                    NodeXRepository.onCallAnswered(currentPeerId)
                } else if (state == PeerConnection.IceConnectionState.FAILED) {
                    // A bare restartIce() does not renegotiate SDP. Create a
                    // real ICE-restart offer so the peer can learn the new
                    // candidates instead of leaving the call stuck on dialing.
                    restartIceNegotiation()
                }
            }
        }

        peerConnection = peerConnectionFactory?.createPeerConnection(rtcConfig, observer)

        // Put Android in voice communication mode before the first remote
        // packet arrives so the WebRTC audio device is actually audible.
        runCatching {
            val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as? AudioManager
            audioManager?.mode = AudioManager.MODE_IN_COMMUNICATION
            audioManager?.isSpeakerphoneOn = true
        }

        // Create local audio track with Acoustic Echo Cancellation & Noise Suppression
        val audioConstraints = MediaConstraints().apply {
            mandatory.add(MediaConstraints.KeyValuePair("googEchoCancellation", "true"))
            mandatory.add(MediaConstraints.KeyValuePair("googAutoGainControl", "true"))
            mandatory.add(MediaConstraints.KeyValuePair("googHighpassFilter", "true"))
            mandatory.add(MediaConstraints.KeyValuePair("googNoiseSuppression", "true"))
        }

        localAudioSource = peerConnectionFactory?.createAudioSource(audioConstraints)
        localAudioTrack = peerConnectionFactory?.createAudioTrack("ARDAMSa0", localAudioSource)
        localAudioTrack?.setEnabled(true)

        val streamLabels = listOf("ARDAMS")
        peerConnection?.addTrack(localAudioTrack, streamLabels)
    }

    private fun restartIceNegotiation() {
        val connection = peerConnection ?: return
        val peer = currentPeerId
        val call = currentCallId
        if (peer.isBlank() || call.isBlank()) return
        val constraints = MediaConstraints().apply {
            mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveAudio", "true"))
            mandatory.add(MediaConstraints.KeyValuePair("OfferToReceiveVideo", "false"))
            mandatory.add(MediaConstraints.KeyValuePair("IceRestart", "true"))
        }
        connection.createOffer(object : SimpleSdpObserver() {
            override fun onCreateSuccess(desc: SessionDescription?) {
                if (desc == null) return
                connection.setLocalDescription(object : SimpleSdpObserver() {
                    override fun onSetSuccess() {
                        P2PTransportEngine.sendCallSignal(
                            contactId = peer,
                            signalType = "Offer",
                            callId = call,
                            myId = NodeXRepository.identity.value.nodeId,
                            sdpOrCandidate = desc.description
                        )
                    }
                }, desc)
            }
        }, constraints)
    }

    fun setMuted(muted: Boolean) {
        localAudioTrack?.setEnabled(!muted)
    }

    fun setSpeakerphone(context: Context, isSpeaker: Boolean) {
        try {
            val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as? AudioManager
            audioManager?.mode = AudioManager.MODE_IN_COMMUNICATION
            audioManager?.isSpeakerphoneOn = isSpeaker
        } catch (e: Exception) {}
    }

    fun endCall() {
        releaseCompatibilityAudio()
        try {
            peerConnection?.close()
            peerConnection?.dispose()
            peerConnection = null

            localAudioTrack?.dispose()
            localAudioTrack = null

            localAudioSource?.dispose()
            localAudioSource = null

            pendingIceCandidates.clear()
            currentPeerId = ""
            currentCallId = ""
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }
}

open class SimpleSdpObserver : SdpObserver {
    override fun onCreateSuccess(desc: SessionDescription?) {}
    override fun onSetSuccess() {}
    override fun onCreateFailure(err: String?) {}
    override fun onSetFailure(err: String?) {}
}

open class SimplePeerConnectionObserver : PeerConnection.Observer {
    override fun onSignalingChange(state: PeerConnection.SignalingState?) {}
    override fun onIceConnectionChange(state: PeerConnection.IceConnectionState?) {}
    override fun onIceConnectionReceivingChange(receiving: Boolean) {}
    override fun onIceGatheringChange(state: PeerConnection.IceGatheringState?) {}
    override fun onIceCandidate(candidate: IceCandidate?) {}
    override fun onIceCandidatesRemoved(candidates: Array<out IceCandidate>?) {}
    override fun onAddStream(stream: MediaStream?) {}
    override fun onRemoveStream(stream: MediaStream?) {}
    override fun onDataChannel(channel: DataChannel?) {}
    override fun onRenegotiationNeeded() {}
    override fun onAddTrack(receiver: RtpReceiver?, streams: Array<out MediaStream>?) {}
}
