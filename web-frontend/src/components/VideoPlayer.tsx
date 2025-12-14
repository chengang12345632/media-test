import React, { useEffect, useRef, useState } from 'react'
import { apiClient } from '../services/api'
import H264Player from './H264Player'
import WebCodecsPlayer from './WebCodecsPlayer'
import './VideoPlayer.css'

interface VideoPlayerProps {
  sessionId: string
  fileId?: string
  isLiveMode?: boolean
}

function VideoPlayer({ sessionId, fileId, isLiveMode = false }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [status, setStatus] = useState<string>('初始化中...')
  const [error, setError] = useState<string | null>(null)
  // 如果是live模式，直接初始化为sse模式
  const [playbackMode, setPlaybackMode] = useState<'direct' | 'sse'>(isLiveMode ? 'sse' : 'direct')
  const [fileInfo, setFileInfo] = useState<any>(null)

  useEffect(() => {
    console.log('VideoPlayer mounted', { sessionId, fileId, isLiveMode })
    
    // 直通播放模式 - 不在这里处理，由H264Player处理
    if (isLiveMode) {
      // playbackMode已经在useState中设置为'sse'
      // 不需要在这里做任何事情，组件会返回H264Player
      return
    }
    
    // 检测文件类型，决定播放模式
    if (fileId) {
      detectPlaybackMode(fileId)
    } else {
      // 默认使用 SSE 模式
      setPlaybackMode('sse')
      startSSEPlayback()
    }

    return () => {
      // 清理资源
      if (videoRef.current) {
        videoRef.current.pause()
        videoRef.current.src = ''
      }
    }
  }, [sessionId, fileId, isLiveMode])

  const detectPlaybackMode = async (fileId: string) => {
    // 检查文件扩展名
    const lowerFileId = fileId.toLowerCase()
    
    if (lowerFileId.endsWith('.mp4') || lowerFileId.includes('.mp4')) {
      console.log('Detected MP4 file, using direct playback')
      setPlaybackMode('direct')
      startDirectPlayback(fileId)
    } else if (lowerFileId.endsWith('.h264') || lowerFileId.endsWith('.264') || lowerFileId.includes('.h264') || lowerFileId.includes('.264')) {
      console.log('Detected H.264 file, using SSE playback')
      setPlaybackMode('sse')
      // H.264 文件使用专用播放器，不需要在这里启动 SSE
    } else {
      console.log('Unknown file type, trying direct playback')
      setPlaybackMode('direct')
      startDirectPlayback(fileId)
    }
  }

  const startDirectPlayback = (fileId: string) => {
    setStatus('加载视频...')
    
    if (videoRef.current) {
      // 直接使用 HTTP 流式传输（支持 Range 请求）
      const streamUrl = `/api/v1/recordings/${encodeURIComponent(fileId)}/stream`
      videoRef.current.src = streamUrl
      
      videoRef.current.onloadedmetadata = () => {
        setStatus('视频已加载，可以播放')
        console.log('Video metadata loaded:', {
          duration: videoRef.current?.duration,
          videoWidth: videoRef.current?.videoWidth,
          videoHeight: videoRef.current?.videoHeight,
        })
      }

      videoRef.current.oncanplay = () => {
        setStatus('准备就绪')
        // 自动播放
        videoRef.current?.play().catch(err => {
          console.error('Autoplay failed:', err)
          setStatus('点击播放按钮开始')
        })
      }

      videoRef.current.onerror = (e) => {
        console.error('Video error:', e)
        setError('视频加载失败，请检查文件格式')
      }
    }
  }

  const startSSEPlayback = () => {
    setStatus('连接到服务器...')
    
    const eventSource = new EventSource(`/api/v1/playback/${sessionId}/segments`)
    
    eventSource.onopen = () => {
      console.log('SSE connection opened')
      setStatus('已连接，等待视频数据...')
    }

    let segmentCount = 0
    let lastTimestamp = 0
    
    eventSource.onmessage = (event) => {
      try {
        const segment = JSON.parse(event.data)
        segmentCount++
        lastTimestamp = segment.timestamp
        
        console.log('Received segment:', {
          id: segment.segment_id,
          timestamp: segment.timestamp,
          size: segment.data_length,
          isKeyframe: segment.flags & 0x01
        })
        
        setStatus(`✅ 数据传输成功！已接收 ${segmentCount} 个分片 (${segment.timestamp.toFixed(2)}s)`)
        
        // TODO: 实现 MSE 播放
        // 需要将 H.264 裸流转换为 fMP4 格式
        // 可以使用 mux.js 或在服务端转换
        
      } catch (err) {
        console.error('Error parsing segment:', err)
      }
    }
    
    eventSource.addEventListener('close', () => {
      console.log('SSE stream closed')
      setStatus(`✅ 传输完成！共接收 ${segmentCount} 个分片，总时长 ${lastTimestamp.toFixed(2)}s`)
      setError('H.264 裸流需要转换为 fMP4 格式才能播放。请使用 MP4 文件或实现 H.264→fMP4 转换。')
    })

    eventSource.onerror = (err) => {
      console.error('SSE error:', err)
      setError('连接错误，请重试')
      eventSource.close()
    }
  }

  // 播放模式状态
  const [selectedPlaybackMode, setSelectedPlaybackMode] = useState<'fast' | 'normal'>('normal')

  // 如果是直通播放模式或 H.264 回放，使用 WebCodecs 播放器
  if (isLiveMode) {
    return (
      <div>
        {/* 播放模式选择器 */}
        <div style={{
          padding: '15px',
          background: '#f5f5f5',
          borderRadius: '8px',
          marginBottom: '15px'
        }}>
          <h4 style={{ margin: '0 0 10px 0', fontSize: '16px', color: '#333' }}>播放模式选择</h4>
          <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
            <button
              onClick={() => setSelectedPlaybackMode('fast')}
              style={{
                padding: '10px 20px',
                border: selectedPlaybackMode === 'fast' ? '2px solid #1890ff' : '1px solid #d9d9d9',
                borderRadius: '6px',
                background: selectedPlaybackMode === 'fast' ? '#e6f7ff' : '#fff',
                cursor: 'pointer',
                fontSize: '14px',
                fontWeight: selectedPlaybackMode === 'fast' ? 'bold' : 'normal',
                transition: 'all 0.3s'
              }}
            >
              ⚡ Fast Mode
              <div style={{ fontSize: '12px', color: '#666', marginTop: '4px' }}>
                立即渲染（&lt;100ms）
              </div>
            </button>
            
            <button
              onClick={() => setSelectedPlaybackMode('normal')}
              style={{
                padding: '10px 20px',
                border: selectedPlaybackMode === 'normal' ? '2px solid #1890ff' : '1px solid #d9d9d9',
                borderRadius: '6px',
                background: selectedPlaybackMode === 'normal' ? '#e6f7ff' : '#fff',
                cursor: 'pointer',
                fontSize: '14px',
                fontWeight: selectedPlaybackMode === 'normal' ? 'bold' : 'normal',
                transition: 'all 0.3s'
              }}
            >
              🎬 Normal Mode
              <div style={{ fontSize: '12px', color: '#666', marginTop: '4px' }}>
                时间戳控制 + 倍速
              </div>
            </button>
            

          </div>
          
          {/* 模式说明 */}
          <div style={{
            marginTop: '15px',
            padding: '12px',
            background: '#fff',
            borderRadius: '6px',
            border: '1px solid #e8e8e8'
          }}>
            <div style={{ fontSize: '13px', lineHeight: '1.6', color: '#666' }}>
              {selectedPlaybackMode === 'fast' && (
                <>
                  <strong style={{ color: '#1890ff' }}>⚡ Fast Mode：</strong>
                  解码后立即渲染，完全跳过缓冲，实现最低延迟（通常 &lt;100ms）。
                  适合对延迟要求极高的场景。
                </>
              )}
              {selectedPlaybackMode === 'normal' && (
                <>
                  <strong style={{ color: '#1890ff' }}>🎬 Normal Mode：</strong>
                  基于 FPS 和时间戳双重控制播放速度，保证流畅稳定。
                  延迟略高（200-500ms），但画面最流畅。
                </>
              )}
            </div>
          </div>
        </div>
        
        <WebCodecsPlayer 
          key={selectedPlaybackMode} 
          sessionId={sessionId} 
          playbackMode={selectedPlaybackMode} 
        />
      </div>
    )
  }
  
  // 如果是 H.264 文件回放，也使用 WebCodecs 播放器
  if (playbackMode === 'sse' && fileId && (fileId.toLowerCase().endsWith('.h264') || fileId.toLowerCase().endsWith('.264') || fileId.toLowerCase().includes('.h264') || fileId.toLowerCase().includes('.264'))) {
    return (
      <WebCodecsPlayer 
        key={selectedPlaybackMode} 
        sessionId={sessionId} 
        playbackMode={selectedPlaybackMode} 
      />
    )
  }

  return (
    <div className="video-player">
      <div className="player-container">
        <video
          ref={videoRef}
          className="video-element"
          controls
          playsInline
        >
          您的浏览器不支持视频播放
        </video>
        
        {(status !== '准备就绪' || error) && (
          <div className="player-overlay">
            <div className="status-info">
              <p className="status">{status}</p>
              {error && <p className="error">{error}</p>}
            </div>
          </div>
        )}
      </div>

      <div className="player-info">
        <h3>播放会话: {sessionId.substring(0, 8)}...</h3>
        <div className="info-row">
          <span className="label">播放模式:</span>
          <span className="value">
            {playbackMode === 'direct' ? '🎬 直接流式传输 (MP4)' : '📡 SSE 实时流 (H.264)'}
          </span>
        </div>
        {fileId && (
          <div className="info-row">
            <span className="label">文件:</span>
            <span className="value">{fileId}</span>
          </div>
        )}
        
        {playbackMode === 'direct' && (
          <p className="hint success">
            ✅ MP4 文件可以直接播放，支持拖动进度条和快进快退
          </p>
        )}
        
        {playbackMode === 'sse' && (
          <div>
            <p className="hint warning">
              ⚠️ H.264 裸流需要转换为 fMP4 格式才能播放
            </p>
            <p className="hint">
              💡 建议：使用 MP4 格式的测试视频，或实现 H.264 到 fMP4 的转换
            </p>
          </div>
        )}
      </div>

      <div className="debug-console">
        <h4>调试信息</h4>
        <p>Session ID: {sessionId}</p>
        <p>Playback Mode: {playbackMode}</p>
        <p>打开浏览器控制台查看详细日志</p>
      </div>
    </div>
  )
}

export default VideoPlayer
