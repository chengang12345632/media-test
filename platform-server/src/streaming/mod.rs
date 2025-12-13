// 统一低延迟视频流传输系统 - Streaming模块
//
// 本模块实现了统一的流处理架构，支持直通播放和录像回放两种模式。
// 
// # 核心组件
//
// - `StreamSource`: 统一的数据源抽象接口
// - `LiveStreamSource`: 直通播放数据源实现
// - `PlaybackSource`: 录像回放数据源实现
// - `UnifiedStreamHandler`: 统一流处理器
// - `FileStreamReader`: 文件流式读取器
//
// # 设计目标
//
// 1. **统一架构**: 直通和回放使用相同的代码路径
// 2. **极低延迟**: 直通<100ms，回放<200ms
// 3. **零缓冲转发**: 平台端处理延迟<5ms
// 4. **代码复用**: 80%以上代码共享

pub mod error;
pub mod file_reader;
pub mod fmp4_converter;
pub mod handler;
pub mod live_source;
pub mod playback_source;
pub mod source;

// 重新导出核心类型
pub use error::{ErrorRecoveryPolicy, RetryStrategy, StreamError};
pub use file_reader::{FileReaderConfig, FileStreamReader};
pub use fmp4_converter::{FMP4Converter, FMP4ConverterConfig};
pub use handler::{BufferConfig, LatencyAlert, StreamConfig, StreamStats, UnifiedStreamHandler};
pub use live_source::LiveStreamSource;
pub use playback_source::PlaybackSource;
pub use source::{
    SegmentFormat, StreamInfo, StreamMode, StreamSource, StreamState, VideoSegment,
};
