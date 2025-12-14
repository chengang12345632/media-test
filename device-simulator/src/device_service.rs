use crate::quic::QuicClient;
use crate::video::{VideoFile, VideoFormat};
use crate::video::{
    DefaultPlaybackController, DefaultTimelineManager, TimelineManager,
    DefaultFFmpegParser, FFmpegParser, DefaultFileStreamReader, FileStreamReader,
    KeyframeIndex, IndexOptimizationStrategy, TimelineFileBuilder,
};
use common::{
    FileListResponse, MessageType, ProtocolMessage, RecordingInfo, Result, VideoSegment,
    VideoStreamError,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct DeviceService {
    client: QuicClient,
    video_files: Vec<VideoFile>,
    device_id: String,
    video_dir: std::path::PathBuf,
    playback_controller: Arc<RwLock<DefaultPlaybackController>>,
    timeline_manager: Arc<DefaultTimelineManager>,
    ffmpeg_parser: Option<Arc<DefaultFFmpegParser>>,
    file_reader: Arc<DefaultFileStreamReader>,
}

impl DeviceService {
    pub fn new(client: QuicClient, video_files: Vec<VideoFile>, device_id: String, video_dir: std::path::PathBuf) -> Self {
        Self::new_with_config(client, video_files, device_id, video_dir, None)
    }
    
    pub fn new_with_config(
        client: QuicClient,
        video_files: Vec<VideoFile>,
        device_id: String,
        video_dir: std::path::PathBuf,
        config: Option<crate::config::Config>,
    ) -> Self {
        // 使用提供的配置或加载默认配置
        let config = config.unwrap_or_else(|| {
            crate::config::Config::load().expect("Failed to load config")
        });
        
        // 初始化播放控制器
        let playback_controller = Arc::new(RwLock::new(DefaultPlaybackController::new()));
        
        // 初始化 Timeline 管理器
        let timeline_manager = Arc::new(DefaultTimelineManager::new());
        
        // 根据配置初始化 FFmpeg 解析器
        let ffmpeg_parser = if config.ffmpeg_enabled {
            let parser = DefaultFFmpegParser::new();
            info!("✓ FFmpeg parser initialized");
            Some(Arc::new(parser))
        } else {
            info!("ℹ FFmpeg parser disabled by configuration");
            None
        };
        
        // 初始化文件读取器
        let file_reader = Arc::new(DefaultFileStreamReader::new());
        
        info!("✓ DeviceService initialized with configuration:");
        info!("  - Keyframe index strategy: {:?}", config.keyframe_index_strategy);
        info!("  - Timeline cache: {}", if config.timeline_cache_enabled { "enabled" } else { "disabled" });
        info!("  - FFmpeg: {}", if config.ffmpeg_enabled { "enabled" } else { "disabled" });
        info!("  - Playback speed range: {}x - {}x", config.playback_speed_min, config.playback_speed_max);
        
        Self {
            client,
            video_files,
            device_id,
            video_dir,
            playback_controller,
            timeline_manager,
            ffmpeg_parser,
            file_reader,
        }
    }

    fn spawn_control_message_handler(&self) -> tokio::task::JoinHandle<()> {
        let conn = self
            .client
            .get_connection()
            .expect("Connection must exist")
            .clone();
        let video_dir = self.video_dir.clone();
        let device_id = self.device_id.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_control_messages(conn, video_dir, device_id).await {
                error!("Control message handler error: {}", e);
            }
        })
    }

    pub async fn run(mut self) -> Result<()> {
        // 启动控制消息处理任务
        let mut control_task_handle = self.spawn_control_message_handler();

        // 启动心跳任务
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        let mut reconnect_attempts = 0u32;
        const MAX_RECONNECT_DELAY_SECS: u64 = 10; // 最大重连间隔10秒

        loop {
            heartbeat_interval.tick().await;

            // 检查连接状态
            if !self.client.is_connected() {
                warn!("Connection lost, attempting to reconnect...");
                reconnect_attempts += 1;

                // 取消旧的控制消息处理任务
                control_task_handle.abort();

                // 计算重连延迟：指数退避，最大10秒
                // 延迟序列：1s, 2s, 4s, 8s, 10s, 10s, ...
                let delay_secs = std::cmp::min(
                    2u64.saturating_pow(reconnect_attempts.saturating_sub(1)),
                    MAX_RECONNECT_DELAY_SECS,
                );

                info!(
                    "Reconnection attempt #{}, waiting {}s before retry...",
                    reconnect_attempts, delay_secs
                );

                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;

                match self.client.reconnect().await {
                    Ok(_) => {
                        info!("✓ Reconnected successfully after {} attempts", reconnect_attempts);
                        reconnect_attempts = 0;
                        // 重置心跳间隔
                        heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
                        // 重新启动控制消息处理任务
                        control_task_handle = self.spawn_control_message_handler();
                        info!("✓ Control message handler restarted");
                    }
                    Err(e) => {
                        warn!(
                            "✗ Reconnection attempt #{} failed: {}",
                            reconnect_attempts, e
                        );
                        continue;
                    }
                }
            }

            // 发送心跳
            match self.client.send_heartbeat().await {
                Ok(_) => {
                    debug!("💓 Heartbeat sent");
                    // 心跳成功，重置重连计数
                    if reconnect_attempts > 0 {
                        info!("✓ Connection restored, resetting reconnect counter");
                        reconnect_attempts = 0;
                    }
                }
                Err(e) => {
                    warn!("✗ Heartbeat failed: {}", e);
                    // 心跳失败，立即断开连接，下一次循环将触发重连
                    self.client.disconnect();
                }
            }
        }
    }

    async fn handle_control_messages(
        connection: quinn::Connection,
        video_dir: std::path::PathBuf,
        device_id: String,
    ) -> Result<()> {
        loop {
            match connection.accept_bi().await {
                Ok((mut send, mut recv)) => {
                    let dir = video_dir.clone();
                    let dev_id = device_id.clone();
                    let conn = connection.clone();
                    tokio::spawn(async move {
                        match recv.read_to_end(1024 * 1024).await {
                            Ok(buf) => {
                                if let Ok(msg) = bincode::deserialize::<ProtocolMessage>(&buf) {
                                    debug!("Received control message: {:?}", msg.message_type);

                                    match msg.message_type {
                                        MessageType::FileListQuery => {
                                            info!("📋 Received file list query");
                                            // 动态扫描视频目录
                                            let files = crate::video::scan_video_files(&dir).unwrap_or_default();
                                            info!("📂 Found {} video file(s) in directory", files.len());
                                            
                                            if let Ok(response) =
                                                Self::build_file_list_response(&files, &dev_id)
                                            {
                                                let response_msg = ProtocolMessage {
                                                    message_type: MessageType::FileListResponse,
                                                    payload: response,
                                                    sequence_number: msg.sequence_number,
                                                    timestamp: SystemTime::now(),
                                                    session_id: msg.session_id,
                                                };

                                                if let Ok(data) = bincode::serialize(&response_msg)
                                                {
                                                    let _ = send.write_all(&data).await;
                                                    let _ = send.finish().await;
                                                    info!("✓ Sent file list with {} files", files.len());
                                                }
                                            }
                                        }
                                        MessageType::FileRequest => {
                                            info!("📹 Received playback request");
                                            // 解析文件请求
                                            if let Ok(file_req) =
                                                bincode::deserialize::<common::FileRequest>(
                                                    &msg.payload,
                                                )
                                            {
                                                info!("  File: {}", file_req.file_path);
                                                info!("  Seek: {:?}", file_req.seek_position);

                                                // 发送确认响应
                                                let _ = send.write_all(b"OK").await;
                                                let _ = send.finish().await;

                                                // 启动回放任务
                                                let conn_clone = conn.clone();
                                                tokio::spawn(async move {
                                                    if let Err(e) = Self::handle_playback_request(
                                                        conn_clone,
                                                        file_req,
                                                        msg.session_id,
                                                    )
                                                    .await
                                                    {
                                                        error!("Playback error: {}", e);
                                                    }
                                                });
                                            }
                                        }
                                        MessageType::StartLiveStream => {
                                            info!("📡 Received start live stream request");
                                            
                                            // 解析请求
                                            let request = bincode::deserialize::<common::StartLiveStreamRequest>(
                                                &msg.payload
                                            ).unwrap_or_else(|_| common::StartLiveStreamRequest {
                                                quality_preference: "low_latency".to_string(),
                                                target_latency_ms: 100,
                                                target_fps: 30,
                                                target_bitrate: 2_000_000, // 2 Mbps
                                            });
                                            
                                            info!("  FPS: {}", request.target_fps);
                                            info!("  Bitrate: {} Mbps", request.target_bitrate / 1_000_000);
                                            
                                            // 发送确认响应
                                            let _ = send.write_all(b"OK").await;
                                            let _ = send.finish().await;
                                            
                                            // 启动直通播放任务
                                            let conn_clone = conn.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = Self::handle_live_stream_request(
                                                    conn_clone,
                                                    request,
                                                    msg.session_id,
                                                )
                                                .await
                                                {
                                                    error!("Live stream error: {}", e);
                                                }
                                            });
                                        }
                                        MessageType::StopLiveStream => {
                                            info!("⏹️ Received stop live stream request");
                                            // 停止逻辑通过关闭 receiver 通道自动实现
                                            // 当前端停止接收时，发送任务会自动结束
                                            let _ = send.write_all(b"OK").await;
                                            let _ = send.finish().await;
                                        }
                                        MessageType::SeekToKeyframe => {
                                            info!("⏩ Received seek to keyframe request");
                                            if let Ok(seek_req) = bincode::deserialize::<common::SeekToKeyframeRequest>(&msg.payload) {
                                                info!("  Target time: {:.2}s", seek_req.target_time);
                                                
                                                // 处理 seek 请求
                                                let response = Self::handle_seek_to_keyframe(seek_req).await;
                                                
                                                // 发送响应
                                                if let Ok(response_data) = bincode::serialize(&response) {
                                                    let response_msg = ProtocolMessage {
                                                        message_type: MessageType::SeekResponse,
                                                        payload: response_data,
                                                        sequence_number: msg.sequence_number,
                                                        timestamp: SystemTime::now(),
                                                        session_id: msg.session_id,
                                                    };
                                                    
                                                    if let Ok(data) = bincode::serialize(&response_msg) {
                                                        let _ = send.write_all(&data).await;
                                                        let _ = send.finish().await;
                                                    }
                                                }
                                            }
                                        }
                                        MessageType::SetPlaybackSpeed => {
                                            info!("⚡ Received set playback speed request");
                                            if let Ok(speed_req) = bincode::deserialize::<common::SetPlaybackSpeedRequest>(&msg.payload) {
                                                info!("  Speed: {}x", speed_req.speed);
                                                
                                                // 处理播放速率变更
                                                let response = Self::handle_set_playback_speed(speed_req).await;
                                                
                                                // 发送响应
                                                if let Ok(response_data) = bincode::serialize(&response) {
                                                    let response_msg = ProtocolMessage {
                                                        message_type: MessageType::StatusResponse,
                                                        payload: response_data,
                                                        sequence_number: msg.sequence_number,
                                                        timestamp: SystemTime::now(),
                                                        session_id: msg.session_id,
                                                    };
                                                    
                                                    if let Ok(data) = bincode::serialize(&response_msg) {
                                                        let _ = send.write_all(&data).await;
                                                        let _ = send.finish().await;
                                                    }
                                                }
                                            }
                                        }
                                        MessageType::GetKeyframeIndex => {
                                            info!("📋 Received get keyframe index request");
                                            if let Ok(index_req) = bincode::deserialize::<common::GetKeyframeIndexRequest>(&msg.payload) {
                                                info!("  File: {}", index_req.file_path);
                                                
                                                // 处理关键帧索引请求
                                                let response = Self::handle_get_keyframe_index(index_req).await;
                                                
                                                // 发送响应
                                                if let Ok(response_data) = bincode::serialize(&response) {
                                                    let response_msg = ProtocolMessage {
                                                        message_type: MessageType::KeyframeIndexResponse,
                                                        payload: response_data,
                                                        sequence_number: msg.sequence_number,
                                                        timestamp: SystemTime::now(),
                                                        session_id: msg.session_id,
                                                    };
                                                    
                                                    if let Ok(data) = bincode::serialize(&response_msg) {
                                                        let _ = send.write_all(&data).await;
                                                        let _ = send.finish().await;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            debug!("Unhandled message type: {:?}", msg.message_type);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to read control message: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("Accept bi-stream error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    fn build_file_list_response(
        video_files: &[VideoFile],
        device_id: &str,
    ) -> Result<Vec<u8>> {
        let recordings: Vec<RecordingInfo> = video_files
            .iter()
            .map(|vf| {
                let metadata = std::fs::metadata(&vf.path).ok();
                let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or_else(SystemTime::now);

                RecordingInfo {
                    file_id: format!("{}_{}", device_id, vf.name),
                    device_id: device_id.to_string(),
                    file_name: vf.name.clone(),
                    file_path: vf.path.to_string_lossy().to_string(),
                    file_size,
                    duration: 10.0, // 估算
                    format: match vf.format {
                        VideoFormat::H264 => "h264".to_string(),
                        VideoFormat::MP4 => "mp4".to_string(),
                    },
                    resolution: "1280x720".to_string(),
                    bitrate: 5_000_000,
                    frame_rate: 60.0,
                    created_time: modified,
                    modified_time: modified,
                }
            })
            .collect();

        let response = FileListResponse { files: recordings };
        bincode::serialize(&response)
            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))
    }

    async fn handle_playback_request(
        connection: quinn::Connection,
        file_req: common::FileRequest,
        session_id: uuid::Uuid,
    ) -> Result<()> {
        use crate::video::{LiveStreamGeneratorFile, VideoFileReader, VideoFormat};
        
        info!("🎬 Starting playback for: {} (session: {})", file_req.file_path, session_id);

        // 从 file_id 中提取文件名（格式: device_001_filename）
        // 分割成最多3部分：device, 001, filename
        let parts: Vec<&str> = file_req.file_path.splitn(3, '_').collect();
        let file_name = if parts.len() >= 3 {
            parts[2]
        } else {
            &file_req.file_path
        };

        // 在 test-videos 目录中查找文件
        let file_path = PathBuf::from("test-videos").join(file_name);
        if !file_path.exists() {
            error!("File not found: {:?}", file_path);
            return Err(VideoStreamError::RecordingNotFound(file_req.file_path));
        }

        // 尝试加载或构建关键帧索引
        let keyframe_index = Self::load_or_build_keyframe_index(&file_path).await;
        
        if let Some(ref index) = keyframe_index {
            info!("✓ Keyframe index loaded: {} keyframes, {:.2}s duration", 
                  index.entries.len(), index.total_duration);
        }

        // 检测文件格式
        let reader = VideoFileReader::new(&file_path).await?;
        let is_h264 = matches!(reader.format(), VideoFormat::H264);
        drop(reader);

        if is_h264 {
            // H.264 文件：使用 LiveStreamGeneratorFile 按 NAL unit 分割
            info!("📹 H.264 file detected, using NAL unit streaming");
            
            let mut generator = LiveStreamGeneratorFile::new(
                session_id,
                30, // 默认 30fps
                5_000_000, // 默认 5Mbps
                file_path,
            ).map_err(|e| VideoStreamError::QuicError(format!("Failed to create generator: {}", e)))?;
            
            let mut receiver = generator.start_streaming().await
                .map_err(|e| VideoStreamError::QuicError(format!("Failed to start streaming: {}", e)))?;
            
            info!("📤 Streaming H.264 file to platform...");
            let mut segment_count = 0;
            
            while let Some(segment) = receiver.recv().await {
                match connection.open_uni().await {
                    Ok(mut stream) => {
                        let data = bincode::serialize(&segment)
                            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;
                        
                        if let Err(e) = stream.write_all(&data).await {
                            error!("Failed to write segment: {}", e);
                            break;
                        }
                        
                        if let Err(e) = stream.finish().await {
                            error!("Failed to finish stream: {}", e);
                            break;
                        }
                        
                        segment_count += 1;
                        if segment_count % 100 == 0 {
                            info!("📦 Sent {} H.264 segments", segment_count);
                        }
                    }
                    Err(e) => {
                        error!("Failed to open stream: {}", e);
                        break;
                    }
                }
            }
            
            info!("✓ H.264 playback completed: {} segments sent", segment_count);
        } else {
            // MP4 或其他格式：使用简单的块读取
            info!("📹 MP4/other format detected, using chunk streaming");
            
            let mut reader = VideoFileReader::new(&file_path).await?;
            let mut timestamp = file_req.seek_position.unwrap_or(0.0);
            let mut segment_count = 0;

            info!("📤 Streaming file to platform...");

            while let Some(chunk) = reader.read_chunk().await? {
                let mut segment = VideoSegment::new(chunk.clone(), timestamp, segment_count % 30 == 0);
                segment.session_id = session_id;

                let mut stream = connection.open_uni().await.map_err(|e| {
                    VideoStreamError::QuicError(format!("Failed to open stream: {}", e))
                })?;

                let data = bincode::serialize(&segment)
                    .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;

                stream
                    .write_all(&data)
                    .await
                    .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;
                stream
                    .finish()
                    .await
                    .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

                segment_count += 1;
                timestamp += 0.033; // ~30fps

                // 控制发送速率
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    (33.0 / file_req.playback_rate) as u64,
                ))
                .await;
            }

            info!("✓ Playback completed: {} segments sent", segment_count);
        }
        
        Ok(())
    }
    
    async fn handle_live_stream_request(
        connection: quinn::Connection,
        request: common::StartLiveStreamRequest,
        session_id: uuid::Uuid,
    ) -> Result<()> {
        use crate::video::LiveStreamGeneratorFile;
        
        info!("🎬 Starting live stream (session: {})", session_id);
        info!("  FPS: {}", request.target_fps);
        info!("  Bitrate: {} Mbps", request.target_bitrate / 1_000_000);
        
        // 使用H.264裸流文件
        let h264_file = std::path::PathBuf::from("test-videos/sample_720p_60fps.h264");
        
        // 创建实时流生成器（从文件读取）
        let mut generator = LiveStreamGeneratorFile::new(
            session_id,
            request.target_fps,
            request.target_bitrate,
            h264_file,
        ).map_err(|e| VideoStreamError::QuicError(format!("Failed to create generator: {}", e)))?;
        
        // 启动流
        let mut receiver = generator.start_streaming().await
            .map_err(|e| VideoStreamError::QuicError(format!("Failed to start streaming: {}", e)))?;
        
        info!("📤 Streaming live video to platform...");
        
        let mut segment_count = 0;
        
        // 接收并发送分片
        while let Some(segment) = receiver.recv().await {
            // 通过QUIC单向流发送分片
            match connection.open_uni().await {
                Ok(mut stream) => {
                    let data = bincode::serialize(&segment)
                        .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;
                    
                    if let Err(e) = stream.write_all(&data).await {
                        error!("Failed to write segment: {}", e);
                        break;
                    }
                    
                    if let Err(e) = stream.finish().await {
                        error!("Failed to finish stream: {}", e);
                        break;
                    }
                    
                    segment_count += 1;
                    
                    if segment_count % 30 == 0 {
                        debug!("📤 Sent {} segments", segment_count);
                    }
                }
                Err(e) => {
                    error!("Failed to open uni stream: {}", e);
                    break;
                }
            }
        }
        
        info!("✓ Live stream completed: {} segments sent", segment_count);
        Ok(())
    }
    
    /// 加载或构建关键帧索引
    async fn load_or_build_keyframe_index(video_path: &PathBuf) -> Option<KeyframeIndex> {
        let timeline_manager = DefaultTimelineManager::new();
        let file_reader = DefaultFileStreamReader::new();
        
        // 1. 尝试从 Timeline 文件加载
        match timeline_manager.load_timeline(video_path).await {
            Ok(Some(timeline)) => {
                // 验证 Timeline 文件
                match timeline_manager.validate_timeline(&timeline, video_path).await {
                    Ok(true) => {
                        info!("✓ Loaded keyframe index from timeline cache");
                        return Some(timeline.keyframe_index);
                    }
                    Ok(false) => {
                        warn!("⚠ Timeline file invalid, rebuilding index");
                    }
                    Err(e) => {
                        warn!("⚠ Timeline validation error: {}, rebuilding index", e);
                    }
                }
            }
            Ok(None) => {
                info!("📋 No timeline cache found, building index");
            }
            Err(e) => {
                warn!("⚠ Failed to load timeline: {}, building index", e);
            }
        }
        
        // 2. 尝试使用 FFmpeg 提取关键帧信息
        let ffmpeg_parser = DefaultFFmpegParser::new();
        if let Ok(true) = ffmpeg_parser.check_availability().await {
            match ffmpeg_parser.extract_metadata(video_path).await {
                Ok(metadata) => {
                    info!("✓ Extracted metadata using FFmpeg");
                    
                    // 使用 FFmpeg 提取的关键帧信息构建索引
                    if let Ok(keyframes) = ffmpeg_parser.extract_keyframes(video_path).await {
                        info!("✓ Extracted {} keyframes using FFmpeg", keyframes.len());
                            
                            // 构建关键帧索引
                            let index = Self::build_index_from_ffmpeg(&keyframes, &metadata);
                            
                            // 保存到 Timeline 文件
                            if let Err(e) = Self::save_timeline_file(
                                video_path,
                                &index,
                                &metadata,
                                &timeline_manager,
                            ).await {
                                warn!("⚠ Failed to save timeline: {}", e);
                            }
                            
                            return Some(index);
                    }
                }
                Err(e) => {
                    warn!("⚠ FFmpeg metadata extraction failed: {}", e);
                }
            }
        }
        
        // 3. 回退到基础解析器
        info!("📋 Using fallback parser to build index");
        match tokio::fs::File::open(video_path).await {
            Ok(mut file) => {
                match file_reader.build_keyframe_index_with_strategy(
                    &mut file,
                    IndexOptimizationStrategy::Adaptive,
                ).await {
                    Ok(index) => {
                        info!("✓ Built keyframe index: {} keyframes", index.entries.len());
                        
                        // 保存到 Timeline 文件（使用基础元数据）
                        if let Err(e) = Self::save_timeline_file_basic(
                            video_path,
                            &index,
                            &timeline_manager,
                        ).await {
                            warn!("⚠ Failed to save timeline: {}", e);
                        }
                        
                        Some(index)
                    }
                    Err(e) => {
                        error!("✗ Failed to build keyframe index: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("✗ Failed to open file: {}", e);
                None
            }
        }
    }
    
    /// 从 FFmpeg 提取的关键帧信息构建索引
    fn build_index_from_ffmpeg(
        keyframe_timestamps: &[f64],
        metadata: &crate::video::FFmpegVideoInfo,
    ) -> KeyframeIndex {
        use crate::video::{KeyframeEntry, FrameType};
        
        let entries: Vec<KeyframeEntry> = keyframe_timestamps
            .iter()
            .enumerate()
            .map(|(i, &timestamp)| KeyframeEntry {
                timestamp,
                file_offset: 0, // FFmpeg 不提供文件偏移
                frame_size: 0,  // FFmpeg 不提供帧大小
                gop_size: if i + 1 < keyframe_timestamps.len() {
                    ((keyframe_timestamps[i + 1] - timestamp) * metadata.frame_rate) as u32
                } else {
                    30 // 默认 GOP 大小
                },
                frame_type: FrameType::I,
            })
            .collect();
        
        KeyframeIndex {
            entries,
            total_duration: metadata.duration,
            index_precision: 1.0 / metadata.frame_rate,
            memory_optimized: true,
            optimization_strategy: IndexOptimizationStrategy::Adaptive,
            memory_usage: keyframe_timestamps.len() * std::mem::size_of::<KeyframeEntry>(),
        }
    }
    
    /// 保存 Timeline 文件（使用 FFmpeg 元数据）
    async fn save_timeline_file(
        video_path: &PathBuf,
        index: &KeyframeIndex,
        _metadata: &crate::video::FFmpegVideoInfo,
        timeline_manager: &DefaultTimelineManager,
    ) -> Result<()> {
        let timeline = TimelineFileBuilder::new(video_path.clone(), index.clone())
            .build(timeline_manager).await
            .map_err(|e| VideoStreamError::QuicError(format!("Failed to build timeline: {}", e)))?;
        
        timeline_manager.save_timeline(&timeline).await
            .map_err(|e| VideoStreamError::QuicError(format!("Failed to save timeline: {}", e)))
    }
    
    /// 保存 Timeline 文件（使用基础元数据）
    async fn save_timeline_file_basic(
        video_path: &PathBuf,
        index: &KeyframeIndex,
        timeline_manager: &DefaultTimelineManager,
    ) -> Result<()> {
        let timeline = TimelineFileBuilder::new(video_path.clone(), index.clone())
            .build(timeline_manager).await
            .map_err(|e| VideoStreamError::QuicError(format!("Failed to build timeline: {}", e)))?;
        
        timeline_manager.save_timeline(&timeline).await
            .map_err(|e| VideoStreamError::QuicError(format!("Failed to save timeline: {}", e)))
    }
    
    /// 处理精确定位到关键帧请求
    async fn handle_seek_to_keyframe(
        request: common::SeekToKeyframeRequest,
    ) -> common::SeekToKeyframeResponse {
        use std::time::Instant;
        
        let start_time = Instant::now();
        
        // TODO: 实现实际的 seek 逻辑
        // 这里需要访问当前播放会话的关键帧索引
        // 暂时返回模拟响应
        
        let execution_time = start_time.elapsed();
        
        common::SeekToKeyframeResponse {
            requested_time: request.target_time,
            actual_time: request.target_time, // 暂时返回请求的时间
            keyframe_offset: 0,
            precision_achieved: 0.0,
            execution_time_ms: execution_time.as_millis() as u64,
            success: true,
            error_message: None,
        }
    }
    
    /// 处理设置播放速率请求
    async fn handle_set_playback_speed(
        request: common::SetPlaybackSpeedRequest,
    ) -> common::SetPlaybackSpeedResponse {
        // 验证播放速率范围
        if request.speed < 0.25 || request.speed > 4.0 {
            return common::SetPlaybackSpeedResponse {
                speed: request.speed,
                success: false,
                error_message: Some(format!(
                    "Invalid playback speed: {}. Must be between 0.25 and 4.0",
                    request.speed
                )),
            };
        }
        
        // TODO: 实现实际的播放速率调整逻辑
        // 这里需要访问当前播放会话的控制器
        
        info!("✓ Playback speed set to {}x", request.speed);
        
        common::SetPlaybackSpeedResponse {
            speed: request.speed,
            success: true,
            error_message: None,
        }
    }
    
    /// 处理获取关键帧索引请求
    async fn handle_get_keyframe_index(
        request: common::GetKeyframeIndexRequest,
    ) -> common::GetKeyframeIndexResponse {
        // 解析文件路径
        let file_path = PathBuf::from(&request.file_path);
        
        // 加载或构建关键帧索引
        match Self::load_or_build_keyframe_index(&file_path).await {
            Some(index) => {
                // 转换为传输格式
                let keyframes: Vec<common::KeyframeEntry> = index
                    .entries
                    .iter()
                    .map(|entry| common::KeyframeEntry {
                        timestamp: entry.timestamp,
                        file_offset: entry.file_offset,
                        frame_size: entry.frame_size,
                    })
                    .collect();
                
                info!("✓ Returning {} keyframes for {}", keyframes.len(), request.file_path);
                
                common::GetKeyframeIndexResponse {
                    file_path: request.file_path,
                    keyframes,
                    total_duration: index.total_duration,
                    success: true,
                    error_message: None,
                }
            }
            None => {
                error!("✗ Failed to load keyframe index for {}", request.file_path);
                
                common::GetKeyframeIndexResponse {
                    file_path: request.file_path,
                    keyframes: vec![],
                    total_duration: 0.0,
                    success: false,
                    error_message: Some("Failed to load or build keyframe index".to_string()),
                }
            }
        }
    }
}
