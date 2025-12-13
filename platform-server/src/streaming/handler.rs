// 统一低延迟视频流传输系统 - UnifiedStreamHandler实现
//
// 本模块实现了统一的流处理器，负责管理所有流会话（直通和回放）。
//
// # 特性
//
// - 统一的流会话管理
// - 零缓冲转发机制（处理延迟<5ms）
// - 并发客户端转发
// - 延迟监控和统计
// - 支持100+并发流会话

use super::source::{StreamError, StreamInfo, StreamSource, VideoSegment};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};
use uuid::Uuid;

/// 缓冲区配置
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// 最小缓冲量（毫秒）
    pub min_buffer_ms: u32,
    /// 目标缓冲量（毫秒）
    pub target_buffer_ms: u32,
    /// 最大缓冲量（毫秒）
    pub max_buffer_ms: u32,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            min_buffer_ms: 100,
            target_buffer_ms: 500,
            max_buffer_ms: 2000,
        }
    }
}

/// 流配置
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// 是否启用低延迟模式
    pub low_latency: bool,
    /// 目标延迟（毫秒）
    pub target_latency_ms: u32,
    /// 延迟告警阈值（毫秒）
    pub latency_alert_threshold_ms: u32,
    /// 缓冲区配置
    pub buffer_config: BufferConfig,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            low_latency: true,
            target_latency_ms: 100,
            latency_alert_threshold_ms: 200, // 默认200ms告警阈值
            buffer_config: BufferConfig::default(),
        }
    }
}

/// 流统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamStats {
    /// 总分片数
    pub total_segments: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 平均延迟（毫秒）
    pub average_latency_ms: f64,
    /// 当前延迟（毫秒）
    pub current_latency_ms: f64,
    /// 最小延迟（毫秒）
    pub min_latency_ms: f64,
    /// 最大延迟（毫秒）
    pub max_latency_ms: f64,
    /// P50延迟（毫秒）
    pub p50_latency_ms: f64,
    /// P95延迟（毫秒）
    pub p95_latency_ms: f64,
    /// P99延迟（毫秒）
    pub p99_latency_ms: f64,
    /// 吞吐量（Mbps）
    pub throughput_mbps: f64,
    /// 丢包率
    pub packet_loss_rate: f64,
    /// 延迟历史（用于计算百分位数）
    #[serde(skip)]
    latency_history: std::collections::VecDeque<f64>,
    /// 统计开始时间
    #[serde(skip)]
    start_time: Option<SystemTime>,
}

/// 延迟告警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyAlert {
    /// 会话ID
    pub session_id: Uuid,
    /// 当前延迟（毫秒）
    pub current_latency_ms: f64,
    /// 阈值（毫秒）
    pub threshold_ms: u32,
    /// 告警时间
    pub timestamp: SystemTime,
    /// 告警消息
    pub message: String,
}

/// 流会话
pub struct StreamSession {
    /// 会话ID
    pub session_id: Uuid,
    /// 数据源
    source: Box<dyn StreamSource>,
    /// 配置
    pub config: StreamConfig,
    /// 统计信息
    pub stats: Arc<tokio::sync::RwLock<StreamStats>>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 分片发送器
    segment_sender: broadcast::Sender<VideoSegment>,
    /// 告警发送器
    alert_sender: broadcast::Sender<LatencyAlert>,
    /// 转发任务句柄
    forward_task: Option<JoinHandle<()>>,
}

impl StreamSession {
    /// 创建新的流会话
    fn new(
        session_id: Uuid,
        source: Box<dyn StreamSource>,
        config: StreamConfig,
    ) -> Self {
        let (segment_sender, _) = broadcast::channel(1000);
        let (alert_sender, _) = broadcast::channel(50);
        
        Self {
            session_id,
            source,
            config,
            stats: Arc::new(tokio::sync::RwLock::new(StreamStats::new())),
            created_at: SystemTime::now(),
            segment_sender,
            alert_sender,
            forward_task: None,
        }
    }

    /// 获取流信息
    pub fn get_info(&self) -> StreamInfo {
        self.source.get_info()
    }

    /// 订阅分片
    pub fn subscribe(&self) -> broadcast::Receiver<VideoSegment> {
        self.segment_sender.subscribe()
    }

    /// 订阅告警
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<LatencyAlert> {
        self.alert_sender.subscribe()
    }

    /// 获取活跃客户端数
    pub fn active_clients(&self) -> usize {
        self.segment_sender.receiver_count()
    }
}

/// 统一流处理器
///
/// 负责管理所有流会话（直通播放和录像回放），实现零缓冲转发和延迟监控。
///
/// # 特性
///
/// - **统一管理**: 使用相同的接口管理直通和回放会话
/// - **零缓冲转发**: 接收到分片后立即转发，处理延迟<5ms
/// - **并发转发**: 同时向多个客户端转发分片
/// - **延迟监控**: 实时监控端到端延迟
/// - **高性能**: 支持100+并发流会话
///
/// # 示例
///
/// ```rust,ignore
/// use std::sync::Arc;
///
/// // 创建统一流处理器
/// let handler = UnifiedStreamHandler::new();
///
/// // 启动直通播放会话
/// let session_id = handler.start_stream(
///     Box::new(live_source),
///     StreamConfig::default(),
/// ).await?;
///
/// // 订阅分片
/// let mut receiver = handler.subscribe(session_id)?;
///
/// // 接收分片
/// while let Ok(segment) = receiver.recv().await {
///     println!("Received segment: {}", segment.segment_id);
/// }
///
/// // 停止会话
/// handler.stop_stream(session_id).await?;
/// ```
#[derive(Clone)]
pub struct UnifiedStreamHandler {
    /// 流会话映射
    sessions: Arc<DashMap<Uuid, Arc<tokio::sync::RwLock<StreamSession>>>>,
}

impl UnifiedStreamHandler {
    /// 创建新的统一流处理器
    pub fn new() -> Self {
        debug!("Creating UnifiedStreamHandler");
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// 启动流会话
    ///
    /// # 参数
    ///
    /// - `source`: 数据源（LiveStreamSource或PlaybackSource）
    /// - `config`: 流配置
    ///
    /// # 返回
    ///
    /// 返回会话ID或错误
    pub async fn start_stream(
        &self,
        source: Box<dyn StreamSource>,
        config: StreamConfig,
    ) -> Result<Uuid, StreamError> {
        let session_id = Uuid::new_v4();
        self.start_stream_with_id(session_id, source, config).await?;
        Ok(session_id)
    }
    
    /// 使用指定的会话ID启动流会话
    ///
    /// 用于需要预先指定session_id的场景（如直通播放）
    ///
    /// # 参数
    ///
    /// - `session_id`: 预先生成的会话ID
    /// - `source`: 流数据源
    /// - `config`: 流配置
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub async fn start_stream_with_id(
        &self,
        session_id: Uuid,
        source: Box<dyn StreamSource>,
        config: StreamConfig,
    ) -> Result<(), StreamError> {
        debug!(
            "Starting stream session with ID: {} with config: {:?}",
            session_id, config
        );

        let mut session = StreamSession::new(session_id, source, config);
        
        // 启动转发任务
        let forward_task = self.start_forwarding_task(session_id, &mut session).await?;
        session.forward_task = Some(forward_task);

        // 存储会话
        self.sessions.insert(
            session_id,
            Arc::new(tokio::sync::RwLock::new(session)),
        );

        debug!("Stream session started: {}", session_id);
        Ok(())
    }

    /// 启动转发任务
    async fn start_forwarding_task(
        &self,
        session_id: Uuid,
        session: &mut StreamSession,
    ) -> Result<JoinHandle<()>, StreamError> {
        let mut source = std::mem::replace(
            &mut session.source,
            Box::new(DummySource::new()),
        );
        
        let segment_sender = session.segment_sender.clone();
        let alert_sender = session.alert_sender.clone();
        let stats = session.stats.clone();
        let sessions = self.sessions.clone();
        let latency_threshold = session.config.latency_alert_threshold_ms;

        let task = tokio::spawn(async move {
            debug!("Forwarding task started for session: {}", session_id);

            loop {
                // 获取下一个分片
                match source.next_segment().await {
                    Ok(Some(mut segment)) => {
                        // 记录接收时间（如果还没有记录）
                        if segment.receive_time.is_none() {
                            segment.receive_time = Some(SystemTime::now());
                        }
                        
                        let forward_start = SystemTime::now();
                        
                        // 零缓冲转发：立即发送到所有客户端
                        let send_result = segment_sender.send(segment.clone());
                        
                        // 记录转发时间
                        segment.forward_time = Some(SystemTime::now());
                        
                        // 计算处理延迟（从接收到转发）
                        let processing_latency_ms = if let Some(receive_time) = segment.receive_time {
                            if let Ok(duration) = forward_start.duration_since(receive_time) {
                                duration.as_micros() as f64 / 1000.0
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };
                        
                        // 计算转发延迟（转发操作本身的耗时）
                        let forward_latency_ms = if let Ok(duration) = forward_start.elapsed() {
                            duration.as_micros() as f64 / 1000.0
                        } else {
                            0.0
                        };
                        
                        // 更新统计信息
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.total_segments += 1;
                            stats_guard.update_latency(processing_latency_ms);
                            stats_guard.update_throughput(segment.data.len() as u64);
                        }
                        
                        // 如果处理延迟超过5ms，记录警告
                        if processing_latency_ms > 5.0 {
                            warn!(
                                "High processing latency: {:.2}ms (forward: {:.2}ms) for session: {}",
                                processing_latency_ms, forward_latency_ms, session_id
                            );
                        }
                        
                        // 如果延迟超过阈值，发送告警
                        if processing_latency_ms > latency_threshold as f64 {
                            let alert = LatencyAlert {
                                session_id,
                                current_latency_ms: processing_latency_ms,
                                threshold_ms: latency_threshold,
                                timestamp: SystemTime::now(),
                                message: format!(
                                    "Latency exceeded threshold: {:.2}ms > {}ms",
                                    processing_latency_ms, latency_threshold
                                ),
                            };
                            
                            // 发送告警（忽略错误，如果没有订阅者）
                            let _ = alert_sender.send(alert.clone());
                            
                            warn!(
                                "Latency alert for session {}: {}",
                                session_id, alert.message
                            );
                        }
                        
                        // 记录详细的延迟分解
                        debug!(
                            "Segment {} latency: processing={:.2}ms, forward={:.2}ms",
                            segment.segment_id, processing_latency_ms, forward_latency_ms
                        );
                        
                        // 检查是否有客户端接收
                        match send_result {
                            Ok(receiver_count) => {
                                debug!(
                                    "Forwarded segment {} to {} clients (session: {})",
                                    segment.segment_id, receiver_count, session_id
                                );
                            }
                            Err(_) => {
                                debug!(
                                    "No active receivers for segment {} (session: {})",
                                    segment.segment_id, session_id
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        // 流结束
                        debug!("Stream ended for session: {}", session_id);
                        break;
                    }
                    Err(e) => {
                        // 发生错误
                        error!("Error reading segment for session {}: {}", session_id, e);
                        
                        // 优雅降级：根据错误类型决定是否继续
                        match e {
                            StreamError::SegmentCorrupted => {
                                // 分片损坏：跳过并继续
                                warn!("Skipping corrupted segment for session: {}", session_id);
                                continue;
                            }
                            StreamError::DeviceOffline | StreamError::ConnectionLost => {
                                // 设备离线或连接丢失：停止流
                                error!("Device offline or connection lost for session: {}", session_id);
                                break;
                            }
                            StreamError::FileReadError(_) => {
                                // 文件读取错误：尝试继续（可能是临时错误）
                                warn!("File read error for session {}, continuing...", session_id);
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                continue;
                            }
                            _ => {
                                // 其他错误：停止流
                                error!("Fatal error for session {}: {}", session_id, e);
                                break;
                            }
                        }
                    }
                }
            }

            // 注意：不在这里清理会话，会话应该由stop_stream显式清理
            debug!("Forwarding task stopped for session: {}", session_id);
        });

        Ok(task)
    }

    /// 停止流会话
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub async fn stop_stream(&self, session_id: Uuid) -> Result<(), StreamError> {
        debug!("Stopping stream session: {}", session_id);

        if let Some((_, session_lock)) = self.sessions.remove(&session_id) {
            let session = session_lock.read().await;
            
            // 取消转发任务
            if let Some(task) = &session.forward_task {
                task.abort();
            }
            
            debug!("Stream session stopped: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 暂停流会话
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    pub async fn pause_stream(&self, session_id: Uuid) -> Result<(), StreamError> {
        debug!("Pausing stream session: {}", session_id);

        if let Some(session_lock) = self.sessions.get(&session_id) {
            let mut session = session_lock.write().await;
            session.source.pause().await?;
            debug!("Stream session paused: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 恢复流会话
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    pub async fn resume_stream(&self, session_id: Uuid) -> Result<(), StreamError> {
        debug!("Resuming stream session: {}", session_id);

        if let Some(session_lock) = self.sessions.get(&session_id) {
            let mut session = session_lock.write().await;
            session.source.resume().await?;
            debug!("Stream session resumed: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 定位到指定位置
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    /// - `position`: 目标位置（秒）
    pub async fn seek_stream(&self, session_id: Uuid, position: f64) -> Result<(), StreamError> {
        debug!("Seeking stream session {} to position: {:.3}s", session_id, position);

        if let Some(session_lock) = self.sessions.get(&session_id) {
            let mut session = session_lock.write().await;
            session.source.seek(position).await?;
            debug!("Stream session seeked: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 设置播放速率
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    /// - `rate`: 播放速率
    pub async fn set_rate(&self, session_id: Uuid, rate: f64) -> Result<(), StreamError> {
        debug!("Setting playback rate for session {} to: {:.2}x", session_id, rate);

        if let Some(session_lock) = self.sessions.get(&session_id) {
            let mut session = session_lock.write().await;
            session.source.set_rate(rate).await?;
            debug!("Playback rate set for session: {}", session_id);
            Ok(())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 订阅会话的分片流
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    ///
    /// # 返回
    ///
    /// 返回分片接收器或错误
    pub async fn subscribe(&self, session_id: Uuid) -> Result<broadcast::Receiver<VideoSegment>, StreamError> {
        if let Some(session_lock) = self.sessions.get(&session_id) {
            let session = session_lock.read().await;
            Ok(session.subscribe())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 订阅会话的延迟告警
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    ///
    /// # 返回
    ///
    /// 返回告警接收器或错误
    pub async fn subscribe_alerts(&self, session_id: Uuid) -> Result<broadcast::Receiver<LatencyAlert>, StreamError> {
        if let Some(session_lock) = self.sessions.get(&session_id) {
            let session = session_lock.read().await;
            Ok(session.subscribe_alerts())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 获取会话信息
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    ///
    /// # 返回
    ///
    /// 返回流信息或错误
    pub async fn get_session_info(&self, session_id: Uuid) -> Result<StreamInfo, StreamError> {
        if let Some(session_lock) = self.sessions.get(&session_id) {
            let session = session_lock.read().await;
            Ok(session.get_info())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 获取会话统计信息
    ///
    /// # 参数
    ///
    /// - `session_id`: 会话ID
    ///
    /// # 返回
    ///
    /// 返回统计信息或错误
    pub async fn get_session_stats(&self, session_id: Uuid) -> Result<StreamStats, StreamError> {
        if let Some(session_lock) = self.sessions.get(&session_id) {
            let session = session_lock.read().await;
            let stats = session.stats.read().await;
            Ok(stats.clone())
        } else {
            warn!("Session not found: {}", session_id);
            Err(StreamError::SessionNotFound)
        }
    }

    /// 列出所有活跃会话
    ///
    /// # 返回
    ///
    /// 返回会话ID列表
    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.iter().map(|entry| *entry.key()).collect()
    }

    /// 获取活跃会话数
    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for UnifiedStreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 虚拟数据源（用于内部实现）
struct DummySource {
    paused: bool,
}

impl DummySource {
    fn new() -> Self {
        Self { paused: false }
    }
}

#[async_trait::async_trait]
impl StreamSource for DummySource {
    async fn next_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
        Ok(None)
    }

    async fn seek(&mut self, _position: f64) -> Result<(), StreamError> {
        Err(StreamError::OperationNotSupported)
    }

    async fn set_rate(&mut self, _rate: f64) -> Result<(), StreamError> {
        Err(StreamError::OperationNotSupported)
    }

    async fn pause(&mut self) -> Result<(), StreamError> {
        self.paused = true;
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), StreamError> {
        self.paused = false;
        Ok(())
    }

    fn get_info(&self) -> StreamInfo {
        use super::source::{StreamMode, StreamState};
        StreamInfo {
            mode: StreamMode::Live {
                device_id: "dummy".to_string(),
            },
            state: if self.paused {
                StreamState::Paused
            } else {
                StreamState::Stopped
            },
            resolution: None,
            frame_rate: None,
            bitrate: None,
            duration: None,
            current_position: 0.0,
            playback_rate: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::source::{SegmentFormat, StreamMode, StreamState};

    /// 测试用数据源
    struct TestSource {
        segments: Vec<VideoSegment>,
        index: usize,
        paused: bool,
    }

    impl TestSource {
        fn new(count: usize) -> Self {
            let segments = (0..count)
                .map(|i| VideoSegment {
                    segment_id: Uuid::new_v4(),
                    timestamp: i as f64 * 0.033,
                    duration: 0.033,
                    data: vec![0u8; 1024],
                    is_keyframe: i % 30 == 0,
                    format: SegmentFormat::H264Raw,
                })
                .collect();

            Self {
                segments,
                index: 0,
                paused: false,
            }
        }
    }

    #[async_trait::async_trait]
    impl StreamSource for TestSource {
        async fn next_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
            if self.paused {
                tokio::time::sleep(Duration::from_millis(100)).await;
                return self.next_segment().await;
            }

            if self.index < self.segments.len() {
                let segment = self.segments[self.index].clone();
                self.index += 1;
                Ok(Some(segment))
            } else {
                Ok(None)
            }
        }

        async fn seek(&mut self, _position: f64) -> Result<(), StreamError> {
            Ok(())
        }

        async fn set_rate(&mut self, _rate: f64) -> Result<(), StreamError> {
            Ok(())
        }

        async fn pause(&mut self) -> Result<(), StreamError> {
            self.paused = true;
            Ok(())
        }

        async fn resume(&mut self) -> Result<(), StreamError> {
            self.paused = false;
            Ok(())
        }

        fn get_info(&self) -> StreamInfo {
            StreamInfo {
                mode: StreamMode::Live {
                    device_id: "test".to_string(),
                },
                state: if self.paused {
                    StreamState::Paused
                } else {
                    StreamState::Streaming
                },
                resolution: Some((1920, 1080)),
                frame_rate: Some(30.0),
                bitrate: Some(5_000_000),
                duration: None,
                current_position: self.index as f64 * 0.033,
                playback_rate: 1.0,
            }
        }
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let handler = UnifiedStreamHandler::new();
        assert_eq!(handler.active_sessions(), 0);
    }

    #[tokio::test]
    async fn test_start_and_stop_stream() {
        let handler = UnifiedStreamHandler::new();
        let source = Box::new(TestSource::new(10));

        // 启动会话
        let session_id = handler
            .start_stream(source, StreamConfig::default())
            .await
            .unwrap();

        assert_eq!(handler.active_sessions(), 1);

        // 停止会话
        handler.stop_stream(session_id).await.unwrap();

        // 等待清理
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_subscribe_and_receive() {
        let handler = UnifiedStreamHandler::new();
        let source = Box::new(TestSource::new(5));

        let session_id = handler
            .start_stream(source, StreamConfig::default())
            .await
            .unwrap();

        let mut receiver = handler.subscribe(session_id).await.unwrap();

        // 接收几个分片
        for i in 0..3 {
            match tokio::time::timeout(Duration::from_secs(1), receiver.recv()).await {
                Ok(Ok(segment)) => {
                    println!("Received segment {}: {}", i, segment.segment_id);
                }
                Ok(Err(e)) => {
                    panic!("Receive error: {}", e);
                }
                Err(_) => {
                    panic!("Timeout waiting for segment");
                }
            }
        }

        handler.stop_stream(session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_pause_and_resume() {
        let handler = UnifiedStreamHandler::new();
        let source = Box::new(TestSource::new(10));

        let session_id = handler
            .start_stream(source, StreamConfig::default())
            .await
            .unwrap();

        // 暂停
        handler.pause_stream(session_id).await.unwrap();

        // 获取信息验证状态
        let info = handler.get_session_info(session_id).await.unwrap();
        assert_eq!(info.state, StreamState::Paused);

        // 恢复
        handler.resume_stream(session_id).await.unwrap();

        handler.stop_stream(session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_sessions() {
        let handler = UnifiedStreamHandler::new();

        // 创建多个会话
        let mut session_ids = Vec::new();
        for _ in 0..5 {
            let source = Box::new(TestSource::new(10));
            let session_id = handler
                .start_stream(source, StreamConfig::default())
                .await
                .unwrap();
            session_ids.push(session_id);
        }

        assert_eq!(handler.active_sessions(), 5);

        // 停止所有会话
        for session_id in session_ids {
            handler.stop_stream(session_id).await.unwrap();
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_get_session_stats() {
        let handler = UnifiedStreamHandler::new();
        let source = Box::new(TestSource::new(100)); // 更多分片以确保会话不会太快结束

        let session_id = handler
            .start_stream(source, StreamConfig::default())
            .await
            .unwrap();

        // 订阅以确保会话保持活跃
        let mut _receiver = handler.subscribe(session_id).await.unwrap();

        // 等待一些分片被处理
        tokio::time::sleep(Duration::from_millis(200)).await;

        let stats = handler.get_session_stats(session_id).await.unwrap();
        assert!(stats.total_segments > 0);

        handler.stop_stream(session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let handler = UnifiedStreamHandler::new();
        let fake_id = Uuid::new_v4();

        assert!(matches!(
            handler.stop_stream(fake_id).await,
            Err(StreamError::SessionNotFound)
        ));

        assert!(matches!(
            handler.pause_stream(fake_id).await,
            Err(StreamError::SessionNotFound)
        ));

        assert!(matches!(
            handler.subscribe(fake_id).await,
            Err(StreamError::SessionNotFound)
        ));
    }
}

impl StreamStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self {
            total_segments: 0,
            total_bytes: 0,
            average_latency_ms: 0.0,
            current_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            throughput_mbps: 0.0,
            packet_loss_rate: 0.0,
            latency_history: VecDeque::with_capacity(1000), // 保留最近1000个样本
            start_time: Some(SystemTime::now()),
        }
    }

    /// 更新延迟统计
    ///
    /// # 参数
    ///
    /// - `latency_ms`: 新的延迟值（毫秒）
    pub fn update_latency(&mut self, latency_ms: f64) {
        // 更新当前延迟
        self.current_latency_ms = latency_ms;

        // 更新最小/最大延迟
        if latency_ms < self.min_latency_ms {
            self.min_latency_ms = latency_ms;
        }
        if latency_ms > self.max_latency_ms {
            self.max_latency_ms = latency_ms;
        }

        // 更新平均延迟（指数移动平均）
        if self.average_latency_ms == 0.0 {
            self.average_latency_ms = latency_ms;
        } else {
            self.average_latency_ms = self.average_latency_ms * 0.9 + latency_ms * 0.1;
        }

        // 添加到历史记录
        self.latency_history.push_back(latency_ms);

        // 限制历史记录大小
        if self.latency_history.len() > 1000 {
            self.latency_history.pop_front();
        }

        // 更新百分位数（每100个样本更新一次）
        if self.total_segments % 100 == 0 {
            self.update_percentiles();
        }
    }

    /// 更新百分位数统计
    fn update_percentiles(&mut self) {
        if self.latency_history.is_empty() {
            return;
        }

        // 复制并排序延迟历史
        let mut sorted: Vec<f64> = self.latency_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = sorted.len();

        // 计算P50（中位数）
        self.p50_latency_ms = if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        };

        // 计算P95
        let p95_index = (len as f64 * 0.95) as usize;
        self.p95_latency_ms = sorted[p95_index.min(len - 1)];

        // 计算P99
        let p99_index = (len as f64 * 0.99) as usize;
        self.p99_latency_ms = sorted[p99_index.min(len - 1)];
    }

    /// 更新吞吐量统计
    ///
    /// # 参数
    ///
    /// - `bytes`: 新增的字节数
    pub fn update_throughput(&mut self, bytes: u64) {
        self.total_bytes += bytes;

        // 计算吞吐量（Mbps）
        if let Some(start_time) = self.start_time {
            if let Ok(duration) = start_time.elapsed() {
                let seconds = duration.as_secs_f64();
                if seconds > 0.0 {
                    let bits = (self.total_bytes * 8) as f64;
                    self.throughput_mbps = bits / seconds / 1_000_000.0;
                }
            }
        }
    }

    /// 获取统计摘要
    pub fn summary(&self) -> String {
        format!(
            "Segments: {}, Bytes: {:.2}MB, Avg Latency: {:.2}ms, \
             P50: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms, \
             Throughput: {:.2}Mbps",
            self.total_segments,
            self.total_bytes as f64 / 1_024_000.0,
            self.average_latency_ms,
            self.p50_latency_ms,
            self.p95_latency_ms,
            self.p99_latency_ms,
            self.throughput_mbps
        )
    }

    /// 重置统计信息
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
