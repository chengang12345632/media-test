# 平台和前端调整需求分析

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | 设备端功能增强后的平台和前端调整 |
| 创建日期 | 2025-12-14 |
| 状态 | 分析文档 |
| 优先级 | 高 |

## 简介

本文档分析了在设备端（Device-Simulator）实现以下功能后，平台端（Platform-Server）和前端（Web-Frontend）需要做的相应调整：

**设备端新增功能**：
1. ✅ 精确关键帧定位系统
2. ✅ Timeline文件缓存系统
3. ✅ FFmpeg命令行集成
4. ✅ 高级播放控制器（倍速播放、帧丢弃策略）

## 术语表

- **Seek**: 视频定位操作，跳转到指定时间位置
- **Keyframe**: 关键帧，视频中的完整帧，可独立解码
- **Timeline File**: 时间线文件，缓存关键帧信息的JSON文件
- **Playback Speed**: 播放速率，支持0.25x到4x
- **Frame Drop Strategy**: 帧丢弃策略，用于倍速播放时优化传输

---

## 平台端（Platform-Server）调整需求

### 需求 1: 支持精确Seek请求

**用户故事**: 作为平台端，我需要接收前端的seek请求并转发给设备端，以便用户能够精确定位视频。

#### 验收标准

1. WHEN 前端发送seek请求 THEN Platform SHALL验证请求参数（device_id, recording_id, target_time）
2. WHEN seek请求有效 THEN Platform SHALL通过QUIC连接转发seek命令到设备端
3. WHEN 设备端返回SeekResult THEN Platform SHALL将结果转发给前端
4. THE SeekResult SHALL包含：请求时间、实际定位时间、精度、执行时间
5. WHEN seek请求失败 THEN Platform SHALL返回明确的错误信息（超出范围、设备离线等）

#### 实现建议

```rust
// platform-server/src/http3/handlers.rs
pub struct SeekRequest {
    pub device_id: String,
    pub recording_id: String,
    pub target_time: f64,  // 秒
}

pub struct SeekResult {
    pub requested_time: f64,
    pub actual_time: f64,
    pub precision: f64,
    pub execution_time_ms: u64,
}

// 新增API端点
// POST /api/playback/seek
async fn handle_seek_request(req: SeekRequest) -> Result<SeekResult> {
    // 1. 验证参数
    // 2. 查找设备连接
    // 3. 通过QUIC发送seek命令
    // 4. 等待设备响应
    // 5. 返回SeekResult
}
```

### 需求 2: 支持倍速播放控制

**用户故事**: 作为平台端，我需要支持倍速播放控制，以便用户能够快速浏览录像。

#### 验收标准

1. WHEN 前端请求改变播放速率 THEN Platform SHALL验证速率范围（0.25x-4x）
2. WHEN 播放速率改变 THEN Platform SHALL通知设备端调整传输速率
3. WHEN 倍速播放时 THEN Platform SHALL根据设备端的帧丢弃策略调整转发逻辑
4. THE Platform SHALL维护当前播放速率状态
5. WHEN 播放速率恢复到1x THEN Platform SHALL恢复正常传输

#### 实现建议

```rust
// platform-server/src/http3/handlers.rs
pub struct PlaybackSpeedRequest {
    pub session_id: String,
    pub speed: f32,  // 0.25 - 4.0
}

// 新增API端点
// POST /api/playback/speed
async fn handle_playback_speed(req: PlaybackSpeedRequest) -> Result<()> {
    // 1. 验证速率范围
    // 2. 查找流会话
    // 3. 通知设备端调整速率
    // 4. 更新会话状态
}
```

### 需求 3: 扩展流会话管理

**用户故事**: 作为平台端，我需要扩展流会话管理，以便支持新的播放控制功能。

#### 验收标准

1. THE 流会话 SHALL维护当前播放位置（current_time）
2. THE 流会话 SHALL维护当前播放速率（playback_speed）
3. THE 流会话 SHALL维护帧丢弃策略（frame_drop_strategy）
4. WHEN seek操作完成 THEN 流会话 SHALL更新当前播放位置
5. WHEN 倍速播放 THEN 流会话 SHALL记录速率变化历史

#### 实现建议

```rust
// platform-server/src/streaming/handler.rs
pub struct StreamSession {
    pub session_id: String,
    pub device_id: String,
    pub recording_id: Option<String>,
    pub stream_type: StreamType,
    
    // 新增字段
    pub current_time: f64,           // 当前播放位置（秒）
    pub playback_speed: f32,         // 当前播放速率
    pub frame_drop_strategy: FrameDropStrategy,
    pub last_seek_time: Option<Instant>,
}

pub enum FrameDropStrategy {
    DropNone,
    DropNonKeyframes,
    DropByRate(f32),
    Adaptive,
}
```

### 需求 4: 增强录像元数据API

**用户故事**: 作为平台端，我需要提供录像的关键帧信息，以便前端实现智能的进度条预览。

#### 验收标准

1. WHEN 前端请求录像元数据 THEN Platform SHALL包含关键帧信息
2. THE 关键帧信息 SHALL包含：时间戳、文件偏移、帧大小
3. WHEN Timeline文件存在 THEN Platform SHALL从Timeline文件读取关键帧信息
4. WHEN Timeline文件不存在 THEN Platform SHALL请求设备端生成
5. THE API响应 SHALL包含视频总时长、关键帧数量、平均GOP大小

#### 实现建议

```rust
// platform-server/src/http3/handlers.rs
pub struct RecordingMetadata {
    pub recording_id: String,
    pub duration: f64,
    pub resolution: (u32, u32),
    pub framerate: f32,
    
    // 新增字段
    pub keyframe_count: usize,
    pub average_gop_size: usize,
    pub keyframes: Vec<KeyframeInfo>,
}

pub struct KeyframeInfo {
    pub timestamp: f64,
    pub file_offset: u64,
    pub frame_size: usize,
}

// 扩展现有API
// GET /api/recordings/{recording_id}/metadata
async fn get_recording_metadata(recording_id: String) -> Result<RecordingMetadata> {
    // 1. 查找录像文件
    // 2. 检查Timeline文件
    // 3. 如果不存在，请求设备端生成
    // 4. 返回完整元数据
}
```

### 需求 5: QUIC协议扩展

**用户故事**: 作为平台端，我需要扩展QUIC协议，以便支持新的播放控制命令。

#### 验收标准

1. THE QUIC协议 SHALL支持Seek命令（命令码：0x10）
2. THE QUIC协议 SHALL支持SetPlaybackSpeed命令（命令码：0x11）
3. THE QUIC协议 SHALL支持GetKeyframeIndex命令（命令码：0x12）
4. WHEN 设备端返回响应 THEN Platform SHALL正确解析响应数据
5. THE 协议 SHALL保持向后兼容性

#### 实现建议

```rust
// platform-server/src/protocol/mod.rs
pub enum PlaybackCommand {
    Seek { target_time: f64 },
    SetPlaybackSpeed { speed: f32 },
    GetKeyframeIndex,
    Pause,
    Resume,
    Stop,
}

impl PlaybackCommand {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PlaybackCommand::Seek { target_time } => {
                let mut buf = vec![0x10];  // Seek命令码
                buf.extend_from_slice(&target_time.to_le_bytes());
                buf
            }
            PlaybackCommand::SetPlaybackSpeed { speed } => {
                let mut buf = vec![0x11];  // SetPlaybackSpeed命令码
                buf.extend_from_slice(&speed.to_le_bytes());
                buf
            }
            // ...
        }
    }
}
```

---

## 前端（Web-Frontend）调整需求

### 需求 6: 实现Seek控制UI

**用户故事**: 作为用户，我希望能够通过拖动进度条精确定位视频，以便快速找到感兴趣的片段。

#### 验收标准

1. THE 播放器 SHALL显示可拖动的进度条
2. WHEN 用户拖动进度条 THEN 前端 SHALL显示目标时间预览
3. WHEN 用户释放进度条 THEN 前端 SHALL发送seek请求到平台端
4. WHEN seek请求返回 THEN 前端 SHALL显示实际定位时间和精度
5. WHEN seek操作进行中 THEN 前端 SHALL显示加载指示器
6. THE 进度条 SHALL显示关键帧位置标记（可选）

#### 实现建议

```typescript
// web-frontend/src/components/UnifiedMSEPlayer.tsx
interface SeekRequest {
  deviceId: string;
  recordingId: string;
  targetTime: number;
}

interface SeekResult {
  requestedTime: number;
  actualTime: number;
  precision: number;
  executionTimeMs: number;
}

async function handleSeek(targetTime: number): Promise<void> {
  setSeeking(true);
  try {
    const result = await api.seekPlayback({
      deviceId,
      recordingId,
      targetTime
    });
    
    // 更新播放位置
    if (videoRef.current) {
      videoRef.current.currentTime = result.actualTime;
    }
    
    // 显示精度信息
    console.log(`Seek precision: ${result.precision}s`);
  } catch (error) {
    console.error('Seek failed:', error);
  } finally {
    setSeeking(false);
  }
}
```

### 需求 7: 实现倍速播放控制

**用户故事**: 作为用户，我希望能够调整播放速度，以便快速浏览或仔细观看录像。

#### 验收标准

1. THE 播放器 SHALL提供倍速选择按钮（0.25x, 0.5x, 1x, 1.5x, 2x, 4x）
2. WHEN 用户选择倍速 THEN 前端 SHALL发送倍速请求到平台端
3. WHEN 倍速改变 THEN 前端 SHALL更新UI显示当前速率
4. WHEN 倍速播放时 THEN 前端 SHALL调整MSE缓冲策略
5. THE 播放器 SHALL在直通播放模式下禁用倍速功能

#### 实现建议

```typescript
// web-frontend/src/components/UnifiedMSEPlayer.tsx
const PLAYBACK_SPEEDS = [0.25, 0.5, 1, 1.5, 2, 4];

async function handleSpeedChange(speed: number): Promise<void> {
  try {
    await api.setPlaybackSpeed({
      sessionId,
      speed
    });
    
    setPlaybackSpeed(speed);
    
    // 调整MSE缓冲策略
    if (speed > 1) {
      // 倍速播放时减少缓冲
      adjustBufferStrategy('minimal');
    } else {
      // 慢速播放时增加缓冲
      adjustBufferStrategy('normal');
    }
  } catch (error) {
    console.error('Speed change failed:', error);
  }
}
```

### 需求 8: 增强进度条显示

**用户故事**: 作为用户，我希望进度条能够显示关键帧位置，以便我知道哪些位置可以精确定位。

#### 验收标准

1. WHEN 播放器加载录像 THEN 前端 SHALL请求录像元数据（包含关键帧信息）
2. WHEN 关键帧信息可用 THEN 进度条 SHALL显示关键帧标记
3. THE 关键帧标记 SHALL使用不同颜色或图标显示
4. WHEN 用户悬停在进度条上 THEN 前端 SHALL显示最近关键帧的时间
5. WHEN 用户点击进度条 THEN 前端 SHALL自动对齐到最近的关键帧

#### 实现建议

```typescript
// web-frontend/src/components/UnifiedMSEPlayer.tsx
interface KeyframeInfo {
  timestamp: number;
  fileOffset: number;
  frameSize: number;
}

function renderProgressBar(keyframes: KeyframeInfo[], duration: number) {
  return (
    <div className="progress-bar">
      <div className="progress-track" onClick={handleProgressClick}>
        {keyframes.map((kf, index) => (
          <div
            key={index}
            className="keyframe-marker"
            style={{ left: `${(kf.timestamp / duration) * 100}%` }}
            title={`Keyframe at ${kf.timestamp.toFixed(2)}s`}
          />
        ))}
        <div className="progress-fill" style={{ width: `${progress}%` }} />
      </div>
    </div>
  );
}
```

### 需求 9: 扩展API服务

**用户故事**: 作为前端开发者，我需要扩展API服务，以便调用新的播放控制功能。

#### 验收标准

1. THE API服务 SHALL提供seekPlayback方法
2. THE API服务 SHALL提供setPlaybackSpeed方法
3. THE API服务 SHALL提供getRecordingMetadata方法（包含关键帧信息）
4. WHEN API调用失败 THEN 服务 SHALL返回明确的错误信息
5. THE API服务 SHALL支持TypeScript类型定义

#### 实现建议

```typescript
// web-frontend/src/services/api.ts
export interface SeekRequest {
  deviceId: string;
  recordingId: string;
  targetTime: number;
}

export interface SeekResult {
  requestedTime: number;
  actualTime: number;
  precision: number;
  executionTimeMs: number;
}

export interface PlaybackSpeedRequest {
  sessionId: string;
  speed: number;
}

export interface RecordingMetadata {
  recordingId: string;
  duration: number;
  resolution: [number, number];
  framerate: number;
  keyframeCount: number;
  averageGopSize: number;
  keyframes: KeyframeInfo[];
}

export const api = {
  // 新增方法
  async seekPlayback(req: SeekRequest): Promise<SeekResult> {
    const response = await fetch(`${HTTP_API_URL}/api/playback/seek`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req)
    });
    return response.json();
  },

  async setPlaybackSpeed(req: PlaybackSpeedRequest): Promise<void> {
    await fetch(`${HTTP_API_URL}/api/playback/speed`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req)
    });
  },

  async getRecordingMetadata(recordingId: string): Promise<RecordingMetadata> {
    const response = await fetch(`${HTTP_API_URL}/api/recordings/${recordingId}/metadata`);
    return response.json();
  }
};
```

### 需求 10: 优化MSE缓冲策略

**用户故事**: 作为前端开发者，我需要优化MSE缓冲策略，以便在倍速播放时保持流畅。

#### 验收标准

1. WHEN 播放速率 > 1x THEN 播放器 SHALL减少目标缓冲（100-300ms）
2. WHEN 播放速率 < 1x THEN 播放器 SHALL增加目标缓冲（1000-2000ms）
3. WHEN 播放速率 = 1x THEN 播放器 SHALL使用正常缓冲（500-1000ms）
4. WHEN SourceBuffer缓冲过多 THEN 播放器 SHALL移除旧数据
5. THE 播放器 SHALL监控缓冲健康度并动态调整

#### 实现建议

```typescript
// web-frontend/src/components/UnifiedMSEPlayer.tsx
function adjustBufferStrategy(speed: number) {
  let targetBuffer: number;
  
  if (speed > 1) {
    // 倍速播放：最小缓冲
    targetBuffer = 0.2; // 200ms
  } else if (speed < 1) {
    // 慢速播放：增加缓冲
    targetBuffer = 1.5; // 1500ms
  } else {
    // 正常播放
    targetBuffer = 0.8; // 800ms
  }
  
  // 更新MSE配置
  if (sourceBuffer && !sourceBuffer.updating) {
    const buffered = sourceBuffer.buffered;
    const currentTime = videoRef.current?.currentTime || 0;
    
    // 移除过多的缓冲
    if (buffered.length > 0) {
      const bufferEnd = buffered.end(buffered.length - 1);
      if (bufferEnd - currentTime > targetBuffer + 1) {
        const removeEnd = bufferEnd - targetBuffer;
        sourceBuffer.remove(currentTime + targetBuffer, removeEnd);
      }
    }
  }
}
```

---

## 实现优先级

### 高优先级（P0）

1. **平台端**：
   - 需求1: 支持精确Seek请求
   - 需求5: QUIC协议扩展（Seek命令）

2. **前端**：
   - 需求6: 实现Seek控制UI
   - 需求9: 扩展API服务（seekPlayback）

### 中优先级（P1）

1. **平台端**：
   - 需求2: 支持倍速播放控制
   - 需求3: 扩展流会话管理

2. **前端**：
   - 需求7: 实现倍速播放控制
   - 需求10: 优化MSE缓冲策略

### 低优先级（P2）

1. **平台端**：
   - 需求4: 增强录像元数据API

2. **前端**：
   - 需求8: 增强进度条显示（关键帧标记）

---

## 兼容性考虑

### 向后兼容性

1. **协议兼容性**：
   - 新增的QUIC命令使用新的命令码（0x10-0x12）
   - 旧版本设备端不支持新命令时，平台端应返回"功能不支持"错误
   - 前端应检测功能可用性，对不支持的设备禁用相关UI

2. **API兼容性**：
   - 新增的API端点不影响现有端点
   - 现有的流会话管理保持不变
   - 新增的字段使用Optional类型，保持结构兼容

3. **前端兼容性**：
   - 新增的UI控件应该是可选的
   - 当功能不可用时，应该隐藏或禁用相关控件
   - 保持现有的播放功能不受影响

### 渐进式部署

建议采用以下部署顺序：

1. **阶段1**：设备端实现新功能
2. **阶段2**：平台端实现协议扩展和API
3. **阶段3**：前端实现UI和API调用
4. **阶段4**：端到端测试和优化

---

## 测试建议

### 平台端测试

1. **单元测试**：
   - Seek请求处理逻辑
   - 倍速播放控制逻辑
   - QUIC命令编解码

2. **集成测试**：
   - 平台端与设备端的Seek交互
   - 平台端与前端的API交互
   - 多客户端并发Seek

3. **性能测试**：
   - Seek响应时间 < 100ms
   - 倍速播放时的吞吐量
   - 并发流会话的资源占用

### 前端测试

1. **单元测试**：
   - API服务方法
   - 缓冲策略调整逻辑
   - 进度条计算逻辑

2. **集成测试**：
   - Seek操作的端到端流程
   - 倍速播放的端到端流程
   - 错误处理和恢复

3. **UI测试**：
   - 进度条拖动交互
   - 倍速按钮交互
   - 加载状态显示

---

## 风险评估

### 高风险

1. **协议兼容性**：新旧版本设备端混合部署时的兼容性问题
   - 缓解：实现版本协商机制，平台端检测设备端能力

2. **MSE缓冲管理**：倍速播放时的缓冲策略可能导致播放卡顿
   - 缓解：实现自适应缓冲策略，监控缓冲健康度

### 中风险

3. **Seek精度**：前端显示的时间与实际播放时间可能不一致
   - 缓解：使用SeekResult中的actual_time更新UI

4. **性能影响**：频繁的Seek操作可能影响系统性能
   - 缓解：实现Seek请求节流，限制请求频率

### 低风险

5. **UI复杂度**：新增控件可能使UI过于复杂
   - 缓解：采用渐进式UI设计，高级功能可折叠

---

## 总结

设备端实现精确关键帧定位、Timeline缓存、FFmpeg集成和高级播放控制后，平台端和前端需要进行以下核心调整：

**平台端核心调整**：
1. 扩展QUIC协议支持Seek和倍速播放命令
2. 实现Seek请求处理和转发逻辑
3. 扩展流会话管理，维护播放状态
4. 提供增强的录像元数据API

**前端核心调整**：
1. 实现可拖动的进度条和Seek控制
2. 实现倍速播放选择和控制
3. 扩展API服务支持新功能
4. 优化MSE缓冲策略适应倍速播放

这些调整将使整个系统能够充分利用设备端的新功能，为用户提供更好的视频回放体验。

