import React, { useEffect, useRef, useState } from 'react'

interface BroadwayPlayerProps {
  sessionId: string
}

/**
 * 使用 Broadway.js 的 H.264 播放器
 * 纯 JavaScript 解码器，兼容性好但性能较低
 */
function BroadwayPlayer({ sessionId }: BroadwayPlayerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [status, setStatus] = useState<string>('初始化中...')
  const [error, setError] = useState<string | null>(null)
  const [segmentCount, setSegmentCount] = useState<number>(0)
  const [fps, setFps] = useState<number>(0)
  const playerRef = useRef<any>(null)
  const eventSourceRef = useRef<EventSource | null>(null)
  const frameCountRef = useRef<number>(0)
  const lastFpsUpdateRef = useRef<number>(Date.now())

  useEffect(() => {
    console.log('BroadwayPlayer mounted', { sessionId })
    
    // 动态加载 Broadway.js
    loadBroadway().then(() => {
      initializePlayer()
    }).catch((err) => {
      console.error('Failed to load Broadway.js:', err)
      setError('加载 Broadway.js 失败')
    })

    return () => {
      cleanup()
    }
  }, [sessionId])

  const loadBroadway = (): Promise<void> => {
    return new Promise((resolve, reject) => {
      // 检查是否已加载
      if ((window as any).Player) {
        resolve()
        return
      }

      // 动态加载 Broadway.js
      const script = document.createElement('script')
      script.src