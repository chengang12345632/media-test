use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// 视频分片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSegment {
    pub stream_type: u8,
    pub segment_id: Uuid,
    pub session_id: Uuid,  // 播放会话ID，用于分发到正确的订阅者
    pub timestamp: f64,
    pub duration: f64,
    pub frame_count: u32,
    pub flags: u8,
    pub data_length: u32,
    pub data: Vec<u8>,
}

impl VideoSegment {
    pub fn new(data: Vec<u8>, timestamp: f64, is_keyframe: bool) -> Self {
        Self {
            stream_type: 0x01, // 视频
            segment_id: Uuid::new_v4(),
            session_id: Uuid::nil(),  // 默认为空，需要在发送前设置
            timestamp,
            duration: 0.033, // 约30fps
            frame_count: 1,
            flags: if is_keyframe { SegmentFlags::IS_KEYFRAME } else { 0 },
            data_length: data.len() as u32,
            data,
        }
    }

    pub fn is_keyframe(&self) -> bool {
        self.flags & SegmentFlags::IS_KEYFRAME != 0
    }
}

/// 分片标志位
pub mod SegmentFlags {
    pub const IS_KEYFRAME: u8 = 0b0000_0001;
    pub const HAS_AUDIO: u8 = 0b0000_0010;
    pub const IS_LAST_SEGMENT: u8 = 0b0000_0100;
    pub const REQUIRES_ACK: u8 = 0b0000_1000;
    pub const HIGH_PRIORITY: u8 = 0b0001_0000;
}

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub sequence_number: u64,
    pub timestamp: SystemTime,
    pub session_id: Uuid,
}

/// 消息类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    SessionStart = 0x01,
    SessionEnd = 0x02,
    SeekRequest = 0x03,
    RateChange = 0x04,
    PauseRequest = 0x05,
    ResumeRequest = 0x06,
    ErrorReport = 0x07,
    StatsRequest = 0x08,
    StatusResponse = 0x09,
    Heartbeat = 0x0A,
    FileRequest = 0x0B,
    PlaybackControl = 0x0C,
    FileListQuery = 0x0D,
    FileListResponse = 0x0E,
    StartLiveStream = 0x10,  // 启动直通播放
    StopLiveStream = 0x11,   // 停止直通播放
    // 新增：高级播放控制命令
    SeekToKeyframe = 0x12,   // 精确定位到关键帧
    SetPlaybackSpeed = 0x13, // 设置播放速率
    GetKeyframeIndex = 0x14, // 获取关键帧索引
    SeekResponse = 0x15,     // Seek 操作响应
    KeyframeIndexResponse = 0x16, // 关键帧索引响应
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub connection_status: ConnectionStatus,
    pub connection_time: SystemTime,
    pub last_heartbeat: SystemTime,
    pub capabilities: DeviceCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Camera,
    Recorder,
    Simulator,
    Gateway,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Online,
    Offline,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub max_resolution: String,
    pub supported_formats: Vec<String>,
    pub max_bitrate: u64,
    pub supports_playback_control: bool,
    pub supports_recording: bool,
}

/// 录像信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub file_id: String,
    pub device_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub duration: f64,
    pub format: String,
    pub resolution: String,
    pub bitrate: u64,
    pub frame_rate: f64,
    pub created_time: SystemTime,
    pub modified_time: SystemTime,
}

/// 播放控制命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackControl {
    pub command: PlaybackCommand,
    pub position: Option<f64>,
    pub rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaybackCommand {
    Play,
    Pause,
    Resume,
    Seek,
    SetRate,
    Stop,
}

/// 网络统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub latency_ms: u64,
    pub packet_loss_rate: f64,
    pub bandwidth_mbps: f64,
    pub jitter_ms: u64,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub network_usage_mbps: f64,
    pub temperature: Option<f64>,
}
