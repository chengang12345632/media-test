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

pub use reader::{scan_video_files, VideoFile, VideoFileReader, VideoFormat};
