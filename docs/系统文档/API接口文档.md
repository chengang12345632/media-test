# HTTP3/QUIC视频流传输系统 - API接口文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 文档版本 | v1.0 |
| API版本 | v1 |
| 创建日期 | 2025-12-12 |
| 基础URL | https://localhost:8443/api/v1 |
| 协议 | HTTP/3 over QUIC |

## 目录

1. [概述](#概述)
2. [认证机制](#认证机制)
3. [设备管理API](#设备管理api)
4. [录像管理API](#录像管理api)
5. [直通播放API](#直通播放api)
6. [录像回放API](#录像回放api)
7. [WebSocket事件API](#websocket事件api)
8. [错误码定义](#错误码定义)
9. [数据模型](#数据模型)

---

## 概述

### API特性

- **协议**: HTTP/3 over QUIC
- **数据格式**: JSON (application/json)
- **字符编码**: UTF-8
- **认证方式**: Demo模式无需认证（生产环境建议使用JWT）
- **时间格式**: ISO 8601 (YYYY-MM-DDTHH:mm:ss.sssZ)
- **分页**: 支持 page 和 page_size 参数

### 通用响应格式

**成功响应**：
```json
{
  "status": "success",
  "data": { /* 实际数据 */ },
  "timestamp": "2025-12-12T08:00:00.000Z"
}
```

**错误响应**：
```json
{
  "status": "error",
  "error": {
    "code": "DEVICE_NOT_FOUND",
    "message": "设备不存在",
    "details": "Device with ID 'device_001' not found"
  },
  "timestamp": "2025-12-12T08:00:00.000Z"
}
```

---

## 认证机制

### Demo模式（当前）

Demo版本无需认证，所有API可直接访问。


### 生产环境（建议）

```http
Authorization: Bearer <JWT_TOKEN>
```

**获取Token**：
```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "password"
}
```

---

## 设备管理API

### 1. 获取设备列表

获取所有已连接的设备信息。

**请求**：
```http
GET /api/v1/devices
```

**查询参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| status | string | 否 | 过滤状态: online, offline, all |
| page | integer | 否 | 页码，默认1 |
| page_size | integer | 否 | 每页数量，默认20 |

**响应示例**：
```json
{
  "status": "success",
  "data": {
    "devices": [
      {
        "device_id": "device_001",
        "device_name": "摄像头-01",
        "device_type": "camera",
        "connection_status": "online",
        "connection_time": "2025-12-12T01:20:00Z",
        "last_heartbeat": "2025-12-12T08:00:00Z",
        "capabilities": {
          "max_resolution": "1920x1080",
          "supported_formats": ["h264", "mp4"],
          "max_bitrate": 10000000,
          "supports_playback_control": true,
          "supports_recording": true
        },
        "network_stats": {
          "latency_ms": 15,
          "packet_loss_rate": 0.01,
          "bandwidth_mbps": 8.5,
          "jitter_ms": 2.5
        },
        "current_performance": {
          "cpu_usage": 25.5,
          "memory_usage_mb": 128.5,
          "network_usage_mbps": 8.5,
          "temperature": 45.2
        }
      }
    ],
    "pagination": {
      "current_page": 1,
      "page_size": 20,
      "total_count": 1,
      "total_pages": 1
    },
    "summary": {
      "total_count": 1,
      "online_count": 1,
      "offline_count": 0
    }
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```


### 2. 获取设备详情

获取指定设备的详细信息。

**请求**：
```http
GET /api/v1/devices/{device_id}
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| device_id | string | 设备ID |

**响应示例**：
```json
{
  "status": "success",
  "data": {
    "device_id": "device_001",
    "device_name": "摄像头-01",
    "device_type": "camera",
    "connection_status": "online",
    "connection_time": "2025-12-12T01:20:00Z",
    "last_heartbeat": "2025-12-12T08:00:00Z",
    "hardware_info": {
      "model": "SimulatedCamera",
      "firmware_version": "1.0.0",
      "serial_number": "SIM001",
      "manufacturer": "Demo"
    },
    "capabilities": {
      "max_resolution": "1920x1080",
      "supported_formats": ["h264", "mp4"],
      "max_bitrate": 10000000,
      "supports_playback_control": true,
      "supports_recording": true,
      "supports_audio": true
    },
    "current_sessions": [
      {
        "session_id": "550e8400-e29b-41d4-a716-446655440000",
        "stream_type": "live",
        "start_time": "2025-12-12T07:30:00Z",
        "client_count": 2,
        "bitrate": 5000000
      }
    ],
    "statistics": {
      "total_uptime_hours": 168.5,
      "total_data_transmitted_gb": 1250.8,
      "average_bitrate_mbps": 6.2,
      "total_sessions": 450
    }
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

**错误响应**：
```json
{
  "status": "error",
  "error": {
    "code": "DEVICE_NOT_FOUND",
    "message": "设备不存在",
    "details": "Device with ID 'device_999' not found"
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

---

## 录像管理API

### 3. 获取设备录像列表

获取指定设备的录像文件列表。

**请求**：
```http
GET /api/v1/devices/{device_id}/recordings
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| device_id | string | 设备ID |

**查询参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| start_time | string | 否 | 开始时间 (ISO 8601) |
| end_time | string | 否 | 结束时间 (ISO 8601) |
| format | string | 否 | 文件格式过滤: h264,mp4 |
| page | integer | 否 | 页码，默认1 |
| page_size | integer | 否 | 每页数量，默认20 |

**请求示例**：
```http
GET /api/v1/devices/device_001/recordings?start_time=2025-12-10T00:00:00Z&end_time=2025-12-11T23:59:59Z&page=1&page_size=20
```


**响应示例**：
```json
{
  "status": "success",
  "data": {
    "recordings": [
      {
        "file_id": "rec_001",
        "device_id": "device_001",
        "file_name": "video_20251211_013000.h264",
        "file_path": "recordings/device_001/2025/12/11/video_20251211_013000.h264",
        "file_size": 1048576000,
        "duration": 3600.0,
        "format": "h264",
        "resolution": "1920x1080",
        "bitrate": 5000000,
        "frame_rate": 30.0,
        "created_time": "2025-12-11T01:30:00Z",
        "modified_time": "2025-12-11T02:30:00Z",
        "checksum": "sha256:abc123...",
        "thumbnail_url": "/api/v1/recordings/rec_001/thumbnail",
        "preview_url": "/api/v1/recordings/rec_001/preview",
        "download_url": "/api/v1/recordings/rec_001/download",
        "stream_url": "/api/v1/recordings/rec_001/stream",
        "metadata": {
          "codec": "H.264",
          "profile": "High",
          "level": "4.1",
          "has_audio": true,
          "audio_codec": "AAC",
          "audio_sample_rate": 48000
        }
      }
    ],
    "pagination": {
      "current_page": 1,
      "page_size": 20,
      "total_count": 150,
      "total_pages": 8
    },
    "summary": {
      "total_files": 150,
      "total_size_gb": 156.8,
      "total_duration_hours": 450.5,
      "date_range": {
        "earliest": "2025-12-01T00:00:00Z",
        "latest": "2025-12-11T02:30:00Z"
      }
    }
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

### 4. 下载录像文件

下载指定的录像文件。

**请求**：
```http
GET /api/v1/recordings/{file_id}/download
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| file_id | string | 录像文件ID |

**响应**：
- Content-Type: application/octet-stream
- Content-Disposition: attachment; filename="video_20251211_013000.h264"
- 二进制文件流

---

## 直通播放API

### 5. 开始直通播放

启动设备的实时视频流播放。

**请求**：
```http
POST /api/v1/devices/{device_id}/live-stream
Content-Type: application/json
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| device_id | string | 设备ID |

**请求体**：
```json
{
  "client_id": "web_client_001",
  "quality_preference": "auto",
  "buffer_size": 30,
  "low_latency_mode": true,
  "audio_enabled": true,
  "timestamp": 1702259825000
}
```

**请求参数说明**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| client_id | string | 是 | 客户端唯一标识 |
| quality_preference | string | 否 | 画质偏好: auto, original, high, medium, low |
| buffer_size | integer | 否 | 缓冲区大小（秒），默认30 |
| low_latency_mode | boolean | 否 | 低延迟模式，默认true |
| audio_enabled | boolean | 否 | 是否启用音频，默认true |
| timestamp | integer | 是 | 客户端时间戳（毫秒） |


**响应示例**：
```json
{
  "status": "success",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "stream_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/segments",
    "control_url": "/api/v1/playback/550e8400-e29b-41d4-a716-446655440000/control",
    "status_url": "/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/status",
    "websocket_url": "wss://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/events",
    "estimated_latency_ms": 85,
    "stream_info": {
      "resolution": "1920x1080",
      "frame_rate": 30.0,
      "bitrate": 5000000,
      "format": "h264",
      "has_audio": true
    }
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

### 6. 停止直通播放

停止正在进行的直通播放。

**请求**：
```http
DELETE /api/v1/stream/{session_id}
Content-Type: application/json
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | string | 会话ID |

**请求体**：
```json
{
  "client_id": "web_client_001",
  "reason": "user_stop",
  "timestamp": 1702259825000
}
```

**响应示例**：
```json
{
  "status": "success",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "stopped_at": "2025-12-12T08:05:00Z",
    "duration_seconds": 300,
    "statistics": {
      "total_segments": 9000,
      "total_bytes": 187500000,
      "average_bitrate_mbps": 5.0,
      "average_latency_ms": 82
    }
  },
  "timestamp": "2025-12-12T08:05:00Z"
}
```

---

## 录像回放API

### 7. 开始录像回放

启动录像文件的回放。

**请求**：
```http
POST /api/v1/recordings/{file_id}/playback
Content-Type: application/json
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| file_id | string | 录像文件ID |

**请求体**：
```json
{
  "client_id": "web_client_001",
  "start_position": 0.0,
  "quality": "high",
  "playback_rate": 1.0,
  "audio_enabled": true,
  "timestamp": 1702259825000
}
```

**请求参数说明**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| client_id | string | 是 | 客户端唯一标识 |
| start_position | number | 否 | 起始位置（秒），默认0.0 |
| quality | string | 否 | 画质: original, high, medium, low |
| playback_rate | number | 否 | 播放速率: 0.25-4.0，默认1.0 |
| audio_enabled | boolean | 否 | 是否启用音频，默认true |
| timestamp | integer | 是 | 客户端时间戳（毫秒） |


**响应示例**：
```json
{
  "status": "success",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440001",
    "playback_url": "/api/v1/playback/550e8400-e29b-41d4-a716-446655440001/segments",
    "control_url": "/api/v1/playback/550e8400-e29b-41d4-a716-446655440001/control",
    "status_url": "/api/v1/playback/550e8400-e29b-41d4-a716-446655440001/status",
    "websocket_url": "wss://localhost:8443/api/v1/playback/550e8400-e29b-41d4-a716-446655440001/events",
    "file_info": {
      "duration": 3600.0,
      "resolution": "1920x1080",
      "frame_rate": 30.0,
      "bitrate": 5000000,
      "format": "h264",
      "has_audio": true
    }
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

### 8. 回放控制

控制录像回放（播放、暂停、拖动、倍速等）。

**请求**：
```http
POST /api/v1/playback/{session_id}/control
Content-Type: application/json
```

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| session_id | string | 回放会话ID |

**控制命令类型**：

#### 8.1 播放
```json
{
  "command": "play",
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

#### 8.2 暂停
```json
{
  "command": "pause",
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

#### 8.3 恢复播放
```json
{
  "command": "resume",
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

#### 8.4 拖动定位
```json
{
  "command": "seek",
  "position": 1800.0,
  "accurate": true,
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

**参数说明**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| position | number | 是 | 目标位置（秒） |
| accurate | boolean | 否 | 是否精确定位到关键帧，默认true |

#### 8.5 调整播放速率
```json
{
  "command": "set_rate",
  "rate": 2.0,
  "maintain_audio": false,
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

**参数说明**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| rate | number | 是 | 播放速率: 0.25, 0.5, 1.0, 1.5, 2.0, 4.0 |
| maintain_audio | boolean | 否 | 是否保持音频，默认false（>1.5x时） |

#### 8.6 停止播放
```json
{
  "command": "stop",
  "reason": "user_request",
  "client_id": "web_client_001",
  "timestamp": 1702259825000
}
```

**响应示例**：
```json
{
  "status": "success",
  "data": {
    "command": "seek",
    "executed_at": "2025-12-12T08:10:00Z",
    "result": {
      "requested_position": 1800.0,
      "actual_position": 1799.8,
      "keyframe_position": 1799.8,
      "execution_time_ms": 15
    }
  },
  "timestamp": "2025-12-12T08:10:00Z"
}
```


---

## WebSocket事件API

### 9. WebSocket连接

建立WebSocket连接以接收实时事件推送。

**连接URL**：
```
wss://localhost:8443/api/v1/stream/{session_id}/events
wss://localhost:8443/api/v1/playback/{session_id}/events
```

**连接示例（JavaScript）**：
```javascript
const ws = new WebSocket('wss://localhost:8443/api/v1/stream/550e8400-e29b-41d4-a716-446655440000/events');

ws.onopen = () => {
  console.log('WebSocket连接已建立');
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  handleEvent(message);
};

ws.onerror = (error) => {
  console.error('WebSocket错误:', error);
};

ws.onclose = () => {
  console.log('WebSocket连接已关闭');
};
```

### 10. WebSocket事件类型

#### 10.1 视频分片接收通知
```json
{
  "event_type": "segment_received",
  "data": {
    "segment_id": "550e8400-e29b-41d4-a716-446655440002",
    "timestamp": 15.5,
    "size": 2048,
    "latency_ms": 12,
    "is_keyframe": true,
    "frame_count": 1
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

#### 10.2 播放状态变化通知
```json
{
  "event_type": "playback_status_changed",
  "data": {
    "status": "playing",
    "position": 30.5,
    "rate": 1.0,
    "buffer_health": 25.5,
    "buffer_duration": 30.0
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

**状态值**：
- `playing` - 正在播放
- `paused` - 已暂停
- `stopped` - 已停止
- `seeking` - 正在定位
- `buffering` - 正在缓冲

#### 10.3 设备连接状态变化通知
```json
{
  "event_type": "device_status_changed",
  "data": {
    "device_id": "device_001",
    "status": "offline",
    "reason": "network_timeout",
    "last_seen": "2025-12-12T07:59:50Z"
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

#### 10.4 延迟告警通知
```json
{
  "event_type": "latency_alert",
  "data": {
    "current_latency_ms": 250,
    "threshold_ms": 200,
    "severity": "warning",
    "affected_session": "550e8400-e29b-41d4-a716-446655440000"
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

**严重级别**：
- `info` - 信息
- `warning` - 警告
- `error` - 错误
- `critical` - 严重

#### 10.5 缓冲区状态通知
```json
{
  "event_type": "buffer_status",
  "data": {
    "buffer_duration": 25.5,
    "target_buffer": 30.0,
    "buffer_health": "healthy",
    "is_buffering": false
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

**缓冲区健康度**：
- `healthy` - 健康（>80%）
- `low` - 偏低（30-80%）
- `critical` - 严重不足（<30%）

#### 10.6 质量切换通知
```json
{
  "event_type": "quality_changed",
  "data": {
    "old_quality": "high",
    "new_quality": "medium",
    "reason": "bandwidth_limitation",
    "new_bitrate": 3000000
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

### 11. WebSocket心跳

**客户端发送**：
```json
{
  "type": "ping",
  "timestamp": 1702259825000
}
```

**服务端响应**：
```json
{
  "type": "pong",
  "timestamp": "2025-12-12T08:00:00Z",
  "server_time": 1702259825000
}
```

**心跳间隔**：建议每30秒发送一次心跳。


---

## 错误码定义

### HTTP状态码

| 状态码 | 说明 |
|--------|------|
| 200 | 请求成功 |
| 201 | 创建成功 |
| 204 | 删除成功（无内容） |
| 400 | 请求参数错误 |
| 401 | 未授权 |
| 403 | 禁止访问 |
| 404 | 资源不存在 |
| 409 | 资源冲突 |
| 429 | 请求过于频繁 |
| 500 | 服务器内部错误 |
| 503 | 服务不可用 |

### 业务错误码

| 错误码 | HTTP状态 | 说明 |
|--------|----------|------|
| DEVICE_NOT_FOUND | 404 | 设备不存在 |
| DEVICE_OFFLINE | 503 | 设备离线 |
| DEVICE_BUSY | 409 | 设备忙碌 |
| RECORDING_NOT_FOUND | 404 | 录像文件不存在 |
| FILE_NOT_ACCESSIBLE | 403 | 文件无法访问 |
| SESSION_NOT_FOUND | 404 | 会话不存在 |
| SESSION_EXPIRED | 404 | 会话已过期 |
| INVALID_COMMAND | 400 | 无效的控制命令 |
| INVALID_PARAMETER | 400 | 参数错误 |
| STREAM_ERROR | 500 | 流传输错误 |
| NETWORK_ERROR | 503 | 网络错误 |
| INSUFFICIENT_BANDWIDTH | 503 | 带宽不足 |
| BUFFER_OVERFLOW | 500 | 缓冲区溢出 |
| UNSUPPORTED_FORMAT | 400 | 不支持的格式 |
| RATE_LIMIT_EXCEEDED | 429 | 超过速率限制 |

### 错误响应示例

```json
{
  "status": "error",
  "error": {
    "code": "DEVICE_OFFLINE",
    "message": "设备离线",
    "details": "Device 'device_001' is currently offline. Last seen: 2025-12-12T07:50:00Z",
    "retry_after": 30,
    "documentation_url": "https://docs.example.com/errors/DEVICE_OFFLINE"
  },
  "timestamp": "2025-12-12T08:00:00Z"
}
```

---

## 数据模型

### DeviceInfo - 设备信息

```typescript
interface DeviceInfo {
  device_id: string;              // 设备ID
  device_name: string;            // 设备名称
  device_type: DeviceType;        // 设备类型
  connection_status: ConnectionStatus; // 连接状态
  connection_time: string;        // 连接时间 (ISO 8601)
  last_heartbeat: string;         // 最后心跳时间
  hardware_info: HardwareInfo;    // 硬件信息
  capabilities: DeviceCapabilities; // 设备能力
  network_stats: NetworkStats;    // 网络统计
  current_performance: PerformanceMetrics; // 性能指标
}

type DeviceType = 'camera' | 'recorder' | 'simulator' | 'gateway';
type ConnectionStatus = 'online' | 'offline' | 'reconnecting';
```

### RecordingInfo - 录像信息

```typescript
interface RecordingInfo {
  file_id: string;                // 文件ID
  device_id: string;              // 设备ID
  file_name: string;              // 文件名
  file_path: string;              // 文件路径
  file_size: number;              // 文件大小（字节）
  duration: number;               // 时长（秒）
  format: string;                 // 格式: h264, mp4
  resolution: string;             // 分辨率: 1920x1080
  bitrate: number;                // 比特率（bps）
  frame_rate: number;             // 帧率
  created_time: string;           // 创建时间
  modified_time: string;          // 修改时间
  checksum: string;               // 校验和
  thumbnail_url: string;          // 缩略图URL
  download_url: string;           // 下载URL
  metadata: VideoMetadata;        // 视频元数据
}
```

### StreamSession - 流会话

```typescript
interface StreamSession {
  session_id: string;             // 会话ID (UUID)
  device_id?: string;             // 设备ID（直通播放）
  file_id?: string;               // 文件ID（录像回放）
  stream_type: StreamType;        // 流类型
  client_id: string;              // 客户端ID
  start_time: string;             // 开始时间
  stream_url: string;             // 流URL
  control_url: string;            // 控制URL
  websocket_url: string;          // WebSocket URL
  stream_info: StreamInfo;        // 流信息
}

type StreamType = 'live' | 'playback';
```

### PlaybackControl - 播放控制

```typescript
interface PlaybackControl {
  command: PlaybackCommand;       // 控制命令
  client_id: string;              // 客户端ID
  timestamp: number;              // 时间戳（毫秒）
  position?: number;              // 位置（秒）- seek命令
  rate?: number;                  // 速率 - set_rate命令
  accurate?: boolean;             // 精确定位 - seek命令
  maintain_audio?: boolean;       // 保持音频 - set_rate命令
  reason?: string;                // 原因 - stop命令
}

type PlaybackCommand = 'play' | 'pause' | 'resume' | 'seek' | 'set_rate' | 'stop';
```

### NetworkStats - 网络统计

```typescript
interface NetworkStats {
  latency_ms: number;             // 延迟（毫秒）
  packet_loss_rate: number;       // 丢包率 (0-1)
  bandwidth_mbps: number;         // 带宽（Mbps）
  jitter_ms: number;              // 抖动（毫秒）
}
```

### PerformanceMetrics - 性能指标

```typescript
interface PerformanceMetrics {
  cpu_usage: number;              // CPU使用率 (%)
  memory_usage_mb: number;        // 内存使用（MB）
  network_usage_mbps: number;     // 网络使用（Mbps）
  temperature?: number;           // 温度（℃）
}
```

---

## 使用示例

### 完整的直通播放流程

```javascript
// 1. 获取设备列表
const devicesResponse = await fetch('https://localhost:8443/api/v1/devices');
const devices = await devicesResponse.json();
const device = devices.data.devices[0];

// 2. 开始直通播放
const liveStreamResponse = await fetch(
  `https://localhost:8443/api/v1/devices/${device.device_id}/live-stream`,
  {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_id: 'web_client_001',
      quality_preference: 'auto',
      low_latency_mode: true,
      audio_enabled: true,
      timestamp: Date.now()
    })
  }
);
const streamSession = await liveStreamResponse.json();

// 3. 建立WebSocket连接
const ws = new WebSocket(streamSession.data.websocket_url);
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('收到事件:', message);
};

// 4. 接收视频流（使用MediaSource API）
const mediaSource = new MediaSource();
videoElement.src = URL.createObjectURL(mediaSource);

// 5. 停止播放
await fetch(
  `https://localhost:8443/api/v1/stream/${streamSession.data.session_id}`,
  {
    method: 'DELETE',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_id: 'web_client_001',
      reason: 'user_stop',
      timestamp: Date.now()
    })
  }
);
```

### 完整的录像回放流程

```javascript
// 1. 获取录像列表
const recordingsResponse = await fetch(
  `https://localhost:8443/api/v1/devices/${device_id}/recordings?page=1&page_size=20`
);
const recordings = await recordingsResponse.json();
const recording = recordings.data.recordings[0];

// 2. 开始回放
const playbackResponse = await fetch(
  `https://localhost:8443/api/v1/recordings/${recording.file_id}/playback`,
  {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_id: 'web_client_001',
      start_position: 0.0,
      quality: 'high',
      playback_rate: 1.0,
      audio_enabled: true,
      timestamp: Date.now()
    })
  }
);
const playbackSession = await playbackResponse.json();

// 3. 播放控制 - 拖动到30秒
await fetch(
  `https://localhost:8443/api/v1/playback/${playbackSession.data.session_id}/control`,
  {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      command: 'seek',
      position: 30.0,
      accurate: true,
      client_id: 'web_client_001',
      timestamp: Date.now()
    })
  }
);

// 4. 播放控制 - 2倍速播放
await fetch(
  `https://localhost:8443/api/v1/playback/${playbackSession.data.session_id}/control`,
  {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      command: 'set_rate',
      rate: 2.0,
      maintain_audio: false,
      client_id: 'web_client_001',
      timestamp: Date.now()
    })
  }
);
```

---

## 附录

### A. 速率限制

| 端点 | 限制 |
|------|------|
| GET /api/v1/devices | 100次/分钟 |
| POST /api/v1/devices/{device_id}/live-stream | 10次/分钟 |
| POST /api/v1/recordings/{file_id}/playback | 20次/分钟 |
| POST /api/v1/playback/{session_id}/control | 60次/分钟 |

### B. 浏览器兼容性

| 浏览器 | 最低版本 | HTTP/3支持 |
|--------|---------|-----------|
| Chrome | 87+ | ✅ |
| Edge | 87+ | ✅ |
| Firefox | 88+ | ✅ |
| Safari | 14+ | ⚠️ 部分支持 |

### C. 相关文档

- [系统架构设计文档](../系统架构设计文档.md)
- [部署指南](./部署指南.md)
- [开发手册](./开发手册.md)
- [快速开始指南](./快速开始指南.md)

---

**文档版本**: v1.0  
**最后更新**: 2025-12-12  
**维护者**: 系统架构团队
