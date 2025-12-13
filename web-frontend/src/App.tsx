import React, { useState } from 'react'
import DeviceList from './components/DeviceList'
import RecordingList from './components/RecordingList'
import VideoPlayer from './components/VideoPlayer'
import './App.css'

type View = 'devices' | 'recordings' | 'player' | 'live'

interface AppState {
  view: View
  selectedDeviceId?: string
  selectedFileId?: string
  sessionId?: string
  isLiveMode?: boolean
}

function App() {
  const [state, setState] = useState<AppState>({
    view: 'devices',
  })

  const handleDeviceSelect = (deviceId: string) => {
    setState({
      view: 'recordings',
      selectedDeviceId: deviceId,
    })
  }

  const handleLiveStream = async (deviceId: string) => {
    console.log('Starting live stream for device:', deviceId)
    
    try {
      // 调用API启动直通播放
      const response = await fetch('/api/v1/stream/start', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          mode: 'live',
          source: {
            device_id: deviceId,
          },
          config: {
            client_id: 'web_client_' + Date.now(),
            low_latency_mode: true,
            target_latency_ms: 100,
          },
        }),
      })

      const data = await response.json()
      
      if (data.status === 'success') {
        console.log('Live stream started:', data.data)
        setState({
          view: 'live',
          selectedDeviceId: deviceId,
          sessionId: data.data.session_id,
          isLiveMode: true,
        })
      } else {
        alert('启动直通播放失败: ' + (data.error || '未知错误'))
      }
    } catch (error) {
      console.error('Failed to start live stream:', error)
      alert('启动直通播放失败: ' + error)
    }
  }

  const handleRecordingSelect = (fileId: string, sessionId: string) => {
    console.log('Recording selected:', { fileId, sessionId })
    setState({
      ...state,
      view: 'player',
      selectedFileId: fileId,
      sessionId,
      isLiveMode: false,
    })
  }

  const handleBack = async () => {
    // 如果正在播放，先停止流
    if ((state.view === 'player' || state.view === 'live') && state.sessionId) {
      try {
        await fetch(`/api/v1/stream/${state.sessionId}`, {
          method: 'DELETE',
        })
        console.log('Stream stopped:', state.sessionId)
      } catch (error) {
        console.error('Failed to stop stream:', error)
      }
    }
    
    // 直播播放返回设备列表，录像播放返回录像列表
    if (state.view === 'live') {
      setState({ view: 'devices', sessionId: undefined, isLiveMode: false })
    } else if (state.view === 'player') {
      setState({ ...state, view: 'recordings', sessionId: undefined, isLiveMode: false })
    } else if (state.view === 'recordings') {
      setState({ view: 'devices' })
    }
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>📹 HTTP3/QUIC 视频流传输系统</h1>
        {state.view !== 'devices' && (
          <button onClick={handleBack} className="back-button">
            ← 返回
          </button>
        )}
      </header>

      <main className="app-main">
        {state.view === 'devices' && (
          <DeviceList 
            onDeviceSelect={handleDeviceSelect}
            onLiveStream={handleLiveStream}
          />
        )}

        {state.view === 'recordings' && state.selectedDeviceId && (
          <RecordingList
            deviceId={state.selectedDeviceId}
            onRecordingSelect={handleRecordingSelect}
          />
        )}

        {state.view === 'player' && state.sessionId && (
          <VideoPlayer 
            sessionId={state.sessionId} 
            fileId={state.selectedFileId}
          />
        )}

        {state.view === 'live' && state.sessionId && (
          <VideoPlayer 
            sessionId={state.sessionId} 
            fileId={state.selectedFileId}
            isLiveMode={true}
          />
        )}
      </main>
    </div>
  )
}

export default App
