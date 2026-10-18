package network.nodex.messenger.audio

import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.os.Build
import kotlinx.coroutines.*
import java.io.File

object VoicePlayer {
    private var mediaPlayer: MediaPlayer? = null
    private var currentPlayingId: String? = null
    private var progressJob: Job? = null
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    fun play(
        context: Context,
        messageId: String,
        filePath: String,
        speed: Float = 1.0f,
        onProgress: (Float) -> Unit,
        onComplete: () -> Unit
    ) {
        stop()

        val file = if (filePath.isNotBlank()) File(filePath) else null
        if (file != null && file.exists() && file.length() > 50L) {
            try {
                val mp = MediaPlayer().apply {
                    setAudioAttributes(
                        AudioAttributes.Builder()
                            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                            .setUsage(AudioAttributes.USAGE_MEDIA)
                            .build()
                    )
                    setDataSource(file.absolutePath)
                    prepare()
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                        try {
                            playbackParams = playbackParams.setSpeed(speed)
                        } catch (e: Exception) {}
                    }
                    setOnCompletionListener {
                        stop()
                        onComplete()
                    }
                    setOnErrorListener { _, _, _ ->
                        stop()
                        onComplete()
                        true
                    }
                    start()
                }

                mediaPlayer = mp
                currentPlayingId = messageId

                progressJob = scope.launch {
                    while (isActive && mp.isPlaying) {
                        val pos = mp.currentPosition.toFloat()
                        val dur = mp.duration.toFloat().coerceAtLeast(1f)
                        onProgress((pos / dur).coerceIn(0f, 1f))
                        delay(50)
                    }
                }
                return
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }

        // File is missing or invalid: notify user gracefully
        try {
            android.widget.Toast.makeText(context, "Аудиозапись не найдена", android.widget.Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {}
        onComplete()
    }

    fun pause() {
        mediaPlayer?.pause()
        progressJob?.cancel()
    }

    fun resume(onProgress: (Float) -> Unit, onComplete: () -> Unit) {
        val mp = mediaPlayer ?: return
        try {
            mp.start()
            progressJob = scope.launch {
                while (isActive && mp.isPlaying) {
                    val pos = mp.currentPosition.toFloat()
                    val dur = mp.duration.toFloat().coerceAtLeast(1f)
                    onProgress((pos / dur).coerceIn(0f, 1f))
                    delay(50)
                }
            }
        } catch (e: Exception) {
            stop()
            onComplete()
        }
    }

    fun seekTo(progress: Float) {
        val mp = mediaPlayer ?: return
        try {
            val targetMs = (progress.coerceIn(0f, 1f) * mp.duration).toInt()
            mp.seekTo(targetMs)
        } catch (e: Exception) {}
    }

    fun seekToFraction(fraction: Float) {
        seekTo(fraction)
    }

    fun setSpeed(speed: Float) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            try {
                mediaPlayer?.let { mp ->
                    mp.playbackParams = mp.playbackParams.setSpeed(speed)
                }
            } catch (e: Exception) {}
        }
    }

    fun isPlaying(messageId: String): Boolean {
        return currentPlayingId == messageId && mediaPlayer?.isPlaying == true
    }

    fun stop() {
        progressJob?.cancel()
        progressJob = null
        try {
            mediaPlayer?.stop()
            mediaPlayer?.release()
        } catch (e: Exception) {
            // ignore
        } finally {
            mediaPlayer = null
            currentPlayingId = null
        }
    }
}
