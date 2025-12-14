// 统一低延迟视频流传输系统 - StreamSource Trait定义
// 
// 本模块定义了统一的数据源抽象接口，用于支持直通播放和录像回放两种模式。
// 通过trait抽象，实现了代码复用和一致的流处理逻辑。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// 视频分片格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentFormat {
    /// H.264裸流
    H264Raw,
    /// 分片MP4格式
    FMP4,
    /// 标准MP4格式
    MP4,
}

/// 分片来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentSourceType {
    /// 直通播放（来自设备）
    Live,
    /// 录像回放（来自文件）
    Playback,
}

/// 视频分片数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSegment {
    /// 分片唯一标识符
    pub segment_id: Uuid,
    /// 相对时间戳（秒）
    pub timestamp: f64,
    /// 分片时长（秒）
    pub duration: f64,
    /// 视频数据
    pub data: Vec<u8>,
    /// 是否为关键帧
    pub is_keyframe: bool,
    /// 分片格式
    pub format: SegmentFormat,
    /// 分片来源类型
    pub source_type: SegmentSourceType,
    /// 接收时间（用于延迟计算）
    #[serde(skip)]
    pub receive_time: Option<SystemTime>,
    /// 转发时间（用于延迟计算）
    #[serde(skip)]
    pub forward_time: Option<SystemTime>,
}

/// 流模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamMode {
    /// 直通播放模式
    Live {
        /// 设备ID
        device_id: String,
    },
    /// 录像回放模式
    Playback {
        /// 文件ID
        file_id: String,
        /// 播放速率
        playback_rate: f64,
    },
}

/// 流状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    /// 初始化中
    Initializing,
    /// 流传输中
    Streaming,
    /// 已暂停
    Paused,
    /// 定位中
    Seeking,
    /// 已停止
    Stopped,
    /// 错误状态
    Error(String),
}

/// 流信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// 流模式
    pub mode: StreamMode,
    /// 当前状态
    pub state: StreamState,
    /// 分辨率（宽x高）
    pub resolution: Option<(u32, u32)>,
    /// 帧率
    pub frame_rate: Option<f64>,
    /// 码率（bps）
    pub bitrate: Option<u64>,
    /// 总时长（秒，仅回放模式）
    pub duration: Option<f64>,
    /// 当前位置（秒）
    pub current_position: f64,
    /// 播放速率
    pub playback_rate: f64,
}

/// 流错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum StreamError {
    /// 连接错误
    #[error("Device not connected")]
    DeviceNotConnected,
    
    #[error("Device offline")]
    DeviceOffline,
    
    #[error("Connection lost")]
    ConnectionLost,
    
    /// 文件错误
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("File not accessible: {0}")]
    FileNotAccessible(String),
    
    #[error("File read error: {0}")]
    FileReadError(String),
    
    /// 传输错误
    #[error("Transmission timeout")]
    TransmissionTimeout,
    
    #[error("Segment corrupted")]
    SegmentCorrupted,
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// 播放错误
    #[error("Operation not supported")]
    OperationNotSupported,
    
    #[error("Invalid seek position: {0}")]
    InvalidSeekPosition(f64),
    
    #[error("Invalid playback rate: {0}")]
    InvalidPlaybackRate(f64),
    
    /// 资源错误
    #[error("Session not found")]
    SessionNotFound,
    
    #[error("Too many sessions")]
    TooManySessions,
    
    #[error("Out of memory")]
    OutOfMemory,
    
    /// 其他错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 统一的流数据源抽象接口
/// 
/// 该trait定义了直通播放和录像回放的统一接口，实现了：
/// - 统一的分片获取机制
/// - 统一的播放控制接口
/// - 统一的状态管理
/// 
/// # 实现
/// 
/// - `LiveStreamSource`: 从QUIC连接接收实时分片
/// - `PlaybackSource`: 从文件系统读取录像分片
#[async_trait]
pub trait StreamSource: Send + Sync {
    /// 获取下一个视频分片
    /// 
    /// # 返回
    /// 
    /// - `Ok(Some(segment))`: 成功获取分片
    /// - `Ok(None)`: 流已结束
    /// - `Err(error)`: 发生错误
    async fn next_segment(&mut self) -> Result<Option<VideoSegment>, StreamError>;
    
    /// 定位到指定时间位置
    /// 
    /// # 参数
    /// 
    /// - `position`: 目标时间位置（秒）
    /// 
    /// # 注意
    /// 
    /// 直通播放模式不支持此操作，会返回 `StreamError::OperationNotSupported`
    async fn seek(&mut self, position: f64) -> Result<(), StreamError>;
    
    /// 设置播放速率
    /// 
    /// # 参数
    /// 
    /// - `rate`: 播放速率（0.25x - 4.0x）
    /// 
    /// # 注意
    /// 
    /// 直通播放模式不支持此操作，会返回 `StreamError::OperationNotSupported`
    async fn set_rate(&mut self, rate: f64) -> Result<(), StreamError>;
    
    /// 暂停流传输
    async fn pause(&mut self) -> Result<(), StreamError>;
    
    /// 恢复流传输
    async fn resume(&mut self) -> Result<(), StreamError>;
    
    /// 获取流信息
    /// 
    /// # 返回
    /// 
    /// 返回当前流的详细信息，包括模式、状态、分辨率、帧率等
    fn get_info(&self) -> StreamInfo;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_segment_creation() {
        let segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.5,
            duration: 0.033,
            data: vec![0u8; 1024],
            is_keyframe: true,
            format: SegmentFormat::FMP4,
            source_type: SegmentSourceType::Live,
            receive_time: Some(SystemTime::now()),
            forward_time: None,
        };
        
        assert_eq!(segment.timestamp, 1.5);
        assert_eq!(segment.duration, 0.033);
        assert_eq!(segment.is_keyframe, true);
        assert_eq!(segment.format, SegmentFormat::FMP4);
        assert!(segment.receive_time.is_some());
    }

    #[test]
    fn test_stream_mode() {
        let live_mode = StreamMode::Live {
            device_id: "device_001".to_string(),
        };
        
        let playback_mode = StreamMode::Playback {
            file_id: "rec_001".to_string(),
            playback_rate: 1.0,
        };
        
        match live_mode {
            StreamMode::Live { device_id } => {
                assert_eq!(device_id, "device_001");
            }
            _ => panic!("Expected Live mode"),
        }
        
        match playback_mode {
            StreamMode::Playback { file_id, playback_rate } => {
                assert_eq!(file_id, "rec_001");
                assert_eq!(playback_rate, 1.0);
            }
            _ => panic!("Expected Playback mode"),
        }
    }

    #[test]
    fn test_stream_state_transitions() {
        let states = vec![
            StreamState::Initializing,
            StreamState::Streaming,
            StreamState::Paused,
            StreamState::Seeking,
            StreamState::Stopped,
        ];
        
        for state in states {
            match state {
                StreamState::Initializing => assert!(true),
                StreamState::Streaming => assert!(true),
                StreamState::Paused => assert!(true),
                StreamState::Seeking => assert!(true),
                StreamState::Stopped => assert!(true),
                StreamState::Error(_) => panic!("Unexpected error state"),
            }
        }
    }

    #[test]
    fn test_stream_error_types() {
        let errors = vec![
            StreamError::DeviceNotConnected,
            StreamError::OperationNotSupported,
            StreamError::InvalidSeekPosition(10.0),
            StreamError::InvalidPlaybackRate(5.0),
        ];
        
        for error in errors {
            let error_msg = error.to_string();
            assert!(!error_msg.is_empty());
        }
    }
}
