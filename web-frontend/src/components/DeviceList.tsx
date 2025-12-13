import React, { useEffect, useState } from 'react'
import { apiClient } from '../services/api'
import type { DeviceInfo } from '../types/api'
import './DeviceList.css'

interface DeviceListProps {
  onDeviceSelect: (deviceId: string) => void
  onLiveStream?: (deviceId: string) => void
}

function DeviceList({ onDeviceSelect, onLiveStream }: DeviceListProps) {
  const [devices, setDevices] = useState<DeviceInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    loadDevices()
    const interval = setInterval(loadDevices, 5000) // 每5秒刷新
    return () => clearInterval(interval)
  }, [])

  const loadDevices = async () => {
    try {
      const data = await apiClient.getDevices()
      setDevices(data)
      setError(null)
    } catch (err) {
      setError('加载设备列表失败')
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  if (loading) {
    return <div className="loading">加载中...</div>
  }

  if (error) {
    return <div className="error">{error}</div>
  }

  if (devices.length === 0) {
    return (
      <div className="empty-state">
        <p>📹 暂无设备在线</p>
        <p className="hint">请启动设备模拟器</p>
      </div>
    )
  }

  return (
    <div className="device-list">
      <h2>设备列表 ({devices.length})</h2>
      <div className="device-grid">
        {devices.map((device) => (
          <div
            key={device.device_id}
            className="device-card"
          >
            <div className="device-header">
              <h3>📹 {device.device_name}</h3>
              <span
                className={`status-badge ${device.connection_status}`}
              >
                {device.connection_status === 'online' ? '🟢 在线' : '🔴 离线'}
              </span>
            </div>
            <div className="device-info">
              <p>设备ID: {device.device_id}</p>
              <p>分辨率: {device.capabilities.max_resolution}</p>
              <p>
                支持格式: {device.capabilities.supported_formats.join(', ')}
              </p>
            </div>
            <div className="device-actions">
              <button 
                className="live-button"
                onClick={(e) => {
                  e.stopPropagation()
                  if (onLiveStream) {
                    onLiveStream(device.device_id)
                  }
                }}
                disabled={device.connection_status !== 'online'}
              >
                🔴 直通播放
              </button>
              <button 
                className="view-button"
                onClick={(e) => {
                  e.stopPropagation()
                  onDeviceSelect(device.device_id)
                }}
              >
                📼 查看录像
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default DeviceList
