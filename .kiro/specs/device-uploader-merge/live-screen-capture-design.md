# 实时录屏功能合并设计文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | Device-Uploader 实时录屏功能合并到 Device-Simulator |
| 创建日期 | 2025-12-14 |
| 状态 | 设计中 |
| 版本 | v1.0 |
| 父文档 | live-screen-capture-requirements.md |

## 概述

本设计文档描述了将 device-uploader 的实时录屏和H.264编码功能合并到 device-simulator 的技术方案。该方案涵盖设备端、平台端和前端三个层面的设计，实现端到端的实时视频流传输。

### 设计目标

1. **实时编码**: 使用FFmpeg实时录制屏幕并编码为H.264
2. **低延迟传输**: 通过QUIC传输，端到端延迟 < 100ms
3. **精确帧率控制**: 前端以正确的帧率播放视频
4. **完整监控**: 提供端到端的延迟监控和性能统计
5. **向后兼容**: 不影响现有的文件播放功能

### 核心优势

- **零延迟编码**: 使用FFmpeg的ultrafast + zerolatency配置
- **零缓冲转发**: 平台端处理延迟 < 5ms
- **硬件加速解码**: 前端使用WebCodecs API
- **精确帧调度**: 使用FrameScheduler控制播放速率
- **完整监控**: 从编码到显示的全链路延迟监控


## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         设备端 (Device)                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │   Screen     │──▶│   FFmpeg     │──▶│ LiveH264     │       │
│  │   Capture    │   │   Encoder    │   │  Encoder     │       │
│  │ (avfoundation)│   │ (libx264)    │   │  Manager     │       │
│  └──────────────┘   └──────────────┘   └──────┬───────┘       │
│                                                 │               │
│                                                 ▼               │
│                                         ┌──────────────┐        │
│                                         │    QUIC      │        │
│                                         │  Transport   │        │
│                                         └──────┬───────┘        │
└────────────────────────────────────────────────┼────────────────┘
                                                 │ H.264 Stream
                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                        平台端 (Platform)                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │    QUIC      │──▶│ LiveStream   │──▶│  Unified     │       │
│  │  Receiver    │   │   Source     │   │   Stream     │       │
│  │              │   │              │   │   Handler    │       │
│  └──────────────┘   └──────────────┘   └──────┬───────┘       │
│                                                 │               │
│  ┌──────────────┐   ┌──────────────┐          │               │
│  │  FrameRate   │   │   Latency    │          │               │
│  │  Detector    │   │   Monitor    │          │               │
│  └──────────────┘   └──────────────┘          │               │
│                                                 ▼               │
│                                         ┌──────────────┐        │
│                                         │     SSE      │        │
│                                         │  Transport   │        │
│                                         └──────┬───────┘        │
└────────────────────────────────────────────────┼────────────────┘
                                                 │ H.264 + Metadata
                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                         前端 (Frontend)                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │     SSE      │──▶│  WebCodecs   │──▶│    Frame     │       │
│  │   Receiver   │   │   Decoder    │   │  Scheduler   │       │
│  │              │   │ (VideoDecoder)│   │              │       │
│  └──────────────┘   └──────────────┘   └──────┬───────┘       │
│                                                 │               │
│  ┌──────────────┐   ┌──────────────┐          │               │
│  │   Latency    │   │    Canvas    │◀─────────┘               │
│  │   Monitor    │   │   Renderer   │                           │
│  └──────────────┘   └──────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
```

### 数据流

```
屏幕画面 → avfoundation → FFmpeg (libx264) → stdout (H.264 Annex B) →
LiveH264Encoder → VideoSegment → QUIC → Platform QUIC Receiver →
LiveStreamSource → UnifiedStreamHandler → SSE → Frontend SSE Receiver →
WebCodecs VideoDecoder → FrameScheduler → Canvas Renderer → 显示
```

### 延迟分解

```
总延迟 (< 100ms) = 编码延迟 + 传输延迟 + 处理延迟 + 解码延迟 + 渲染延迟

- 编码延迟: < 50ms (FFmpeg ultrafast + zerolatency)
- 传输延迟: < 20ms (QUIC本地网络)
- 处理延迟: < 5ms (平台端零缓冲转发)
- 解码延迟: < 20ms (WebCodecs硬件加速)
- 渲染延迟: < 16ms (60fps Canvas渲染)
```


## 设备端设计

### 1. LiveH264Encoder (live_encoder.rs)

**职责**: 管理FFmpeg进程，实时编码屏幕内容为H.264流

**核心结构**:
```rust
pub struct LiveH264Encoder {
    config: LiveEncoderConfig,
    encoding_state: Arc<RwLock<EncodingState>>,
    output_sender: Option<mpsc::Sender<Vec<u8>>>,
    output_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    timestamp_generator: TimestampGenerator,
    stats: Arc<Mutex<EncodingStats>>,
    encoding_task: Option<JoinHandle<()>>,
    ffmpeg_process: Option<tokio::process::Child>,
}
```

**关键方法**:
- `start_encoding(stream_id)`: 启动FFmpeg进程和编码循环
- `stop_encoding()`: 停止FFmpeg进程和清理资源
- `get_next_segment()`: 获取下一个编码后的H.264分片
- `get_stats()`: 获取编码性能统计

**FFmpeg命令配置**:
```bash
ffmpeg \
  -f avfoundation -i "4" \           # macOS屏幕捕获
  -r 30 -s 1280x720 \                # 帧率和分辨率
  -c:v libx264 \                     # H.264编码器
  -preset ultrafast \                # 最快编码速度
  -tune zerolatency \                # 零延迟调优
  -profile:v baseline -level 3.1 \  # 兼容性配置
  -pix_fmt yuv420p \                 # 像素格式
  -b:v 2000k -g 30 \                 # 码率和GOP
  -vf "drawtext=..." \               # 时间戳叠加（可选）
  -f h264 - \                        # 输出到stdout
  -y -loglevel error
```

**编码循环**:
```rust
async fn ffmpeg_encoding_loop() {
    // 1. 启动FFmpeg进程
    let mut child = ffmpeg_cmd.spawn()?;
    let stdout = child.stdout.take()?;
    
    // 2. 异步读取H.264数据
    let mut stdout_reader = BufReader::new(stdout);
    let mut buffer = vec![0u8; 64 * 1024];
    
    loop {
        // 3. 读取数据块
        let n = stdout_reader.read(&mut buffer).await?;
        let data = buffer[..n].to_vec();
        
        // 4. 更新统计信息
        update_stats(n);
        
        // 5. 发送到输出通道
        sender.send(data).await?;
    }
}
```

### 2. LiveEncoderConfig

**配置结构**:
```rust
pub struct LiveEncoderConfig {
    pub quality: LiveStreamQuality,      // 视频质量参数
    pub timestamp_overlay: bool,         // 是否叠加时间戳
    pub screen_capture: bool,            // 是否使用屏幕捕获
    pub output_format: OutputFormat,     // 输出格式
    pub segment_duration_ms: u64,        // 分片时长
    pub timestamp_format: TimestampFormat, // 时间戳格式
}

pub struct LiveStreamQuality {
    pub width: u32,              // 分辨率宽度
    pub height: u32,             // 分辨率高度
    pub fps: u32,                // 帧率
    pub bitrate_kbps: u32,       // 码率
    pub keyframe_interval: u32,  // 关键帧间隔
}
```

**默认配置**:
- 分辨率: 1280x720
- 帧率: 30fps
- 码率: 2000kbps
- GOP大小: 30帧（1秒）
- 时间戳叠加: 启用

### 3. TimestampGenerator

**职责**: 生成单调递增的时间戳和帧编号

**实现**:
```rust
pub struct TimestampGenerator {
    start_time: Instant,
    frame_count: u64,
    fps: u32,
}

impl TimestampGenerator {
    pub fn next_timestamp(&mut self) -> f64 {
        self.frame_count += 1;
        (self.frame_count - 1) as f64 / self.fps as f64
    }
    
    pub fn current_timestamp_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}
```

### 4. 集成到 device_service.rs

**修改点**:
```rust
pub struct DeviceService {
    // 现有字段...
    live_encoder: Option<LiveH264Encoder>,  // 新增
}

// 处理StartLiveStream信令
async fn handle_start_live_stream(&mut self, session_id: Uuid) {
    // 1. 创建LiveH264Encoder
    let config = LiveEncoderConfig::default();
    let mut encoder = LiveH264Encoder::new(config);
    
    // 2. 启动编码
    encoder.start_encoding(session_id.to_string()).await?;
    
    // 3. 启动分片发送任务
    self.start_segment_forwarding_task(encoder);
    
    // 4. 保存encoder引用
    self.live_encoder = Some(encoder);
}

// 分片转发任务
async fn start_segment_forwarding_task(&self, mut encoder: LiveH264Encoder) {
    tokio::spawn(async move {
        while let Some(segment) = encoder.get_next_segment().await {
            // 通过QUIC发送分片
            send_segment_via_quic(segment).await;
        }
    });
}
```


## 平台端设计

### 1. LiveStreamSource 增强

**现有功能**: 从QUIC接收器获取实时分片

**新增功能**: 帧率检测

**修改**:
```rust
pub struct LiveStreamSource {
    device_id: String,
    quic_receiver: broadcast::Receiver<CommonVideoSegment>,
    state: SourceState,
    current_position: f64,
    resolution: Option<(u32, u32)>,
    frame_rate: Option<f64>,
    bitrate: Option<u64>,
    frame_rate_detector: FrameRateDetector,  // 新增
}

async fn next_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
    match self.quic_receiver.recv().await {
        Ok(common_segment) => {
            // 添加时间戳样本用于帧率检测
            let pts_us = (common_segment.timestamp * 1_000_000.0) as u64;
            let receive_time = SystemTime::now();
            self.frame_rate_detector.add_timestamp_sample(pts_us, receive_time);
            
            // 更新检测到的帧率
            if let Some(detected_fps) = self.frame_rate_detector.get_fps() {
                self.frame_rate = Some(detected_fps);
            }
            
            // 转换并返回分片
            Ok(Some(convert_segment(common_segment)))
        }
        // ...
    }
}
```

### 2. FrameRateDetector

**职责**: 基于时间戳样本检测实际帧率

**实现**:
```rust
pub struct FrameRateDetector {
    samples: VecDeque<TimestampSample>,
    max_samples: usize,
    min_samples_for_detection: usize,
}

struct TimestampSample {
    pts_us: u64,           // 显示时间戳（微秒）
    receive_time: SystemTime,
}

impl FrameRateDetector {
    pub fn add_timestamp_sample(&mut self, pts_us: u64, receive_time: SystemTime) {
        self.samples.push_back(TimestampSample { pts_us, receive_time });
        
        // 保持样本数量在限制内
        if self.samples.len() > self.max_samples {
            self.samples.pop_front();
        }
    }
    
    pub fn get_fps(&self) -> Option<f64> {
        if self.samples.len() < self.min_samples_for_detection {
            return None;
        }
        
        // 计算平均帧间隔
        let first = self.samples.front()?;
        let last = self.samples.back()?;
        
        let time_span_us = last.pts_us - first.pts_us;
        let frame_count = self.samples.len() - 1;
        
        if time_span_us == 0 || frame_count == 0 {
            return None;
        }
        
        // FPS = 帧数 / 时间跨度（秒）
        let fps = (frame_count as f64) / (time_span_us as f64 / 1_000_000.0);
        
        Some(fps)
    }
}
```

### 3. UnifiedStreamHandler 延迟监控

**现有功能**: 零缓冲转发、延迟监控

**增强**: 支持实时流的延迟监控

**关键代码**:
```rust
async fn start_forwarding_task() {
    loop {
        match source.next_segment().await {
            Ok(Some(mut segment)) => {
                // 记录接收时间
                let receive_time = SystemTime::now();
                segment.receive_time = Some(receive_time);
                
                // 根据分片来源类型记录延迟监控时间戳
                match segment.source_type {
                    SegmentSourceType::Live => {
                        // 直通播放：记录完整的延迟链路
                        let device_send_time = receive_time; // 从元数据获取
                        latency_monitor.record_device_send(segment.segment_id, device_send_time);
                        latency_monitor.record_platform_receive(segment.segment_id, receive_time);
                    }
                    SegmentSourceType::Playback => {
                        // 回放：只记录平台内部延迟
                        latency_monitor.record_platform_receive(segment.segment_id, receive_time);
                    }
                }
                
                // 零缓冲转发
                let forward_time = SystemTime::now();
                segment_sender.send(segment.clone())?;
                segment.forward_time = Some(forward_time);
                
                // 记录转发时间
                latency_monitor.record_platform_forward(segment.segment_id, forward_time);
                
                // 计算处理延迟
                let processing_latency_ms = forward_time
                    .duration_since(receive_time)?
                    .as_micros() as f64 / 1000.0;
                
                // 更新统计
                stats_manager.record_segment_latency(
                    &session_id,
                    forward_time.duration_since(receive_time)?,
                    segment.data.len(),
                );
            }
            // ...
        }
    }
}
```

### 4. SSE 传输增强

**现有功能**: 通过SSE推送视频分片

**增强**: 包含帧率和延迟元数据

**响应格式**:
```json
{
  "segment_id": "uuid",
  "timestamp": 1.5,
  "duration": 0.033,
  "is_keyframe": true,
  "format": "h264",
  "data": "base64_encoded_h264_data",
  "metadata": {
    "frame_number": 45,
    "encoding_fps": 30,
    "detected_fps": 29.8,
    "send_time_ms": 1234567890,
    "receive_time_ms": 1234567910,
    "forward_time_ms": 1234567912
  }
}
```


## 前端设计

### 1. WebCodecsPlayer 增强

**现有功能**: 使用WebCodecs解码H.264流

**增强**: 帧率控制和延迟监控

**核心修改**:
```typescript
function WebCodecsPlayer({ sessionId }: WebCodecsPlayerProps) {
  const [targetFps, setTargetFps] = useState<number>(30);
  const [actualFps, setActualFps] = useState<number>(0);
  const [droppedFrames, setDroppedFrames] = useState<number>(0);
  const [averageDelay, setAverageDelay] = useState<number>(0);
  
  const frameSchedulerRef = useRef<FrameScheduler | null>(null);
  
  useEffect(() => {
    // 创建 FrameScheduler
    const scheduler = new FrameScheduler(targetFps);
    frameSchedulerRef.current = scheduler;
    
    // 设置帧显示回调
    scheduler.setDisplayCallback((frame: VideoFrame) => {
      displayFrame(frame, canvas, ctx);
    });
    
    // 创建 VideoDecoder
    const decoder = new VideoDecoder({
      output: (frame: VideoFrame) => {
        // 将帧交给调度器，而不是立即显示
        const pts = frame.timestamp || 0;
        scheduler.addFrame(frame, pts);
        
        // 更新统计信息
        const stats = scheduler.getStats();
        setDroppedFrames(stats.droppedFrames);
        setAverageDelay(stats.averageDelay);
      },
      error: (err: Error) => {
        console.error('Decoder error:', err);
      }
    });
    
    // ...
  }, [sessionId]);
}
```

### 2. FrameScheduler

**职责**: 控制视频帧的显示速率，确保以正确的帧率播放

**核心实现**:
```typescript
export class FrameScheduler {
  private targetFps: number;
  private frameInterval: number;  // 目标帧间隔（ms）
  private frameQueue: Array<{ frame: VideoFrame; pts: number }> = [];
  private displayCallback: ((frame: VideoFrame) => void) | null = null;
  private schedulerTask: number | null = null;
  private stats: SchedulerStats;
  
  constructor(targetFps: number) {
    this.targetFps = targetFps;
    this.frameInterval = 1000 / targetFps;
    this.stats = {
      droppedFrames: 0,
      displayedFrames: 0,
      averageDelay: 0,
      delayHistory: []
    };
    
    this.startScheduler();
  }
  
  public addFrame(frame: VideoFrame, pts: number) {
    // 添加帧到队列
    this.frameQueue.push({ frame, pts });
    
    // 限制队列长度，防止内存溢出
    if (this.frameQueue.length > 10) {
      const dropped = this.frameQueue.shift();
      if (dropped) {
        dropped.frame.close();
        this.stats.droppedFrames++;
      }
    }
  }
  
  private startScheduler() {
    let lastDisplayTime = performance.now();
    
    const scheduleNextFrame = () => {
      const now = performance.now();
      const elapsed = now - lastDisplayTime;
      
      // 检查是否到了显示下一帧的时间
      if (elapsed >= this.frameInterval) {
        if (this.frameQueue.length > 0) {
          const { frame, pts } = this.frameQueue.shift()!;
          
          // 显示帧
          if (this.displayCallback) {
            this.displayCallback(frame);
          }
          
          // 关闭帧
          frame.close();
          
          // 更新统计
          this.stats.displayedFrames++;
          const delay = now - lastDisplayTime - this.frameInterval;
          this.updateDelayStats(delay);
          
          lastDisplayTime = now;
        }
      }
      
      // 使用 requestAnimationFrame 进行下一次调度
      this.schedulerTask = requestAnimationFrame(scheduleNextFrame);
    };
    
    this.schedulerTask = requestAnimationFrame(scheduleNextFrame);
  }
  
  private updateDelayStats(delay: number) {
    this.stats.delayHistory.push(delay);
    
    // 保持最近30个样本
    if (this.stats.delayHistory.length > 30) {
      this.stats.delayHistory.shift();
    }
    
    // 计算平均延迟
    const sum = this.stats.delayHistory.reduce((a, b) => a + b, 0);
    this.stats.averageDelay = sum / this.stats.delayHistory.length;
  }
  
  public getStats(): SchedulerStats {
    return { ...this.stats };
  }
  
  public destroy() {
    if (this.schedulerTask !== null) {
      cancelAnimationFrame(this.schedulerTask);
    }
    
    // 清理队列中的帧
    for (const { frame } of this.frameQueue) {
      frame.close();
    }
    this.frameQueue = [];
  }
}
```

### 3. LatencyMonitor 组件

**职责**: 显示实时延迟统计信息

**实现**:
```typescript
function LatencyMonitor({ sessionId, apiBaseUrl }: LatencyMonitorProps) {
  const [stats, setStats] = useState<LatencyStats | null>(null);
  const [alerts, setAlerts] = useState<LatencyAlert[]>([]);
  
  useEffect(() => {
    // 连接到延迟统计SSE端点
    const statsUrl = `${apiBaseUrl}/api/v1/latency/${sessionId}/stats`;
    const statsSource = new EventSource(statsUrl);
    
    statsSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setStats(data);
    };
    
    // 连接到延迟告警SSE端点
    const alertsUrl = `${apiBaseUrl}/api/v1/latency/${sessionId}/alerts`;
    const alertsSource = new EventSource(alertsUrl);
    
    alertsSource.onmessage = (event) => {
      const alert = JSON.parse(event.data);
      setAlerts(prev => [...prev, alert].slice(-5)); // 保留最近5个告警
    };
    
    return () => {
      statsSource.close();
      alertsSource.close();
    };
  }, [sessionId, apiBaseUrl]);
  
  return (
    <div className="latency-monitor">
      <h4>📊 延迟监控</h4>
      {stats && (
        <div className="stats-grid">
          <div className="stat-item">
            <span className="label">当前延迟:</span>
            <span className="value">{stats.current_latency_ms.toFixed(1)}ms</span>
          </div>
          <div className="stat-item">
            <span className="label">平均延迟:</span>
            <span className="value">{stats.average_latency_ms.toFixed(1)}ms</span>
          </div>
          <div className="stat-item">
            <span className="label">P95延迟:</span>
            <span className="value">{stats.p95_latency_ms.toFixed(1)}ms</span>
          </div>
          <div className="stat-item">
            <span className="label">P99延迟:</span>
            <span className="value">{stats.p99_latency_ms.toFixed(1)}ms</span>
          </div>
        </div>
      )}
      
      {alerts.length > 0 && (
        <div className="alerts">
          <h5>⚠️ 延迟告警</h5>
          {alerts.map((alert, index) => (
            <div key={index} className="alert-item">
              {alert.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

### 4. 资源清理

**关键点**: 确保所有资源正确清理，避免内存泄漏

**实现**:
```typescript
const cleanup = () => {
  // 关闭SSE连接
  if (eventSourceRef.current) {
    eventSourceRef.current.close();
    eventSourceRef.current = null;
  }
  
  // 关闭VideoDecoder
  if (decoderRef.current) {
    try {
      decoderRef.current.close();
    } catch (e) {
      console.warn('Failed to close decoder:', e);
    }
    decoderRef.current = null;
  }
  
  // 销毁FrameScheduler
  if (frameSchedulerRef.current) {
    frameSchedulerRef.current.destroy();
    frameSchedulerRef.current = null;
  }
  
  // 清理状态
  isConfiguredRef.current = false;
  pendingChunksRef.current = [];
};

useEffect(() => {
  initializePlayer();
  return () => cleanup();
}, [sessionId]);
```

