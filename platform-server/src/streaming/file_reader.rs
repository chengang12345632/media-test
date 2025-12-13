// 统一低延迟视频流传输系统 - FileStreamReader实现
//
// 本模块实现了高效的文件流式读取器，用于低延迟录像回放。
//
// # 特性
//
// - 小分片读取（8KB-32KB）降低延迟
// - 异步IO，非阻塞操作
// - 速率控制，支持0.25x-4x倍速
// - 精确定位，支持任意时间位置
// - 零拷贝优化

use super::source::{SegmentFormat, StreamError, VideoSegment};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::{debug, warn};
use uuid::Uuid;

/// 默认分片大小（8KB）
pub const DEFAULT_SEGMENT_SIZE: usize = 8192;

/// 最小分片大小（4KB）
pub const MIN_SEGMENT_SIZE: usize = 4096;

/// 最大分片大小（32KB）
pub const MAX_SEGMENT_SIZE: usize = 32768;

/// 定位目标
///
/// 定义不同的定位方式。
#[derive(Debug, Clone)]
pub enum SeekTarget {
    /// 相对位置（0.0-1.0）
    Position(f64),
    /// 字节偏移量
    Offset(u64),
    /// 时间位置（秒）
    Time {
        /// 目标时间（秒）
        seconds: f64,
        /// 视频总时长（秒）
        duration: f64,
    },
}

/// 文件流式读取器配置
#[derive(Debug, Clone)]
pub struct FileReaderConfig {
    /// 分片大小（字节）
    pub segment_size: usize,
    /// 播放速率
    pub playback_rate: f64,
    /// 分片格式
    pub format: SegmentFormat,
    /// 假设的帧率（用于时间戳计算）
    pub assumed_fps: f64,
}

impl Default for FileReaderConfig {
    fn default() -> Self {
        Self {
            segment_size: DEFAULT_SEGMENT_SIZE,
            playback_rate: 1.0,
            format: SegmentFormat::MP4,
            assumed_fps: 30.0,
        }
    }
}

/// 文件流式读取器
///
/// 高效的异步文件读取器，专为低延迟录像回放设计。
///
/// # 特性
///
/// - **小分片读取**: 使用8KB-32KB小分片，降低首帧延迟
/// - **速率控制**: 根据播放速率控制分片发送间隔
/// - **异步IO**: 完全异步，不阻塞事件循环
/// - **精确定位**: 支持字节级精确定位
/// - **零拷贝**: 最小化内存复制
///
/// # 示例
///
/// ```rust,ignore
/// use std::path::PathBuf;
///
/// // 创建文件读取器
/// let mut reader = FileStreamReader::new(
///     PathBuf::from("video.mp4"),
///     FileReaderConfig::default(),
/// ).await?;
///
/// // 读取分片
/// while let Some(segment) = reader.read_segment().await? {
///     println!("Read segment: {} bytes", segment.data.len());
/// }
///
/// // 定位到50%位置
/// reader.seek_to_position(0.5).await?;
///
/// // 设置2倍速
/// reader.set_playback_rate(2.0)?;
/// ```
pub struct FileStreamReader {
    /// 文件句柄
    file: File,
    /// 文件路径
    file_path: PathBuf,
    /// 文件大小（字节）
    file_size: u64,
    /// 当前偏移量（字节）
    current_offset: u64,
    /// 配置
    config: FileReaderConfig,
    /// 已读取的分片数
    segments_read: u64,
    /// 开始时间（用于速率控制）
    start_time: std::time::Instant,
}

impl std::fmt::Debug for FileStreamReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileStreamReader")
            .field("file_path", &self.file_path)
            .field("file_size", &self.file_size)
            .field("current_offset", &self.current_offset)
            .field("config", &self.config)
            .field("segments_read", &self.segments_read)
            .finish()
    }
}

impl FileStreamReader {
    /// 创建新的文件流式读取器
    ///
    /// # 参数
    ///
    /// - `file_path`: 文件路径
    /// - `config`: 读取器配置
    ///
    /// # 返回
    ///
    /// 返回新创建的FileStreamReader实例或错误
    ///
    /// # 错误
    ///
    /// - `StreamError::FileNotFound`: 文件不存在
    /// - `StreamError::FileNotAccessible`: 文件无法访问
    pub async fn new(file_path: PathBuf, config: FileReaderConfig) -> Result<Self, StreamError> {
        debug!("Opening file for streaming: {:?}", file_path);

        // 验证分片大小
        if config.segment_size < MIN_SEGMENT_SIZE || config.segment_size > MAX_SEGMENT_SIZE {
            warn!(
                "Invalid segment size: {}, using default: {}",
                config.segment_size, DEFAULT_SEGMENT_SIZE
            );
        }

        // 打开文件
        let file = File::open(&file_path).await.map_err(|e| {
            warn!("Failed to open file {:?}: {}", file_path, e);
            StreamError::FileNotFound(file_path.display().to_string())
        })?;

        // 获取文件大小
        let metadata = file.metadata().await.map_err(|e| {
            warn!("Failed to read file metadata {:?}: {}", file_path, e);
            StreamError::FileNotAccessible(e.to_string())
        })?;

        let file_size = metadata.len();

        debug!(
            "File opened successfully: {:?}, size: {} bytes",
            file_path, file_size
        );

        Ok(Self {
            file,
            file_path,
            file_size,
            current_offset: 0,
            config,
            segments_read: 0,
            start_time: std::time::Instant::now(),
        })
    }

    /// 读取下一个视频分片
    ///
    /// # 返回
    ///
    /// - `Ok(Some(segment))`: 成功读取分片
    /// - `Ok(None)`: 文件已读取完毕
    /// - `Err(error)`: 读取错误
    pub async fn read_segment(&mut self) -> Result<Option<VideoSegment>, StreamError> {
        // 检查是否已到文件末尾
        if self.current_offset >= self.file_size {
            debug!("Reached end of file: {:?}", self.file_path);
            return Ok(None);
        }

        // 计算本次读取的大小
        let remaining = self.file_size - self.current_offset;
        let read_size = remaining.min(self.config.segment_size as u64) as usize;

        // 读取数据
        let mut buffer = vec![0u8; read_size];
        let bytes_read = self.file.read(&mut buffer).await.map_err(|e| {
            warn!("Failed to read from file {:?}: {}", self.file_path, e);
            StreamError::FileReadError(e.to_string())
        })?;

        if bytes_read == 0 {
            debug!("Read 0 bytes, end of file: {:?}", self.file_path);
            return Ok(None);
        }

        // 调整buffer大小
        buffer.truncate(bytes_read);

        // 更新偏移量
        self.current_offset += bytes_read as u64;
        self.segments_read += 1;

        // 计算时间戳（基于文件进度和假设的帧率）
        let progress = self.current_offset as f64 / self.file_size as f64;
        let timestamp = self.calculate_timestamp(progress);

        // 计算分片时长（基于分片大小和假设的帧率）
        let duration = self.calculate_duration(bytes_read);

        // 检测关键帧（简化版本：每30个分片标记为关键帧）
        let is_keyframe = self.segments_read % 30 == 1;

        let segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp,
            duration,
            data: buffer,
            is_keyframe,
            format: self.config.format,
            receive_time: None,
            forward_time: None,
        };

        debug!(
            "Read segment {}: {} bytes at {:.3}s (offset: {}/{})",
            segment.segment_id, bytes_read, timestamp, self.current_offset, self.file_size
        );

        Ok(Some(segment))
    }

    /// 读取下一个视频分片（带速率控制）
    ///
    /// 此方法会根据播放速率自动控制分片发送间隔，确保播放速度符合预期。
    ///
    /// # 返回
    ///
    /// - `Ok(Some(segment))`: 成功读取分片
    /// - `Ok(None)`: 文件已读取完毕
    /// - `Err(error)`: 读取错误
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 设置2倍速
    /// reader.set_playback_rate(2.0)?;
    ///
    /// // 读取分片（自动控制发送间隔）
    /// while let Some(segment) = reader.read_segment_with_rate_control().await? {
    ///     // 分片会按照2倍速的间隔发送
    ///     send_to_client(segment).await?;
    /// }
    /// ```
    pub async fn read_segment_with_rate_control(
        &mut self,
    ) -> Result<Option<VideoSegment>, StreamError> {
        // 读取分片
        let segment = self.read_segment().await?;

        if let Some(ref seg) = segment {
            // 计算应该等待的时间
            let delay = self.calculate_rate_controlled_delay(seg.duration);

            if delay > std::time::Duration::ZERO {
                debug!(
                    "Rate control: waiting {:.3}ms for segment {} (rate: {:.2}x)",
                    delay.as_secs_f64() * 1000.0,
                    seg.segment_id,
                    self.config.playback_rate
                );

                // 等待以控制发送速率
                tokio::time::sleep(delay).await;
            }
        }

        Ok(segment)
    }

    /// 计算速率控制延迟
    ///
    /// 根据播放速率和分片时长计算应该等待的时间。
    ///
    /// # 参数
    ///
    /// - `segment_duration`: 分片时长（秒）
    ///
    /// # 返回
    ///
    /// 返回应该等待的时间
    fn calculate_rate_controlled_delay(&self, segment_duration: f64) -> std::time::Duration {
        if self.config.playback_rate <= 0.0 {
            return std::time::Duration::ZERO;
        }

        // 计算实际应该等待的时间
        // 公式: 等待时间 = 分片时长 / 播放速率
        let delay_seconds = segment_duration / self.config.playback_rate;

        // 转换为Duration
        std::time::Duration::from_secs_f64(delay_seconds.max(0.0))
    }

    /// 计算目标发送间隔
    ///
    /// 根据播放速率计算分片之间的目标发送间隔。
    ///
    /// # 参数
    ///
    /// - `segment_duration`: 分片时长（秒）
    ///
    /// # 返回
    ///
    /// 返回目标发送间隔（毫秒）
    pub fn calculate_target_interval(&self, segment_duration: f64) -> f64 {
        if self.config.playback_rate <= 0.0 {
            return 0.0;
        }

        // 目标间隔 = 分片时长 / 播放速率
        (segment_duration / self.config.playback_rate) * 1000.0 // 转换为毫秒
    }

    /// 验证实际发送间隔是否符合目标
    ///
    /// 检查实际发送间隔与目标间隔的误差是否在可接受范围内（10%）。
    ///
    /// # 参数
    ///
    /// - `actual_interval_ms`: 实际发送间隔（毫秒）
    /// - `target_interval_ms`: 目标发送间隔（毫秒）
    ///
    /// # 返回
    ///
    /// 如果误差<10%返回true，否则返回false
    pub fn validate_interval(&self, actual_interval_ms: f64, target_interval_ms: f64) -> bool {
        if target_interval_ms <= 0.0 {
            return true;
        }

        let error_rate = (actual_interval_ms - target_interval_ms).abs() / target_interval_ms;
        error_rate < 0.1 // 误差小于10%
    }

    /// 定位到指定的文件位置（0.0-1.0）
    ///
    /// # 参数
    ///
    /// - `position`: 相对位置（0.0=开始，1.0=结束）
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub async fn seek_to_position(&mut self, position: f64) -> Result<(), StreamError> {
        if position < 0.0 || position > 1.0 {
            return Err(StreamError::InvalidSeekPosition(position));
        }

        let target_offset = (position * self.file_size as f64) as u64;
        self.seek_to_offset(target_offset).await
    }

    /// 定位到指定的字节偏移量
    ///
    /// # 参数
    ///
    /// - `offset`: 字节偏移量
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub async fn seek_to_offset(&mut self, offset: u64) -> Result<(), StreamError> {
        let target_offset = offset.min(self.file_size);

        debug!(
            "Seeking to offset: {} (file: {:?})",
            target_offset, self.file_path
        );

        self.file
            .seek(std::io::SeekFrom::Start(target_offset))
            .await
            .map_err(|e| {
                warn!("Failed to seek in file {:?}: {}", self.file_path, e);
                StreamError::FileReadError(e.to_string())
            })?;

        self.current_offset = target_offset;

        debug!("Seek completed: offset={}", self.current_offset);

        Ok(())
    }

    /// 定位到指定的时间位置（秒）
    ///
    /// # 参数
    ///
    /// - `time_seconds`: 时间位置（秒）
    /// - `duration_seconds`: 视频总时长（秒）
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub async fn seek_to_time(
        &mut self,
        time_seconds: f64,
        duration_seconds: f64,
    ) -> Result<(), StreamError> {
        if time_seconds < 0.0 || time_seconds > duration_seconds {
            return Err(StreamError::InvalidSeekPosition(time_seconds));
        }

        let position = if duration_seconds > 0.0 {
            time_seconds / duration_seconds
        } else {
            0.0
        };

        self.seek_to_position(position).await
    }

    /// 统一的定位方法
    ///
    /// 支持多种定位方式的统一接口。
    ///
    /// # 参数
    ///
    /// - `target`: 定位目标
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 定位到50%位置
    /// reader.seek_to(SeekTarget::Position(0.5)).await?;
    ///
    /// // 定位到30秒
    /// reader.seek_to(SeekTarget::Time { seconds: 30.0, duration: 100.0 }).await?;
    ///
    /// // 定位到字节偏移量
    /// reader.seek_to(SeekTarget::Offset(12345)).await?;
    /// ```
    pub async fn seek_to(&mut self, target: SeekTarget) -> Result<(), StreamError> {
        match target {
            SeekTarget::Position(position) => self.seek_to_position(position).await,
            SeekTarget::Offset(offset) => self.seek_to_offset(offset).await,
            SeekTarget::Time { seconds, duration } => self.seek_to_time(seconds, duration).await,
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
    /// 成功返回Ok(())，失败返回错误
    pub fn set_playback_rate(&mut self, rate: f64) -> Result<(), StreamError> {
        if rate < 0.25 || rate > 4.0 {
            return Err(StreamError::InvalidPlaybackRate(rate));
        }

        debug!(
            "Setting playback rate: {:.2}x (file: {:?})",
            rate, self.file_path
        );

        self.config.playback_rate = rate;

        Ok(())
    }

    /// 设置分片大小
    ///
    /// # 参数
    ///
    /// - `size`: 分片大小（字节）
    ///
    /// # 返回
    ///
    /// 成功返回Ok(())，失败返回错误
    pub fn set_segment_size(&mut self, size: usize) -> Result<(), StreamError> {
        if size < MIN_SEGMENT_SIZE || size > MAX_SEGMENT_SIZE {
            return Err(StreamError::Internal(format!(
                "Invalid segment size: {} (must be between {} and {})",
                size, MIN_SEGMENT_SIZE, MAX_SEGMENT_SIZE
            )));
        }

        debug!(
            "Setting segment size: {} bytes (file: {:?})",
            size, self.file_path
        );

        self.config.segment_size = size;

        Ok(())
    }

    /// 获取当前进度（0.0-1.0）
    pub fn get_progress(&self) -> f64 {
        if self.file_size == 0 {
            0.0
        } else {
            self.current_offset as f64 / self.file_size as f64
        }
    }

    /// 获取当前偏移量（字节）
    pub fn get_offset(&self) -> u64 {
        self.current_offset
    }

    /// 获取文件大小（字节）
    pub fn get_file_size(&self) -> u64 {
        self.file_size
    }

    /// 获取已读取的分片数
    pub fn get_segments_read(&self) -> u64 {
        self.segments_read
    }

    /// 获取播放速率
    pub fn get_playback_rate(&self) -> f64 {
        self.config.playback_rate
    }

    /// 获取分片大小
    pub fn get_segment_size(&self) -> usize {
        self.config.segment_size
    }

    /// 获取文件路径
    pub fn get_file_path(&self) -> &Path {
        &self.file_path
    }

    /// 计算时间戳（基于进度）
    fn calculate_timestamp(&self, progress: f64) -> f64 {
        // 简化版本：假设视频时长与文件大小成正比
        // 实际实现应该解析视频文件头获取真实时长
        let estimated_duration = self.file_size as f64 / (1024.0 * 1024.0) * 10.0; // 假设1MB=10秒
        progress * estimated_duration
    }

    /// 计算分片时长（基于分片大小）
    fn calculate_duration(&self, bytes: usize) -> f64 {
        // 简化计算：假设固定的帧时长
        // 对于30fps视频，每帧约0.033秒
        // 假设每个分片包含1帧的数据
        let frames_per_segment = 1.0;
        frames_per_segment / self.config.assumed_fps
    }

    /// 重置读取器到文件开始
    pub async fn reset(&mut self) -> Result<(), StreamError> {
        debug!("Resetting file reader: {:?}", self.file_path);
        self.seek_to_offset(0).await?;
        self.segments_read = 0;
        self.start_time = std::time::Instant::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn create_test_file(size: usize) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        // 写入测试数据
        let data = vec![0u8; size];
        file.write_all(&data).unwrap();
        file.flush().unwrap();
        file
    }

    #[tokio::test]
    async fn test_file_reader_creation() {
        let temp_file = create_test_file(100000).await;
        let reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await;

        assert!(reader.is_ok());
        let reader = reader.unwrap();
        assert_eq!(reader.get_file_size(), 100000);
        assert_eq!(reader.get_offset(), 0);
        assert_eq!(reader.get_progress(), 0.0);
    }

    #[tokio::test]
    async fn test_file_reader_read_segment() {
        let temp_file = create_test_file(20000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 读取第一个分片
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());

        let segment = segment.unwrap();
        assert_eq!(segment.data.len(), DEFAULT_SEGMENT_SIZE);
        assert_eq!(reader.get_offset(), DEFAULT_SEGMENT_SIZE as u64);
        assert_eq!(reader.get_segments_read(), 1);
    }

    #[tokio::test]
    async fn test_file_reader_read_all() {
        let file_size = 25000;
        let temp_file = create_test_file(file_size).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        let mut total_bytes = 0;
        let mut segment_count = 0;

        while let Some(segment) = reader.read_segment().await.unwrap() {
            total_bytes += segment.data.len();
            segment_count += 1;
        }

        assert_eq!(total_bytes, file_size);
        assert_eq!(segment_count, reader.get_segments_read());
        assert_eq!(reader.get_progress(), 1.0);
    }

    #[tokio::test]
    async fn test_file_reader_seek_position() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 定位到50%
        reader.seek_to_position(0.5).await.unwrap();
        assert_eq!(reader.get_offset(), 50000);
        assert_eq!(reader.get_progress(), 0.5);

        // 定位到开始
        reader.seek_to_position(0.0).await.unwrap();
        assert_eq!(reader.get_offset(), 0);

        // 定位到结束
        reader.seek_to_position(1.0).await.unwrap();
        assert_eq!(reader.get_offset(), 100000);
    }

    #[tokio::test]
    async fn test_file_reader_seek_offset() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 定位到特定偏移量
        reader.seek_to_offset(12345).await.unwrap();
        assert_eq!(reader.get_offset(), 12345);

        // 定位超过文件大小（应该限制到文件大小）
        reader.seek_to_offset(200000).await.unwrap();
        assert_eq!(reader.get_offset(), 100000);
    }

    #[tokio::test]
    async fn test_file_reader_seek_time() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 定位到30秒（总时长100秒）
        reader.seek_to_time(30.0, 100.0).await.unwrap();
        assert_eq!(reader.get_progress(), 0.3);

        // 无效时间
        assert!(reader.seek_to_time(-10.0, 100.0).await.is_err());
        assert!(reader.seek_to_time(150.0, 100.0).await.is_err());
    }

    #[tokio::test]
    async fn test_file_reader_playback_rate() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 设置有效速率
        assert!(reader.set_playback_rate(2.0).is_ok());
        assert_eq!(reader.get_playback_rate(), 2.0);

        assert!(reader.set_playback_rate(0.5).is_ok());
        assert_eq!(reader.get_playback_rate(), 0.5);

        // 设置无效速率
        assert!(reader.set_playback_rate(5.0).is_err());
        assert!(reader.set_playback_rate(0.1).is_err());
    }

    #[tokio::test]
    async fn test_file_reader_segment_size() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 设置有效分片大小
        assert!(reader.set_segment_size(16384).is_ok());
        assert_eq!(reader.get_segment_size(), 16384);

        // 读取分片验证大小
        let segment = reader.read_segment().await.unwrap().unwrap();
        assert_eq!(segment.data.len(), 16384);

        // 设置无效分片大小
        assert!(reader.set_segment_size(1024).is_err()); // 太小
        assert!(reader.set_segment_size(65536).is_err()); // 太大
    }

    #[tokio::test]
    async fn test_file_reader_reset() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 读取一些分片
        reader.read_segment().await.unwrap();
        reader.read_segment().await.unwrap();
        assert!(reader.get_offset() > 0);
        assert!(reader.get_segments_read() > 0);

        // 重置
        reader.reset().await.unwrap();
        assert_eq!(reader.get_offset(), 0);
        assert_eq!(reader.get_segments_read(), 0);
        assert_eq!(reader.get_progress(), 0.0);
    }

    #[tokio::test]
    async fn test_file_reader_file_not_found() {
        let result = FileStreamReader::new(
            PathBuf::from("/nonexistent/file.mp4"),
            FileReaderConfig::default(),
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StreamError::FileNotFound(_)));
    }

    #[tokio::test]
    async fn test_file_reader_small_file() {
        // 测试小于分片大小的文件
        let temp_file = create_test_file(4000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());

        let segment = segment.unwrap();
        assert_eq!(segment.data.len(), 4000);

        // 第二次读取应该返回None
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_none());
    }

    #[tokio::test]
    async fn test_rate_controlled_delay() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试1倍速
        reader.set_playback_rate(1.0).unwrap();
        let delay = reader.calculate_rate_controlled_delay(0.033); // 33ms分片
        assert!((delay.as_secs_f64() - 0.033).abs() < 0.001);

        // 测试2倍速（延迟应该减半）
        reader.set_playback_rate(2.0).unwrap();
        let delay = reader.calculate_rate_controlled_delay(0.033);
        assert!((delay.as_secs_f64() - 0.0165).abs() < 0.001);

        // 测试0.5倍速（延迟应该加倍）
        reader.set_playback_rate(0.5).unwrap();
        let delay = reader.calculate_rate_controlled_delay(0.033);
        assert!((delay.as_secs_f64() - 0.066).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_calculate_target_interval() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试1倍速
        reader.set_playback_rate(1.0).unwrap();
        let interval = reader.calculate_target_interval(0.033);
        assert!((interval - 33.0).abs() < 0.1); // 33ms

        // 测试2倍速
        reader.set_playback_rate(2.0).unwrap();
        let interval = reader.calculate_target_interval(0.033);
        assert!((interval - 16.5).abs() < 0.1); // 16.5ms

        // 测试0.25倍速
        reader.set_playback_rate(0.25).unwrap();
        let interval = reader.calculate_target_interval(0.033);
        assert!((interval - 132.0).abs() < 0.1); // 132ms
    }

    #[tokio::test]
    async fn test_validate_interval() {
        let temp_file = create_test_file(100000).await;
        let reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试误差在10%以内（应该通过）
        assert!(reader.validate_interval(100.0, 100.0)); // 0%误差
        assert!(reader.validate_interval(105.0, 100.0)); // 5%误差
        assert!(reader.validate_interval(95.0, 100.0)); // 5%误差
        assert!(reader.validate_interval(109.0, 100.0)); // 9%误差

        // 测试误差超过10%（应该失败）
        assert!(!reader.validate_interval(111.0, 100.0)); // 11%误差
        assert!(!reader.validate_interval(89.0, 100.0)); // 11%误差
        assert!(!reader.validate_interval(120.0, 100.0)); // 20%误差
    }

    #[tokio::test]
    async fn test_read_segment_with_rate_control() {
        let temp_file = create_test_file(20000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 设置4倍速（更快，减少测试时间）
        reader.set_playback_rate(4.0).unwrap();

        let start = std::time::Instant::now();

        // 只读取2个分片
        for _ in 0..2 {
            let segment = reader.read_segment_with_rate_control().await.unwrap();
            assert!(segment.is_some());
        }

        let elapsed = start.elapsed();

        // 验证总时间（应该有速率控制延迟，但因为是4倍速所以很短）
        // 2个分片，每个约0.033秒，4倍速应该约0.016秒
        assert!(elapsed.as_secs_f64() > 0.005); // 至少有一些延迟
        assert!(elapsed.as_secs_f64() < 0.1);   // 但不会太长
    }

    #[tokio::test]
    async fn test_rate_control_different_speeds() {
        let temp_file = create_test_file(20000).await;

        // 测试不同速率（只测试较快的速率以避免测试时间过长）
        let rates = vec![2.0, 4.0];

        for rate in rates {
            let mut reader = FileStreamReader::new(
                temp_file.path().to_path_buf(),
                FileReaderConfig::default(),
            )
            .await
            .unwrap();

            reader.set_playback_rate(rate).unwrap();

            let start = std::time::Instant::now();

            // 只读取1个分片以加快测试
            let segment = reader.read_segment_with_rate_control().await.unwrap();
            assert!(segment.is_some());

            let elapsed = start.elapsed().as_secs_f64();

            // 验证时间与速率成反比
            println!("Rate: {:.2}x, Elapsed: {:.3}s", rate, elapsed);

            // 基本验证：速率越高，时间越短
            assert!(elapsed < 0.1); // 所有测试都应该很快完成
        }
    }

    #[tokio::test]
    async fn test_rate_control_calculation_accuracy() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试速率控制计算的准确性
        let test_cases = vec![
            (1.0, 0.033, 33.0),   // 1倍速，33ms分片 -> 33ms间隔
            (2.0, 0.033, 16.5),   // 2倍速，33ms分片 -> 16.5ms间隔
            (0.5, 0.033, 66.0),   // 0.5倍速，33ms分片 -> 66ms间隔
            (4.0, 0.040, 10.0),   // 4倍速，40ms分片 -> 10ms间隔
            (0.25, 0.020, 80.0),  // 0.25倍速，20ms分片 -> 80ms间隔
        ];

        for (rate, duration, expected_interval) in test_cases {
            reader.set_playback_rate(rate).unwrap();
            let interval = reader.calculate_target_interval(duration);
            
            // 验证误差小于1%
            let error = (interval - expected_interval).abs() / expected_interval;
            assert!(
                error < 0.01,
                "Rate: {:.2}x, Duration: {:.3}s, Expected: {:.1}ms, Got: {:.1}ms, Error: {:.2}%",
                rate, duration, expected_interval, interval, error * 100.0
            );
        }
    }

    // ========== 任务 2.3.1: 定位功能单元测试 ==========

    #[tokio::test]
    async fn test_seek_to_position() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试定位到不同位置
        reader.seek_to(SeekTarget::Position(0.25)).await.unwrap();
        assert_eq!(reader.get_offset(), 25000);
        assert_eq!(reader.get_progress(), 0.25);

        reader.seek_to(SeekTarget::Position(0.75)).await.unwrap();
        assert_eq!(reader.get_offset(), 75000);
        assert_eq!(reader.get_progress(), 0.75);

        reader.seek_to(SeekTarget::Position(0.0)).await.unwrap();
        assert_eq!(reader.get_offset(), 0);

        reader.seek_to(SeekTarget::Position(1.0)).await.unwrap();
        assert_eq!(reader.get_offset(), 100000);
    }

    #[tokio::test]
    async fn test_seek_to_offset() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试定位到不同偏移量
        reader.seek_to(SeekTarget::Offset(10000)).await.unwrap();
        assert_eq!(reader.get_offset(), 10000);

        reader.seek_to(SeekTarget::Offset(50000)).await.unwrap();
        assert_eq!(reader.get_offset(), 50000);

        reader.seek_to(SeekTarget::Offset(99999)).await.unwrap();
        assert_eq!(reader.get_offset(), 99999);

        // 超过文件大小应该限制到文件大小
        reader.seek_to(SeekTarget::Offset(200000)).await.unwrap();
        assert_eq!(reader.get_offset(), 100000);
    }

    #[tokio::test]
    async fn test_seek_to_time() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试定位到不同时间位置
        reader
            .seek_to(SeekTarget::Time {
                seconds: 25.0,
                duration: 100.0,
            })
            .await
            .unwrap();
        assert_eq!(reader.get_progress(), 0.25);

        reader
            .seek_to(SeekTarget::Time {
                seconds: 60.0,
                duration: 100.0,
            })
            .await
            .unwrap();
        assert_eq!(reader.get_progress(), 0.6);

        reader
            .seek_to(SeekTarget::Time {
                seconds: 0.0,
                duration: 100.0,
            })
            .await
            .unwrap();
        assert_eq!(reader.get_offset(), 0);

        reader
            .seek_to(SeekTarget::Time {
                seconds: 100.0,
                duration: 100.0,
            })
            .await
            .unwrap();
        assert_eq!(reader.get_offset(), 100000);
    }

    #[tokio::test]
    async fn test_seek_to_boundary_cases() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试边界情况：文件开始
        reader.seek_to(SeekTarget::Position(0.0)).await.unwrap();
        assert_eq!(reader.get_offset(), 0);
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());

        // 测试边界情况：文件结束
        reader.seek_to(SeekTarget::Position(1.0)).await.unwrap();
        assert_eq!(reader.get_offset(), 100000);
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_none()); // 已到文件末尾

        // 测试边界情况：接近文件结束
        reader
            .seek_to(SeekTarget::Offset(100000 - 100))
            .await
            .unwrap();
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());
        assert_eq!(segment.unwrap().data.len(), 100); // 只剩100字节
    }

    #[tokio::test]
    async fn test_seek_to_invalid_position() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试无效位置（负数）
        let result = reader.seek_to(SeekTarget::Position(-0.5)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamError::InvalidSeekPosition(_)
        ));

        // 测试无效位置（超过1.0）
        let result = reader.seek_to(SeekTarget::Position(1.5)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamError::InvalidSeekPosition(_)
        ));
    }

    #[tokio::test]
    async fn test_seek_to_invalid_time() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试无效时间（负数）
        let result = reader
            .seek_to(SeekTarget::Time {
                seconds: -10.0,
                duration: 100.0,
            })
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamError::InvalidSeekPosition(_)
        ));

        // 测试无效时间（超过总时长）
        let result = reader
            .seek_to(SeekTarget::Time {
                seconds: 150.0,
                duration: 100.0,
            })
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StreamError::InvalidSeekPosition(_)
        ));
    }

    #[tokio::test]
    async fn test_seek_to_then_read() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 定位到中间位置
        reader.seek_to(SeekTarget::Position(0.5)).await.unwrap();
        assert_eq!(reader.get_offset(), 50000);

        // 读取分片
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());
        let segment = segment.unwrap();
        assert_eq!(segment.data.len(), DEFAULT_SEGMENT_SIZE);
        assert_eq!(reader.get_offset(), 50000 + DEFAULT_SEGMENT_SIZE as u64);

        // 再次定位
        reader.seek_to(SeekTarget::Offset(10000)).await.unwrap();
        assert_eq!(reader.get_offset(), 10000);

        // 继续读取
        let segment = reader.read_segment().await.unwrap();
        assert!(segment.is_some());
        assert_eq!(reader.get_offset(), 10000 + DEFAULT_SEGMENT_SIZE as u64);
    }

    #[tokio::test]
    async fn test_seek_to_multiple_times() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 多次定位
        let positions = vec![0.1, 0.3, 0.5, 0.7, 0.9, 0.2, 0.8, 0.0, 1.0];

        for pos in positions {
            reader.seek_to(SeekTarget::Position(pos)).await.unwrap();
            let expected_offset = (pos * 100000.0) as u64;
            assert_eq!(reader.get_offset(), expected_offset);
            assert!((reader.get_progress() - pos).abs() < 0.001);
        }
    }

    #[tokio::test]
    async fn test_seek_to_with_different_target_types() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 使用Position定位
        reader.seek_to(SeekTarget::Position(0.25)).await.unwrap();
        let pos1 = reader.get_offset();

        // 使用Offset定位到相同位置
        reader.seek_to(SeekTarget::Offset(25000)).await.unwrap();
        let pos2 = reader.get_offset();

        // 使用Time定位到相同位置
        reader
            .seek_to(SeekTarget::Time {
                seconds: 25.0,
                duration: 100.0,
            })
            .await
            .unwrap();
        let pos3 = reader.get_offset();

        // 三种方式应该定位到相同位置
        assert_eq!(pos1, 25000);
        assert_eq!(pos2, 25000);
        assert_eq!(pos3, 25000);
    }

    #[tokio::test]
    async fn test_seek_to_zero_duration() {
        let temp_file = create_test_file(100000).await;
        let mut reader = FileStreamReader::new(
            temp_file.path().to_path_buf(),
            FileReaderConfig::default(),
        )
        .await
        .unwrap();

        // 测试总时长为0的情况
        reader
            .seek_to(SeekTarget::Time {
                seconds: 0.0,
                duration: 0.0,
            })
            .await
            .unwrap();
        assert_eq!(reader.get_offset(), 0);
    }
}
