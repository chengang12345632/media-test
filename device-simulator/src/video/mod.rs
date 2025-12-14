mod reader;

// 使用文件版本（从真实H.264文件读取）
mod live_stream_generator_file;
pub use live_stream_generator_file::LiveStreamGeneratorFile;

// 使用模拟版本进行快速测试（无需FFmpeg依赖）
mod live_stream_generator_mock;
pub use live_stream_generator_mock::LiveStreamGenerator;

// 真实实现（需要FFmpeg依赖）
// mod screen_capture;
// mod h264_encoder;
// mod live_stream_generator;
// pub use screen_capture::ScreenCapturer;
// pub use h264_encoder::H264Encoder;
// pub use live_stream_generator::LiveStreamGenerator;

// New modules for device-uploader merge
pub mod types;
pub mod errors;
pub mod file_reader;
pub mod timeline;
pub mod ffmpeg_parser;
pub mod controller;

#[cfg(test)]
mod types_test;

pub use reader::{scan_video_files, VideoFile, VideoFileReader, VideoFormat};

// Re-export commonly used types
pub use types::{
    AudioSegment, BufferHealth, BufferManager, CongestionLevel, DropFrameStrategy, FFmpegConfig,
    FFmpegVideoInfo, FrameType, IndexOptimizationStrategy, KeyframeEntry, KeyframeIndex,
    NetworkConditions, Resolution, SeekResult, SyncInfo, TimelineFile, VideoFileInfo,
    VideoSegment,
};

// Re-export error types
pub use errors::{FFmpegError, FileError, PlaybackError, TimelineError};

// Re-export file reader types
pub use file_reader::{FileStreamReader, DefaultFileStreamReader};

// Re-export timeline types
pub use timeline::{TimelineManager, DefaultTimelineManager, TimelineFileBuilder};

// Re-export FFmpeg parser types
pub use ffmpeg_parser::{DefaultFFmpegParser, FFmpegParser};

// Re-export controller types
pub use controller::{DefaultPlaybackController, PlaybackController};
