// 统一低延迟视频流传输系统 - LiveStreamSource实现
//
// 本模块实现了直通播放数据源，从QUIC连接接收实时视频分片。
//
// # 特性
//
// - 从QUIC接收器获取实时分片
// - 支持暂停/恢复功能
// - 不支持定位和倍速（直通播放特性）
// - 零缓冲转发，最低延迟

use super::source::{
    SegmentFormat, SegmentSourceType, StreamError, StreamInfo, StreamMode, StreamSource, StreamState,
};
use super::source::VideoSegment as SourceVideoSegment;
use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::{debug, warn};

// 使用common中的VideoSegment（从QUIC接收）
use common::VideoSegment as CommonVideoSegment;

/// 直通播放数据源状态
#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceState {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
}

/// 直通播放数据源
///
/// 从QUIC连接接收实时视频分片，实现极低延迟的直通播放。
///
/// # 特性
///
/// - **实时传输**: 从设备端实时接收视频分片
/// - **零缓冲**: 边接收边转发，无额外缓冲延迟
/// - **暂停/恢复**: 支持暂停和恢复播放
/// - **不支持定位**: 直通播放不支持时间定位
/// - **不支持倍速**: 直通播放不支持变速播放
///
/// # 示例
///
/// ```rust,ignore
/// use tokio::sync::broadcast;
///
/// // 创建QUIC接收器
/// let (tx, rx) = broadcast::channel(1000);
///
/// // 创建直通播放数据源
/// let source = LiveStreamSource::new("device_001".to_string(), rx);
///
/// // 获取视频分片
/// if let Ok(Some(segment)) = source.next_segment().await {
///     println!("Received segment: {}", segment.segment_id);
/// }
/// ```
pub struct LiveStreamSource {
    /// 设备ID
    device_id: String,
    /// QUIC接收器（使用common::VideoSegment）
    quic_receiver: broadcast::Receiver<CommonVideoSegment>,
    /// 当前状态
    state: SourceState,
    /// 当前位置（秒）
    current_position: f64,
    /// 分辨率
    resolution: Option<(u32, u32)>,
    /// 帧率
    frame_rate: Option<f64>,
    /// 码率
    bitrate: Option<u64>,
}

impl LiveStreamSource {
    /// 创建新的直通播放数据源
    ///
    /// # 参数
    ///
    /// - `device_id`: 设备ID
    /// - `quic_receiver`: QUIC视频分片接收器
    ///
    /// # 返回
    ///
    /// 返回新创建的LiveStreamSource实例
    pub fn new(device_id: String, quic_receiver: broadcast::Receiver<CommonVideoSegment>) -> Self {
        debug!("Creating LiveStreamSource for device: {}", device_id);
        Self {
            device_id,
            quic_receiver,
            state: SourceState::Initializing,
            current_position: 0.0,
            resolution: None,
            frame_rate: None,
            bitrate: None,
        }
    }

    /// 设置流信息
    ///
    /// # 参数
    ///
    /// - `resolution`: 分辨率（宽x高）
    /// - `frame_rate`: 帧率
    /// - `bitrate`: 码率（bps）
    pub fn set_stream_info(
        &mut self,
        resolution: Option<(u32, u32)>,
        frame_rate: Option<f64>,
        bitrate: Option<u64>,
    ) {
        self.resolution = resolution;
        self.frame_rate = frame_rate;
        self.bitrate = bitrate;
    }

    /// 获取设备ID
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// 检查是否处于暂停状态
    fn is_paused(&self) -> bool {
        self.state == SourceState::Paused
    }
}

#[async_trait]
impl StreamSource for LiveStreamSource {
    /// 获取下一个视频分片
    ///
    /// 从QUIC接收器获取实时视频分片。如果处于暂停状态，会丢弃接收到的分片。
    ///
    /// # 返回
    ///
    /// - `Ok(Some(segment))`: 成功获取分片
    /// - `Ok(None)`: 流已结束或接收器关闭
    /// - `Err(error)`: 发生错误
    async fn next_segment(&mut self) -> Result<Option<SourceVideoSegment>, StreamError> {
        // 如果是初始化状态，切换到运行状态
        if self.state == SourceState::Initializing {
            self.state = SourceState::Running;
            debug!("LiveStreamSource started for device: {}", self.device_id);
        }

        // 如果已停止，返回None
        if self.state == SourceState::Stopped {
            return Ok(None);
        }

        // 从QUIC接收器获取分片
        match self.quic_receiver.recv().await {
            Ok(common_segment) => {
                // 如果暂停，丢弃分片但继续接收（避免缓冲区溢出）
                if self.is_paused() {
                    debug!(
                        "Dropping segment {} (paused)",
                        common_segment.segment_id
                    );
                    // 递归调用以获取下一个分片
                    return self.next_segment().await;
                }

                // 更新当前位置
                self.current_position = common_segment.timestamp;

                debug!(
                    "Received live segment: {} at {:.3}s",
                    common_segment.segment_id, common_segment.timestamp
                );

                // 转换common::VideoSegment到source::VideoSegment
                let source_segment = SourceVideoSegment {
                    segment_id: common_segment.segment_id,
                    timestamp: common_segment.timestamp,
                    duration: common_segment.duration,
                    data: common_segment.data,
                    is_keyframe: common_segment.flags & 0x01 != 0,
                    format: SegmentFormat::H264Raw,
                    source_type: SegmentSourceType::Live,
                    receive_time: Some(std::time::SystemTime::now()),
                    forward_time: None,
                };

                Ok(Some(source_segment))
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                // 接收器落后，跳过了一些分片
                warn!(
                    "LiveStreamSource lagged, skipped {} segments for device: {}",
                    skipped, self.device_id
                );
                // 继续接收下一个分片
                self.next_segment().await
            }
            Err(broadcast::error::RecvError::Closed) => {
                // 发送器已关闭，流结束
                debug!("QUIC receiver closed for device: {}", self.device_id);
                self.state = SourceState::Stopped;
                Ok(None)
            }
        }
    }

    /// 定位到指定时间位置
    ///
    /// 直通播放不支持定位操作。
    ///
    /// # 返回
    ///
    /// 总是返回 `StreamError::OperationNotSupported`
    async fn seek(&mut self, _position: f64) -> Result<(), StreamError> {
        warn!(
            "Seek operation not supported for live stream (device: {})",
            self.device_id
        );
        Err(StreamError::OperationNotSupported)
    }

    /// 设置播放速率
    ///
    /// 直通播放不支持变速播放。
    ///
    /// # 返回
    ///
    /// 总是返回 `StreamError::OperationNotSupported`
    async fn set_rate(&mut self, _rate: f64) -> Result<(), StreamError> {
        warn!(
            "Set rate operation not supported for live stream (device: {})",
            self.device_id
        );
        Err(StreamError::OperationNotSupported)
    }

    /// 暂停流传输
    ///
    /// 暂停后，接收到的分片会被丢弃，但QUIC连接保持活跃。
    async fn pause(&mut self) -> Result<(), StreamError> {
        if self.state == SourceState::Running {
            self.state = SourceState::Paused;
            debug!("LiveStreamSource paused for device: {}", self.device_id);
            Ok(())
        } else {
            warn!(
                "Cannot pause LiveStreamSource in state: {:?} (device: {})",
                self.state, self.device_id
            );
            Err(StreamError::Internal(format!(
                "Cannot pause in state: {:?}",
                self.state
            )))
        }
    }

    /// 恢复流传输
    ///
    /// 恢复后，继续接收和转发视频分片。
    async fn resume(&mut self) -> Result<(), StreamError> {
        if self.state == SourceState::Paused {
            self.state = SourceState::Running;
            debug!("LiveStreamSource resumed for device: {}", self.device_id);
            Ok(())
        } else {
            warn!(
                "Cannot resume LiveStreamSource in state: {:?} (device: {})",
                self.state, self.device_id
            );
            Err(StreamError::Internal(format!(
                "Cannot resume in state: {:?}",
                self.state
            )))
        }
    }

    /// 获取流信息
    ///
    /// # 返回
    ///
    /// 返回当前流的详细信息
    fn get_info(&self) -> StreamInfo {
        StreamInfo {
            mode: StreamMode::Live {
                device_id: self.device_id.clone(),
            },
            state: match self.state {
                SourceState::Initializing => StreamState::Initializing,
                SourceState::Running => StreamState::Streaming,
                SourceState::Paused => StreamState::Paused,
                SourceState::Stopped => StreamState::Stopped,
            },
            resolution: self.resolution,
            frame_rate: self.frame_rate,
            bitrate: self.bitrate,
            duration: None, // 直通播放无总时长
            current_position: self.current_position,
            playback_rate: 1.0, // 直通播放固定为1.0x
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use uuid::Uuid;

    fn create_test_segment(timestamp: f64) -> VideoSegment {
        VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp,
            duration: 0.033,
            data: vec![0u8; 1024],
            is_keyframe: false,
            format: SegmentFormat::H264Raw,
            source_type: SegmentSourceType::Live,
            receive_time: None,
            forward_time: None,
        }
    }

    #[tokio::test]
    async fn test_live_source_creation() {
        let (_, rx) = broadcast::channel(100);
        let source = LiveStreamSource::new("device_001".to_string(), rx);

        assert_eq!(source.device_id(), "device_001");
        assert_eq!(source.state, SourceState::Initializing);
    }

    #[tokio::test]
    async fn test_live_source_receive_segment() {
        let (tx, rx) = broadcast::channel(100);
        let mut source = LiveStreamSource::new("device_001".to_string(), rx);

        // 发送测试分片
        let test_segment = create_test_segment(1.0);
        tx.send(test_segment.clone()).unwrap();

        // 接收分片
        let result = source.next_segment().await;
        assert!(result.is_ok());
        
        let segment = result.unwrap();
        assert!(segment.is_some());
        
        let received = segment.unwrap();
        assert_eq!(received.timestamp, 1.0);
        assert_eq!(source.state, SourceState::Running);
    }

    #[tokio::test]
    async fn test_live_source_pause_resume() {
        let (tx, rx) = broadcast::channel(100);
        let mut source = LiveStreamSource::new("device_001".to_string(), rx);

        // 启动源
        let test_segment = create_test_segment(1.0);
        tx.send(test_segment).unwrap();
        let _ = source.next_segment().await;

        // 测试暂停
        assert!(source.pause().await.is_ok());
        assert_eq!(source.state, SourceState::Paused);

        // 测试恢复
        assert!(source.resume().await.is_ok());
        assert_eq!(source.state, SourceState::Running);
    }

    #[tokio::test]
    async fn test_live_source_unsupported_operations() {
        let (_, rx) = broadcast::channel(100);
        let mut source = LiveStreamSource::new("device_001".to_string(), rx);

        // 测试不支持的操作
        assert!(matches!(
            source.seek(10.0).await,
            Err(StreamError::OperationNotSupported)
        ));

        assert!(matches!(
            source.set_rate(2.0).await,
            Err(StreamError::OperationNotSupported)
        ));
    }

    #[tokio::test]
    async fn test_live_source_get_info() {
        let (_, rx) = broadcast::channel(100);
        let mut source = LiveStreamSource::new("device_001".to_string(), rx);
        
        source.set_stream_info(Some((1920, 1080)), Some(30.0), Some(5_000_000));

        let info = source.get_info();
        
        match info.mode {
            StreamMode::Live { device_id } => {
                assert_eq!(device_id, "device_001");
            }
            _ => panic!("Expected Live mode"),
        }

        assert_eq!(info.resolution, Some((1920, 1080)));
        assert_eq!(info.frame_rate, Some(30.0));
        assert_eq!(info.bitrate, Some(5_000_000));
        assert_eq!(info.playback_rate, 1.0);
        assert!(info.duration.is_none());
    }

    #[tokio::test]
    async fn test_live_source_paused_drops_segments() {
        let (tx, rx) = broadcast::channel(100);
        let mut source = LiveStreamSource::new("device_001".to_string(), rx);

        // 启动并暂停
        tx.send(create_test_segment(1.0)).unwrap();
        let _ = source.next_segment().await;
        source.pause().await.unwrap();

        // 发送多个分片
        tx.send(create_test_segment(2.0)).unwrap();
        tx.send(create_test_segment(3.0)).unwrap();
        tx.send(create_test_segment(4.0)).unwrap();

        // 恢复并接收
        source.resume().await.unwrap();
        tx.send(create_test_segment(5.0)).unwrap();

        let result = source.next_segment().await;
        assert!(result.is_ok());
        
        // 应该接收到恢复后的分片
        let segment = result.unwrap().unwrap();
        assert_eq!(segment.timestamp, 5.0);
    }
}
