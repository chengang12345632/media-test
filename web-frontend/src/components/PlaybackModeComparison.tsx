import React from 'react'
import './PlaybackModeComparison.css'

/**
 * 播放模式对比说明组件
 * 展示四种播放模式的特点和适用场景
 */
function PlaybackModeComparison() {
  return (
    <div className="playback-mode-comparison">
      <h2>📊 播放模式对比</h2>
      
      <div className="comparison-grid">
        {/* Ultra Mode */}
        <div className="mode-card ultra">
          <div className="mode-header">
            <span className="mode-icon">🚀</span>
            <h3>Ultra Mode</h3>
            <span className="mode-badge">极速</span>
          </div>
          <div className="mode-content">
            <div className="mode-metric">
              <span className="metric-label">延迟：</span>
              <span className="metric-value highlight">&lt; 100ms</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">缓冲：</span>
              <span className="metric-value">0 帧</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">策略：</span>
              <span className="metric-value">解码后立即渲染</span>
            </div>
            
            <div className="mode-features">
              <h4>特点：</h4>
              <ul>
                <li>✅ 延迟最低</li>
                <li>✅ 响应最快</li>
                <li>⚠️ 可能轻微卡顿</li>
                <li>⚠️ 帧率不稳定</li>
              </ul>
            </div>
            
            <div className="mode-usecase">
              <h4>适用场景：</h4>
              <p>实时监控、远程控制、对延迟要求极高的场景</p>
            </div>
          </div>
        </div>

        {/* Fast Mode */}
        <div className="mode-card fast">
          <div className="mode-header">
            <span className="mode-icon">⚡</span>
            <h3>Fast Mode</h3>
            <span className="mode-badge">快速</span>
          </div>
          <div className="mode-content">
            <div className="mode-metric">
              <span className="metric-label">延迟：</span>
              <span className="metric-value highlight">&lt; 100ms</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">缓冲：</span>
              <span className="metric-value">0 帧</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">策略：</span>
              <span className="metric-value">立即渲染</span>
            </div>
            
            <div className="mode-features">
              <h4>特点：</h4>
              <ul>
                <li>✅ 延迟极低</li>
                <li>✅ 实时性强</li>
                <li>⚠️ 可能有抖动</li>
                <li>⚠️ 网络波动敏感</li>
              </ul>
            </div>
            
            <div className="mode-usecase">
              <h4>适用场景：</h4>
              <p>视频会议、直播互动、需要快速反馈的应用</p>
            </div>
          </div>
        </div>

        {/* Normal Mode */}
        <div className="mode-card normal">
          <div className="mode-header">
            <span className="mode-icon">🎬</span>
            <h3>Normal Mode</h3>
            <span className="mode-badge recommended">推荐</span>
          </div>
          <div className="mode-content">
            <div className="mode-metric">
              <span className="metric-label">延迟：</span>
              <span className="metric-value">200-500ms</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">缓冲：</span>
              <span className="metric-value">按需缓冲</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">策略：</span>
              <span className="metric-value">严格按帧率渲染</span>
            </div>
            
            <div className="mode-features">
              <h4>特点：</h4>
              <ul>
                <li>✅ 播放流畅</li>
                <li>✅ 帧率稳定</li>
                <li>✅ 画面质量好</li>
                <li>⚠️ 延迟略高</li>
              </ul>
            </div>
            
            <div className="mode-usecase">
              <h4>适用场景：</h4>
              <p>视频点播、录像回放、对画质要求高的场景</p>
            </div>
          </div>
        </div>

        {/* Timestamp Mode */}
        <div className="mode-card timestamp">
          <div className="mode-header">
            <span className="mode-icon">⏱️</span>
            <h3>Timestamp Mode</h3>
            <span className="mode-badge">精确</span>
          </div>
          <div className="mode-content">
            <div className="mode-metric">
              <span className="metric-label">延迟：</span>
              <span className="metric-value">100-300ms</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">缓冲：</span>
              <span className="metric-value">动态调整</span>
            </div>
            <div className="mode-metric">
              <span className="metric-label">策略：</span>
              <span className="metric-value">时间戳精确同步</span>
            </div>
            
            <div className="mode-features">
              <h4>特点：</h4>
              <ul>
                <li>✅ 时间精确</li>
                <li>✅ 音画同步好</li>
                <li>✅ 平衡延迟和流畅</li>
                <li>⚠️ 实现复杂</li>
              </ul>
            </div>
            
            <div className="mode-usecase">
              <h4>适用场景：</h4>
              <p>音视频同步、多路流同步、需要精确时间的场景</p>
            </div>
          </div>
        </div>
      </div>

      {/* 性能对比表格 */}
      <div className="comparison-table-container">
        <h3>性能指标对比</h3>
        <table className="comparison-table">
          <thead>
            <tr>
              <th>指标</th>
              <th>🚀 Ultra</th>
              <th>⚡ Fast</th>
              <th>🎬 Normal</th>
              <th>⏱️ Timestamp</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>端到端延迟</td>
              <td className="excellent">&lt; 100ms</td>
              <td className="excellent">&lt; 100ms</td>
              <td className="good">200-500ms</td>
              <td className="good">100-300ms</td>
            </tr>
            <tr>
              <td>帧率稳定性</td>
              <td className="poor">低</td>
              <td className="poor">低</td>
              <td className="excellent">高</td>
              <td className="good">中</td>
            </tr>
            <tr>
              <td>画面流畅度</td>
              <td className="poor">中</td>
              <td className="poor">中</td>
              <td className="excellent">高</td>
              <td className="good">高</td>
            </tr>
            <tr>
              <td>网络适应性</td>
              <td className="poor">弱</td>
              <td className="poor">弱</td>
              <td className="excellent">强</td>
              <td className="good">中</td>
            </tr>
            <tr>
              <td>CPU 占用</td>
              <td className="excellent">低</td>
              <td className="excellent">低</td>
              <td className="good">中</td>
              <td className="good">中</td>
            </tr>
            <tr>
              <td>内存占用</td>
              <td className="excellent">极低</td>
              <td className="excellent">极低</td>
              <td className="good">低</td>
              <td className="good">低</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* 技术实现说明 */}
      <div className="technical-details">
        <h3>🔧 技术实现</h3>
        <div className="tech-grid">
          <div className="tech-item">
            <h4>Ultra/Fast 模式</h4>
            <pre><code>{`decoder.output = (frame) => {
  // 解码后立即渲染
  canvas.drawImage(frame)
  frame.close()
}`}</code></pre>
            <p>完全跳过缓冲，解码完成立即显示</p>
          </div>

          <div className="tech-item">
            <h4>Normal 模式</h4>
            <pre><code>{`const frameInterval = 1000 / fps
setTimeout(() => {
  renderFrame()
}, frameInterval)`}</code></pre>
            <p>严格按照视频帧率间隔调度渲染</p>
          </div>

          <div className="tech-item">
            <h4>Timestamp 模式</h4>
            <pre><code>{`const systemElapsed = now - lastTime
const frameElapsed = frame.ts - lastTs
const delay = frameElapsed - systemElapsed
setTimeout(render, delay)`}</code></pre>
            <p>对比系统时间和帧时间戳，精确同步</p>
          </div>
        </div>
      </div>
    </div>
  )
}

export default PlaybackModeComparison
