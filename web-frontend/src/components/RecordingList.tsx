import React, { useEffect, useState } from 'react'
import { apiClient } from '../services/api'
import type { RecordingInfo } from '../types/api'
import './RecordingList.css'

interface RecordingListProps {
  deviceId: string
  onRecordingSelect: (fileId: string, sessionId: string) => void
}

function RecordingList({ deviceId, onRecordingSelect }: RecordingListProps) {
  const [recordings, setRecordings] = useState<RecordingInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    loadRecordings()
  }, [deviceId])

  const loadRecordings = async () => {
    try {
      const data = await apiClient.getRecordings(deviceId)
      setRecordings(data)
      setError(null)
    } catch (err) {
      setError('加载录像列表失败')
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  const handlePlay = async (fileId: string) => {
    try {
      console.log('Starting playback for file:', fileId)
      const response = await apiClient.startPlayback(fileId, {
        client_id: `web_${Date.now()}`,
        start_position: 0,
      })
      console.log('Playback started successfully:', response)
      onRecordingSelect(fileId, response.session_id)
    } catch (err) {
      console.error('Playback failed:', err)
      alert(`启动播放失败: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  const formatFileSize = (bytes: number): string => {
    const mb = bytes / (1024 * 1024)
    return `${mb.toFixed(2)} MB`
  }

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  if (loading) {
    return <div className="loading">加载中...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  if (recordings.length === 0) {
    return (
      <div className="empty-state">
        <p>📁 暂无录像文件</p>
        <p className="hint">请在 test-videos 目录添加视频文件</p>
      </div>
    )
  }

  return (
    <div className="recording-list">
      <h2>录像列表 ({recordings.length})</h2>
      <div className="recording-grid">
        {recordings.map((recording) => (
          <div key={recording.file_id} className="recording-card">
            <div className="recording-thumbnail">
              🎬
            </div>
            <div className="recording-info">
              <h3>{recording.file_name}</h3>
              <div className="recording-meta">
                <span>📏 {formatFileSize(recording.file_size)}</span>
                <span>⏱️ {formatDuration(recording.duration)}</span>
                <span>📺 {recording.resolution}</span>
              </div>
              <div className="recording-details">
                <span>格式: {recording.format.toUpperCase()}</span>
                <span>帧率: {recording.frame_rate} fps</span>
              </div>
            </div>
            <button
              className="play-button"
              onClick={() => handlePlay(recording.file_id)}
            >
              ▶️ 播放
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}

export default RecordingList
