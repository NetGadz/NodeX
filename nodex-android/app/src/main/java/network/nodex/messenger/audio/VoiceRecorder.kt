package network.nodex.messenger.audio

import android.content.Context
import android.media.MediaRecorder
import android.os.Build
import java.io.File
import kotlin.concurrent.thread

class VoiceRecorder(private val context: Context) {
    private var mediaRecorder: MediaRecorder? = null
    private var outputFile: File? = null
    private var startTime: Long = 0
    @Volatile private var _isRecording = false
    @Volatile private var currentAmp = 0.15f
    private val amplitudeSamples = mutableListOf<Float>()
    private var ampMonitorThread: Thread? = null

    val isRecording: Boolean
        get() = _isRecording

    fun startRecording(): File? {
        try {
            val audioDir = File(context.filesDir, "audio_notes").apply { mkdirs() }
            val file = File(audioDir, "voice_${System.currentTimeMillis()}.m4a")
            outputFile = file
            amplitudeSamples.clear()

            val mr = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                MediaRecorder(context)
            } else {
                @Suppress("DEPRECATION")
                MediaRecorder()
            }

            try {
                mr.setAudioSource(MediaRecorder.AudioSource.MIC)
            } catch (e: Exception) {
                // Fallback to DEFAULT audio source
                mr.setAudioSource(MediaRecorder.AudioSource.DEFAULT)
            }

            mr.setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
            mr.setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
            mr.setAudioChannels(1)
            mr.setAudioSamplingRate(44100)
            mr.setAudioEncodingBitRate(32000)
            mr.setOutputFile(file.absolutePath)
            mr.prepare()
            mr.start()

            mediaRecorder = mr
            _isRecording = true
            startTime = System.currentTimeMillis()

            // Monitor amplitude in background thread for live waveform
            ampMonitorThread = thread(start = true, name = "NodeX-AmpMonitor") {
                while (_isRecording) {
                    try {
                        val maxAmp = mediaRecorder?.maxAmplitude ?: 0
                        val normalized = (maxAmp.toFloat() / 32767f * 2.8f).coerceIn(0.15f, 1.0f)
                        currentAmp = normalized
                        synchronized(amplitudeSamples) {
                            if (amplitudeSamples.size < 60) {
                                amplitudeSamples.add(normalized)
                            }
                        }
                    } catch (e: Exception) {
                        // ignore recorder query glitch
                    }
                    try {
                        Thread.sleep(80)
                    } catch (e: Exception) {
                        break
                    }
                }
            }

            return file
        } catch (e: Exception) {
            e.printStackTrace()
            cleanup()
            return null
        }
    }

    fun sampleAmplitude(): Float {
        return currentAmp
    }

    fun stopRecording(): Triple<File?, Int, List<Float>> {
        val duration = ((System.currentTimeMillis() - startTime) / 1000).toInt().coerceAtLeast(1)
        _isRecording = false

        // Ensure minimum recording duration to avoid MediaRecorder "stop failed" crash
        val elapsed = System.currentTimeMillis() - startTime
        if (elapsed < 600) {
            try {
                Thread.sleep(600 - elapsed)
            } catch (e: Exception) {}
        }

        try {
            mediaRecorder?.stop()
        } catch (e: Exception) {
            e.printStackTrace()
        } finally {
            cleanup()
        }

        try {
            ampMonitorThread?.join(300)
        } catch (e: Exception) {}
        ampMonitorThread = null

        // Hand off file ownership to caller so cancelRecording or onDispose never deletes it
        val file = outputFile
        outputFile = null

        val finalFile = if (file != null && file.exists() && file.length() > 50L) {
            file
        } else {
            null
        }

        val waves = synchronized(amplitudeSamples) {
            if (amplitudeSamples.isNotEmpty()) amplitudeSamples.toList()
            else List(24) { (0.2f + Math.random().toFloat() * 0.6f) }
        }

        return Triple(finalFile, duration, waves)
    }

    fun cancelRecording() {
        if (!_isRecording && outputFile == null) return
        _isRecording = false
        try {
            mediaRecorder?.stop()
        } catch (e: Exception) {
            // ignore
        } finally {
            cleanup()
        }

        try {
            ampMonitorThread?.join(300)
        } catch (e: Exception) {}
        ampMonitorThread = null

        outputFile?.delete()
        outputFile = null
        amplitudeSamples.clear()
    }

    private fun cleanup() {
        try {
            mediaRecorder?.reset()
            mediaRecorder?.release()
        } catch (e: Exception) {
            // ignore
        }
        mediaRecorder = null
    }
}
