package com.liquify.video

import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.media.ImageReader
import android.graphics.PixelFormat
import android.hardware.HardwareBuffer
import android.util.Log
import android.view.Surface

import android.content.Context
import android.content.res.AssetFileDescriptor

class VideoDecoder(context: Context, path: String) {
    private val TAG = "VideoDecoder"
    private var extractor: MediaExtractor? = null
    private var codec: MediaCodec? = null
    private var reader: ImageReader? = null
    
    var width: Int = 0
    var height: Int = 0
    var duration: Long = 0

    init {
        try {
            Log.i(TAG, "Initializing VideoDecoder with path: $path")
            extractor = MediaExtractor()
            
            if (path.startsWith("asset:///")) {
                val assetName = path.substring("asset:///".length)
                val afd: AssetFileDescriptor = context.assets.openFd(assetName)
                extractor!!.setDataSource(afd.fileDescriptor, afd.startOffset, afd.length)
                afd.close()
                Log.i(TAG, "Loaded from assets: $assetName")
            } else {
                extractor!!.setDataSource(path)
            }
            
            for (i in 0 until extractor!!.trackCount) {
                val format = extractor!!.getTrackFormat(i)
                val mime = format.getString(MediaFormat.KEY_MIME)
                if (mime?.startsWith("video/") == true) {
                    extractor!!.selectTrack(i)
                    
                    width = format.getInteger(MediaFormat.KEY_WIDTH)
                    height = format.getInteger(MediaFormat.KEY_HEIGHT)
                    duration = format.getLong(MediaFormat.KEY_DURATION)
                    
                    Log.i(TAG, "Found video track: $mime, ${width}x${height}, duration: $duration")

                    reader = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 3, HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE)
                    
                    codec = MediaCodec.createDecoderByType(mime)
                    codec!!.configure(format, reader!!.surface, null, 0)
                    codec!!.start()
                    Log.i(TAG, "MediaCodec started successfully")
                    break
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize VideoDecoder", e)
        }
    }

    fun getNextFrameBuffer(): HardwareBuffer? {
        try {
            val info = MediaCodec.BufferInfo()
            val inputIndex = codec!!.dequeueInputBuffer(1000)
            if (inputIndex >= 0) {
                val buffer = codec!!.getInputBuffer(inputIndex)
                val sampleSize = extractor!!.readSampleData(buffer!!, 0)
                if (sampleSize < 0) {
                    extractor!!.seekTo(0, MediaExtractor.SEEK_TO_PREVIOUS_SYNC)
                } else {
                    codec!!.queueInputBuffer(inputIndex, 0, sampleSize, extractor!!.sampleTime, 0)
                    extractor!!.advance()
                }
            }

            val outputIndex = codec!!.dequeueOutputBuffer(info, 1000)
            if (outputIndex >= 0) {
                codec!!.releaseOutputBuffer(outputIndex, true)
                
                val image = reader!!.acquireLatestImage()
                if (image != null) {
                    val buffer = image.hardwareBuffer
                    image.close()
                    return buffer
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error during getNextFrameBuffer", e)
        }
        return null
    }

    fun release() {
        codec?.stop()
        codec?.release()
        extractor?.release()
        reader?.close()
    }
}
