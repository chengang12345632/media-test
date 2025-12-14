import React, { useEffect, useRef, useState } from 'react'
import LatencyMonitor from './LatencyMonitor'
import { FrameScheduler } from '../utils/frameScheduler'

interface WebCodecsPlayerProps {
  sessionId: string
}

/**
 * 使用 WebCodecs API 的 H.264 播放器
 * 支持浏览器原生 H.264 解码，低延迟高性能
 */
function WebCodecsPlayer({ sessionId }: WebCodecsPlayerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [status, setStatus] = useState<string>('初始化中...')
  const [error, setError] = useState<string | null>(null)
  const [segmentCount, setSegmentCount] = useState<number>(0)
  const [fps, setFps] = useState<number>(0)
  const [targetFps, setTargetFps] = useState<number>(30) // 默认30fps
  const [droppedFrames, setDroppedFrames] = useState<number>(0)
  const [averageDelay, setAverageDelay] = useState<number>(0)
  const decoderRef = useRef<VideoDecoder | null>(null)
  const eventSourceRef = useRef<EventSource | null>(null)
  const frameSchedulerRef = useRef<FrameScheduler | null>(null)
  const frameCountRef = useRef<number>(0)
  const lastFpsUpdateRef = useRef<number>(Date.now())
  const isConfiguredRef = useRef<boolean>(false)
  const pendingChunksRef = useRef<{ data: Uint8Array, timestamp: number }[]>([])

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
      // 创建 FrameScheduler（默认30fps，后续可从服务器获取）
      const scheduler = new FrameScheduler(targetFps)
      frameSchedulerRef.current = scheduler

      // 设置帧显示回调
      scheduler.setDisplayCallback((frame: VideoFrame) => {
        displayFrame(frame, canvas, ctx)
      })

      // 创建 VideoDecoder
      const decoder = new VideoDecoder({
        output: (frame: VideoFrame) => {
          // 将帧交给调度器处理，而不是立即显示
          try {
            const pts = frame.timestamp || 0 // 使用帧的时间戳
            scheduler.addFrame(frame, pts)

            // 更新统计信息
            const stats = scheduler.getStats()
            setDroppedFrames(stats.droppedFrames)
            setAverageDelay(stats.averageDelay)
          } catch (err) {
            console.error('Failed to schedule frame:', err)
            frame.close()
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
   * 显示帧到 canvas（由 FrameScheduler 调用）
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
        setFps(frameCountRef.current)
        frameCountRef.current = 0
        lastFpsUpdateRef.current = now
      }
    } catch (err) {
      console.error('Failed to render frame:', err)
    }
  }

  const startSSEStream = () => {
    setStatus('连接到服务器...')
    
    const streamUrl = `/api/v1/stream/${sessionId}/segments`
    console.log('Connecting to SSE stream:', streamUrl)
    const eventSource = new EventSource(streamUrl)
    eventSourceRef.current = eventSource
    
    let count = 0
    let hasReceivedSPS = false
    let timestamp = 0

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
        
        // 调试：打印前几个分片的信息
        if (count <= 3) {
          const firstBytes = Array.from(h264Data.slice(0, 16)).map(b => b.toString(16).padStart(2, '0')).join(' ')
          console.log(`Segment #${count}: ${h264Data.length} bytes, first 16: ${firstBytes}`)
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
          pendingChunksRef.current.push({ data: h264Data, timestamp })
          timestamp += 33333
          return
        }
        
        setSegmentCount(count)
        setStatus(`正在播放... ${count} 个分片`)
        
        // 解码 H.264 数据
        decodeH264Data(h264Data, timestamp)
        timestamp += 33333 // 假设 30fps，每帧约 33ms
        
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
    
    if (decoderRef.current) {
      try {
        decoderRef.current.close()
      } catch (e) {
        // ignore
      }
      decoderRef.current = null
    }

    if (frameSchedulerRef.current) {
      frameSchedulerRef.current.destroy()
      frameSchedulerRef.current = null
    }
    
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
        <h3>🚀 WebCodecs 实时播放</h3>
        <div className="info-row">
          <span className="label">会话 ID:</span>
          <span className="value">{sessionId.substring(0, 8)}...</span>
        </div>
        <div className="info-row">
          <span className="label">接收分片:</span>
          <span className="value">{segmentCount}</span>
        </div>
        
        {/* 帧率统计 */}
        <div className="info-section">
          <h4 style={{ margin: '10px 0 5px 0', fontSize: '14px', color: '#666' }}>📊 帧率统计</h4>
          <div className="info-row">
            <span className="label">目标 FPS:</span>
            <span className="value">{targetFps}</span>
          </div>
          <div className="info-row">
            <span className="label">实际 FPS:</span>
            <span className="value" style={{ 
              color: Math.abs(fps - targetFps) / targetFps > 0.05 ? '#ff6b6b' : '#51cf66' 
            }}>
              {fps}
            </span>
          </div>
          <div className="info-row">
            <span className="label">速度误差:</span>
            <span className="value" style={{ 
              color: Math.abs(fps - targetFps) / targetFps > 0.05 ? '#ff6b6b' : '#51cf66' 
            }}>
              {targetFps > 0 ? ((fps - targetFps) / targetFps * 100).toFixed(1) : '0.0'}%
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
          {Math.abs(fps - targetFps) / targetFps > 0.05 && fps > 0 && (
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
