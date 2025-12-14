import React, { useEffect, useRef, useState } from 'react'
import LatencyMonitor from './LatencyMonitor'

interface WebCodecsPlayerProps {
  sessionId: string
  playbackMode?: 'fast' | 'normal' // 播放模式
}

/**
 * 使用 WebCodecs API 的 H.264 播放器
 * 支持浏览器原生 H.264 解码，低延迟高性能
 * 
 * 播放模式说明：
 * - fast: 快速模式，解码后立即渲染，最低延迟（<100ms）
 * - normal: 正常模式，基于 FPS 和时间戳双重控制播放速度，保证流畅稳定
 */
function WebCodecsPlayer({ sessionId, playbackMode = 'normal' }: WebCodecsPlayerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [status, setStatus] = useState<string>('初始化中...')
  const [error, setError] = useState<string | null>(null)
  const [segmentCount, setSegmentCount] = useState<number>(0)
  const [fps, setFps] = useState<number>(0)
  const targetFpsRef = useRef<number>(30) // 使用 ref 存储目标帧率，避免重新渲染
  const [droppedFrames, setDroppedFrames] = useState<number>(0)
  const [averageDelay, setAverageDelay] = useState<number>(0)

  const decoderRef = useRef<VideoDecoder | null>(null)
  const eventSourceRef = useRef<EventSource | null>(null)

  const frameCountRef = useRef<number>(0)
  const lastFpsUpdateRef = useRef<number>(Date.now())
  const isConfiguredRef = useRef<boolean>(false)
  const pendingChunksRef = useRef<{ data: Uint8Array, timestamp: number }[]>([])
  const pendingFramesRef = useRef<VideoFrame[]>([]) // 用于 normal 模式的帧队列
  const renderTimerRef = useRef<number | null>(null) // 用于调度渲染
  
  // 播放时钟基准（类似抖音的实现）
  const playbackStartTimeRef = useRef<number>(0) // 播放开始的系统时间（毫秒）
  const playbackStartTimestampRef = useRef<number>(0) // 播放开始的视频时间戳（毫秒）

  useEffect(() => {
    console.log('WebCodecsPlayer mounted', { sessionId })
    
    // 检查浏览器支持
    if (!('VideoDecoder' in window)) {
      setError('浏览器不支持 WebCodecs API (需要 Chrome 94+ 或 Edge 94+)')
      return
    }

    initializePlayer()

    return () => {
      cleanup()
    }
  }, [sessionId])

  const initializePlayer = async () => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) {
      setError('无法获取 Canvas 上下文')
      return
    }

    console.log('Initializing WebCodecs player')
    
    try {
      // 创建 VideoDecoder
      const decoder = new VideoDecoder({
        output: (frame: VideoFrame) => {
          if (playbackMode === 'fast') {
            // ⚡ Fast模式：解码后立即渲染，最低延迟
            try {
              displayFrame(frame, canvas, ctx)
              frame.close()
            } catch (err) {
              console.error('Failed to render frame:', err)
              frame.close()
            }
          } else {
            // 🎬 Normal模式：基于播放时钟控制播放速度
            pendingFramesRef.current.push(frame)
            scheduleNextFrame()
          }
        },
        error: (err: Error) => {
          console.error('Decoder error:', err)
          setError(`解码错误: ${err.message}`)
        }
      })

      decoderRef.current = decoder
      console.log('✅ VideoDecoder created (waiting for SPS/PPS to configure)')

      // 开始接收 SSE 数据
      startSSEStream()
      
    } catch (err) {
      console.error('Failed to initialize decoder:', err)
      setError('解码器初始化失败: ' + err)
    }
  }

  /**
   * 显示帧到 canvas
   */
  const displayFrame = (frame: VideoFrame, canvas: HTMLCanvasElement, ctx: CanvasRenderingContext2D) => {
    try {
      // 调整 canvas 大小以匹配视频
      if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
        canvas.width = frame.displayWidth
        canvas.height = frame.displayHeight
        console.log(`Canvas resized to ${canvas.width}x${canvas.height}`)
      }

      ctx.drawImage(frame, 0, 0)

      // 更新 FPS
      frameCountRef.current++
      const now = Date.now()
      if (now - lastFpsUpdateRef.current >= 1000) {
        const currentFps = frameCountRef.current
        setFps(currentFps)
        
        frameCountRef.current = 0
        lastFpsUpdateRef.current = now
      }
    } catch (err) {
      console.error('Failed to render frame:', err)
    }
  }

  /**
   * 调度下一帧渲染（用于 normal 模式）
   * 
   * 策略：
   * 1. 使用播放时钟算法，严格按照时间戳播放
   * 2. 如果缓冲区堆积过多（数据推送太快），只保留最近的帧
   * 3. 通过丢弃旧帧来适应快速推送的数据流
   */
  const scheduleNextFrame = () => {
    if (renderTimerRef.current !== null) return // 已经有定时器在运行
    if (pendingFramesRef.current.length === 0) return // 没有待渲染的帧
    
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    
    // ========== 关键策略：控制缓冲区大小 ==========
    // 如果缓冲区堆积过多，说明数据推送速度 > 播放速度
    // 解决方案：丢弃旧帧，跳到最新的位置
    const maxBufferSize = 10 // 最多保留10帧（约333ms @ 30fps）
    
    if (pendingFramesRef.current.length > maxBufferSize) {
      // 计算需要丢弃的帧数
      const framesToDrop = pendingFramesRef.current.length - maxBufferSize
      
      console.warn(`⚠️ Buffer overflow: ${pendingFramesRef.current.length} frames, dropping ${framesToDrop} old frames`)
      
      // 丢弃旧帧
      for (let i = 0; i < framesToDrop; i++) {
        const frame = pendingFramesRef.current.shift()
        if (frame) {
          frame.close()
          setDroppedFrames(prev => prev + 1)
        }
      }
      
      // 重置播放时钟，从当前位置重新开始
      playbackStartTimeRef.current = 0
      playbackStartTimestampRef.current = 0
      console.log(`🔄 Playback clock reset due to buffer overflow`)
    }
    
    const frame = pendingFramesRef.current[0]
    if (!frame) return
    
    const now = performance.now() // 当前系统时间（毫秒）
    const currentFrameTimestamp = frame.timestamp / 1000 // 当前帧时间戳（微秒转毫秒）
    
    // 初始化播放时钟基准（第一帧）
    if (playbackStartTimeRef.current === 0) {
      playbackStartTimeRef.current = now
      playbackStartTimestampRef.current = currentFrameTimestamp
      console.log(`🎬 Playback clock initialized: system=${now.toFixed(1)}ms, frame=${currentFrameTimestamp.toFixed(1)}ms`)
      
      // 第一帧立即播放
      renderTimerRef.current = window.setTimeout(() => {
        renderTimerRef.current = null
        const frame = pendingFramesRef.current.shift()
        if (frame) {
          displayFrame(frame, canvas, ctx)
          frame.close()
          if (pendingFramesRef.current.length > 0) {
            scheduleNextFrame()
          }
        }
      }, 0)
      return
    }
    
    // ========== 播放时钟算法 ==========
    // 计算当前帧相对于开始帧的时间偏移
    const frameTimeOffset = currentFrameTimestamp - playbackStartTimestampRef.current
    
    // 计算当前帧应该播放的系统时间
    const targetPlayTime = playbackStartTimeRef.current + frameTimeOffset
    
    // 计算需要等待的时间
    let waitTime = targetPlayTime - now
    
    // 如果等待时间为负数，说明帧已经"迟到"，立即播放
    if (waitTime < 0) {
      waitTime = 0
    }
    
    // 调试日志（前30帧）
    if (frameCountRef.current < 30) {
      console.log(`📊 Frame #${frameCountRef.current}:`)
      console.log(`   - Buffer size: ${pendingFramesRef.current.length} frames`)
      console.log(`   - Frame timestamp: ${currentFrameTimestamp.toFixed(1)}ms`)
      console.log(`   - Frame offset: ${frameTimeOffset.toFixed(1)}ms`)
      console.log(`   - Target play time: ${targetPlayTime.toFixed(1)}ms`)
      console.log(`   - Current time: ${now.toFixed(1)}ms`)
      console.log(`   - Wait time: ${waitTime.toFixed(1)}ms`)
    }
    
    // 限制最大等待时间，防止异常时间戳
    if (waitTime > 5000) {
      console.warn(`⚠️ Abnormal wait time (${waitTime.toFixed(0)}ms), resetting playback clock`)
      playbackStartTimeRef.current = now
      playbackStartTimestampRef.current = currentFrameTimestamp
      waitTime = 0
    }
    
    renderTimerRef.current = window.setTimeout(() => {
      renderTimerRef.current = null
      
      const frame = pendingFramesRef.current.shift()
      if (frame) {
        displayFrame(frame, canvas, ctx)
        frame.close()
        
        // 如果还有待渲染的帧，继续调度
        if (pendingFramesRef.current.length > 0) {
          scheduleNextFrame()
        }
      }
    }, waitTime)
  }

  const startSSEStream = () => {
    setStatus('连接到服务器...')
    
    const streamUrl = `/api/v1/stream/${sessionId}/segments`
    console.log('Connecting to SSE stream:', streamUrl)
    const eventSource = new EventSource(streamUrl)
    eventSourceRef.current = eventSource
    
    let count = 0
    let hasReceivedSPS = false

    eventSource.onopen = () => {
      console.log('SSE connection opened')
      setStatus('已连接，接收视频数据...')
    }

    eventSource.onmessage = (event) => {
      try {
        const segment = JSON.parse(event.data)
        count++
        
        // 将 base64 数据转换为 Uint8Array
        const h264Data = Uint8Array.from(atob(segment.data), c => c.charCodeAt(0))
        
        // 🔧 使用服务端发送的真实时间戳（秒转微秒）
        const realTimestamp = segment.timestamp * 1000000 // 秒转微秒
        
        // 调试：打印前几个分片的信息
        if (count <= 5) {
          const firstBytes = Array.from(h264Data.slice(0, 16)).map(b => b.toString(16).padStart(2, '0')).join(' ')
          console.log(`📦 Segment #${count}:`)
          console.log(`   - Size: ${h264Data.length} bytes`)
          console.log(`   - Timestamp (from server): ${segment.timestamp.toFixed(3)}s`)
          console.log(`   - Timestamp (converted): ${realTimestamp}μs = ${(realTimestamp / 1000).toFixed(1)}ms`)
          console.log(`   - First 16 bytes: ${firstBytes}`)
        }
        
        // 检查是否包含SPS (NAL type 7)
        const hasSPS = checkForSPS(h264Data)
        if (hasSPS && !hasReceivedSPS) {
          hasReceivedSPS = true
          console.log('✅ Received SPS/PPS! Configuring decoder (Annex B mode)...')
          
          // 简单配置解码器，不使用 description
          // 让解码器从数据流中读取 SPS/PPS
          configureDecoderSimple()
        }
        
        // 如果解码器还没配置好，缓存数据
        if (!isConfiguredRef.current) {
          console.log(`⏭️ Buffering segment #${count} (waiting for decoder configuration)`)
          pendingChunksRef.current.push({ data: h264Data, timestamp: realTimestamp })
          return
        }
        
        setSegmentCount(count)
        setStatus(`正在播放... ${count} 个分片`)
        
        // 解码 H.264 数据，使用真实时间戳
        decodeH264Data(h264Data, realTimestamp)
        
      } catch (err) {
        console.error('Error processing segment:', err)
      }
    }

    eventSource.onerror = (err) => {
      console.error('SSE error:', err)
      eventSource.close()
      setStatus(`连接断开，共接收 ${count} 个分片`)
    }
  }

  const checkForSPS = (data: Uint8Array): boolean => {
    for (let i = 0; i < data.length - 4; i++) {
      // 查找起始码 + SPS (NAL type 7)
      if ((data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x00 && data[i+3] === 0x01 && (data[i+4] & 0x1F) === 7) ||
          (data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x01 && (data[i+3] & 0x1F) === 7)) {
        return true
      }
    }
    return false
  }

  const checkForKeyFrame = (data: Uint8Array): boolean => {
    for (let i = 0; i < data.length - 4; i++) {
      // 查找起始码 + IDR (NAL type 5)
      if ((data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x00 && data[i+3] === 0x01 && (data[i+4] & 0x1F) === 5) ||
          (data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x01 && (data[i+3] & 0x1F) === 5)) {
        return true
      }
    }
    return false
  }

  const configureDecoderSimple = () => {
    const decoder = decoderRef.current
    if (!decoder) return
    
    try {
      // 简单配置：不使用 description
      // WebCodecs 会从第一个 key chunk 中读取 SPS/PPS
      decoder.configure({
        codec: 'avc1.42E01E', // H.264 Baseline Profile Level 3.0
        optimizeForLatency: true
      })
      
      isConfiguredRef.current = true
      console.log('✅ VideoDecoder configured (Annex B mode, in-band SPS/PPS)')
      
      // 处理缓存的数据（第一个包含 SPS/PPS/IDR）
      if (pendingChunksRef.current.length > 0) {
        console.log(`📤 Processing ${pendingChunksRef.current.length} buffered chunks`)
        for (const chunk of pendingChunksRef.current) {
          decodeH264Data(chunk.data, chunk.timestamp)
        }
        pendingChunksRef.current = []
      }
    } catch (err) {
      console.error('Failed to configure decoder:', err)
      setError('解码器配置失败: ' + err)
    }
  }

  const decodeH264Data = (data: Uint8Array, timestamp: number) => {
    const decoder = decoderRef.current
    if (!decoder || decoder.state !== 'configured') {
      console.warn('Decoder not ready, state:', decoder?.state)
      return
    }

    try {
      // 检查是否包含关键帧（IDR 或 SPS/PPS）
      const isKeyFrame = checkForKeyFrame(data) || checkForSPS(data)
      
      // 创建 EncodedVideoChunk
      const chunk = new EncodedVideoChunk({
        type: isKeyFrame ? 'key' : 'delta',
        timestamp,
        data: data.buffer
      })

      decoder.decode(chunk)
    } catch (err) {
      console.error('Failed to decode chunk:', err)
    }
  }

  const cleanup = () => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close()
      eventSourceRef.current = null
    }
    
    // 清理渲染定时器
    if (renderTimerRef.current !== null) {
      clearTimeout(renderTimerRef.current)
      renderTimerRef.current = null
    }
    
    // 清理待渲染的帧
    pendingFramesRef.current.forEach(frame => frame.close())
    pendingFramesRef.current = []
    
    if (decoderRef.current) {
      try {
        decoderRef.current.close()
      } catch (e) {
        // ignore
      }
      decoderRef.current = null
    }

    
    // 重置播放时钟
    playbackStartTimeRef.current = 0
    playbackStartTimestampRef.current = 0
    
    isConfiguredRef.current = false
    pendingChunksRef.current = []
  }

  return (
    <div className="webcodecs-player">
      <div className="player-container">
        <canvas
          ref={canvasRef}
          className="video-canvas"
          style={{
            width: '100%',
            height: 'auto',
            backgroundColor: '#000',
            maxHeight: '70vh'
          }}
        />
        
        {(status || error) && (
          <div className="player-overlay">
            <div className="status-info">
              <p className="status">{status}</p>
              {error && <p className="error">{error}</p>}
            </div>
          </div>
        )}
      </div>

      {/* 延迟监控组件 */}
      <LatencyMonitor sessionId={sessionId} apiBaseUrl="http://localhost:8080" />

      <div className="player-info">
        <h3>
          {playbackMode === 'fast' && '⚡ Fast Mode'}
          {playbackMode === 'normal' && '🎬 Normal Mode'}
          {' - WebCodecs 实时播放'}
        </h3>
        <div className="info-row">
          <span className="label">播放模式:</span>
          <span className="value">
            {playbackMode === 'fast' && '快速模式（立即渲染）'}
            {playbackMode === 'normal' && '正常模式（FPS + 时间戳双重控制）'}
          </span>
        </div>
        <div className="info-row">
          <span className="label">会话 ID:</span>
          <span className="value">{sessionId.substring(0, 8)}...</span>
        </div>
        <div className="info-row">
          <span className="label">接收分片:</span>
          <span className="value">{segmentCount}</span>
        </div>
        {playbackMode === 'normal' && (
          <div className="info-row">
            <span className="label">缓冲帧数:</span>
            <span className="value" style={{
              color: pendingFramesRef.current.length > 10 ? '#ff6b6b' : '#51cf66'
            }}>
              {pendingFramesRef.current.length}
            </span>
          </div>
        )}
        
        {/* 帧率统计 */}
        <div className="info-section">
          <h4 style={{ margin: '10px 0 5px 0', fontSize: '14px', color: '#666' }}>📊 帧率统计</h4>
          <div className="info-row">
            <span className="label">目标 FPS:</span>
            <span className="value">{targetFpsRef.current}</span>
          </div>
          <div className="info-row">
            <span className="label">实际 FPS:</span>
            <span className="value" style={{ 
              color: Math.abs(fps - targetFpsRef.current) / targetFpsRef.current > 0.05 ? '#ff6b6b' : '#51cf66' 
            }}>
              {fps}
            </span>
          </div>
          <div className="info-row">
            <span className="label">速度误差:</span>
            <span className="value" style={{ 
              color: Math.abs(fps - targetFpsRef.current) / targetFpsRef.current > 0.05 ? '#ff6b6b' : '#51cf66' 
            }}>
              {targetFpsRef.current > 0 ? ((fps - targetFpsRef.current) / targetFpsRef.current * 100).toFixed(1) : '0.0'}%
            </span>
          </div>
          <div className="info-row">
            <span className="label">丢帧数:</span>
            <span className="value" style={{ color: droppedFrames > 0 ? '#ff6b6b' : '#51cf66' }}>
              {droppedFrames}
            </span>
          </div>
          <div className="info-row">
            <span className="label">平均延迟:</span>
            <span className="value" style={{ 
              color: averageDelay > 16 ? '#ff6b6b' : '#51cf66' 
            }}>
              {averageDelay.toFixed(1)}ms
            </span>
          </div>
        </div>
        
        <div className="info-row">
          <span className="label">解码方式:</span>
          <span className="value">🎯 WebCodecs API (硬件加速)</span>
        </div>
        
        <div className="hint-box">
          <p className="hint success">
            ✅ 使用浏览器原生 H.264 解码器
          </p>
          <p className="hint info">
            💡 超低延迟，硬件加速
          </p>
          
          {/* 模式特定提示 */}
          {playbackMode === 'fast' && (
            <p className="hint info" style={{ color: '#1890ff' }}>
              ⚡ Fast 模式：解码后立即渲染，延迟最低（&lt;100ms）
            </p>
          )}
          {playbackMode === 'normal' && (
            <p className="hint info" style={{ color: '#52c41a' }}>
              🎬 Normal 模式：FPS + 时间戳双重控制，保证流畅稳定
            </p>
          )}
          
          {Math.abs(fps - targetFpsRef.current) / targetFpsRef.current > 0.05 && fps > 0 && playbackMode === 'normal' && (
            <p className="hint warning" style={{ color: '#ff922b' }}>
              ⚠️ 播放速度偏差超过 5%
            </p>
          )}
          {droppedFrames > 10 && (
            <p className="hint warning" style={{ color: '#ff922b' }}>
              ⚠️ 丢帧较多，可能影响播放流畅度
            </p>
          )}
        </div>
      </div>
    </div>
  )
}

export default WebCodecsPlayer
