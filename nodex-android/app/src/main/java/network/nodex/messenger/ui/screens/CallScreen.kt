package network.nodex.messenger.ui.screens

import android.content.Context
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaRecorder
import android.media.RingtoneManager
import android.media.ToneGenerator
import android.os.Build
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.data.P2PTransportEngine
import network.nodex.messenger.ui.theme.*

@Composable
fun CallScreen(
    peerId: String,
    peerName: String,
    isIncoming: Boolean,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val activeCall by NodeXRepository.activeCall.collectAsState()
    var callSeconds by remember { mutableIntStateOf(0) }
    var isMuted by remember { mutableStateOf(false) }
    var isSpeakerOn by remember { mutableStateOf(false) }
    val isConnected = activeCall?.isConnected == true
    val isConnecting = activeCall?.isConnecting == true

    // Close screen if call ended
    LaunchedEffect(activeCall) {
        if (activeCall == null) {
            onBack()
        }
    }

    LaunchedEffect(activeCall?.callId) {
        val callId = activeCall?.callId ?: return@LaunchedEffect
        delay(45_000)
        val current = NodeXRepository.activeCall.value
        if (current?.callId == callId && !current.isConnected) {
            NodeXRepository.endCall()
        }
    }

    // Call duration timer
    LaunchedEffect(isConnected) {
        if (isConnected) {
            while (isActive) {
                delay(1000)
                callSeconds++
                NodeXRepository.updateCallDuration(callSeconds)
            }
        }
    }

    // Outgoing Call Offer Retransmission while dialing
    LaunchedEffect(isConnected, isIncoming) {
        if (!isConnected && !isIncoming) {
            while (isActive && !isConnected) {
                delay(1200)
                if (activeCall?.isConnected == true) break
                P2PTransportEngine.retransmitOfferIfDialing()
            }
        }
    }

    // Audible Call Audio: Ringback tone for outgoing, Ringtone for incoming
    LaunchedEffect(isConnected, isIncoming, isConnecting) {
        if (!isConnected && !(isIncoming && isConnecting)) {
            if (isIncoming) {
                var ringtone: android.media.Ringtone? = null
                try {
                    val uri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_RINGTONE)
                    ringtone = RingtoneManager.getRingtone(context, uri)
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                        ringtone?.isLooping = true
                    }
                    ringtone?.play()
                } catch (e: Exception) {
                    e.printStackTrace()
                }
                try {
                    while (isActive && !isConnected) {
                        delay(300)
                    }
                } finally {
                    try { ringtone?.stop() } catch (e: Exception) {}
                }
            } else {
                var toneGen: ToneGenerator? = null
                try {
                    toneGen = ToneGenerator(AudioManager.STREAM_VOICE_CALL, 85)
                } catch (e: Exception) {
                    try { toneGen = ToneGenerator(AudioManager.STREAM_MUSIC, 85) } catch (e2: Exception) {}
                }
                try {
                    while (isActive && !isConnected) {
                        toneGen?.startTone(ToneGenerator.TONE_SUP_RINGTONE, 1200)
                        delay(3500)
                    }
                } finally {
                    try {
                        toneGen?.stopTone()
                        toneGen?.release()
                    } catch (e: Exception) {}
                }
            }
        }
    }

    // Hardware Mute & Speakerphone Synchronization
    LaunchedEffect(isMuted) {
        network.nodex.messenger.webrtc.WebRtcCallManager.setMuted(isMuted)
    }

    LaunchedEffect(isSpeakerOn) {
        network.nodex.messenger.webrtc.WebRtcCallManager.setSpeakerphone(context, isSpeakerOn)
    }

    // Radar Pulse Animation
    val infiniteTransition = rememberInfiniteTransition(label = "callPulse")
    val pulseScale1 by infiniteTransition.animateFloat(
        initialValue = 0.95f,
        targetValue = 1.25f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse1"
    )
    val pulseScale2 by infiniteTransition.animateFloat(
        initialValue = 1.0f,
        targetValue = 1.45f,
        animationSpec = infiniteRepeatable(
            animation = tween(1600, easing = LinearOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse2"
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        NodeXBackground,
                        NodeXBackground.copy(red = 0.10f, green = 0.13f, blue = 0.20f),
                        NodeXSurface
                    )
                )
            )
            .statusBarsPadding()
            .navigationBarsPadding(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.SpaceBetween
        ) {
            // Header: Peer Info & Call Status
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Spacer(modifier = Modifier.height(32.dp))

                Text(
                    text = peerName,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold,
                    color = Color.White,
                    fontSize = 28.sp
                )

                Spacer(modifier = Modifier.height(8.dp))

                Text(
                    text = when {
                    isConnected -> "%02d:%02d • P2P-соединение".format(callSeconds / 60, callSeconds % 60)
                    isConnecting -> "Подключение к собеседнику…"
                        isIncoming -> "Входящий аудиозвонок..."
                        else -> "Вызов... Ожидание ответа..."
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    color = if (isConnected) NodeXEmerald else NodeXPrimary,
                    fontSize = 15.sp
                )
            }

            // Central Pulsing Avatar
            Box(
                modifier = Modifier.size(240.dp),
                contentAlignment = Alignment.Center
            ) {
                // Outer glow waves
                Box(
                    modifier = Modifier
                        .size(210.dp)
                        .scale(pulseScale2)
                        .clip(CircleShape)
                        .background(
                            (if (isConnected) NodeXEmerald else NodeXPrimary).copy(alpha = 0.08f)
                        )
                )
                Box(
                    modifier = Modifier
                        .size(170.dp)
                        .scale(pulseScale1)
                        .clip(CircleShape)
                        .background(
                            (if (isConnected) NodeXEmerald else NodeXPrimary).copy(alpha = 0.18f)
                        )
                )

                // Main Avatar Circle
                Box(
                    modifier = Modifier
                        .size(125.dp)
                        .clip(CircleShape)
                        .background(
                            Brush.linearGradient(
                                colors = listOf(
                                    NodeXPrimary.copy(alpha = 0.45f),
                                    NodeXPrimary
                                )
                            )
                        ),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = peerName.filter { it.isLetterOrDigit() }.take(1).uppercase().ifEmpty { "U" },
                        color = Color.White,
                        fontWeight = FontWeight.Bold,
                        fontSize = 48.sp
                    )
                }
            }

            // Bottom Controls
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                if (isIncoming && !isConnected && !isConnecting) {
                    // Incoming Call: Reject or Accept Buttons
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // Decline Call
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            FloatingActionButton(
                                onClick = {
                                    NodeXRepository.endCall()
                                    onBack()
                                },
                                containerColor = Color(0xFFE53935),
                                contentColor = Color.White,
                                shape = CircleShape,
                                modifier = Modifier.size(68.dp)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.CallEnd,
                                    contentDescription = "Отклонить",
                                    modifier = Modifier.size(32.dp)
                                )
                            }
                            Spacer(modifier = Modifier.height(8.dp))
                            Text("Отклонить", color = Color.White.copy(alpha = 0.7f), fontSize = 13.sp)
                        }

                        // Accept Call
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            FloatingActionButton(
                                onClick = {
                                    NodeXRepository.acceptCall()
                                },
                                containerColor = NodeXEmerald,
                                contentColor = Color.White,
                                shape = CircleShape,
                                modifier = Modifier.size(68.dp)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Call,
                                    contentDescription = "Принять",
                                    modifier = Modifier.size(32.dp)
                                )
                            }
                            Spacer(modifier = Modifier.height(8.dp))
                            Text("Принять", color = Color.White.copy(alpha = 0.7f), fontSize = 13.sp)
                        }
                    }
                } else {
                    // Outgoing / Active Call: Mute, Hangup, Speaker
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 20.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // Mute Mic Toggle
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            IconButton(
                                onClick = {
                                    isMuted = !isMuted
                                    NodeXRepository.toggleMute()
                                },
                                modifier = Modifier
                                    .size(60.dp)
                                    .clip(CircleShape)
                                    .background(if (isMuted) Color(0xFFE45D64) else Color.White.copy(alpha = 0.12f))
                            ) {
                                Icon(
                                    imageVector = if (isMuted) Icons.Default.MicOff else Icons.Default.Mic,
                                    contentDescription = "Микрофон",
                                    tint = Color.White,
                                    modifier = Modifier.size(28.dp)
                                )
                            }
                            Spacer(modifier = Modifier.height(6.dp))
                            Text(
                                text = if (isMuted) "Выкл" else "Вкл",
                                color = Color.White.copy(alpha = 0.7f),
                                fontSize = 12.sp
                            )
                        }

                        // End Call Button
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            FloatingActionButton(
                                onClick = {
                                    NodeXRepository.endCall()
                                    onBack()
                                },
                                containerColor = Color(0xFFD32F2F),
                                contentColor = Color.White,
                                shape = CircleShape,
                                modifier = Modifier.size(68.dp)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.CallEnd,
                                    contentDescription = "Завершить",
                                    modifier = Modifier.size(32.dp)
                                )
                            }
                            Spacer(modifier = Modifier.height(6.dp))
                            Text("Сброс", color = Color.White.copy(alpha = 0.7f), fontSize = 12.sp)
                        }

                        // Speaker Toggle
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            IconButton(
                                onClick = {
                                    isSpeakerOn = !isSpeakerOn
                                    NodeXRepository.toggleSpeaker()
                                },
                                modifier = Modifier
                                    .size(60.dp)
                                    .clip(CircleShape)
                                    .background(if (isSpeakerOn) NodeXPrimary else Color.White.copy(alpha = 0.12f))
                            ) {
                                Icon(
                                    imageVector = if (isSpeakerOn) Icons.Default.VolumeUp else Icons.Default.VolumeDown,
                                    contentDescription = "Динамик",
                                    tint = Color.White,
                                    modifier = Modifier.size(28.dp)
                                )
                            }
                            Spacer(modifier = Modifier.height(6.dp))
                            Text(
                                text = if (isSpeakerOn) "Динамик" else "Трубка",
                                color = Color.White.copy(alpha = 0.7f),
                                fontSize = 12.sp
                            )
                        }
                    }
                }
                Spacer(modifier = Modifier.height(30.dp))
            }
        }
    }
}
