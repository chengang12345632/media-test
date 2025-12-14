import React from 'react'
import PlaybackModeComparison from '../components/PlaybackModeComparison'

/**
 * 播放模式测试页面
 * 展示四种播放模式的对比和说明
 */
function PlaybackModeTest() {
  return (
    <div style={{ padding: '20px', maxWidth: '1400px', margin: '0 auto' }}>
      <h1 style={{ textAlign: 'center', marginBottom: '30px' }}>
        播放模式功能测试
      </h1>
      
      <div style={{
        padding: '20px',
        background: '#e6f7ff',
        borderRadius: '8px',
        marginBottom: '30px',
        border: '1px solid #91d5ff'
      }}>
        <h3 style={{ marginTop: 0 }}>✅ 功能已实现</h3>
        <p>四种播放模式已成功集成到 WebCodecs 播放器中：</p>
        <ul>
          <li><strong>🚀 Ultra Mode</strong> - 极速模式，延迟 &lt;100ms</li>
          <li><strong>⚡ Fast Mode</strong> - 快速模式，立即渲染</li>
          <li><strong>🎬 Normal Mode</strong> - 正常模式，按帧率播放（推荐）</li>
          <li><strong>⏱️ Timestamp Mode</strong> - 时间戳模式，精确同步</li>
        </ul>
        <p style={{ marginBottom: 0 }}>
          在设备列表中选择一个在线设备，点击"开始直播"即可看到播放模式选择器。
        </p>
      </div>
      
      <PlaybackModeComparison />
    </div>
  )
}

export default PlaybackModeTest
