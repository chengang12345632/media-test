import React, { useEffect, useRef, useState } from 'react'

interface H264PlayerProps {
  sessionId: string
}

function H264Player({ sessionId }: H264PlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [status, setStatus] = useState<string>('初始化中...')
  const [error, setError] = useState<string | null>(null)
  const [segmentCount, setSegmentCount] = useState<number>(0)
  const mediaSourceRef = useRef<MediaSource | null>(null)
  const sourceBufferRef = useRef<SourceBuffer | null>(null)
  const queueRef = useRef<Uint8Array[]>([])
  const isInitializedRef = useRef<boolean>(false)

  useEffect(() => {
    console.log('H264Player mounted', { sessionId })
    
    // 检查浏览器支持
    if (!window.MediaSource) {
      setError('浏览器不支持 Media Source Extensions')
      return
    }

    // 动态加载 mux.js
    loadMuxJS().then(() => {
      initializePlayer()
    }).catch((err) => {
      console.error('Failed to load mux.js:', err)
      setError('加载 mux.js 失败')
    })

    return () => {
      cleanup()
    }
  }, [sessionId])

  const loadMuxJS = (): Promise<void> => {
    return new Promise((resolve, reject) => {
      // 检查是否已加载
      if ((window as any).muxjs) {
        resolve()
        return
      }

      // 动态加载 mux.js
      const script = document.createElement('script')
      script.src = 'https://cdn.jsdelivr.net/npm/mux.js@7.0.3/dist/mux.min.js'
      script.onload = () => {
        console.log('mux.js loaded')
        resolve()
      }
      script.onerror = reject
      document.head.appendChild(script)
    })
  }

  const initializePlayer = () => {
    const video = videoRef.current
    if (!video) return

    console.log('Initializing MSE player')
    
    // 添加视频事件监听
    const handleLoadedMetadata = () => {
      console.log('Video metadata loaded:', {
        duration: video.duration,
        videoWidth: video.videoWidth,
        videoHeight: video.videoHeight
      })
      setStatus('视频元数据已加载')
    }

    const handleCanPlay = () => {
      console.log('Video can play')
      setStatus('视频可以播放了')
    }

    const handlePlaying = () => {
      console.log('Video is playing')
      setStatus('正在播放')
    }

    const handleError = (e: Event) => {
      console.error('Video error:', e, video.error)
      if (video.error) {
        setError(`视频错误: ${video.error.message || '未知错误'}`)
      }
    }

    video.addEventListener('loadedmetadata', handleLoadedMetadata)
    video.addEventListener('canplay', handleCanPlay)
    video.addEventListener('playing', handlePlaying)
    video.addEventListener('error', handleError)
    
    // 创建 MediaSource
    const mediaSource = new MediaSource()
    mediaSourceRef.current = mediaSource
    video.src = URL.createObjectURL(mediaSource)

    mediaSource.addEventListener('sourceopen', () => {
      console.log('MediaSource opened')
      
      try {
        // 创建 SourceBuffer - 使用更通用的 MIME 类型
        let mimeCodec = 'video/mp4; codecs="avc1.42E01E,mp4a.40.2"'
        
        if (!MediaSource.isTypeSupported(mimeCodec)) {
          console.warn('Codec with audio not supported, trying video only')
          mimeCodec = 'video/mp4; codecs="avc1.42E01E"'
          
          if (!MediaSource.isTypeSupported(mimeCodec)) {
            console.warn('avc1.42E01E not supported, trying avc1.64001F')
            mimeCodec = 'video/mp4; codecs="avc1.64001F"'
            
            if (!MediaSource.isTypeSupported(mimeCodec)) {
              setError('浏览器不支持 H.264 编解码器')
              return
            }
          }
        }

        console.log('Using MIME codec:', mimeCodec)
        const sourceBuffer = mediaSource.addSourceBuffer(mimeCodec)
        sourceBufferRef.current = sourceBuffer
        sourceBuffer.mode = 'sequence' // 使用 sequence 模式，自动处理时间戳
        
        sourceBuffer.addEventListener('updateend', () => {
          console.log('SourceBuffer update ended, queue length:', queueRef.current.length)
          
          // 处理队列中的下一个数据
          if (queueRef.current.length > 0 && !sourceBuffer.updating) {
            const nextData = queueRef.current.shift()
            if (nextData) {
              try {
                console.log('Appending queued data, size:', nextData.byteLength)
                sourceBuffer.appendBuffer(nextData)
              } catch (e) {
                console.error('Failed to append queued buffer:', e)
              }
            }
          }

          // 尝试播放
          if (videoRef.current && videoRef.current.paused && videoRef.current.readyState >= 2) {
            console.log('Video ready, attempting to play')
            videoRef.current.play().catch(err => {
              console.warn('Autoplay failed:', err)
              setStatus('点击播放按钮开始')
            })
          }
        })

        sourceBuffer.addEventListener('error', (e) => {
          console.error('SourceBuffer error:', e)
          setError('视频缓冲区错误')
        })

        sourceBuffer.addEventListener('abort', (e) => {
          console.warn('SourceBuffer aborted:', e)
        })

        // 开始接收 SSE 数据
        startSSEStream()
        
      } catch (e) {
        console.error('Failed to initialize player:', e)
        setError('播放器初始化失败: ' + e)
      }
    })

    mediaSource.addEventListener('sourceended', () => {
      console.log('MediaSource ended')
      setStatus('播放完成')
    })

    mediaSource.addEventListener('error', (e) => {
      console.error('MediaSource error:', e)
      setError('媒体源错误')
    })
  }

  const startSSEStream = () => {
    setStatus('连接到服务器...')
    
    // 尝试使用 stream 端点（统一API），如果失败则回退到 playback 端点
    const streamUrl = `/api/v1/stream/${sessionId}/segments`
    console.log('Connecting to SSE stream:', streamUrl)
    const eventSource = new EventSource(streamUrl)
    let count = 0
    const h264Segments: { data: Uint8Array, timestamp: number, isKeyframe: boolean }[] = []

    eventSource.onopen = () => {
      console.log('SSE connection opened')
      setStatus('已连接，接收视频数据...')
    }

    let hasReceivedSPS = false
    
    eventSource.onmessage = (event) => {
      try {
        const segment = JSON.parse(event.data)
        count++
        
        // 将 base64 数据转换为 Uint8Array
        const h264Data = Uint8Array.from(atob(segment.data), c => c.charCodeAt(0))
        
        // 检查是否包含SPS (NAL type 7)
        const hasSPS = checkForSPS(h264Data)
        if (hasSPS && !hasReceivedSPS) {
          hasReceivedSPS = true
          console.log('✅ Received SPS! Starting playback...')
        }
        
        // 如果还没收到SPS，跳过这个分片
        if (!hasReceivedSPS) {
          console.log(`⏭️ Skipping segment #${count} (waiting for SPS)`)
          return
        }
        
        // 记录前几个分片的详细信息
        if (count <= 5) {
          const firstBytes = Array.from(h264Data.slice(0, 8)).map(b => b.toString(16).padStart(2, '0')).join(' ')
          console.log(`Received H.264 segment #${count}:`, {
            id: segment.segment_id,
            timestamp: segment.timestamp,
            size: segment.data_length,
            isKeyframe: segment.flags & 0x01,
            firstBytes,
            hasSPS
          })
        }
        
        setSegmentCount(count)
        setStatus(`接收并转换中... ${count} 个分片 (${segment.timestamp.toFixed(2)}s)`)
        
        h264Segments.push({
          data: h264Data,
          timestamp: segment.timestamp,
          isKeyframe: (segment.flags & 0x01) !== 0
        })

        // 立即处理前几个分片（包含SPS/PPS/IDR）
        // 或者收集足够的数据后开始转换
        if (count <= 20 || h264Segments.length >= 10 || (segment.flags & 0x01)) {
          processH264Segments(h264Segments.splice(0))
        }
        
      } catch (err) {
        console.error('Error processing segment:', err)
        setError('处理视频分片失败: ' + err)
      }
    }
    
    // 辅助函数：检查数据中是否包含SPS
    function checkForSPS(data: Uint8Array): boolean {
      for (let i = 0; i < data.length - 4; i++) {
        // 查找起始码 + SPS (NAL type 7)
        if ((data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x00 && data[i+3] === 0x01 && (data[i+4] & 0x1F) === 7) ||
            (data[i] === 0x00 && data[i+1] === 0x00 && data[i+2] === 0x01 && (data[i+3] & 0x1F) === 7)) {
          return true
        }
      }
      return false
    }

    eventSource.onerror = (err) => {
      console.error('SSE error:', err)
      eventSource.close()
      
      // 处理剩余数据
      if (h264Segments.length > 0) {
        processH264Segments(h264Segments)
      }
      
      // 结束流
      setTimeout(() => {
        if (mediaSourceRef.current && mediaSourceRef.current.readyState === 'open') {
          try {
            mediaSourceRef.current.endOfStream()
            setStatus(`✅ 播放完成！共 ${count} 个分片`)
          } catch (e) {
            console.error('Failed to end stream:', e)
          }
        }
      }, 1000)
    }
  }

  const processH264Segments = (segments: { data: Uint8Array, timestamp: number, isKeyframe: boolean }[]) => {
    if (segments.length === 0) return

    try {
      const muxjs = (window as any).muxjs
      if (!muxjs) {
        console.error('mux.js not loaded')
        return
      }

      console.log(`Processing ${segments.length} H.264 segments`)

      // 合并所有 H.264 数据
      const totalLength = segments.reduce((sum, seg) => sum + seg.data.length, 0)
      const combinedData = new Uint8Array(totalLength)
      let offset = 0
      for (const seg of segments) {
        combinedData.set(seg.data, offset)
        offset += seg.data.length
      }

      console.log('Combined H.264 data size:', combinedData.length)
      console.log('First 32 bytes:', Array.from(combinedData.slice(0, 32)).map(b => b.toString(16).padStart(2, '0')).join(' '))

      // 检查数据是否以 NAL unit start code 开头
      const hasStartCode = combinedData.length >= 4 && 
        ((combinedData[0] === 0x00 && combinedData[1] === 0x00 && combinedData[2] === 0x00 && combinedData[3] === 0x01) ||
         (combinedData[0] === 0x00 && combinedData[1] === 0x00 && combinedData[2] === 0x01))
      
      console.log('H.264 data has NAL start code:', hasStartCode)

      if (!hasStartCode) {
        console.error('❌ H.264 data does not have NAL start code - this is not a valid Annex B H.264 stream')
        setError('H.264 文件格式不正确。请使用标准 Annex B 格式的 H.264 文件，或使用 MP4 文件代替。')
        return
      }

      // 使用 mux.js 转换
      const transmuxer = new muxjs.mp4.Transmuxer()

      let hasReceivedData = false

      transmuxer.on('data', (segment: any) => {
        hasReceivedData = true
        console.log('✅ Transmuxed segment received:', {
          hasInitSegment: !!segment.initSegment,
          hasData: !!segment.data,
          type: segment.type,
          initSegmentSize: segment.initSegment?.byteLength,
          dataSize: segment.data?.byteLength,
          tracks: segment.tracks
        })
        
        const sourceBuffer = sourceBufferRef.current
        if (!sourceBuffer) {
          console.error('SourceBuffer not available')
          return
        }

        try {
          // 第一次需要发送 init segment
          if (!isInitializedRef.current && segment.initSegment) {
            console.log('📦 Appending init segment, size:', segment.initSegment.byteLength)
            isInitializedRef.current = true
            
            const initData = new Uint8Array(segment.initSegment.byteLength)
            initData.set(segment.initSegment)
            
            if (!sourceBuffer.updating) {
              sourceBuffer.appendBuffer(initData)
            } else {
              queueRef.current.push(initData)
            }
          }

          // 发送 media segment
          if (segment.data && segment.data.byteLength > 0) {
            console.log('📦 Appending media segment, size:', segment.data.byteLength)
            
            const mediaData = new Uint8Array(segment.data.byteLength)
            mediaData.set(segment.data)
            
            if (!sourceBuffer.updating) {
              sourceBuffer.appendBuffer(mediaData)
            } else {
              queueRef.current.push(mediaData)
            }

            // 尝试自动播放
            if (videoRef.current && videoRef.current.paused && videoRef.current.readyState >= 2) {
              console.log('🎬 Attempting to play video')
              videoRef.current.play().catch(err => {
                console.warn('Autoplay failed:', err)
              })
            }
          }
        } catch (e) {
          console.error('Failed to append buffer:', e)
          setError('添加视频数据失败: ' + e)
        }
      })

      transmuxer.on('done', () => {
        console.log('✅ Transmuxing done for this batch, received data:', hasReceivedData)
        if (!hasReceivedData) {
          console.error('❌ No data received from transmuxer - H.264 format may be invalid')
        }
      })

      // 推送数据到 transmuxer
      console.log('🔄 Pushing H.264 data to transmuxer...')
      transmuxer.push(combinedData)
      transmuxer.flush()

    } catch (err) {
      console.error('Error in processH264Segments:', err)
      setError('转换失败: ' + err)
    }
  }

  const cleanup = () => {
    if (sourceBufferRef.current) {
      sourceBufferRef.current = null
    }
    if (mediaSourceRef.current) {
      if (mediaSourceRef.current.readyState === 'open') {
        try {
          mediaSourceRef.current.endOfStream()
        } catch (e) {
          // ignore
        }
      }
      mediaSourceRef.current = null
    }
    if (videoRef.current) {
      videoRef.current.src = ''
    }
    queueRef.current = []
    isInitializedRef.current = false
  }

  return (
    <div className="h264-player">
      <div className="player-container">
        <video
          ref={videoRef}
          className="video-element"
          controls
          playsInline
        >
          您的浏览器不支持视频播放
        </video>
        
        {(status || error) && (
          <div className="player-overlay">
            <div className="status-info">
              <p className="status">{status}</p>
              {error && <p className="error">{error}</p>}
            </div>
          </div>
        )}
      </div>

      <div className="player-info">
        <h3>H.264 实时播放</h3>
        <div className="info-row">
          <span className="label">会话 ID:</span>
          <span className="value">{sessionId.substring(0, 8)}...</span>
        </div>
        <div className="info-row">
          <span className="label">接收分片:</span>
          <span className="value">{segmentCount}</span>
        </div>
        <div className="info-row">
          <span className="label">转换方式:</span>
          <span className="value">🔄 mux.js 实时转换</span>
        </div>
        <div className="info-row">
          <span className="label">播放器:</span>
          <span className="value">MSE (Media Source Extensions)</span>
        </div>
        
        <div className="hint-box">
          <p className="hint warning">
            ⚠️ H.264 播放需要标准 Annex B 格式
          </p>
          <p className="hint info">
            💡 H.264 文件必须包含 NAL 起始码（00 00 00 01 或 00 00 01）
          </p>
          <p className="hint success">
            ✅ 建议使用 MP4 格式文件，可以直接播放无需转换
          </p>
        </div>
      </div>
    </div>
  )
}

export default H264Player
