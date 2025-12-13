# 直通播放功能详细技术实现方案

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | 直通播放（Live Streaming）技术实现方案 |
| 创建日期 | 2025-12-13 |
| 版本 | v1.0 |
| 状态 | 实施中 |

## 概述

本文档详细描述了直通播放功能的技术实现方案，包括设备端屏幕录制、H.264编码、信令流程、数据传输和前端播放的完整流程。

## 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        设备端 (Device)                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │ Screen       │──▶│ H.264        │──▶│ Live Stream  │       │
│  │ Capturer     │   │ Encoder      │   │ Generator    │       │
│  │ (scrap)      │   │ (ffmpeg)     │   │              │       │
│  └──────────────┘   └──────────────┘   └──────┬───────┘       │
│                                                 │               │
│                                                 ▼               │
│                                         ┌──────────────┐        │
│                                         │ QUIC Client  │        │
│                                         │ (发送分片)   │        │
│                                         └──────┬───────┘        │
└────────────────────────────────────────────────┼────────────────┘
                                                 │ QUIC
                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                        平台端 (Platform)                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │ QUIC Server  │──▶│ Live Stream  │──▶│ Unified      │       │
│  │ (接收分片)   │   │ Source       │   │ Stream       │       │
│  │              │   │              │   │ Handler      │       │
│  └──────────────┘   └──────────────┘   └──────┬───────┘       │
│                                                 │               │
│                                                 ▼               │
│                                         ┌──────────────┐        │
│                                         │ HTTP3/SSE    │        │
│                                         │ Transport    │        │
│                                         └──────┬───────┘        │
└────────────────────────────────────────────────┼────────────────┘
                                                 │ HTTP3/SSE
                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                        前端 (Frontend)                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐       │
│  │ EventSource  │──▶│ Unified MSE  │──▶│ Video        │       │
│  │ (SSE Client) │   │ Player       │   │ Element      │       │
│  │              │   │              │   │              │       │
│  └──────────────┘   └──────────────┘   └──────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

## 第一阶段：设备端屏幕录制和编码

### 任务 0.1: 添加依赖

**目标**：在设备模拟器中添加屏幕录制和H.264编码所需的依赖。

**修改文件**：`device-simulator/Cargo.toml`

**添加依赖**：
```toml
[dependencies]
# 现有依赖...

# 屏幕捕获
scrap = "0.5"

# H.264编码（方案1：ffmpeg）
ffmpeg-next = "6.0"

# 或者 H.264编码（方案2：openh264，更轻量）
# openh264 = "0.5"

# 图像处理
image = "0.24"

# 帧率控制
tokio-util = { version = "0.7", features = ["time"] }
```

**系统依赖**：
- **Linux**: `sudo apt-get install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev`
- **macOS**: `brew install ffmpeg`
- **Windows**: 下载ffmpeg预编译库

### 任务 0.2: 实现屏幕捕获模块

**目标**：实现跨平台的屏幕捕获功能。

**创建文件**：`device-simulator/src/video/screen_capture.rs`

**核心结构**：
```rust
use scrap::{Capturer, Display};
use std::io::ErrorKind;
use std::time::Duration;

pub struct ScreenCapturer {
    capturer: Capturer,
    width: usize,
    height: usize,
    frame_interval: Duration,
}

impl ScreenCapturer {
    /// 创建屏幕捕获器
    /// 
    /// # 参数
    /// - fps: 目标帧率（默认30）
    pub fn new(fps: u32) -> Result<Self, Box<dyn std::error::Error>> {
        // 获取主显示器
        let display = Display::primary()?;
        let capturer = Capturer::new(display)?;
        
        let width = capturer.width();
        let height = capturer.height();
        let frame_interval = Duration::from_secs_f64(1.0 / fps as f64);
        
        Ok(Self {
            capturer,
            width,
            height,
            frame_interval,
        })
    }

    
    /// 捕获一帧
    /// 
    /// # 返回
    /// - Ok(Some(frame)): 成功捕获帧
    /// - Ok(None): 帧未准备好（需要重试）
    /// - Err: 捕获错误
    pub fn capture_frame(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match self.capturer.frame() {
            Ok(frame) => {
                // 转换BGRA到RGB
                let rgb_frame = self.bgra_to_rgb(&frame);
                Ok(Some(rgb_frame))
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // 帧未准备好
                Ok(None)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
    
    /// 将BGRA格式转换为RGB格式
    fn bgra_to_rgb(&self, bgra: &[u8]) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.width * self.height * 3);
        
        for chunk in bgra.chunks(4) {
            rgb.push(chunk[2]); // R
            rgb.push(chunk[1]); // G
            rgb.push(chunk[0]); // B
        }
        
        rgb
    }
    
    pub fn width(&self) -> usize {
        self.width
    }
    
    pub fn height(&self) -> usize {
        self.height
    }
    
    pub fn frame_interval(&self) -> Duration {
        self.frame_interval
    }
}
```

**关键点**：
- 使用`scrap`库进行跨平台屏幕捕获
- 支持配置帧率（默认30fps）
- 处理`WouldBlock`错误（帧未准备好）
- 转换BGRA到RGB格式（ffmpeg需要）


### 任务 0.3: 实现H.264编码器模块

**目标**：使用ffmpeg实现低延迟H.264编码。

**创建文件**：`device-simulator/src/video/h264_encoder.rs`

**核心结构**：
```rust
use ffmpeg_next as ffmpeg;
use ffmpeg::codec;
use ffmpeg::format::Pixel;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

pub struct H264Encoder {
    encoder: ffmpeg::encoder::Video,
    scaler: Context,
    frame_count: i64,
    time_base: ffmpeg::Rational,
}

impl H264Encoder {
    /// 创建H.264编码器
    /// 
    /// # 参数
    /// - width: 视频宽度
    /// - height: 视频高度
    /// - fps: 帧率
    /// - bitrate: 目标码率（bps）
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        ffmpeg::init()?;
        
        // 创建编码器
        let codec = ffmpeg::encoder::find(codec::Id::H264)
            .ok_or("H264 codec not found")?;
        
        let mut encoder = codec.video()?;
        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(Pixel::YUV420P);
        encoder.set_bit_rate(bitrate);
        encoder.set_time_base(ffmpeg::Rational::new(1, fps as i32));
        encoder.set_frame_rate(Some(ffmpeg::Rational::new(fps as i32, 1)));

        
        // 低延迟配置
        encoder.set_gop(fps); // GOP = 1秒
        encoder.set_max_b_frames(0); // 禁用B帧
        
        // 设置编码参数（低延迟优化）
        let mut dict = ffmpeg::Dictionary::new();
        dict.set("preset", "ultrafast"); // 最快编码速度
        dict.set("tune", "zerolatency"); // 零延迟调优
        dict.set("profile", "baseline"); // baseline profile
        
        let encoder = encoder.open_with(dict)?;
        
        // 创建缩放器（RGB -> YUV420P）
        let scaler = Context::get(
            Pixel::RGB24,
            width,
            height,
            Pixel::YUV420P,
            width,
            height,
            Flags::BILINEAR,
        )?;
        
        Ok(Self {
            encoder,
            scaler,
            frame_count: 0,
            time_base: ffmpeg::Rational::new(1, fps as i32),
        })
    }
    
    /// 编码一帧
    /// 
    /// # 参数
    /// - rgb_data: RGB24格式的帧数据
    /// 
    /// # 返回
    /// - 编码后的H.264数据包（可能为空，因为编码器可能缓冲）
    pub fn encode_frame(
        &mut self,
        rgb_data: &[u8],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        // 创建RGB帧
        let mut rgb_frame = Video::new(Pixel::RGB24, self.encoder.width(), self.encoder.height());
        rgb_frame.data_mut(0).copy_from_slice(rgb_data);
        
        // 转换为YUV420P
        let mut yuv_frame = Video::new(Pixel::YUV420P, self.encoder.width(), self.encoder.height());
        self.scaler.run(&rgb_frame, &mut yuv_frame)?;
        
        // 设置时间戳
        yuv_frame.set_pts(Some(self.frame_count));
        self.frame_count += 1;
        
        // 编码
        self.encoder.send_frame(&yuv_frame)?;
        
        // 接收编码后的数据包
        let mut packets = Vec::new();
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(_) => {
                    packets.push(packet.data().unwrap().to_vec());
                }
                Err(ffmpeg::Error::Other { errno: 11 }) => break, // EAGAIN
                Err(e) => return Err(Box::new(e)),
            }
        }
        
        Ok(packets)
    }
    
    /// 刷新编码器（获取缓冲的帧）
    pub fn flush(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        self.encoder.send_eof()?;
        
        let mut packets = Vec::new();
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(_) => {
                    packets.push(packet.data().unwrap().to_vec());
                }
                Err(_) => break,
            }
        }
        
        Ok(packets)
    }
}
```

**关键配置**：
- **preset=ultrafast**: 最快编码速度
- **tune=zerolatency**: 零延迟调优
- **profile=baseline**: 兼容性最好
- **max_b_frames=0**: 禁用B帧（降低延迟）
- **GOP=30**: 1秒一个关键帧（30fps）


### 任务 0.4: 实现实时流生成器

**目标**：整合屏幕捕获和H.264编码，生成实时视频流。

**创建文件**：`device-simulator/src/video/live_stream_generator.rs`

**核心结构**：
```rust
use super::screen_capture::ScreenCapturer;
use super::h264_encoder::H264Encoder;
use common::VideoSegment;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;

pub struct LiveStreamGenerator {
    capturer: ScreenCapturer,
    encoder: H264Encoder,
    session_id: Uuid,
    is_running: bool,
}

impl LiveStreamGenerator {
    /// 创建实时流生成器
    pub fn new(
        session_id: Uuid,
        fps: u32,
        bitrate: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let capturer = ScreenCapturer::new(fps)?;
        let width = capturer.width() as u32;
        let height = capturer.height() as u32;
        
        let encoder = H264Encoder::new(width, height, fps, bitrate)?;
        
        Ok(Self {
            capturer,
            encoder,
            session_id,
            is_running: false,
        })
    }
    
    /// 启动实时流
    /// 
    /// # 返回
    /// - 视频分片接收器
    pub async fn start_streaming(
        &mut self,
    ) -> Result<mpsc::Receiver<VideoSegment>, Box<dyn std::error::Error>> {
        if self.is_running {
            return Err("Stream already running".into());
        }
        
        self.is_running = true;
        let (tx, rx) = mpsc::channel(100);
        
        // 启动捕获和编码任务
        self.spawn_capture_task(tx).await?;
        
        Ok(rx)
    }

    
    async fn spawn_capture_task(
        &mut self,
        tx: mpsc::Sender<VideoSegment>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame_interval = self.capturer.frame_interval();
        let mut interval_timer = interval(frame_interval);
        let session_id = self.session_id;
        
        let mut frame_count = 0u64;
        let mut timestamp = 0.0f64;
        let frame_duration = frame_interval.as_secs_f64();
        
        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                
                // 捕获屏幕帧
                let rgb_frame = match self.capturer.capture_frame() {
                    Ok(Some(frame)) => frame,
                    Ok(None) => continue, // 帧未准备好
                    Err(e) => {
                        tracing::error!("Screen capture error: {}", e);
                        break;
                    }
                };
                
                // 编码帧
                let packets = match self.encoder.encode_frame(&rgb_frame) {
                    Ok(packets) => packets,
                    Err(e) => {
                        tracing::error!("Encoding error: {}", e);
                        continue;
                    }
                };
                
                // 发送编码后的数据包
                for packet in packets {
                    let is_keyframe = frame_count % 30 == 0; // 每30帧一个关键帧
                    
                    let segment = VideoSegment {
                        segment_id: Uuid::new_v4(),
                        session_id,
                        timestamp,
                        duration: frame_duration,
                        data: packet,
                        flags: if is_keyframe { 1 } else { 0 },
                    };
                    
                    if tx.send(segment).await.is_err() {
                        tracing::warn!("Receiver dropped, stopping stream");
                        break;
                    }
                }
                
                frame_count += 1;
                timestamp += frame_duration;
            }
            
            tracing::info!("Live stream generator stopped");
        });
        
        Ok(())
    }
    
    /// 停止实时流
    pub fn stop_streaming(&mut self) {
        self.is_running = false;
        // 通道关闭会自动停止任务
    }
}
```

**关键点**：
- 使用`tokio::time::interval`精确控制帧率
- 异步捕获和编码，不阻塞主线程
- 通过`mpsc::channel`传递视频分片
- 自动标记关键帧（每30帧）


## 第二阶段：信令流程实现

### 任务 0.5: 设备端处理直通播放信令

**目标**：设备端接收并处理平台的直通播放启动请求。

**修改文件**：`device-simulator/src/device_service.rs`

**添加消息类型**（在`common/src/protocol.rs`）：
```rust
pub enum MessageType {
    // 现有类型...
    StartLiveStream = 0x10,  // 新增：启动直通播放
    StopLiveStream = 0x11,   // 新增：停止直通播放
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartLiveStreamRequest {
    pub quality_preference: String,  // "low_latency" | "high_quality"
    pub target_latency_ms: u32,
    pub target_fps: u32,
    pub target_bitrate: usize,
}
```

**在`device_service.rs`中添加处理逻辑**：
```rust
use crate::video::LiveStreamGenerator;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DeviceService {
    // 现有字段...
    live_generator: Arc<Mutex<Option<LiveStreamGenerator>>>,
}

impl DeviceService {
    async fn handle_control_messages(
        connection: quinn::Connection,
        video_dir: std::path::PathBuf,
        device_id: String,
    ) -> Result<()> {
        loop {
            match connection.accept_bi().await {
                Ok((mut send, mut recv)) => {
                    // ... 现有代码 ...
                    
                    match msg.message_type {
                        // 现有处理...
                        
                        MessageType::StartLiveStream => {
                            info!("📡 Received start live stream request");
                            
                            // 解析请求
                            let request = bincode::deserialize::<StartLiveStreamRequest>(
                                &msg.payload
                            ).unwrap_or_else(|_| StartLiveStreamRequest {
                                quality_preference: "low_latency".to_string(),
                                target_latency_ms: 100,
                                target_fps: 30,
                                target_bitrate: 2_000_000, // 2 Mbps
                            });
                            
                            info!("  FPS: {}", request.target_fps);
                            info!("  Bitrate: {} bps", request.target_bitrate);
                            
                            // 创建实时流生成器
                            let mut generator = LiveStreamGenerator::new(
                                msg.session_id,
                                request.target_fps,
                                request.target_bitrate,
                            ).unwrap();
                            
                            // 启动流
                            let mut receiver = generator.start_streaming().await.unwrap();
                            
                            // 发送确认响应
                            let _ = send.write_all(b"OK").await;
                            let _ = send.finish().await;
                            
                            // 启动分片发送任务
                            let conn_clone = conn.clone();
                            tokio::spawn(async move {
                                while let Some(segment) = receiver.recv().await {
                                    // 通过QUIC单向流发送分片
                                    if let Ok(mut stream) = conn_clone.open_uni().await {
                                        let data = bincode::serialize(&segment).unwrap();
                                        let _ = stream.write_all(&data).await;
                                        let _ = stream.finish().await;
                                    }
                                }
                                info!("✓ Live stream ended");
                            });
                        }
                        
                        MessageType::StopLiveStream => {
                            info!("⏹️ Received stop live stream request");
                            // 停止流生成器
                            // TODO: 实现停止逻辑
                            let _ = send.write_all(b"OK").await;
                            let _ = send.finish().await;
                        }
                        
                        _ => {
                            debug!("Unhandled message type: {:?}", msg.message_type);
                        }
                    }
                }
                Err(e) => {
                    warn!("Accept bi-stream error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}
```

**关键点**：
- 接收`StartLiveStream`信令
- 创建并启动`LiveStreamGenerator`
- 通过QUIC单向流持续发送视频分片
- 发送确认响应给平台


### 任务 0.6: 平台端发送直通播放信令

**目标**：平台端向设备发送直通播放启动信令，并创建LiveStreamSource。

**修改文件**：`platform-server/src/http3/handlers.rs`

**完善`unified_stream_start`函数**：
```rust
pub async fn unified_stream_start(
    State(handler): State<Arc<UnifiedStreamHandler>>,
    State((device_manager, _, _, _)): State<AppState>,
    Json(req): Json<UnifiedStreamStartRequest>,
) -> Result<Json<ApiResponse<UnifiedStreamStartResponse>>, StatusCode> {
    let mode = req.mode.to_lowercase();
    
    let config = if let Some(cfg) = req.config {
        StreamConfig {
            low_latency: cfg.low_latency_mode.unwrap_or(true),
            target_latency_ms: cfg.target_latency_ms.unwrap_or(100),
            ..Default::default()
        }
    } else {
        StreamConfig::default()
    };

    let source: Box<dyn crate::streaming::StreamSource> = match mode.as_str() {
        "live" => {
            // 直通播放模式
            let device_id = req.source.device_id
                .ok_or(StatusCode::BAD_REQUEST)?;
            
            // 检查设备是否在线
            if !device_manager.is_device_online(&device_id) {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }

            // 获取设备连接
            let connection = device_manager
                .get_connection(&device_id)
                .ok_or(StatusCode::NOT_FOUND)?;

            // 创建会话ID
            let session_id = Uuid::new_v4();

            // 构建StartLiveStream请求
            let live_request = common::StartLiveStreamRequest {
                quality_preference: "low_latency".to_string(),
                target_latency_ms: config.target_latency_ms,
                target_fps: 30,
                target_bitrate: 2_000_000, // 2 Mbps
            };

            let request_data = bincode::serialize(&live_request)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // 发送信令到设备
            let signal_msg = ProtocolMessage {
                message_type: MessageType::StartLiveStream,
                payload: request_data,
                sequence_number: 1,
                timestamp: SystemTime::now(),
                session_id,
            };

            let data = bincode::serialize(&signal_msg)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // 打开双向流
            let (mut send, mut recv) = connection
                .open_bi()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // 发送请求
            send.write_all(&data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            send.finish()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // 等待确认
            let _ = recv
                .read_to_end(1024)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            tracing::info!("✓ Live stream started on device: {}", device_id);

            // 创建LiveStreamSource
            // 注意：需要传递一个channel接收器，用于接收从QUIC来的分片
            let (segment_tx, segment_rx) = tokio::sync::mpsc::channel(100);
            
            // 注册到QUIC服务器，接收该session_id的分片
            // TODO: 实现QUIC分片接收注册机制（任务0.7）
            
            let live_source = LiveStreamSource::new(device_id, segment_rx);
            Box::new(live_source)
        }
        "playback" => {
            // 录像回放模式（现有代码）
            // ...
        }
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 启动流会话
    let session_id = handler
        .start_stream(source, config.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 构建响应
    let response = UnifiedStreamStartResponse {
        session_id: session_id.to_string(),
        stream_url: format!("/api/v1/stream/{}/segments", session_id),
        control_url: format!("/api/v1/stream/{}/control", session_id),
        estimated_latency_ms: config.target_latency_ms,
    };

    Ok(Json(ApiResponse::success(response)))
}
```

**关键点**：
- 向设备发送`StartLiveStream` QUIC信令
- 等待设备确认响应
- 创建`LiveStreamSource`并传递分片接收器
- 需要实现QUIC分片接收注册机制（下一任务）


### 任务 0.7: 平台端QUIC实时分片接收

**目标**：平台端从QUIC接收设备发送的实时视频分片，并转发到LiveStreamSource。

**修改文件**：`platform-server/src/quic/server.rs`

**添加分片路由机制**：
```rust
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// QUIC分片路由器
/// 
/// 负责将接收到的视频分片路由到对应的会话
pub struct SegmentRouter {
    routes: Arc<RwLock<HashMap<Uuid, mpsc::Sender<VideoSegment>>>>,
}

impl SegmentRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册会话路由
    pub async fn register_session(
        &self,
        session_id: Uuid,
        sender: mpsc::Sender<VideoSegment>,
    ) {
        let mut routes = self.routes.write().await;
        routes.insert(session_id, sender);
        tracing::info!("✓ Registered segment route for session: {}", session_id);
    }
    
    /// 取消注册会话路由
    pub async fn unregister_session(&self, session_id: &Uuid) {
        let mut routes = self.routes.write().await;
        routes.remove(session_id);
        tracing::info!("✓ Unregistered segment route for session: {}", session_id);
    }
    
    /// 路由分片到对应会话
    pub async fn route_segment(&self, segment: VideoSegment) -> Result<(), String> {
        let routes = self.routes.read().await;
        
        if let Some(sender) = routes.get(&segment.session_id) {
            sender.send(segment).await
                .map_err(|e| format!("Failed to send segment: {}", e))?;
            Ok(())
        } else {
            Err(format!("No route found for session: {}", segment.session_id))
        }
    }
}

/// QUIC服务器（修改现有实现）
pub struct QuicServer {
    // 现有字段...
    segment_router: Arc<SegmentRouter>,
}

impl QuicServer {
    pub fn new(/* ... */) -> Self {
        Self {
            // 现有字段...
            segment_router: Arc::new(SegmentRouter::new()),
        }
    }
    
    pub fn get_segment_router(&self) -> Arc<SegmentRouter> {
        self.segment_router.clone()
    }
    
    /// 处理设备连接（修改现有方法）
    async fn handle_connection(
        connection: quinn::Connection,
        segment_router: Arc<SegmentRouter>,
    ) {
        tracing::info!("New device connection from: {}", connection.remote_address());
        
        // 处理单向流（视频分片）
        let router = segment_router.clone();
        tokio::spawn(async move {
            loop {
                match connection.accept_uni().await {
                    Ok(mut recv) => {
                        // 读取分片数据
                        match recv.read_to_end(10 * 1024 * 1024).await {
                            Ok(data) => {
                                // 解析VideoSegment
                                match bincode::deserialize::<VideoSegment>(&data) {
                                    Ok(segment) => {
                                        tracing::debug!(
                                            "📦 Received segment for session: {}",
                                            segment.session_id
                                        );
                                        
                                        // 路由分片
                                        if let Err(e) = router.route_segment(segment).await {
                                            tracing::warn!("Failed to route segment: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to deserialize segment: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to read uni stream: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Accept uni stream error: {}", e);
                        break;
                    }
                }
            }
        });
        
        // 处理双向流（控制信令）
        // ... 现有代码 ...
    }
}
```

**在`unified_stream_start`中注册路由**：
```rust
// 创建LiveStreamSource
let (segment_tx, segment_rx) = tokio::sync::mpsc::channel(100);

// 注册到QUIC服务器
let quic_server = /* 获取QUIC服务器实例 */;
let router = quic_server.get_segment_router();
router.register_session(session_id, segment_tx).await;

let live_source = LiveStreamSource::new(device_id, segment_rx);
```

**关键点**：
- 实现`SegmentRouter`管理会话路由
- 监听QUIC单向流接收视频分片
- 根据`session_id`路由分片到对应的`LiveStreamSource`
- 自动处理会话注册和取消注册


### 任务 0.8: 完善零缓冲转发机制

**目标**：实现<5ms的平台端处理延迟，边接收边转发。

**修改文件**：`platform-server/src/streaming/handler.rs`

**完善`UnifiedStreamHandler`**：
```rust
impl UnifiedStreamHandler {
    /// 启动流会话（修改现有方法）
    pub async fn start_stream(
        &self,
        source: Box<dyn StreamSource>,
        config: StreamConfig,
    ) -> Result<Uuid, StreamError> {
        let session_id = Uuid::new_v4();
        
        // 创建广播通道（用于多客户端订阅）
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        // 创建会话
        let session = StreamSession {
            session_id,
            source,
            config,
            state: StreamState::Initializing,
            stats: StreamStats::default(),
            created_at: SystemTime::now(),
            broadcast_tx: broadcast_tx.clone(),
        };
        
        self.sessions.insert(session_id, session);
        
        // 启动零缓冲转发任务
        self.start_forwarding_task(session_id).await?;
        
        Ok(session_id)
    }
    
    /// 启动零缓冲转发任务
    async fn start_forwarding_task(&self, session_id: Uuid) -> Result<(), StreamError> {
        let sessions = self.sessions.clone();
        
        tokio::spawn(async move {
            tracing::info!("🚀 Starting forwarding task for session: {}", session_id);
            
            loop {
                // 获取会话
                let mut session = match sessions.get_mut(&session_id) {
                    Some(s) => s,
                    None => {
                        tracing::warn!("Session not found: {}", session_id);
                        break;
                    }
                };
                
                // 从StreamSource获取下一个分片
                let segment = match session.source.next_segment().await {
                    Ok(Some(seg)) => seg,
                    Ok(None) => {
                        // 流结束
                        tracing::info!("Stream ended for session: {}", session_id);
                        session.state = StreamState::Stopped;
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Error reading segment: {}", e);
                        session.state = StreamState::Error(e.to_string());
                        break;
                    }
                };
                
                // 记录接收时间
                let receive_time = std::time::Instant::now();
                
                // 立即广播到所有订阅者（零缓冲转发）
                if let Err(e) = session.broadcast_tx.send(segment.clone()) {
                    tracing::warn!("No active subscribers: {}", e);
                }
                
                // 计算处理延迟
                let processing_latency = receive_time.elapsed();
                
                // 更新统计信息
                session.stats.total_segments += 1;
                session.stats.total_bytes += segment.data.len() as u64;
                session.stats.current_latency_ms = processing_latency.as_secs_f64() * 1000.0;
                
                // 如果处理延迟>5ms，记录警告
                if processing_latency.as_millis() > 5 {
                    tracing::warn!(
                        "⚠️ High processing latency: {}ms (session: {})",
                        processing_latency.as_millis(),
                        session_id
                    );
                }
                
                // 更新状态
                if session.state == StreamState::Initializing {
                    session.state = StreamState::Streaming;
                }
            }
            
            tracing::info!("✓ Forwarding task stopped for session: {}", session_id);
        });
        
        Ok(())
    }
}
```

**关键优化**：
- 使用`broadcast::channel`实现多客户端零拷贝广播
- 边接收边转发，无额外缓冲
- 记录处理延迟，超过5ms时告警
- 异步任务不阻塞主线程


### 任务 0.9: 端到端集成测试

**目标**：验证直通播放功能的完整流程和延迟指标。

**测试步骤**：

1. **启动设备模拟器**：
```bash
cd device-simulator
cargo run -- --device-id device_001 --server-addr 127.0.0.1:8443
```

2. **启动平台服务器**：
```bash
cd platform-server
cargo run
```

3. **启动前端**：
```bash
cd web-frontend
npm run dev
```

4. **测试直通播放**：
   - 在前端选择设备
   - 点击"开始直通播放"
   - 观察视频是否正常播放
   - 检查延迟指标

5. **验证延迟**：
```rust
// 在平台端添加延迟测试代码
#[tokio::test]
async fn test_end_to_end_latency() {
    // 启动模拟设备
    let device = start_mock_device().await;
    
    // 启动直通播放
    let session_id = start_live_stream("device_001").await.unwrap();
    
    // 订阅分片
    let mut receiver = subscribe_segments(session_id).await.unwrap();
    
    // 测量延迟
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let segment = receiver.recv().await.unwrap();
        
        // 计算端到端延迟
        let capture_time = segment.timestamp; // 设备端捕获时间
        let receive_time = SystemTime::now(); // 平台端接收时间
        let latency = receive_time.duration_since(UNIX_EPOCH).unwrap().as_secs_f64() - capture_time;
        
        latencies.push(latency * 1000.0); // 转换为毫秒
    }
    
    // 计算平均延迟
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let max_latency = latencies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    println!("Average latency: {:.2}ms", avg_latency);
    println!("Max latency: {:.2}ms", max_latency);
    
    // 验证延迟<100ms
    assert!(avg_latency < 100.0, "Average latency too high: {:.2}ms", avg_latency);
    assert!(max_latency < 200.0, "Max latency too high: {:.2}ms", max_latency);
}
```

6. **测试播放控制**：
   - 测试暂停功能
   - 测试恢复功能
   - 测试停止功能

7. **压力测试**：
   - 测试多客户端并发观看
   - 测试长时间运行稳定性
   - 测试网络抖动情况

**验收标准**：
- ✅ 视频能够正常播放
- ✅ 端到端延迟<100ms
- ✅ 平台端处理延迟<5ms
- ✅ 支持暂停/恢复控制
- ✅ 支持多客户端并发观看
- ✅ 长时间运行无内存泄漏

## 延迟优化建议

### 1. 设备端优化
- 使用硬件编码器（如果可用）
- 减小GOP大小（更频繁的关键帧）
- 使用更快的编码preset

### 2. 网络优化
- 使用QUIC的0-RTT连接
- 启用QUIC的快速重传
- 优化MTU大小

### 3. 平台端优化
- 使用零拷贝技术
- 减少序列化/反序列化开销
- 使用更高效的数据结构

### 4. 前端优化
- 减小MSE缓冲区大小
- 使用低延迟播放模式
- 及时清理旧缓冲

## 故障排查

### 问题1：编码器初始化失败
**原因**：ffmpeg库未安装或版本不兼容
**解决**：
```bash
# Linux
sudo apt-get install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev

# macOS
brew install ffmpeg

# Windows
# 下载ffmpeg预编译库并配置环境变量
```

### 问题2：屏幕捕获失败
**原因**：权限不足或显示器配置问题
**解决**：
- Linux: 确保有X11访问权限
- macOS: 在系统偏好设置中授予屏幕录制权限
- Windows: 以管理员身份运行

### 问题3：延迟过高
**原因**：编码参数配置不当或网络问题
**解决**：
- 检查编码preset（使用ultrafast）
- 检查tune参数（使用zerolatency）
- 检查网络延迟和丢包率
- 减小GOP大小
- 降低分辨率或码率

### 问题4：视频卡顿
**原因**：帧率不稳定或缓冲策略不当
**解决**：
- 确保帧率控制精确（使用tokio::time::interval）
- 调整前端缓冲策略
- 检查CPU占用率

## 总结

本实施方案详细描述了直通播放功能的完整实现流程，包括：

1. **设备端**：屏幕录制、H.264编码、实时流生成
2. **信令流程**：平台→设备的启动信令，设备→平台的确认响应
3. **数据传输**：QUIC实时分片传输，零缓冲转发
4. **前端播放**：MSE播放器，智能缓冲策略

通过遵循本方案，可以实现端到端延迟<100ms的低延迟直通播放功能。

## 下一步

1. 开始执行任务0.1：添加屏幕录制依赖
2. 按顺序完成任务0.2-0.9
3. 进行端到端集成测试
4. 根据测试结果进行优化调整

---

**文档版本**: v1.0  
**最后更新**: 2025-12-13  
**作者**: 系统架构团队
