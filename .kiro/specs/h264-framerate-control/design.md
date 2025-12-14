# H264帧率控制设计文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | H264帧率控制 |
| 创建日期 | 2025-12-14 |
| 状态 | 设计中 |
| 版本 | v1.0 |

## 概述

本设计文档描述了H264帧率控制系统的技术架构和实现方案。该系统通过精确的帧率检测、时间戳管理和播放控制，确保视频以正确的速度播放，解决当前"接收多少播放多少"导致的播放速度不正确问题。

### 设计目标

1. **精确帧率控制**: 播放速度误差<±5%
2. **自动帧率检测**: 从SPS或时间戳自动检测帧率
3. **低延迟**: 帧率控制增加的延迟<10ms
4. **倍速支持**: 支持0.25x-4x倍速播放
5. **自适应调整**: 根据网络和性能自适应调整帧率

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    设备端/文件源                             │
├─────────────────────────────────────────────────────────────┤
│  H264视频流 + 时间戳                                         │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    平台端帧率控制层                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐   │
│  │         FrameRateDetector (帧率检测器)              │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  • SPS解析                                           │   │
│  │  • 时间戳分析                                        │   │
│  │  • 帧率计算                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         FrameRatePacer (帧率控制器)                 │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  • 发送间隔计算                                      │   │
│  │  • 速率控制                                          │   │
│  │  • 倍速支持                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└──────────────────────────┬──────────────────────────────────┘
                           │ HTTP3 + SSE
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    前端播放控制层                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐   │
│  │         FrameScheduler (帧调度器)                   │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  • 时间戳同步                                        │   │
│  │  • 帧显示时机控制                                    │   │
│  │  • 跳帧/重复帧                                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         FrameRateMonitor (帧率监控器)               │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  • FPS计算                                           │   │
│  │  • 统计收集                                          │   │
│  │  • 性能监控                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```


### 核心组件

#### 1. FrameRateDetector (平台端)

帧率检测器，负责自动检测视频流的帧率。

**职责**:
- 从H.264 SPS中解析帧率信息
- 通过时间戳分析计算实际帧率
- 处理可变帧率（VFR）和固定帧率（CFR）
- 检测帧率变化并通知系统

#### 2. FrameRatePacer (平台端)

帧率控制器，负责控制视频分片的发送速率。

**职责**:
- 根据目标帧率计算分片发送间隔
- 实现精确的速率控制
- 支持倍速播放
- 动态调整发送速率以适应网络条件

#### 3. TimestampManager (平台端)

时间戳管理器，负责管理视频帧的时间戳。

**职责**:
- 生成或保持原始时间戳
- 确保时间戳单调递增
- 转换时间戳格式（90kHz时钟 ↔ 微秒）
- 处理时间戳不连续情况

#### 4. FrameScheduler (前端)

帧调度器，负责控制视频帧的显示时机。

**职责**:
- 根据时间戳调度帧显示
- 实现帧同步机制
- 处理过早/过晚到达的帧
- 执行跳帧或重复帧操作

#### 5. FrameRateMonitor (前端)

帧率监控器，负责监控和统计实际播放帧率。

**职责**:
- 实时计算当前FPS
- 收集帧率统计数据
- 检测播放速度偏差
- 提供性能监控API

## 数据模型

### FrameRateInfo

```rust
pub struct FrameRateInfo {
    pub fps: f64,                    // 帧率（帧/秒）
    pub frame_duration_us: u64,      // 帧持续时间（微秒）
    pub is_variable: bool,           // 是否可变帧率
    pub detection_method: DetectionMethod,
    pub confidence: f32,             // 检测置信度 (0.0-1.0)
}

pub enum DetectionMethod {
    FromSPS,                         // 从SPS解析
    FromTimestamp,                   // 从时间戳计算
    Default,                         // 使用默认值
}
```

### FrameTimestamp

```rust
pub struct FrameTimestamp {
    pub pts: u64,                    // 显示时间戳（微秒）
    pub dts: Option<u64>,            // 解码时间戳（微秒）
    pub duration: u64,               // 帧持续时间（微秒）
    pub is_keyframe: bool,
}
```

### FrameRateStats

```rust
pub struct FrameRateStats {
    pub current_fps: f64,            // 当前FPS
    pub average_fps: f64,            // 平均FPS
    pub min_fps: f64,                // 最小FPS
    pub max_fps: f64,                // 最大FPS
    pub target_fps: f64,             // 目标FPS
    pub dropped_frames: u64,         // 丢帧数
    pub duplicated_frames: u64,      // 重复帧数
    pub speed_error_percent: f64,    // 速度误差百分比
}
```

### FrameRateConfig

```rust
pub struct FrameRateConfig {
    pub target_fps: Option<f64>,    // 目标帧率（None=自动检测）
    pub tolerance_percent: f64,      // 容差百分比（默认5%）
    pub sync_strategy: SyncStrategy,
    pub adaptive_enabled: bool,      // 是否启用自适应调整
    pub adaptive_thresholds: AdaptiveThresholds,
}

pub enum SyncStrategy {
    DropFrames,                      // 丢帧策略
    DuplicateFrames,                 // 重复帧策略
    AdjustSpeed,                     // 调整速度策略
}

pub struct AdaptiveThresholds {
    pub bandwidth_low_mbps: f64,     // 低带宽阈值
    pub cpu_high_percent: f64,       // 高CPU阈值
    pub buffer_low_ms: u64,          // 低缓冲阈值
}
```


## 组件接口

### FrameRateDetector

```rust
pub struct FrameRateDetector {
    detected_fps: Option<f64>,
    timestamp_history: VecDeque<(u64, SystemTime)>,
    confidence: f32,
}

impl FrameRateDetector {
    /// 从SPS解析帧率
    pub fn detect_from_sps(&mut self, sps_data: &[u8]) -> Result<FrameRateInfo, Error>;
    
    /// 从时间戳序列计算帧率
    pub fn detect_from_timestamps(&mut self, timestamps: &[u64]) -> Result<FrameRateInfo, Error>;
    
    /// 添加新的时间戳样本
    pub fn add_timestamp_sample(&mut self, pts: u64, receive_time: SystemTime);
    
    /// 获取当前检测到的帧率
    pub fn get_frame_rate(&self) -> Option<FrameRateInfo>;
    
    /// 检测帧率是否发生变化
    pub fn has_frame_rate_changed(&self) -> bool;
}
```

### FrameRatePacer

```rust
pub struct FrameRatePacer {
    target_fps: f64,
    playback_rate: f64,
    last_send_time: Option<Instant>,
}

impl FrameRatePacer {
    /// 创建新的帧率控制器
    pub fn new(target_fps: f64) -> Self;
    
    /// 计算下一帧的发送延迟
    pub fn calculate_send_delay(&mut self, frames_in_segment: u32) -> Duration;
    
    /// 设置播放速率（倍速）
    pub fn set_playback_rate(&mut self, rate: f64);
    
    /// 等待直到可以发送下一帧
    pub async fn wait_for_next_frame(&mut self);
    
    /// 根据网络条件调整发送速率
    pub fn adjust_for_network(&mut self, bandwidth_mbps: f64, buffer_ms: u64);
}
```

### TimestampManager

```rust
pub struct TimestampManager {
    base_timestamp: u64,
    last_timestamp: u64,
    frame_duration_us: u64,
    clock_rate: u32,  // 通常是90000 (90kHz)
}

impl TimestampManager {
    /// 创建新的时间戳管理器
    pub fn new(fps: f64) -> Self;
    
    /// 生成下一个时间戳
    pub fn generate_next_timestamp(&mut self) -> u64;
    
    /// 验证时间戳是否单调递增
    pub fn validate_timestamp(&self, pts: u64) -> Result<(), Error>;
    
    /// 转换时间戳格式（90kHz → 微秒）
    pub fn convert_to_microseconds(&self, timestamp_90khz: u64) -> u64;
    
    /// 转换时间戳格式（微秒 → 90kHz）
    pub fn convert_to_90khz(&self, timestamp_us: u64) -> u64;
    
    /// 处理时间戳不连续
    pub fn handle_discontinuity(&mut self, new_timestamp: u64);
}
```

### FrameScheduler (TypeScript)

```typescript
class FrameScheduler {
    private targetFps: number;
    private baseTime: number;
    private frameQueue: Array<{frame: VideoFrame, pts: number}>;
    
    constructor(targetFps: number);
    
    // 添加帧到调度队列
    addFrame(frame: VideoFrame, pts: number): void;
    
    // 获取下一个应该显示的帧
    getNextFrame(currentTime: number): VideoFrame | null;
    
    // 检查帧是否应该显示
    shouldDisplayFrame(pts: number, currentTime: number): boolean;
    
    // 计算帧显示延迟
    calculateDisplayDelay(pts: number, currentTime: number): number;
    
    // 执行跳帧
    dropLateFrames(currentTime: number): number;
}
```

### FrameRateMonitor (TypeScript)

```typescript
class FrameRateMonitor {
    private frameCount: number;
    private lastUpdateTime: number;
    private fpsHistory: number[];
    
    constructor();
    
    // 记录新帧
    recordFrame(): void;
    
    // 获取当前FPS
    getCurrentFps(): number;
    
    // 获取统计信息
    getStats(): FrameRateStats;
    
    // 检测速度偏差
    detectSpeedDeviation(targetFps: number): number;
    
    // 重置统计
    reset(): void;
}
```


## 正确性属性

*属性是一个特征或行为，应该在系统的所有有效执行中保持为真——本质上是关于系统应该做什么的正式陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。*

### 属性 1: 帧率检测准确性

*对于任何*包含有效SPS的H.264视频流，从SPS解析的帧率应该与实际帧率的误差小于1%。

**验证**: 需求 1.1

### 属性 2: 时间戳分析帧率计算

*对于任何*连续的时间戳序列（至少10个样本），通过时间戳分析计算的帧率应该与实际帧率的误差小于5%。

**验证**: 需求 1.2

### 属性 3: 帧率元数据完整性

*对于任何*视频流，当帧率检测完成后，流元数据中应该包含有效的帧率信息（fps > 0）。

**验证**: 需求 1.3

### 属性 4: 发送间隔计算正确性

*对于任何*目标帧率和分片包含的帧数，计算的发送间隔应该等于 (帧数 / 目标帧率) 秒，误差小于1ms。

**验证**: 需求 2.1, 2.4

### 属性 5: 直通模式低延迟保持

*对于任何*直通播放会话，添加帧率控制后的端到端延迟增加应该小于10ms。

**验证**: 需求 2.2

### 属性 6: 倍速播放间隔调整

*对于任何*倍速设置（0.25x-4x），实际帧发送间隔应该等于 (基础间隔 / 倍速)，误差小于5%。

**验证**: 需求 2.3, 6.1

### 属性 7: WebCodecs帧显示时机

*对于任何*使用WebCodecs的视频帧，实际显示时间与目标显示时间（基于PTS）的误差应该小于16ms（1帧@60fps）。

**验证**: 需求 3.3, 3.4

### 属性 8: 时间戳单调递增

*对于任何*生成或处理的时间戳序列，所有时间戳应该严格单调递增。

**验证**: 需求 4.3

### 属性 9: fMP4转换时间戳保真

*对于任何*H.264到fMP4的转换，转换前后的时间戳差异应该为0（完全保持）。

**验证**: 需求 4.4

### 属性 10: 时间戳精度保证

*对于任何*时间戳操作，精度应该达到微秒级（误差 < 1微秒）。

**验证**: 需求 4.5

### 属性 11: 播放速度误差上界

*对于任何*播放会话，实际播放速度与目标速度的误差应该小于±5%。

**验证**: 需求 5.5

### 属性 12: 倍速时间戳相对关系

*对于任何*倍速播放，相邻帧的时间戳差值应该保持不变（相对关系不变）。

**验证**: 需求 6.2

### 属性 13: FPS计算实时性

*对于任何*播放会话，当前FPS的计算延迟应该小于1秒。

**验证**: 需求 7.1

### 属性 14: 统计数据准确性

*对于任何*帧率统计，平均FPS应该等于所有采样FPS的算术平均值，误差小于0.1。

**验证**: 需求 7.2

### 属性 15: 默认帧率回退

*对于任何*无法检测帧率的视频流，系统应该使用30 FPS作为默认值。

**验证**: 需求 9.1

### 属性 16: 时间戳不连续恢复

*对于任何*时间戳不连续事件，系统应该在3帧内重新同步播放时钟。

**验证**: 需求 9.2

### 属性 17: 配置热更新无中断

*对于任何*配置更新操作，播放应该继续进行，不应该出现明显的卡顿（>100ms）。

**验证**: 需求 10.5

## 错误处理

### 错误类型

```rust
#[derive(Debug, Clone)]
pub enum FrameRateError {
    // 检测错误
    SPSParseError(String),
    InsufficientSamples,
    InvalidFrameRate(f64),
    
    // 时间戳错误
    TimestampNotMonotonic { current: u64, previous: u64 },
    TimestampDiscontinuity { gap_ms: u64 },
    TimestampOverflow,
    
    // 同步错误
    FrameTooEarly { pts: u64, current_time: u64 },
    FrameTooLate { pts: u64, current_time: u64 },
    SyncLost,
    
    // 配置错误
    InvalidTargetFps(f64),
    InvalidPlaybackRate(f64),
    InvalidTolerance(f64),
}
```

### 错误恢复策略

```rust
pub struct FrameRateErrorRecovery {
    pub max_timestamp_gap_ms: u64,      // 最大时间戳间隔
    pub max_sync_attempts: u32,          // 最大同步尝试次数
    pub fallback_fps: f64,               // 回退帧率
    pub smooth_transition_frames: u32,   // 平滑过渡帧数
}

impl Default for FrameRateErrorRecovery {
    fn default() -> Self {
        Self {
            max_timestamp_gap_ms: 5000,
            max_sync_attempts: 3,
            fallback_fps: 30.0,
            smooth_transition_frames: 10,
        }
    }
}
```


## 实现流程

### 平台端帧率控制流程

```
1. 接收H.264视频分片
   ↓
2. FrameRateDetector检测帧率
   ├─ 尝试从SPS解析
   ├─ 如果失败，从时间戳分析
   └─ 如果仍失败，使用默认30fps
   ↓
3. TimestampManager管理时间戳
   ├─ 验证时间戳单调性
   ├─ 处理不连续情况
   └─ 转换时间戳格式
   ↓
4. FrameRatePacer控制发送速率
   ├─ 计算发送间隔
   ├─ 应用倍速调整
   └─ 等待到发送时间
   ↓
5. 通过HTTP3/SSE发送分片
   └─ 包含帧率元数据
```

### 前端播放控制流程

```
1. 接收视频分片（SSE）
   ↓
2. 解析帧率元数据
   ↓
3. FrameScheduler调度帧显示
   ├─ 检查帧到达时间
   ├─ 计算显示延迟
   └─ 决定显示/跳过/重复
   ↓
4. 显示视频帧
   ├─ MSE: 依赖浏览器时间戳
   └─ WebCodecs: 手动控制显示时机
   ↓
5. FrameRateMonitor监控FPS
   ├─ 记录帧显示
   ├─ 计算实时FPS
   └─ 检测速度偏差
```

### 帧率检测详细流程

```rust
// 1. 从SPS检测
pub fn detect_from_sps(sps: &[u8]) -> Result<f64, Error> {
    // 解析SPS NAL单元
    let sps_data = parse_sps(sps)?;
    
    // 提取时间信息
    let time_scale = sps_data.vui_parameters.time_scale;
    let num_units_in_tick = sps_data.vui_parameters.num_units_in_tick;
    
    // 计算帧率
    let fps = time_scale as f64 / (2.0 * num_units_in_tick as f64);
    
    Ok(fps)
}

// 2. 从时间戳检测
pub fn detect_from_timestamps(timestamps: &[(u64, SystemTime)]) -> Result<f64, Error> {
    if timestamps.len() < 10 {
        return Err(Error::InsufficientSamples);
    }
    
    // 计算相邻帧的时间间隔
    let mut intervals = Vec::new();
    for i in 1..timestamps.len() {
        let duration = timestamps[i].1.duration_since(timestamps[i-1].1)?;
        intervals.push(duration.as_secs_f64());
    }
    
    // 计算平均间隔
    let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
    
    // 计算帧率
    let fps = 1.0 / avg_interval;
    
    Ok(fps)
}
```

### 发送速率控制详细实现

```rust
impl FrameRatePacer {
    pub async fn pace_segment(&mut self, segment: &VideoSegment) {
        // 计算基础帧间隔
        let base_interval = Duration::from_secs_f64(1.0 / self.target_fps);
        
        // 应用倍速调整
        let adjusted_interval = base_interval.div_f64(self.playback_rate);
        
        // 考虑分片中的帧数
        let frames_in_segment = segment.frame_count.unwrap_or(1);
        let total_interval = adjusted_interval * frames_in_segment;
        
        // 计算实际需要等待的时间
        if let Some(last_send) = self.last_send_time {
            let elapsed = last_send.elapsed();
            if elapsed < total_interval {
                let wait_time = total_interval - elapsed;
                tokio::time::sleep(wait_time).await;
            }
        }
        
        self.last_send_time = Some(Instant::now());
    }
}
```

### WebCodecs帧调度详细实现

```typescript
class FrameScheduler {
    private baseTime: number = performance.now();
    private frameQueue: Array<{frame: VideoFrame, pts: number}> = [];
    
    addFrame(frame: VideoFrame, pts: number): void {
        this.frameQueue.push({ frame, pts });
        this.scheduleNextFrame();
    }
    
    private scheduleNextFrame(): void {
        if (this.frameQueue.length === 0) return;
        
        const { frame, pts } = this.frameQueue[0];
        const currentTime = performance.now() - this.baseTime;
        const targetTime = pts / 1000; // 转换为毫秒
        
        if (currentTime >= targetTime) {
            // 时间已到，立即显示
            this.displayFrame(frame);
            this.frameQueue.shift();
            this.scheduleNextFrame();
        } else if (currentTime < targetTime - 100) {
            // 太早，跳过此帧（可能是时间戳错误）
            console.warn('Frame too early, dropping');
            frame.close();
            this.frameQueue.shift();
            this.scheduleNextFrame();
        } else {
            // 等待到正确时间
            const delay = targetTime - currentTime;
            setTimeout(() => this.scheduleNextFrame(), delay);
        }
    }
    
    private displayFrame(frame: VideoFrame): void {
        const canvas = document.getElementById('video-canvas') as HTMLCanvasElement;
        const ctx = canvas.getContext('2d');
        
        if (ctx) {
            canvas.width = frame.displayWidth;
            canvas.height = frame.displayHeight;
            ctx.drawImage(frame, 0, 0);
        }
        
        frame.close();
    }
}
```

## 测试策略

### 单元测试

**平台端测试**:
- FrameRateDetector的SPS解析测试
- FrameRateDetector的时间戳分析测试
- TimestampManager的时间戳生成和验证测试
- FrameRatePacer的间隔计算测试

**前端测试**:
- FrameScheduler的帧调度逻辑测试
- FrameRateMonitor的FPS计算测试

**测试工具**: Jest (前端), Rust标准测试框架 (后端)

### 属性测试

**测试属性**:
- 属性1-17的所有正确性属性
- 使用随机生成的帧率、时间戳序列、倍速设置等
- 验证属性在各种输入下都保持为真

**测试工具**: proptest (Rust), fast-check (TypeScript)

### 集成测试

**测试场景**:
- 端到端帧率控制（直通播放）
- 端到端帧率控制（录像回放）
- 倍速播放帧率控制
- 帧率变化处理
- 时间戳不连续恢复

**测试工具**: 模拟视频流 + 真实播放器

### 性能测试

**测试指标**:
- 帧率检测延迟（<1秒）
- 时间戳计算精度（≥1微秒）
- 帧率控制误差（<±5%）
- 帧显示时机误差（<±16ms）
- CPU占用增加（<2%）

**测试工具**: 自定义性能测试框架


## 性能优化策略

### 1. 帧率检测优化

```rust
// 使用缓存避免重复解析SPS
pub struct CachedFrameRateDetector {
    sps_cache: HashMap<Vec<u8>, FrameRateInfo>,
    timestamp_buffer: CircularBuffer<(u64, SystemTime)>,
}

impl CachedFrameRateDetector {
    pub fn detect(&mut self, sps: &[u8]) -> Result<FrameRateInfo, Error> {
        // 检查缓存
        if let Some(info) = self.sps_cache.get(sps) {
            return Ok(info.clone());
        }
        
        // 解析并缓存
        let info = self.parse_sps(sps)?;
        self.sps_cache.insert(sps.to_vec(), info.clone());
        Ok(info)
    }
}
```

### 2. 时间戳计算优化

```rust
// 预计算常用转换因子
pub struct OptimizedTimestampManager {
    us_to_90khz_factor: f64,  // 90000.0 / 1_000_000.0
    khz_to_us_factor: f64,    // 1_000_000.0 / 90000.0
}

impl OptimizedTimestampManager {
    pub fn convert_to_90khz(&self, us: u64) -> u64 {
        (us as f64 * self.us_to_90khz_factor) as u64
    }
    
    pub fn convert_to_us(&self, khz: u64) -> u64 {
        (khz as f64 * self.khz_to_us_factor) as u64
    }
}
```

### 3. 帧调度优化

```typescript
// 使用requestAnimationFrame优化帧显示
class OptimizedFrameScheduler {
    private rafId: number | null = null;
    
    private scheduleFrame(): void {
        if (this.rafId !== null) {
            cancelAnimationFrame(this.rafId);
        }
        
        this.rafId = requestAnimationFrame((timestamp) => {
            this.displayNextFrame(timestamp);
        });
    }
    
    private displayNextFrame(timestamp: number): void {
        const frame = this.getFrameForTime(timestamp);
        if (frame) {
            this.renderFrame(frame);
        }
        
        if (this.frameQueue.length > 0) {
            this.scheduleFrame();
        }
    }
}
```

### 4. FPS计算优化

```typescript
// 使用滑动窗口优化FPS计算
class OptimizedFrameRateMonitor {
    private frameTimestamps: number[] = [];
    private windowSize: number = 60; // 1秒@60fps
    
    recordFrame(): void {
        const now = performance.now();
        this.frameTimestamps.push(now);
        
        // 保持窗口大小
        if (this.frameTimestamps.length > this.windowSize) {
            this.frameTimestamps.shift();
        }
    }
    
    getCurrentFps(): number {
        if (this.frameTimestamps.length < 2) return 0;
        
        const duration = this.frameTimestamps[this.frameTimestamps.length - 1] 
                       - this.frameTimestamps[0];
        const fps = (this.frameTimestamps.length - 1) / (duration / 1000);
        
        return fps;
    }
}
```

## 配置示例

### 平台端配置

```rust
// config.toml
[framerate_control]
# 帧率检测
detection_method = "auto"  # auto, sps, timestamp
default_fps = 30.0
min_samples_for_detection = 10

# 速率控制
enable_rate_control = true
tolerance_percent = 5.0

# 自适应调整
adaptive_enabled = true
bandwidth_low_threshold_mbps = 1.0
cpu_high_threshold_percent = 80.0
buffer_low_threshold_ms = 100

# 错误恢复
max_timestamp_gap_ms = 5000
fallback_fps = 30.0
smooth_transition_frames = 10
```

### 前端配置

```typescript
// frameRateConfig.ts
export const frameRateConfig = {
    // 帧调度
    maxEarlyFrameMs: 100,      // 最大提前时间
    maxLateFrameMs: 50,        // 最大延迟时间
    dropLateFrames: true,      // 是否丢弃延迟帧
    
    // FPS监控
    fpsWindowSize: 60,         // FPS计算窗口大小
    fpsUpdateInterval: 1000,   // FPS更新间隔（ms）
    
    // 同步策略
    syncStrategy: 'drop',      // 'drop' | 'duplicate' | 'adjust'
    speedAdjustmentStep: 0.01, // 速度调整步长
    
    // 性能优化
    useRequestAnimationFrame: true,
    enableHardwareAcceleration: true,
};
```

## 监控和日志

### 关键指标

```rust
pub struct FrameRateMetrics {
    // 检测指标
    pub detection_latency_ms: Histogram,
    pub detection_confidence: Gauge,
    pub detection_method: Counter,
    
    // 控制指标
    pub actual_fps: Gauge,
    pub target_fps: Gauge,
    pub speed_error_percent: Gauge,
    
    // 同步指标
    pub dropped_frames: Counter,
    pub duplicated_frames: Counter,
    pub late_frames: Counter,
    pub early_frames: Counter,
    
    // 性能指标
    pub timestamp_calc_time_us: Histogram,
    pub frame_schedule_time_us: Histogram,
}
```

### 日志级别

- **ERROR**: 严重错误（帧率检测失败、时间戳严重不连续）
- **WARN**: 警告（速度偏差超过阈值、频繁丢帧）
- **INFO**: 重要事件（帧率检测完成、帧率变化）
- **DEBUG**: 详细调试信息（每帧的时间戳、发送间隔）
- **TRACE**: 最详细的跟踪信息

## 总结

本设计文档描述了H264帧率控制系统的完整技术方案。通过精确的帧率检测、时间戳管理和播放控制，系统能够确保视频以正确的速度播放，解决"接收多少播放多少"的问题。

### 关键创新点

1. **自动帧率检测**: 从SPS或时间戳自动检测帧率，无需手动配置
2. **精确速率控制**: 播放速度误差<±5%
3. **微秒级时间戳**: 时间戳精度达到微秒级
4. **智能帧调度**: 根据时间戳精确控制帧显示时机
5. **自适应调整**: 根据网络和性能自动调整帧率

### 与现有系统集成

- **统一低延迟流系统**: 在现有的UnifiedStreamHandler中集成帧率控制
- **PlaybackSource**: 在录像回放中添加帧率控制
- **LiveStreamSource**: 在直通播放中添加帧率控制
- **MSE/WebCodecs播放器**: 在前端播放器中集成帧调度和监控

### 下一步

1. 创建任务列表（tasks.md）
2. 实现核心组件
3. 编写单元测试和属性测试
4. 进行集成测试和性能测试
5. 优化和调优

## 附录

### 参考资料

- H.264/AVC标准 ITU-T H.264
- ISO/IEC 14496-10 (MPEG-4 Part 10)
- WebCodecs API规范
- Performance API规范
- 统一低延迟视频流传输系统设计文档

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-14 | 初始版本 | 系统架构团队 |
