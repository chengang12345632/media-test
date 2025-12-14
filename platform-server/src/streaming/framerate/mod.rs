// H264帧率控制 - 帧率检测和管理模块
//
// 本模块实现了H264视频流的帧率检测、时间戳管理和速率控制功能。
//
// # 核心组件
//
// - `FrameRateDetector`: 帧率检测器，自动检测视频流帧率
// - `TimestampManager`: 时间戳管理器，管理视频帧时间戳
// - `FrameRatePacer`: 帧率控制器，控制视频分片发送速率
//
// # 设计目标
//
// 1. **精确帧率控制**: 播放速度误差<±5%
// 2. **自动帧率检测**: 从SPS或时间戳自动检测帧率
// 3. **低延迟**: 帧率控制增加的延迟<10ms
// 4. **倍速支持**: 支持0.25x-4x倍速播放

pub mod detector;
pub mod pacer;
pub mod timestamp;

// 重新导出核心类型
pub use detector::{DetectionMethod, FrameRateDetector, FrameRateInfo};
pub use pacer::FrameRatePacer;
pub use timestamp::TimestampManager;
