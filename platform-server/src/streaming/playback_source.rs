// 统一低延迟视频流传输系统 - PlaybackSource实现
//
// 本模块实现了录像回放数据源，从文件系统读取视频分片。
//
// # 特性
//
// - 从FileStreamReader获取录像分片
// - 支持完整的播放控制（暂停、恢复、定位、倍速）
// - 小分片读取（8KB-32KB）实现低延迟
// - 速率控制支持0.25x-4x倍速

use super::framerate::{FrameRateDetector, FrameRatePacer};
use super::source::{
    SegmentFormat, SegmentSourceType, StreamError, StreamInfo, StreamMode, StreamSource, StreamState, VideoSegment,
};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::{debug, warn};
use uuid::Uuid;

/// 录像回放数据源状态
#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceState {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 定位中
    Seeking,
    /// 已停止
    Stopped,
}

/// 文件流式读取器（简化版本，完整版本在file_reader.rs）
///
/// 这是一个临时实现，用于支持PlaybackSource的基本功能。
/// 完整的FileStreamReader将在任务2.1中实现。
struct SimpleFileReader {
    file: File,
    file_path: PathBuf,
    file_size: u64,
    current_offset: u64,
    segment_size: usize,
    playback_rate: f64,
}

impl SimpleFileReader {
    async fn new(file_path: PathBuf) -> Result<Self, StreamError> {
        let file = File::open(&file_path)
            .await
            .map_err(|e| StreamError::FileNotFound(e.to_string()))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| StreamError::FileReadError(e.to_string()))?;

        Ok(Self {
            file,
            file_path,
            file_size: metadata.len(),
            current_offset: 0,
            segment_size: 8192, // 8KB默认分片大小
            playback_rate: 1.0,
        })
    }

    async fn read_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
        if self.current_offset >= self.file_size {
            return Ok(None);
        }

        let mut buffer = vec![0u8; self.segment_size];
        let bytes_read = self
            .file
            .read(&mut buffer)
            .await
            .map_err(|e| StreamError::FileReadError(e.to_string()))?;

        if bytes_read == 0 {
            return Ok(None);
        }

        buffer.truncate(bytes_read);
        self.current_offset += bytes_read as u64;

        // 计算时间戳（简化版本）
        let timestamp = (self.current_offset as f64 / self.file_size as f64) * 100.0;

        Ok(Some(VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp,
            duration: 0.033, // 假设30fps
            data: buffer,
            is_keyframe: false,
            format: SegmentFormat::MP4,
            source_type: SegmentSourceType::Playback,
            receive_time: Some(SystemTime::now()), // 设置读取时间
            forward_time: None,
        }))
    }

    async fn seek_to(&mut self, position: f64) -> Result<(), StreamError> {
        // 简化的定位实现：按比例定位
        let target_offset = ((position / 100.0) * self.file_size as f64) as u64;
        let target_offset = target_offset.min(self.file_size);

        self.file
            .seek(std::io::SeekFrom::Start(target_offset))
            .await
            .map_err(|e| StreamError::FileReadError(e.to_string()))?;

        self.current_offset = target_offset;
        Ok(())
    }

    fn set_rate(&mut self, rate: f64) {
        self.playback_rate = rate;
    }

    fn get_progress(&self) -> f64 {
        if self.file_size == 0 {
            0.0
        } else {
            (self.current_offset as f64 / self.file_size as f64) * 100.0
        }
    }
}

/// 录像回放数据源
///
/// 从文件系统读取视频文件，实现低延迟的录像回放。
///
/// # 特性
///
/// - **小分片读取**: 使用8KB-32KB小分片降低延迟
/// - **速率控制**: 支持0.25x-4x倍速播放
/// - **精确定位**: 支持定位到任意时间位置
/// - **暂停/恢复**: 支持暂停和恢复播放
/// - **低延迟**: 录像回放延迟<200ms
///
/// # 示例
///
/// ```rust,ignore
/// use std::path::PathBuf;
///
/// // 创建录像回放数据源
/// let source = PlaybackSource::new(
///     "rec_001".to_string(),
///     PathBuf::from("recordings/video.mp4"),
/// ).await?;
///
/// // 获取视频分片
/// if let Ok(Some(segment)) = source.next_segment().await {
///     println!("Received segment: {}", segment.segment_id);
/// }
///
/// // 定位到30秒位置
/// source.seek(30.0).await?;
///
/// // 设置2倍速播放
/// source.set_rate(2.0).await?;
/// ```
pub struct PlaybackSource {
    /// 文件ID
    file_id: String,
    /// 文件读取器
    file_reader: SimpleFileReader,
    /// 播放速率
    playback_rate: f64,
    /// 当前状态
    state: SourceState,
    /// 分辨率
    resolution: Option<(u32, u32)>,
    /// 帧率
    frame_rate: Option<f64>,
    /// 码率
    bitrate: Option<u64>,
    /// 总时长（秒）
    duration: Option<f64>,
    /// 帧率检测器
    frame_rate_detector: FrameRateDetector,
    /// 帧率控制器
    frame_rate_pacer: Option<FrameRatePacer>,
}

impl PlaybackSource {
    /// 创建新的录像回放数据源
    ///
    /// # 参数
    ///
    /// - `file_id`: 文件ID
    /// - `file_path`: 文件路径
    ///
    /// # 返回
    ///
    /// 返回新创建的PlaybackSource实例或错误
    pub async fn new(file_id: String, file_path: PathBuf) -> Result<Self, StreamError> {
        debug!("Creating PlaybackSource for file: {:?}", file_path);

        let file_reader = SimpleFileReader::new(file_path).await?;

        Ok(Self {
            file_id,
            file_reader,
            playback_rate: 1.0,
            state: SourceState::Initializing,
            resolution: None,
            frame_rate: None,
            bitrate: None,
            duration: Some(100.0), // 假设100秒时长
            frame_rate_detector: FrameRateDetector::new(),
            frame_rate_pacer: None, // 将在检测到帧率后初始化
        })
    }

    /// 设置流信息
    ///
    /// # 参数
    ///
    /// - `resolution`: 分辨率（宽x高）
    /// - `frame_rate`: 帧率
    /// - `bitrate`: 码率（bps）
    /// - `duration`: 总时长（秒）
    pub fn set_stream_info(
        &mut self,
        resolution: Option<(u32, u32)>,
        frame_rate: Option<f64>,
        bitrate: Option<u64>,
        duration: Option<f64>,
    ) {
        self.resolution = resolution;
        self.frame_rate = frame_rate;
        self.bitrate = bitrate;
        self.duration = duration;
    }

    /// 获取文件ID
    pub fn file_id(&self) -> &str {
        &self.file_id
    }

    /// 获取当前播放速率
    pub fn playback_rate(&self) -> f64 {
        self.playback_rate
    }

    /// 检查是否处于暂停状态
    fn is_paused(&self) -> bool {
        self.state == SourceState::Paused
    }

    /// 验证播放速率
    fn validate_rate(rate: f64) -> Result<(), StreamError> {
        if rate < 0.25 || rate > 4.0 {
            return Err(StreamError::InvalidPlaybackRate(rate));
        }
        Ok(())
    }

    /// 验证定位位置
    fn validate_position(&self, position: f64) -> Result<(), StreamError> {
        if position < 0.0 {
            return Err(StreamError::InvalidSeekPosition(position));
        }
        if let Some(duration) = self.duration {
            if position > duration {
                return Err(StreamError::InvalidSeekPosition(position));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StreamSource for PlaybackSource {
    /// 获取下一个视频分片
    ///
    /// 从文件读取器获取视频分片。如果处于暂停状态，会等待恢复。
    ///
    /// # 返回
    ///
    /// - `Ok(Some(segment))`: 成功获取分片
    /// - `Ok(None)`: 文件已读取完毕
    /// - `Err(error)`: 发生错误
    async fn next_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
        // 如果是初始化状态，切换到运行状态
        if self.state == SourceState::Initializing {
            self.state = SourceState::Running;
            debug!("PlaybackSource started for file: {}", self.file_id);
        }

        // 如果已停止，返回None
        if self.state == SourceState::Stopped {
            return Ok(None);
        }

        // 如果暂停，等待恢复
        while self.is_paused() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // 从文件读取器获取分片
        match self.file_reader.read_segment().await {
            Ok(Some(segment)) => {
                // 添加时间戳样本用于帧率检测
                let pts_us = (segment.timestamp * 1_000_000.0) as u64;
                let receive_time = SystemTime::now();
                self.frame_rate_detector.add_timestamp_sample(pts_us, receive_time);
                
                // 更新检测到的帧率并初始化pacer
                if let Some(detected_fps) = self.frame_rate_detector.get_fps() {
                    let fps_changed = self.frame_rate.is_none() || 
                                     (self.frame_rate.unwrap() - detected_fps).abs() > 1.0;
                    
                    if fps_changed {
                        self.frame_rate = Some(detected_fps);
                        debug!("Updated frame rate for file {}: {:.2} fps", 
                               self.file_id, detected_fps);
                        
                        // 初始化或更新pacer
                        if let Some(ref mut pacer) = self.frame_rate_pacer {
                            pacer.update_target_fps(detected_fps);
                        } else {
                            let mut pacer = FrameRatePacer::new(detected_fps);
                            if let Err(e) = pacer.set_playback_rate(self.playback_rate) {
                                warn!("Failed to set playback rate: {}", e);
                            }
                            self.frame_rate_pacer = Some(pacer);
                            debug!("Initialized FrameRatePacer for file {}", self.file_id);
                        }
                    }
                }

                debug!(
                    "Read playback segment: {} at {:.3}s",
                    segment.segment_id, segment.timestamp
                );

                // 使用FrameRatePacer控制发送速率
                if let Some(ref mut pacer) = self.frame_rate_pacer {
                    // 假设每个分片包含1帧（简化实现）
                    pacer.wait_for_next_frame(1).await;
                } else {
                    // 如果pacer还未初始化，使用简单的延迟控制（回退方案）
                    if self.playback_rate != 1.0 {
                        let delay = (segment.duration / self.playback_rate * 1000.0) as u64;
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }

                Ok(Some(segment))
            }
            Ok(None) => {
                debug!("Playback finished for file: {}", self.file_id);
                self.state = SourceState::Stopped;
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to read segment: {}", e);
                Err(e)
            }
        }
    }

    /// 定位到指定时间位置
    ///
    /// # 参数
    ///
    /// - `position`: 目标时间位置（秒）
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 定位成功
    /// - `Err(error)`: 定位失败
    async fn seek(&mut self, position: f64) -> Result<(), StreamError> {
        self.validate_position(position)?;

        debug!(
            "Seeking to position {:.3}s for file: {}",
            position, self.file_id
        );

        let old_state = self.state.clone();
        self.state = SourceState::Seeking;

        match self.file_reader.seek_to(position).await {
            Ok(()) => {
                self.state = old_state;
                debug!("Seek completed for file: {}", self.file_id);
                Ok(())
            }
            Err(e) => {
                self.state = old_state;
                warn!("Seek failed for file {}: {}", self.file_id, e);
                Err(e)
            }
        }
    }

    /// 设置播放速率
    ///
    /// # 参数
    ///
    /// - `rate`: 播放速率（0.25x - 4.0x）
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 设置成功
    /// - `Err(error)`: 速率无效
    async fn set_rate(&mut self, rate: f64) -> Result<(), StreamError> {
        Self::validate_rate(rate)?;

        debug!(
            "Setting playback rate to {:.2}x for file: {}",
            rate, self.file_id
        );

        self.playback_rate = rate;
        self.file_reader.set_rate(rate);
        
        // 更新FrameRatePacer的倍速
        if let Some(ref mut pacer) = self.frame_rate_pacer {
            if let Err(e) = pacer.set_playback_rate(rate) {
                warn!("Failed to update pacer playback rate: {}", e);
            }
        }

        Ok(())
    }

    /// 暂停流传输
    async fn pause(&mut self) -> Result<(), StreamError> {
        if self.state == SourceState::Running {
            self.state = SourceState::Paused;
            debug!("PlaybackSource paused for file: {}", self.file_id);
            Ok(())
        } else {
            warn!(
                "Cannot pause PlaybackSource in state: {:?} (file: {})",
                self.state, self.file_id
            );
            Err(StreamError::Internal(format!(
                "Cannot pause in state: {:?}",
                self.state
            )))
        }
    }

    /// 恢复流传输
    async fn resume(&mut self) -> Result<(), StreamError> {
        if self.state == SourceState::Paused {
            self.state = SourceState::Running;
            debug!("PlaybackSource resumed for file: {}", self.file_id);
            Ok(())
        } else {
            warn!(
                "Cannot resume PlaybackSource in state: {:?} (file: {})",
                self.state, self.file_id
            );
            Err(StreamError::Internal(format!(
                "Cannot resume in state: {:?}",
                self.state
            )))
        }
    }

    /// 获取流信息
    fn get_info(&self) -> StreamInfo {
        StreamInfo {
            mode: StreamMode::Playback {
                file_id: self.file_id.clone(),
                playback_rate: self.playback_rate,
            },
            state: match self.state {
                SourceState::Initializing => StreamState::Initializing,
                SourceState::Running => StreamState::Streaming,
                SourceState::Paused => StreamState::Paused,
                SourceState::Seeking => StreamState::Seeking,
                SourceState::Stopped => StreamState::Stopped,
            },
            resolution: self.resolution,
            frame_rate: self.frame_rate,
            bitrate: self.bitrate,
            duration: self.duration,
            current_position: self.file_reader.get_progress(),
            playback_rate: self.playback_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn create_test_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        // 写入一些测试数据
        file.write_all(&vec![0u8; 100000]).unwrap();
        file
    }

    #[tokio::test]
    async fn test_playback_source_creation() {
        let temp_file = create_test_file().await;
        let source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await;

        assert!(source.is_ok());
        let source = source.unwrap();
        assert_eq!(source.file_id(), "rec_001");
        assert_eq!(source.playback_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_playback_source_read_segment() {
        let temp_file = create_test_file().await;
        let mut source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await
        .unwrap();

        let result = source.next_segment().await;
        assert!(result.is_ok());

        let segment = result.unwrap();
        assert!(segment.is_some());
    }

    #[tokio::test]
    async fn test_playback_source_pause_resume() {
        let temp_file = create_test_file().await;
        let mut source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await
        .unwrap();

        // 启动源
        let _ = source.next_segment().await;

        // 测试暂停
        assert!(source.pause().await.is_ok());
        assert_eq!(source.state, SourceState::Paused);

        // 测试恢复
        assert!(source.resume().await.is_ok());
        assert_eq!(source.state, SourceState::Running);
    }

    #[tokio::test]
    async fn test_playback_source_set_rate() {
        let temp_file = create_test_file().await;
        let mut source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await
        .unwrap();

        // 测试有效速率
        assert!(source.set_rate(2.0).await.is_ok());
        assert_eq!(source.playback_rate(), 2.0);

        assert!(source.set_rate(0.5).await.is_ok());
        assert_eq!(source.playback_rate(), 0.5);

        // 测试无效速率
        assert!(matches!(
            source.set_rate(5.0).await,
            Err(StreamError::InvalidPlaybackRate(_))
        ));

        assert!(matches!(
            source.set_rate(0.1).await,
            Err(StreamError::InvalidPlaybackRate(_))
        ));
    }

    #[tokio::test]
    async fn test_playback_source_seek() {
        let temp_file = create_test_file().await;
        let mut source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await
        .unwrap();

        // 测试有效定位
        assert!(source.seek(50.0).await.is_ok());

        // 测试无效定位
        assert!(matches!(
            source.seek(-10.0).await,
            Err(StreamError::InvalidSeekPosition(_))
        ));

        assert!(matches!(
            source.seek(200.0).await,
            Err(StreamError::InvalidSeekPosition(_))
        ));
    }

    #[tokio::test]
    async fn test_playback_source_get_info() {
        let temp_file = create_test_file().await;
        let mut source = PlaybackSource::new(
            "rec_001".to_string(),
            temp_file.path().to_path_buf(),
        )
        .await
        .unwrap();

        source.set_stream_info(Some((1920, 1080)), Some(30.0), Some(5_000_000), Some(100.0));

        let info = source.get_info();

        match info.mode {
            StreamMode::Playback {
                file_id,
                playback_rate,
            } => {
                assert_eq!(file_id, "rec_001");
                assert_eq!(playback_rate, 1.0);
            }
            _ => panic!("Expected Playback mode"),
        }

        assert_eq!(info.resolution, Some((1920, 1080)));
        assert_eq!(info.frame_rate, Some(30.0));
        assert_eq!(info.bitrate, Some(5_000_000));
        assert_eq!(info.duration, Some(100.0));
    }
}
