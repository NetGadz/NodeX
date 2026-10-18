package network.nodex.messenger.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.*
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import network.nodex.messenger.audio.VoiceRecorder
import network.nodex.messenger.ui.theme.*

@Composable
fun VoiceRecordBar(
    isRecording: Boolean,
    onCancelRecording: () -> Unit,
    onSendVoice: (durationSecs: Int, waveform: List<Float>, audioFilePath: String) -> Unit
) {
    val context = LocalContext.current
    val recorder = remember { VoiceRecorder(context) }
    var recordingSeconds by remember(isRecording) { mutableIntStateOf(0) }
    val capturedWaveform = remember(isRecording) { mutableStateListOf<Float>() }
    var isFinishing by remember { mutableStateOf(false) }

    DisposableEffect(Unit) {
        onDispose {
            if (!isFinishing) {
                recorder.cancelRecording()
            }
        }
    }

    // Pulsing dot animation
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 0.8f,
        targetValue = 1.3f,
        animationSpec = infiniteRepeatable(
            animation = tween(600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulseScale"
    )

    // Hardware recording loop
    LaunchedEffect(isRecording) {
        if (isRecording) {
            recordingSeconds = 0
            capturedWaveform.clear()
            recorder.startRecording()
            val startMs = System.currentTimeMillis()
            while (isActive) {
                delay(80)
                val elapsed = ((System.currentTimeMillis() - startMs) / 1000L).toInt()
                recordingSeconds = elapsed
                val amp = recorder.sampleAmplitude()
                if (capturedWaveform.size > 24) {
                    capturedWaveform.removeAt(0)
                }
                capturedWaveform.add(amp)
            }
        }
    }

    AnimatedVisibility(
        visible = isRecording,
        enter = fadeIn(),
        exit = fadeOut()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(24.dp))
                .background(NodeXSurface)
                .padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Cancel / Trash button
            IconButton(
                onClick = {
                    recorder.cancelRecording()
                    onCancelRecording()
                },
                modifier = Modifier.size(36.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = "Отмена",
                    tint = NodeXDestructive
                )
            }

            Spacer(modifier = Modifier.width(6.dp))

            // Pulsing recording dot
            Box(
                modifier = Modifier
                    .size(10.dp)
                    .scale(pulseScale)
                    .clip(CircleShape)
                    .background(Color(0xFFE53935))
            )

            Spacer(modifier = Modifier.width(8.dp))

            // Recording Timer
            Text(
                text = "%d:%02d".format(recordingSeconds / 60, recordingSeconds % 60),
                color = TextPrimary,
                fontWeight = FontWeight.Bold,
                fontSize = 14.sp
            )

            Spacer(modifier = Modifier.width(12.dp))

            // Live Waveform Visualizer
            Row(
                modifier = Modifier
                    .weight(1f)
                    .height(20.dp),
                horizontalArrangement = Arrangement.spacedBy(2.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                capturedWaveform.takeLast(20).forEach { amp ->
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height((amp * 20).dp.coerceAtLeast(3.dp))
                            .clip(RoundedCornerShape(2.dp))
                            .background(NodeXPrimary)
                    )
                }
            }

            Spacer(modifier = Modifier.width(8.dp))

            // Send Voice Button
            Box(
                modifier = Modifier
                    .size(40.dp)
                    .clip(CircleShape)
                    .background(NodeXPrimary)
                    .clickable {
                        isFinishing = true
                        val (file, duration, _) = recorder.stopRecording()
                        val filePath = file?.absolutePath ?: ""
                        val wave = if (capturedWaveform.isNotEmpty()) capturedWaveform.toList() else emptyList()
                        onSendVoice(duration.coerceAtLeast(1), wave, filePath)
                    },
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.Send,
                    contentDescription = "Отправить голосовое",
                    tint = Color.White,
                    modifier = Modifier.size(18.dp)
                )
            }
        }
    }
}
