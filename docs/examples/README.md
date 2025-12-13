# 统一低延迟视频流传输系统 - 使用示例

本目录包含统一低延迟视频流传输系统的使用示例和测试脚本。

## 目录结构

```
examples/
├── README.md                      # 本文件
├── unified-stream-example.html    # 完整的Web示例页面
└── test-stream-api.sh            # API测试脚本
```

## 1. Web示例页面

### 1.1 功能说明

`unified-stream-example.html` 是一个完整的Web示例页面，演示了如何使用统一低延迟视频流传输系统的API。

**功能特性**：
- ✅ 支持直通播放和录像回放两种模式
- ✅ 完整的播放控制（暂停、恢复、定位、倍速）
- ✅ 实时统计信息显示
- ✅ 详细的日志输出
- ✅ MSE播放器集成
- ✅ SSE自动重连

### 1.2 使用方法

#### 方法一：直接打开

```bash
# 在浏览器中打开
open unified-stream-example.html
# 或
firefox unified-stream-example.html
# 或
chrome unified-stream-example.html
```

#### 方法二：通过HTTP服务器

```bash
# 使用Python启动简单HTTP服务器
cd docs/examples
python3 -m http.server 8000

# 然后在浏览器中访问
# http://localhost:8000/unified-stream-example.html
```

### 1.3 操作步骤

1. **配置参数**
   - 选择模式：直通播放 (Live) 或 录像回放 (Playback)
   - 输入源ID：设备ID（如 `device_001`）或文件ID（如 `rec_001`）
   - 设置起始位置（仅回放模式）

2. **启动流**
   - 点击"启动流"按钮
   - 等待视频开始播放

3. **播放控制**
   - 暂停/恢复：控制播放状态
   - 定位：跳转到指定时间位置（仅回放）
   - 倍速：调整播放速度（仅回放）

4. **查看统计**
   - 实时查看延迟、吞吐量等性能指标
   - 监控接收的分片数和数据量

5. **停止流**
   - 点击"停止流"按钮结束播放

### 1.4 注意事项

- 确保平台端服务器正在运行（`https://localhost:8443`）
- 浏览器需要支持MSE（Media Source Extensions）
- 浏览器需要支持EventSource（SSE）
- 由于使用自签名证书，首次访问需要信任证书

## 2. API测试脚本

### 2.1 功能说明

`test-stream-api.sh` 是一个命令行测试脚本，用于测试统一低延迟视频流传输系统的API。

**测试内容**：
- ✅ 启动流会话
- ✅ 查询流状态
- ✅ 暂停/恢复播放
- ✅ 定位功能（回放模式）
- ✅ 倍速功能（回放模式）
- ✅ 停止流会话

### 2.2 依赖要求

```bash
# 必需
curl

# 可选（用于格式化JSON输出）
jq
```

### 2.3 使用方法

#### 基本用法

```bash
# 添加执行权限
chmod +x test-stream-api.sh

# 测试录像回放（默认）
./test-stream-api.sh

# 测试直通播放
./test-stream-api.sh live device_001

# 测试录像回放（指定文件）
./test-stream-api.sh playback rec_001
```

#### 参数说明

```bash
./test-stream-api.sh [mode] [source_id]

参数:
  mode       - 流模式: live（直通播放）或 playback（录像回放）
  source_id  - 源ID: 设备ID或文件ID
```

### 2.4 输出示例

```
=========================================
统一低延迟视频流传输系统 - API测试
=========================================

[INFO] 检查依赖...
[SUCCESS] 依赖检查完成

[INFO] 启动流会话 (模式: playback, 源: rec_001)...
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "stream_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/segments",
  "control_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/control",
  "estimated_latency_ms": 100
}
[SUCCESS] 流会话已创建: 550e8400-e29b-41d4-a716-446655440000

[INFO] 查询流状态...
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "mode": "playback",
  "state": "streaming",
  "current_position": 2.5,
  "stats": {
    "average_latency_ms": 95.5,
    "current_latency_ms": 87.2,
    "throughput_mbps": 5.2
  }
}
[SUCCESS] 状态查询完成

...

=========================================
[SUCCESS] 测试完成！
=========================================
```

## 3. 代码示例

### 3.1 JavaScript示例

#### 启动直通播放

```javascript
async function startLiveStream(deviceId) {
  const response = await fetch('https://localhost:8443/api/v1/stream/start', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      mode: 'live',
      source: {device_id: deviceId},
      config: {
        client_id: 'web_client_001',
        low_latency_mode: true,
        target_latency_ms: 50
      }
    })
  })
  
  return await response.json()
}

// 使用
const session = await startLiveStream('device_001')
console.log('Session ID:', session.session_id)
```

#### 连接SSE接收分片

```javascript
function connectSSE(streamUrl) {
  const eventSource = new EventSource(streamUrl)
  
  eventSource.addEventListener('segment', (event) => {
    const segment = JSON.parse(event.data)
    
    // 解码Base64数据
    const binaryData = atob(segment.data)
    const bytes = new Uint8Array(binaryData.length)
    for (let i = 0; i < binaryData.length; i++) {
      bytes[i] = binaryData.charCodeAt(i)
    }
    
    // 追加到SourceBuffer
    if (!sourceBuffer.updating) {
      sourceBuffer.appendBuffer(bytes.buffer)
    }
  })
  
  return eventSource
}
```

#### 播放控制

```javascript
// 暂停
async function pause(sessionId) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({command: 'pause'})
  })
}

// 定位到30秒
async function seek(sessionId, position) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({command: 'seek', position: 30.0})
  })
}

// 2倍速播放
async function setRate(sessionId, rate) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({command: 'set_rate', rate: 2.0})
  })
}
```

### 3.2 cURL示例

#### 启动录像回放

```bash
curl -X POST https://localhost:8443/api/v1/stream/start \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "playback",
    "source": {
      "file_id": "rec_001",
      "start_position": 0.0,
      "playback_rate": 1.0
    },
    "config": {
      "client_id": "test_client",
      "low_latency_mode": true,
      "target_latency_ms": 100
    }
  }'
```

#### 查询状态

```bash
curl https://localhost:8443/api/v1/stream/{session_id}/status
```

#### 播放控制

```bash
# 暂停
curl -X POST https://localhost:8443/api/v1/stream/{session_id}/control \
  -H "Content-Type: application/json" \
  -d '{"command": "pause"}'

# 定位
curl -X POST https://localhost:8443/api/v1/stream/{session_id}/control \
  -H "Content-Type: application/json" \
  -d '{"command": "seek", "position": 30.0}'

# 倍速
curl -X POST https://localhost:8443/api/v1/stream/{session_id}/control \
  -H "Content-Type: application/json" \
  -d '{"command": "set_rate", "rate": 2.0}'
```

#### 停止流

```bash
curl -X DELETE https://localhost:8443/api/v1/stream/{session_id}
```

## 4. 故障排查

### 4.1 常见问题

#### 问题1：无法连接到服务器

**症状**：
```
Failed to connect to localhost:8443
```

**解决方法**：
1. 确认平台端服务器正在运行
2. 检查端口8443是否被占用
3. 检查防火墙设置

#### 问题2：证书错误

**症状**：
```
SSL certificate problem: self signed certificate
```

**解决方法**：
- cURL: 使用 `-k` 或 `--insecure` 参数
- 浏览器: 手动信任证书
- 生产环境: 使用有效的SSL证书

#### 问题3：MSE不支持

**症状**：
```
浏览器不支持MSE
```

**解决方法**：
- 使用支持MSE的现代浏览器（Chrome 90+, Firefox 88+, Safari 14+）
- 检查浏览器版本

#### 问题4：SSE连接断开

**症状**：
```
SSE连接错误
```

**解决方法**：
- 检查网络连接
- 查看服务器日志
- 确认会话ID有效
- 等待自动重连（最多5次）

### 4.2 调试技巧

#### 启用详细日志

在浏览器控制台中：
```javascript
// 查看所有日志
console.log('Detailed logging enabled')

// 监控MSE事件
sourceBuffer.addEventListener('error', (e) => {
  console.error('SourceBuffer error:', e)
})

// 监控SSE事件
eventSource.addEventListener('error', (e) => {
  console.error('SSE error:', e)
})
```

#### 查看网络请求

1. 打开浏览器开发者工具（F12）
2. 切换到"网络"标签
3. 筛选"XHR"和"EventStream"
4. 查看请求和响应详情

#### 查看服务器日志

```bash
# 查看平台端日志
tail -f platform-server/logs/app.log

# 查看设备端日志
tail -f device-simulator/logs/app.log
```

## 5. 性能测试

### 5.1 延迟测试

使用测试脚本测量端到端延迟：

```bash
# 运行测试
./test-stream-api.sh playback rec_001

# 查看统计信息中的延迟数据
# - average_latency_ms: 平均延迟
# - current_latency_ms: 当前延迟
# - p50_latency_ms: P50延迟
# - p95_latency_ms: P95延迟
# - p99_latency_ms: P99延迟
```

### 5.2 吞吐量测试

监控统计信息中的吞吐量：

```javascript
// 定期查询状态
setInterval(async () => {
  const response = await fetch(`/api/v1/stream/${sessionId}/status`)
  const data = await response.json()
  console.log('Throughput:', data.stats.throughput_mbps, 'Mbps')
}, 1000)
```

### 5.3 并发测试

使用多个客户端同时连接：

```bash
# 启动多个测试脚本
for i in {1..10}; do
  ./test-stream-api.sh playback rec_001 &
done

# 等待所有测试完成
wait
```

## 6. 参考资料

- [API使用文档](../统一低延迟流API使用文档.md)
- [系统架构设计文档](../系统架构设计文档.md)
- [快速开始指南](../快速开始指南.md)

## 7. 联系方式

如有问题或建议，请联系：
- 邮箱: support@example.com
- 文档: https://docs.example.com
- GitHub: https://github.com/example/project

---

**最后更新**: 2025-12-13
**维护团队**: 系统架构团队
