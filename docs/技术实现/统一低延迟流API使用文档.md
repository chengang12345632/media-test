# 统一低延迟视频流传输系统 - API使用文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 文档版本 | v1.0 |
| 创建日期 | 2025-12-13 |
| 文档状态 | 正式版 |
| 作者 | 系统架构团队 |

## 目录

1. [概述](#1-概述)
2. [认证和授权](#2-认证和授权)
3. [流管理API](#3-流管理api)
4. [播放控制API](#4-播放控制api)
5. [状态查询API](#5-状态查询api)
6. [SSE媒体流](#6-sse媒体流)
7. [错误处理](#7-错误处理)
8. [使用示例](#8-使用示例)

---

## 1. 概述

### 1.1 API基础信息

**Base URL**: `https://localhost:8443/api/v1`

**协议**: HTTP/3 (QUIC)

**内容类型**: `application/json`

**字符编码**: UTF-8

### 1.2 API设计理念

- **统一接口**: 直通播放和录像回放使用相同的API
- **RESTful风格**: 遵循REST设计原则
- **低延迟优化**: 使用SSE推送，避免轮询
- **错误友好**: 详细的错误信息和状态码

### 1.3 支持的流模式

| 模式 | 说明 | 数据源 | 支持功能 |
|------|------|--------|----------|
| `live` | 直通播放 | 设备端实时流 | 暂停、恢复 |
| `playback` | 录像回放 | 文件系统 | 暂停、恢复、定位、倍速 |

---

## 2. 认证和授权

### 2.1 Demo版本

**当前版本为Demo演示版本，无需认证**。

所有API端点可以直接访问，无需提供认证令牌。

### 2.2 生产版本（未来）

生产版本将使用JWT（JSON Web Token）进行认证：

```http
Authorization: Bearer <jwt_token>
```

---

## 3. 流管理API

### 3.1 启动流会话

创建一个新的流会话（直通播放或录像回放）。

#### 请求

```http
POST /api/v1/stream/start HTTP/3
Content-Type: application/json
```

**请求体**：

```json
{
  "mode": "live" | "playback",
  "source": {
    // 直通播放模式
    "device_id": "device_001"
    
    // 或录像回放模式
    "file_id": "rec_001",
    "start_position": 0.0,
    "playback_rate": 1.0
  },
  "config": {
    "client_id": "web_client_001",
    "low_latency_mode": true,
    "target_latency_ms": 100
  }
}
```

**参数说明**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| mode | string | 是 | 流模式：`live`（直通）或`playback`（回放） |
| source.device_id | string | 条件 | 设备ID（直通模式必填） |
| source.file_id | string | 条件 | 文件ID（回放模式必填） |
| source.start_position | number | 否 | 起始位置（秒），默认0.0 |
| source.playback_rate | number | 否 | 播放速率，默认1.0 |
| config.client_id | string | 是 | 客户端标识符 |
| config.low_latency_mode | boolean | 否 | 是否启用低延迟模式，默认true |
| config.target_latency_ms | number | 否 | 目标延迟（毫秒），默认100 |

#### 响应

**成功响应** (200 OK):

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "stream_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/segments",
  "control_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/control",
  "estimated_latency_ms": 100,
  "stream_info": {
    "mode": "playback",
    "state": "initializing",
    "resolution": "1920x1080",
    "frame_rate": 30.0,
    "bitrate": 5000000,
    "duration": 120.5,
    "current_position": 0.0,
    "playback_rate": 1.0
  }
}
```

**错误响应**：

```json
{
  "error": "DeviceNotConnected",
  "message": "Device device_001 is not connected",
  "code": 404
}
```

#### 示例

**直通播放**：

```bash
curl -X POST https://localhost:8443/api/v1/stream/start \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "live",
    "source": {
      "device_id": "device_001"
    },
    "config": {
      "client_id": "web_client_001",
      "low_latency_mode": true,
      "target_latency_ms": 50
    }
  }'
```

**录像回放**：

```bash
curl -X POST https://localhost:8443/api/v1/stream/start \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "playback",
    "source": {
      "file_id": "rec_001",
      "start_position": 10.0,
      "playback_rate": 1.0
    },
    "config": {
      "client_id": "web_client_001",
      "low_latency_mode": true,
      "target_latency_ms": 100
    }
  }'
```

### 3.2 停止流会话

停止一个正在运行的流会话。

#### 请求

```http
DELETE /api/v1/stream/{session_id} HTTP/3
```

**路径参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | UUID | 会话ID |

#### 响应

**成功响应** (200 OK):

```json
{
  "status": "stopped",
  "message": "Stream session stopped successfully"
}
```

#### 示例

```bash
curl -X DELETE https://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000
```

---

## 4. 播放控制API

### 4.1 控制播放

控制流会话的播放状态（暂停、恢复、定位、倍速、停止）。

#### 请求

```http
POST /api/v1/stream/{session_id}/control HTTP/3
Content-Type: application/json
```

**路径参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | UUID | 会话ID |

**请求体**：

```json
// 暂停
{"command": "pause"}

// 恢复
{"command": "resume"}

// 定位（仅回放模式）
{"command": "seek", "position": 30.0}

// 倍速（仅回放模式）
{"command": "set_rate", "rate": 2.0}

// 停止
{"command": "stop"}
```

**参数说明**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| command | string | 是 | 控制命令：`pause`、`resume`、`seek`、`set_rate`、`stop` |
| position | number | 条件 | 目标位置（秒），seek命令必填 |
| rate | number | 条件 | 播放速率（0.25-4.0），set_rate命令必填 |

#### 响应

**成功响应** (200 OK):

```json
{
  "status": "success",
  "current_state": "paused",
  "current_position": 15.5,
  "playback_rate": 1.0,
  "message": "Command executed successfully"
}
```

**错误响应**：

```json
{
  "error": "OperationNotSupported",
  "message": "Seek operation is not supported in live mode",
  "code": 400
}
```

#### 示例

**暂停播放**：

```bash
curl -X POST https://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/control \
  -H "Content-Type: application/json" \
  -d '{"command": "pause"}'
```

**定位到30秒**：

```bash
curl -X POST https://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/control \
  -H "Content-Type: application/json" \
  -d '{"command": "seek", "position": 30.0}'
```

**2倍速播放**：

```bash
curl -X POST https://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/control \
  -H "Content-Type: application/json" \
  -d '{"command": "set_rate", "rate": 2.0}'
```

---

## 5. 状态查询API

### 5.1 获取流状态

查询流会话的当前状态和统计信息。

#### 请求

```http
GET /api/v1/stream/{session_id}/status HTTP/3
```

**路径参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | UUID | 会话ID |

#### 响应

**成功响应** (200 OK):

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "mode": "playback",
  "state": "streaming",
  "current_position": 15.5,
  "playback_rate": 1.0,
  "stats": {
    "total_segments": 1500,
    "total_bytes": 15728640,
    "average_latency_ms": 95.5,
    "current_latency_ms": 87.2,
    "min_latency_ms": 65.0,
    "max_latency_ms": 150.0,
    "p50_latency_ms": 90.0,
    "p95_latency_ms": 120.0,
    "p99_latency_ms": 140.0,
    "throughput_mbps": 5.2,
    "packet_loss_rate": 0.01
  },
  "stream_info": {
    "resolution": "1920x1080",
    "frame_rate": 30.0,
    "bitrate": 5000000,
    "duration": 120.5
  }
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| session_id | UUID | 会话ID |
| mode | string | 流模式（live/playback） |
| state | string | 当前状态（streaming/paused/stopped） |
| current_position | number | 当前位置（秒） |
| playback_rate | number | 播放速率 |
| stats.total_segments | number | 总分片数 |
| stats.total_bytes | number | 总字节数 |
| stats.average_latency_ms | number | 平均延迟（毫秒） |
| stats.current_latency_ms | number | 当前延迟（毫秒） |
| stats.min_latency_ms | number | 最小延迟（毫秒） |
| stats.max_latency_ms | number | 最大延迟（毫秒） |
| stats.p50_latency_ms | number | P50延迟（毫秒） |
| stats.p95_latency_ms | number | P95延迟（毫秒） |
| stats.p99_latency_ms | number | P99延迟（毫秒） |
| stats.throughput_mbps | number | 吞吐量（Mbps） |
| stats.packet_loss_rate | number | 丢包率 |

#### 示例

```bash
curl https://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/status
```

---

## 6. SSE媒体流

### 6.1 接收视频分片

通过SSE（Server-Sent Events）接收视频分片流。

#### 请求

```http
GET /api/v1/stream/{session_id}/segments HTTP/3
Accept: text/event-stream
```

**路径参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | UUID | 会话ID |

#### 响应

**SSE事件流**：

```
event: segment
data: {
  "segment_id": "650e8400-e29b-41d4-a716-446655440001",
  "timestamp": 15.5,
  "duration": 0.033,
  "is_keyframe": true,
  "format": "fmp4",
  "data": "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAAAIZnJlZQAA..."
}

event: segment
data: {
  "segment_id": "650e8400-e29b-41d4-a716-446655440002",
  "timestamp": 15.533,
  "duration": 0.033,
  "is_keyframe": false,
  "format": "fmp4",
  "data": "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAAAIZnJlZQAA..."
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| segment_id | UUID | 分片唯一标识符 |
| timestamp | number | 相对时间戳（秒） |
| duration | number | 分片时长（秒） |
| is_keyframe | boolean | 是否为关键帧 |
| format | string | 分片格式（fmp4/h264/mp4） |
| data | string | Base64编码的视频数据 |

#### 示例

**JavaScript (EventSource)**：

```javascript
const sessionId = '550e8400-e29b-41d4-a716-446655440000'
const streamUrl = `https://localhost:8443/api/v1/stream/${sessionId}/segments`

const eventSource = new EventSource(streamUrl)

eventSource.addEventListener('segment', (event) => {
  const segment = JSON.parse(event.data)
  console.log('Received segment:', segment.segment_id)
  
  // 解码Base64数据
  const binaryData = atob(segment.data)
  const bytes = new Uint8Array(binaryData.length)
  for (let i = 0; i < binaryData.length; i++) {
    bytes[i] = binaryData.charCodeAt(i)
  }
  
  // 追加到SourceBuffer
  sourceBuffer.appendBuffer(bytes.buffer)
})

eventSource.onerror = (error) => {
  console.error('SSE error:', error)
  eventSource.close()
}
```

---

## 7. 错误处理

### 7.1 错误响应格式

所有错误响应遵循统一格式：

```json
{
  "error": "ErrorType",
  "message": "Human readable error message",
  "code": 400,
  "details": {
    "field": "Additional error details"
  }
}
```

### 7.2 HTTP状态码

| 状态码 | 说明 | 示例 |
|--------|------|------|
| 200 | 成功 | 请求成功处理 |
| 400 | 请求错误 | 参数无效、操作不支持 |
| 404 | 未找到 | 会话不存在、设备未连接 |
| 409 | 冲突 | 会话已存在 |
| 500 | 服务器错误 | 内部错误 |
| 503 | 服务不可用 | 系统过载 |

### 7.3 错误类型

| 错误类型 | HTTP状态码 | 说明 |
|----------|-----------|------|
| DeviceNotConnected | 404 | 设备未连接 |
| DeviceOffline | 404 | 设备离线 |
| FileNotFound | 404 | 文件未找到 |
| SessionNotFound | 404 | 会话不存在 |
| OperationNotSupported | 400 | 操作不支持 |
| InvalidSeekPosition | 400 | 无效的定位位置 |
| InvalidPlaybackRate | 400 | 无效的播放速率 |
| TooManySessions | 503 | 会话过多 |
| ConnectionLost | 500 | 连接丢失 |
| Internal | 500 | 内部错误 |

### 7.4 错误处理最佳实践

#### 7.4.1 自动重试

对于临时性错误（如网络错误），建议使用指数退避策略重试：

```javascript
async function startStreamWithRetry(config, maxRetries = 5) {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      const response = await fetch('/api/v1/stream/start', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(config)
      })
      
      if (response.ok) {
        return await response.json()
      }
      
      const error = await response.json()
      
      // 不可重试的错误
      if (error.code === 400 || error.code === 404) {
        throw new Error(error.message)
      }
      
    } catch (error) {
      if (attempt === maxRetries - 1) {
        throw error
      }
      
      // 指数退避
      const delay = Math.min(1000 * Math.pow(2, attempt), 30000)
      await new Promise(resolve => setTimeout(resolve, delay))
    }
  }
}
```

#### 7.4.2 SSE重连

SSE连接断开时自动重连：

```javascript
function connectSSE(streamUrl, maxRetries = 5) {
  let retryCount = 0
  
  function connect() {
    const eventSource = new EventSource(streamUrl)
    
    eventSource.onopen = () => {
      console.log('SSE connected')
      retryCount = 0 // 重置重试计数
    }
    
    eventSource.onerror = (error) => {
      console.error('SSE error:', error)
      eventSource.close()
      
      if (retryCount < maxRetries) {
        retryCount++
        const delay = Math.min(1000 * Math.pow(2, retryCount - 1), 30000)
        console.log(`Reconnecting in ${delay}ms (attempt ${retryCount}/${maxRetries})`)
        setTimeout(connect, delay)
      } else {
        console.error('Max retries reached')
      }
    }
    
    return eventSource
  }
  
  return connect()
}
```

---

## 8. 使用示例

### 8.1 完整的直通播放示例

```javascript
// 1. 启动直通播放会话
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
  
  const data = await response.json()
  return data
}

// 2. 连接SSE接收视频分片
function connectStream(sessionId) {
  const streamUrl = `https://localhost:8443/api/v1/stream/${sessionId}/segments`
  const eventSource = new EventSource(streamUrl)
  
  eventSource.addEventListener('segment', (event) => {
    const segment = JSON.parse(event.data)
    appendSegmentToPlayer(segment)
  })
  
  return eventSource
}

// 3. 追加分片到MSE播放器
function appendSegmentToPlayer(segment) {
  if (sourceBuffer.updating) {
    segmentQueue.push(segment)
    return
  }
  
  const binaryData = base64ToArrayBuffer(segment.data)
  sourceBuffer.appendBuffer(binaryData)
}

// 4. 控制播放
async function pauseStream(sessionId) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({command: 'pause'})
  })
}

// 5. 停止播放
async function stopStream(sessionId) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}`, {
    method: 'DELETE'
  })
}

// 使用示例
const session = await startLiveStream('device_001')
const eventSource = connectStream(session.session_id)

// 10秒后暂停
setTimeout(() => pauseStream(session.session_id), 10000)

// 20秒后停止
setTimeout(() => {
  stopStream(session.session_id)
  eventSource.close()
}, 20000)
```

### 8.2 完整的录像回放示例

```javascript
// 1. 启动录像回放会话
async function startPlayback(fileId, startPosition = 0) {
  const response = await fetch('https://localhost:8443/api/v1/stream/start', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      mode: 'playback',
      source: {
        file_id: fileId,
        start_position: startPosition,
        playback_rate: 1.0
      },
      config: {
        client_id: 'web_client_001',
        low_latency_mode: true,
        target_latency_ms: 100
      }
    })
  })
  
  return await response.json()
}

// 2. 定位到指定位置
async function seekTo(sessionId, position) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      command: 'seek',
      position: position
    })
  })
}

// 3. 设置播放速率
async function setPlaybackRate(sessionId, rate) {
  await fetch(`https://localhost:8443/api/v1/stream/${sessionId}/control`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      command: 'set_rate',
      rate: rate
    })
  })
}

// 4. 查询播放状态
async function getStatus(sessionId) {
  const response = await fetch(
    `https://localhost:8443/api/v1/stream/${sessionId}/status`
  )
  return await response.json()
}

// 使用示例
const session = await startPlayback('rec_001', 0)
const eventSource = connectStream(session.session_id)

// 定位到30秒
await seekTo(session.session_id, 30.0)

// 2倍速播放
await setPlaybackRate(session.session_id, 2.0)

// 查询状态
const status = await getStatus(session.session_id)
console.log('Current position:', status.current_position)
console.log('Average latency:', status.stats.average_latency_ms, 'ms')
```

---

## 附录

### A. 支持的播放速率

| 速率 | 说明 | 支持模式 |
|------|------|----------|
| 0.25x | 1/4倍速 | 仅回放 |
| 0.5x | 半速 | 仅回放 |
| 0.75x | 3/4倍速 | 仅回放 |
| 1.0x | 正常速度 | 直通和回放 |
| 1.25x | 1.25倍速 | 仅回放 |
| 1.5x | 1.5倍速 | 仅回放 |
| 2.0x | 2倍速 | 仅回放 |
| 4.0x | 4倍速 | 仅回放 |

### B. 支持的视频格式

| 格式 | MIME类型 | 说明 |
|------|----------|------|
| fMP4 | video/mp4; codecs="avc1.64001f" | 分片MP4（推荐） |
| H.264 | video/h264 | H.264裸流（需转换） |
| MP4 | video/mp4 | 标准MP4 |

### C. 性能指标参考

| 指标 | 直通播放 | 录像回放 |
|------|----------|----------|
| 端到端延迟 | 50-100ms | 100-200ms |
| 平台端处理延迟 | 2-5ms | 2-5ms |
| 目标缓冲 | 100-500ms | 500-2000ms |
| 分片大小 | 8-32KB | 8-32KB |

---

**文档版本**: v1.0
**最后更新**: 2025-12-13
**维护团队**: 系统架构团队
