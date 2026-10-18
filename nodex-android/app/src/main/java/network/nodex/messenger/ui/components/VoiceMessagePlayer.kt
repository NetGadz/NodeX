package network.nodex.messenger.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import network.nodex.messenger.audio.VoicePlayer
import network.nodex.messenger.ui.theme.*

@Composable
fun VoiceMessagePlayer(
    messageId: String,
    durationSecs: Int,
    waveform: List<Float>,
    isMe: Boolean,
    audioFilePath: String = ""
) {
    val context = LocalContext.current
    var isPlaying by remember { mutableStateOf(false) }
    var currentProgress by remember { mutableFloatStateOf(0f) }
    var playbackSpeed by remember { mutableFloatStateOf(1.0f) }

    val bars = remember(waveform) {
        if (waveform.isNotEmpty()) {
            waveform
        } else {
            listOf(
                0.25f, 0.45f, 0.70f, 0.90f, 0.60f, 0.35f, 0.50f, 0.85f,
                0.95f, 0.75f, 0.40f, 0.30f, 0.55f, 0.80f, 0.65f, 0.45f,
                0.35f, 0.60f, 0.90f, 0.70f, 0.40f, 0.30f, 0.50f, 0.35f
            )
        }
    }

    DisposableEffect(messageId) {
        onDispose {
            if (VoicePlayer.isPlaying(messageId)) {
                VoicePlayer.stop()
            }
        }
    }

    val animatedProgress by animateFloatAsState(targetValue = currentProgress, label = "voiceProgress")

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Play / Pause Circle Button with Gradient
        val buttonBrush = if (isMe) {
            Brush.linearGradient(listOf(Color(0xFF2F81F7), Color(0xFF1F4E79)))
        } else {
            Brush.linearGradient(listOf(Color(0xFF238636), Color(0xFF152E24)))
        }

        Box(
            modifier = Modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(buttonBrush)
                .clickable {
                    if (isPlaying) {
                        VoicePlayer.stop()
                        isPlaying = false
                        currentProgress = 0f
                    } else {
                        isPlaying = true
                        VoicePlayer.play(
                            context = context,
                            messageId = messageId,
                            filePath = audioFilePath,
                            speed = playbackSpeed,
                            onProgress = { currentProgress = it },
                            onComplete = {
                                isPlaying = false
                                currentProgress = 0f
                            }
                        )
                    }
                },
            contentAlignment = Alignment.Center
        ) {
            Icon(
                imageVector = if (isPlaying) Icons.Default.Pause else Icons.Default.PlayArrow,
                contentDescription = if (isPlaying) "Пауза" else "Слушать",
                tint = Color.White,
                modifier = Modifier.size(22.dp)
            )
        }

        Spacer(modifier = Modifier.width(10.dp))

        // Waveform Bars + Timer Column
        Column(modifier = Modifier.weight(1f)) {
            // Interactive Waveform
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(28.dp)
                    .pointerInput(Unit) {
                        detectTapGestures { offset ->
                            val fraction = (offset.x / size.width).coerceIn(0f, 1f)
                            currentProgress = fraction
                            VoicePlayer.seekToFraction(fraction)
                        }
                    },
                horizontalArrangement = Arrangement.spacedBy(2.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                bars.forEachIndexed { index, amp ->
                    val barFraction = index.toFloat() / bars.size.toFloat()
                    val isPlayed = barFraction <= animatedProgress
                    val activeColor = if (isMe) Color.White else NodeXPrimary
                    val inactiveColor = if (isMe) Color.White.copy(alpha = 0.35f) else TextMuted.copy(alpha = 0.5f)

                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height((amp * 26).dp.coerceAtLeast(3.dp))
                            .clip(RoundedCornerShape(2.dp))
                            .background(if (isPlayed) activeColor else inactiveColor)
                    )
                }
            }

            Spacer(modifier = Modifier.height(2.dp))

            // Time & Speed Switcher
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                val elapsedSecs = (currentProgress * durationSecs).toInt()
                val displaySecs = if (isPlaying && elapsedSecs > 0) elapsedSecs else durationSecs
                Text(
                    text = "%d:%02d".format(displaySecs / 60, displaySecs % 60),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (isMe) TextPrimary.copy(alpha = 0.85f) else TextSecondary,
                    fontSize = 11.sp
                )

                // Playback speed toggle chip
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (isMe) Color.White.copy(alpha = 0.15f) else NodeXSurfaceVariant)
                        .clickable {
                            playbackSpeed = when (playbackSpeed) {
                                1.0f -> 1.5f
                                1.5f -> 2.0f
                                else -> 1.0f
                            }
                            if (isPlaying) {
                                VoicePlayer.setSpeed(playbackSpeed)
                            }
                        }
                        .padding(horizontal = 6.dp, vertical = 2.dp)
                ) {
                    Text(
                        text = "${playbackSpeed}x",
                        color = Color.White,
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        fontSize = 10.sp
                    )
                }
            }
        }
    }
}
