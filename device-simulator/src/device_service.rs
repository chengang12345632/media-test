use crate::quic::QuicClient;
use crate::video::{VideoFile, VideoFileReader, VideoFormat};
use common::{
    FileListResponse, MessageType, ProtocolMessage, RecordingInfo, Result, VideoSegment,
    VideoStreamError,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

pub struct DeviceService {
    client: QuicClient,
    video_files: Vec<VideoFile>,
    device_id: String,
    video_dir: std::path::PathBuf,
}

impl DeviceService {
    pub fn new(client: QuicClient, video_files: Vec<VideoFile>, device_id: String, video_dir: std::path::PathBuf) -> Self {
        Self {
            client,
            video_files,
            device_id,
            video_dir,
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

        // 读取并发送视频数据
        let mut reader = VideoFileReader::new(&file_path).await?;
        let mut timestamp = file_req.seek_position.unwrap_or(0.0);
        let mut segment_count = 0;

        info!("📤 Streaming file to platform...");

        while let Some(chunk) = reader.read_chunk().await? {
            let mut segment = VideoSegment::new(chunk.clone(), timestamp, segment_count % 30 == 0);
            // 设置正确的 session_id，以便服务端能正确分发
            segment.session_id = session_id;

            // 通过单向流发送分片
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
}
