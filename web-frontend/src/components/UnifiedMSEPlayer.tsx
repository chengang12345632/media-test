import React, { useEffect, useRef, useState } from 'react'
import './UnifiedMSEPlayer.css'

interface UnifiedMSEPlayerProps {
  sessionId: string
  mode: 'live' | 'playback'
  streamUrl: string
  controlUrl?: string
  onError?: (error: string) => void
  onStatusChange?: (status: string) => void
}

interface BufferConfig {
  targetBuffer: number // 目标缓冲时间（秒）
  minBuffer: number    // 最小缓冲时间（秒）
  maxBuffer: number    // 最大缓冲时间（秒）
}

function UnifiedMSEPlayer({
  sessionId,
  mode,
  streamUrl,
  controlUrl,
  onError,
  onStatusChange,
}: UnifiedMSEPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const mediaSourceRef = useRef<MediaSource | null>(null)
  const sourceBufferRef = useRef<SourceBuffer | null>(null)
  const eventSourceRef = useRef<EventSource | null>(null)
  
  const [status, setStatus] = useState<string>('初始化中...')
  const [isReady, setIsReady] = useState(false)
  const [bufferConfig, setBufferConfig] = useState<BufferConfig>({
    targetBuffer: mode === 'live' ? 0.3 : 1.0,  // 直通300ms，回放1s
    minBuffer: mode === 'live' ? 0.1 : 0.5,     // 直通100ms，回放500ms
    maxBuffer: mode === 'live' ? 0.5 : 2.0,     // 直通500ms，回放2s
  })
  
  const [stats, setStats] = useState({
    segmentsReceived: 0,
    bytesReceived: 0,
    currentBuffer: 0,
    droppedFrames: 0,
  })

  const [playbackRate, setPlaybackRate] = useState(1.0)
  const [isPaused, setIsPaused] = useState(false)
  const [duration, setDuration] = useState(0)
  const [currentTime, setCurrentTime] = useState(0)

  const segmentQueueRef = useRef<ArrayBuffer[]>([])
  const isAppendingRef = useRef(false)
  const reconnectAttemptsRef = useRef(0)
  const maxReconnectAttempts = 5

  // 初始化 MediaSource
  useEffect(() => {
    if (!videoRef.current) return

    console.log('[UnifiedMSEPlayer] Initializing MediaSource', { sessionId, mode })
    
    // 检查浏览器支持
    if (!window.MediaSource) {
      const error = '浏览器不支持 Media Source Extensions (MSE)'
      setStatus(error)
      onError?.(error)
      return
    }

    // 创建 MediaSource
    const mediaSource = new MediaSource()
    mediaSourceRef.current = mediaSource
    
    // 设置 video 元素的 src
    const objectUrl = URL.createObjectURL(mediaSource)
    videoRef.current.src = objectUrl

    // 监听 sourceopen 事件
    mediaSource.addEventListener('sourceopen', handleSourceOpen)
    
    // 监听 sourceended 事件
    mediaSource.addEventListener('sourceended', () => {
      console.log('[UnifiedMSEPlayer] MediaSource ended')
      setStatus('播放结束')
    })
    
    // 监听 sourceclose 事件
    mediaSource.addEventListener('sourceclose', () => {
      console.log('[UnifiedMSEPlayer] MediaSource closed')
    })

    return () => {
      cleanup()
      URL.revokeObjectURL(objectUrl)
    }
  }, [sessionId, mode])

  // 处理 sourceopen 事件
  const handleSourceOpen = () => {
    console.log('[UnifiedMSEPlayer] MediaSource opened')
    
    const mediaSource = mediaSourceRef.current
    if (!mediaSource) return

    try {
      // 创建 SourceBuffer
      // 使用 fMP4 格式，H.264 视频编码
      const mimeType = 'video/mp4; codecs="avc1.64001f"'
      
      if (!MediaSource.isTypeSupported(mimeType)) {
        const error = `不支持的 MIME 类型: ${mimeType}`
        console.error('[UnifiedMSEPlayer]', error)
        setStatus(error)
        onError?.(error)
        return
      }

      const sourceBuffer = mediaSource.addSourceBuffer(mimeType)
      sourceBufferRef.current = sourceBuffer
      
      // 设置 SourceBuffer 模式
      if (mode === 'live') {
        sourceBuffer.mode = 'sequence' // 直通播放使用序列模式
      } else {
        sourceBuffer.mode = 'segments' // 回放使用分片模式
      }

      // 监听 updateend 事件
      sourceBuffer.addEventListener('updateend', () => {
        isAppendingRef.current = false
        // 继续处理队列中的分片
        processSegmentQueue()
        // 更新缓冲统计
        updateBufferStats()
      })
      
      // 监听 error 事件
      sourceBuffer.addEventListener('error', (e) => {
        console.error('[UnifiedMSEPlayer] SourceBuffer error:', e)
        isAppendingRef.current = false
      })

      console.log('[UnifiedMSEPlayer] SourceBuffer created', {
        mimeType,
        mode: sourceBuffer.mode,
      })

      setStatus('准备就绪')
      setIsReady(true)
      onStatusChange?.('ready')
      
      // SourceBuffer 准备好后，建立 SSE 连接
      connectSSE()
      
    } catch (error) {
      const errorMsg = `创建 SourceBuffer 失败: ${error}`
      console.error('[UnifiedMSEPlayer]', errorMsg)
      setStatus(errorMsg)
      onError?.(errorMsg)
    }
  }

  // 建立 SSE 连接
  const connectSSE = () => {
    console.log('[UnifiedMSEPlayer] Connecting to SSE:', streamUrl)
    setStatus('连接到服务器...')
    
    try {
      const eventSource = new EventSource(streamUrl)
      eventSourceRef.current = eventSource
      
      eventSource.onopen = () => {
        console.log('[UnifiedMSEPlayer] SSE connection opened')
        setStatus('已连接，等待视频数据...')
        // 重置重连计数
        reconnectAttemptsRef.current = 0
      }
      
      eventSource.addEventListener('segment', handleSegmentEvent)
      
      eventSource.onerror = (error) => {
        console.error('[UnifiedMSEPlayer] SSE error:', error)
        
        // 关闭当前连接
        eventSource.close()
        eventSourceRef.current = null
        
        // 检查是否应该重连
        if (reconnectAttemptsRef.current < maxReconnectAttempts) {
          reconnectAttemptsRef.current += 1
          
          // 计算退避延迟（指数退避）
          const delay = Math.min(1000 * Math.pow(2, reconnectAttemptsRef.current - 1), 30000)
          
          const errorMsg = `连接断开，${delay / 1000}秒后重试 (${reconnectAttemptsRef.current}/${maxReconnectAttempts})`
          console.log('[UnifiedMSEPlayer]', errorMsg)
          setStatus(errorMsg)
          
          // 延迟后重连
          setTimeout(() => {
            console.log('[UnifiedMSEPlayer] Attempting to reconnect...')
            connectSSE()
          }, delay)
        } else {
          const errorMsg = `连接失败，已达到最大重试次数 (${maxReconnectAttempts})`
          console.error('[UnifiedMSEPlayer]', errorMsg)
          setStatus(errorMsg)
          onError?.(errorMsg)
        }
      }
      
    } catch (error) {
      const errorMsg = `建立 SSE 连接失败: ${error}`
      console.error('[UnifiedMSEPlayer]', errorMsg)
      setStatus(errorMsg)
      onError?.(errorMsg)
    }
  }

  // 处理接收到的分片事件
  const handleSegmentEvent = (event: MessageEvent) => {
    // 检查 EventSource 是否仍然活跃
    if (!eventSourceRef.current) {
      console.log('[UnifiedMSEPlayer] EventSource closed, ignoring segment')
      return
    }
    
    try {
      const segment = JSON.parse(event.data)
      
      console.log('[UnifiedMSEPlayer] Received segment:', {
        segment_id: segment.segment_id,
        timestamp: segment.timestamp,
        duration: segment.duration,
        is_keyframe: segment.is_keyframe,
        format: segment.format,
        data_length: segment.data?.length || 0,
      })
      
      // 解码 base64 数据
      if (!segment.data) {
        console.warn('[UnifiedMSEPlayer] Segment has no data, skipping')
        // 优雅降级：跳过空分片，继续播放
        return
      }
      
      let binaryData: ArrayBuffer
      try {
        binaryData = base64ToArrayBuffer(segment.data)
      } catch (error) {
        console.error('[UnifiedMSEPlayer] Failed to decode segment data:', error)
        // 优雅降级：跳过损坏的分片
        setStats(prev => ({
          ...prev,
          droppedFrames: prev.droppedFrames + 1,
        }))
        return
      }
      
      // 验证分片数据
      if (binaryData.byteLength === 0) {
        console.warn('[UnifiedMSEPlayer] Segment data is empty, skipping')
        return
      }
      
      // 更新统计信息
      setStats(prev => ({
        ...prev,
        segmentsReceived: prev.segmentsReceived + 1,
        bytesReceived: prev.bytesReceived + binaryData.byteLength,
      }))
      
      // 将分片加入队列
      segmentQueueRef.current.push(binaryData)
      
      // 尝试追加分片到 SourceBuffer
      processSegmentQueue()
      
      setStatus(`播放中 (${segment.timestamp.toFixed(2)}s)`)
      
    } catch (error) {
      console.error('[UnifiedMSEPlayer] Error processing segment:', error)
      // 优雅降级：记录错误但继续处理后续分片
      setStats(prev => ({
        ...prev,
        droppedFrames: prev.droppedFrames + 1,
      }))
    }
  }

  // Base64 转 ArrayBuffer
  const base64ToArrayBuffer = (base64: string): ArrayBuffer => {
    const binaryString = atob(base64)
    const bytes = new Uint8Array(binaryString.length)
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }
    return bytes.buffer
  }

  // 处理分片队列
  const processSegmentQueue = () => {
    // 检查是否已停止（EventSource 已关闭）
    if (!eventSourceRef.current) {
      console.log('[UnifiedMSEPlayer] EventSource closed, stopping queue processing')
      segmentQueueRef.current = []
      return
    }
    
    if (isAppendingRef.current || segmentQueueRef.current.length === 0) {
      return
    }
    
    const sourceBuffer = sourceBufferRef.current
    if (!sourceBuffer || sourceBuffer.updating) {
      return
    }
    
    // 检查是否需要清理旧缓冲
    manageBuffer()
    
    // 从队列中取出一个分片
    const segment = segmentQueueRef.current.shift()
    if (!segment) return
    
    try {
      isAppendingRef.current = true
      sourceBuffer.appendBuffer(segment)
      
      console.log('[UnifiedMSEPlayer] Appending segment to buffer', {
        queueLength: segmentQueueRef.current.length,
        segmentSize: segment.byteLength,
      })
      
    } catch (error) {
      console.error('[UnifiedMSEPlayer] Error appending buffer:', error)
      isAppendingRef.current = false
      onError?.(`追加缓冲失败: ${error}`)
      
      // 如果是 QuotaExceededError，尝试清理缓冲
      if (error instanceof DOMException && error.name === 'QuotaExceededError') {
        console.warn('[UnifiedMSEPlayer] Quota exceeded, attempting to clean buffer')
        forceCleanBuffer()
      }
    }
  }

  // 管理缓冲区
  const manageBuffer = () => {
    const video = videoRef.current
    const sourceBuffer = sourceBufferRef.current
    
    if (!video || !sourceBuffer || sourceBuffer.updating) {
      return
    }
    
    try {
      const buffered = sourceBuffer.buffered
      if (buffered.length === 0) return
      
      const currentTime = video.currentTime
      const bufferedEnd = buffered.end(buffered.length - 1)
      const bufferedAmount = bufferedEnd - currentTime
      
      // 如果缓冲超过最大值，移除旧数据
      if (bufferedAmount > bufferConfig.maxBuffer) {
        const removeEnd = currentTime - 1 // 保留当前时间前1秒
        
        if (removeEnd > 0 && buffered.start(0) < removeEnd) {
          console.log('[UnifiedMSEPlayer] Removing old buffer', {
            from: buffered.start(0),
            to: removeEnd,
            bufferedAmount,
            maxBuffer: bufferConfig.maxBuffer,
          })
          
          sourceBuffer.remove(buffered.start(0), removeEnd)
        }
      }
      
    } catch (error) {
      console.warn('[UnifiedMSEPlayer] Error managing buffer:', error)
    }
  }

  // 强制清理缓冲（当配额超限时）
  const forceCleanBuffer = () => {
    const video = videoRef.current
    const sourceBuffer = sourceBufferRef.current
    
    if (!video || !sourceBuffer || sourceBuffer.updating) {
      return
    }
    
    try {
      const buffered = sourceBuffer.buffered
      if (buffered.length === 0) return
      
      const currentTime = video.currentTime
      const removeEnd = currentTime - 0.5 // 保留当前时间前0.5秒
      
      if (removeEnd > 0 && buffered.start(0) < removeEnd) {
        console.log('[UnifiedMSEPlayer] Force cleaning buffer', {
          from: buffered.start(0),
          to: removeEnd,
        })
        
        sourceBuffer.remove(buffered.start(0), removeEnd)
      }
      
    } catch (error) {
      console.error('[UnifiedMSEPlayer] Error force cleaning buffer:', error)
    }
  }

  // 检查是否有足够的数据播放
  const hasEnoughData = (): boolean => {
    const video = videoRef.current
    if (!video) return false
    
    try {
      const buffered = video.buffered
      if (buffered.length === 0) return false
      
      const currentTime = video.currentTime
      const bufferedEnd = buffered.end(buffered.length - 1)
      const bufferedAmount = bufferedEnd - currentTime
      
      return bufferedAmount >= bufferConfig.minBuffer
      
    } catch (error) {
      console.warn('[UnifiedMSEPlayer] Error checking buffer:', error)
      return false
    }
  }

  // 监听视频播放事件
  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    
    const handleWaiting = () => {
      console.log('[UnifiedMSEPlayer] Video waiting for data')
      setStatus('缓冲中...')
    }
    
    const handlePlaying = () => {
      console.log('[UnifiedMSEPlayer] Video playing')
      setStatus('播放中')
    }
    
    const handleCanPlay = () => {
      console.log('[UnifiedMSEPlayer] Video can play')
      if (video.paused && mode === 'live') {
        video.play().catch(err => {
          console.warn('[UnifiedMSEPlayer] Autoplay failed:', err)
        })
      }
    }
    
    const handleError = (e: Event) => {
      console.error('[UnifiedMSEPlayer] Video error:', e)
      const error = video.error
      if (error) {
        const errorMsg = `视频错误: ${error.message} (code: ${error.code})`
        setStatus(errorMsg)
        onError?.(errorMsg)
      }
    }
    
    video.addEventListener('waiting', handleWaiting)
    video.addEventListener('playing', handlePlaying)
    video.addEventListener('canplay', handleCanPlay)
    video.addEventListener('error', handleError)
    
    return () => {
      video.removeEventListener('waiting', handleWaiting)
      video.removeEventListener('playing', handlePlaying)
      video.removeEventListener('canplay', handleCanPlay)
      video.removeEventListener('error', handleError)
    }
  }, [mode, onError])

  // 清理资源
  const cleanup = () => {
    console.log('[UnifiedMSEPlayer] Cleaning up resources')
    
    // 关闭 EventSource（如果还未关闭）
    if (eventSourceRef.current) {
      try {
        eventSourceRef.current.close()
        console.log('[UnifiedMSEPlayer] EventSource closed')
      } catch (error) {
        console.warn('[UnifiedMSEPlayer] Error closing EventSource:', error)
      }
      eventSourceRef.current = null
    }
    
    // 清空分片队列
    segmentQueueRef.current = []
    isAppendingRef.current = false
    
    // 清理 SourceBuffer
    if (sourceBufferRef.current) {
      try {
        // 如果正在更新，等待完成
        if (!sourceBufferRef.current.updating) {
          // 尝试中止任何待处理的操作
          sourceBufferRef.current.abort()
        }
      } catch (error) {
        console.warn('[UnifiedMSEPlayer] Error aborting SourceBuffer:', error)
      }
      sourceBufferRef.current = null
    }
    
    // 清理 MediaSource
    if (mediaSourceRef.current) {
      if (mediaSourceRef.current.readyState === 'open') {
        try {
          mediaSourceRef.current.endOfStream()
          console.log('[UnifiedMSEPlayer] MediaSource ended')
        } catch (error) {
          console.warn('[UnifiedMSEPlayer] Error ending stream:', error)
        }
      }
      mediaSourceRef.current = null
    }
    
    // 清理 video 元素
    if (videoRef.current) {
      videoRef.current.pause()
      videoRef.current.removeAttribute('src')
      videoRef.current.load()
      console.log('[UnifiedMSEPlayer] Video element cleaned')
    }
  }

  // 更新缓冲统计
  const updateBufferStats = () => {
    const video = videoRef.current
    if (!video) return
    
    try {
      const buffered = video.buffered
      if (buffered.length > 0) {
        const currentTime = video.currentTime
        const bufferedEnd = buffered.end(buffered.length - 1)
        const currentBuffer = bufferedEnd - currentTime
        
        setStats(prev => ({
          ...prev,
          currentBuffer: Math.max(0, currentBuffer),
        }))
      }
    } catch (error) {
      console.warn('[UnifiedMSEPlayer] Error updating buffer stats:', error)
    }
  }

  // 定期更新缓冲统计和智能缓冲管理
  useEffect(() => {
    const interval = setInterval(() => {
      updateBufferStats()
      intelligentBufferManagement()
    }, 1000)
    return () => clearInterval(interval)
  }, [])

  // 智能缓冲管理
  const intelligentBufferManagement = () => {
    const video = videoRef.current
    if (!video || !isReady) return
    
    try {
      const buffered = video.buffered
      if (buffered.length === 0) return
      
      const currentTime = video.currentTime
      const bufferedEnd = buffered.end(buffered.length - 1)
      const bufferedAmount = bufferedEnd - currentTime
      
      // 根据缓冲量调整播放状态
      if (bufferedAmount < bufferConfig.minBuffer) {
        // 缓冲不足，暂停播放
        if (!video.paused && video.readyState < 3) {
          console.log('[UnifiedMSEPlayer] Buffer underrun, pausing', {
            bufferedAmount,
            minBuffer: bufferConfig.minBuffer,
          })
          setStatus('缓冲不足，等待数据...')
        }
      } else if (bufferedAmount >= bufferConfig.targetBuffer) {
        // 缓冲充足，可以播放
        if (video.paused && video.readyState >= 3) {
          console.log('[UnifiedMSEPlayer] Buffer sufficient, resuming', {
            bufferedAmount,
            targetBuffer: bufferConfig.targetBuffer,
          })
          
          video.play().catch(err => {
            console.warn('[UnifiedMSEPlayer] Resume play failed:', err)
          })
          
          setStatus('播放中')
        }
      }
      
      // 对于直通播放，保持低延迟
      if (mode === 'live' && bufferedAmount > bufferConfig.maxBuffer) {
        // 跳到最新位置
        const newTime = bufferedEnd - bufferConfig.targetBuffer
        if (newTime > currentTime) {
          console.log('[UnifiedMSEPlayer] Jumping to live edge', {
            from: currentTime,
            to: newTime,
            bufferedAmount,
          })
          video.currentTime = newTime
        }
      }
      
    } catch (error) {
      console.warn('[UnifiedMSEPlayer] Error in intelligent buffer management:', error)
    }
  }

  // 动态调整缓冲目标（根据网络条件）
  const adjustBufferTarget = (networkQuality: 'good' | 'medium' | 'poor') => {
    const baseConfig = mode === 'live' 
      ? { target: 0.3, min: 0.1, max: 0.5 }
      : { target: 1.0, min: 0.5, max: 2.0 }
    
    let multiplier = 1.0
    
    switch (networkQuality) {
      case 'poor':
        multiplier = 1.5 // 增加缓冲
        break
      case 'medium':
        multiplier = 1.2
        break
      case 'good':
        multiplier = 1.0
        break
    }
    
    setBufferConfig({
      targetBuffer: baseConfig.target * multiplier,
      minBuffer: baseConfig.min * multiplier,
      maxBuffer: baseConfig.max * multiplier,
    })
    
    console.log('[UnifiedMSEPlayer] Buffer config adjusted', {
      networkQuality,
      multiplier,
      newConfig: bufferConfig,
    })
  }

  // 播放控制 API 调用
  const sendControlCommand = async (command: string, params?: any) => {
    if (!controlUrl) {
      console.warn('[UnifiedMSEPlayer] No control URL provided')
      return
    }
    
    try {
      const response = await fetch(controlUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          command,
          ...params,
        }),
      })
      
      if (!response.ok) {
        throw new Error(`Control command failed: ${response.statusText}`)
      }
      
      const result = await response.json()
      console.log('[UnifiedMSEPlayer] Control command result:', result)
      return result
      
    } catch (error) {
      console.error('[UnifiedMSEPlayer] Control command error:', error)
      onError?.(`控制命令失败: ${error}`)
    }
  }

  // 暂停播放
  const handlePause = async () => {
    const video = videoRef.current
    if (!video) return
    
    video.pause()
    setIsPaused(true)
    
    // 通知服务器暂停
    await sendControlCommand('pause')
  }

  // 恢复播放
  const handleResume = async () => {
    const video = videoRef.current
    if (!video) return
    
    video.play().catch(err => {
      console.error('[UnifiedMSEPlayer] Resume failed:', err)
    })
    setIsPaused(false)
    
    // 通知服务器恢复
    await sendControlCommand('resume')
  }

  // 定位到指定时间（仅回放模式）
  const handleSeek = async (position: number) => {
    if (mode === 'live') {
      console.warn('[UnifiedMSEPlayer] Seek not supported in live mode')
      return
    }
    
    const video = videoRef.current
    if (!video) return
    
    setStatus('定位中...')
    
    // 通知服务器定位
    await sendControlCommand('seek', { position })
    
    // 清空当前缓冲
    segmentQueueRef.current = []
    
    // 更新视频时间
    video.currentTime = position
    setCurrentTime(position)
  }

  // 设置播放速率（仅回放模式）
  const handleSetRate = async (rate: number) => {
    if (mode === 'live') {
      console.warn('[UnifiedMSEPlayer] Playback rate not supported in live mode')
      return
    }
    
    const video = videoRef.current
    if (!video) return
    
    // 通知服务器调整速率
    await sendControlCommand('set_rate', { rate })
    
    // 更新本地播放速率
    video.playbackRate = rate
    setPlaybackRate(rate)
  }

  // 停止播放
  const handleStop = async () => {
    console.log('[UnifiedMSEPlayer] Stopping playback')
    
    // 立即关闭 SSE 连接，停止接收数据
    if (eventSourceRef.current) {
      console.log('[UnifiedMSEPlayer] Closing SSE connection')
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }
    
    // 清空分片队列
    segmentQueueRef.current = []
    
    // 暂停视频播放
    if (videoRef.current) {
      videoRef.current.pause()
    }
    
    // 通知服务器停止（不等待响应）
    sendControlCommand('stop').catch(err => {
      console.warn('[UnifiedMSEPlayer] Failed to send stop command:', err)
    })
    
    // 清理所有资源
    cleanup()
    
    setStatus('已停止')
  }

  // 监听视频时间更新
  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    
    const handleTimeUpdate = () => {
      setCurrentTime(video.currentTime)
    }
    
    const handleDurationChange = () => {
      setDuration(video.duration)
    }
    
    video.addEventListener('timeupdate', handleTimeUpdate)
    video.addEventListener('durationchange', handleDurationChange)
    
    return () => {
      video.removeEventListener('timeupdate', handleTimeUpdate)
      video.removeEventListener('durationchange', handleDurationChange)
    }
  }, [])

  // 格式化时间显示
  const formatTime = (seconds: number): string => {
    if (!isFinite(seconds)) return '00:00'
    
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }

  // 更新状态通知
  useEffect(() => {
    onStatusChange?.(status)
  }, [status, onStatusChange])

  return (
    <div className="unified-mse-player">
      <div className="player-container">
        <video
          ref={videoRef}
          className="video-element"
          controls
          playsInline
          autoPlay={mode === 'live'}
        >
          您的浏览器不支持视频播放
        </video>
        
        {!isReady && (
          <div className="player-overlay">
            <div className="status-info">
              <div className="spinner"></div>
              <p className="status">{status}</p>
            </div>
          </div>
        )}
      </div>

      {/* 播放控制栏 */}
      <div className="player-controls">
        <div className="controls-left">
          {/* 播放/暂停按钮 */}
          <button
            className="control-btn"
            onClick={isPaused ? handleResume : handlePause}
            title={isPaused ? '播放' : '暂停'}
          >
            {isPaused ? '▶️' : '⏸️'}
          </button>
          
          {/* 停止按钮 */}
          <button
            className="control-btn"
            onClick={handleStop}
            title="停止"
          >
            ⏹️
          </button>
          
          {/* 时间显示 */}
          <span className="time-display">
            {formatTime(currentTime)} / {formatTime(duration)}
          </span>
        </div>

        {/* 进度条（仅回放模式） */}
        {mode === 'playback' && (
          <div className="progress-container">
            <input
              type="range"
              className="progress-bar"
              min="0"
              max={duration || 100}
              value={currentTime}
              onChange={(e) => handleSeek(parseFloat(e.target.value))}
              disabled={!isReady}
            />
          </div>
        )}

        {/* 倍速选择器（仅回放模式） */}
        {mode === 'playback' && (
          <div className="controls-right">
            <label className="rate-label">倍速:</label>
            <select
              className="rate-selector"
              value={playbackRate}
              onChange={(e) => handleSetRate(parseFloat(e.target.value))}
              disabled={!isReady}
            >
              <option value="0.25">0.25x</option>
              <option value="0.5">0.5x</option>
              <option value="0.75">0.75x</option>
              <option value="1.0">1.0x</option>
              <option value="1.25">1.25x</option>
              <option value="1.5">1.5x</option>
              <option value="2.0">2.0x</option>
              <option value="4.0">4.0x</option>
            </select>
          </div>
        )}
      </div>

      <div className="player-info">
        <div className="info-header">
          <h3>
            {mode === 'live' ? '📡 直通播放' : '📼 录像回放'}
          </h3>
          <span className="session-id">会话: {sessionId.substring(0, 8)}...</span>
        </div>
        
        <div className="info-grid">
          <div className="info-item">
            <span className="label">模式:</span>
            <span className="value">{mode === 'live' ? '实时流' : '录像回放'}</span>
          </div>
          <div className="info-item">
            <span className="label">状态:</span>
            <span className={`value status-${isReady ? 'ready' : 'loading'}`}>
              {isReady ? '就绪' : '初始化'}
            </span>
          </div>
          <div className="info-item">
            <span className="label">目标缓冲:</span>
            <span className="value">{(bufferConfig.targetBuffer * 1000).toFixed(0)}ms</span>
          </div>
          <div className="info-item">
            <span className="label">当前缓冲:</span>
            <span className="value">{(stats.currentBuffer * 1000).toFixed(0)}ms</span>
          </div>
        </div>

        <div className="stats-grid">
          <div className="stat-item">
            <span className="stat-label">接收分片:</span>
            <span className="stat-value">{stats.segmentsReceived}</span>
          </div>
          <div className="stat-item">
            <span className="stat-label">接收数据:</span>
            <span className="stat-value">
              {(stats.bytesReceived / 1024 / 1024).toFixed(2)} MB
            </span>
          </div>
          <div className="stat-item">
            <span className="stat-label">丢帧:</span>
            <span className="stat-value">{stats.droppedFrames}</span>
          </div>
        </div>
      </div>
    </div>
  )
}

export default UnifiedMSEPlayer
