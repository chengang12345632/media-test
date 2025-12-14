use crate::types::*;
use serde::{Deserialize, Serialize};

/// 会话开始请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartRequest {
    pub device_id: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub capabilities: DeviceCapabilities,
}

/// 文件列表请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequest {
    pub filter: Option<String>,
}

/// 文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<RecordingInfo>,
}

/// 文件请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    pub file_path: String,
    pub priority: u8,
    pub seek_position: Option<f64>,
    pub playback_rate: f64,
}

/// 定位请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekRequest {
    pub position: f64,
    pub accurate: bool,
}

/// 定位结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekResult {
    pub requested_time: f64,
    pub actual_time: f64,
    pub keyframe_offset: u64,
    pub precision_achieved: f64,
    pub execution_time_ms: u64,
}

/// 速率变更请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateChangeRequest {
    pub rate: f64,
}

/// 状态码
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u16)]
pub enum StatusCode {
    Success = 200,
    BadRequest = 400,
    Unauthorized = 401,
    NotFound = 404,
    InternalError = 500,
    ServiceUnavailable = 503,
}

/// 状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub code: StatusCode,
    pub message: String,
    pub data: Option<Vec<u8>>,
}

/// 启动直通播放请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartLiveStreamRequest {
    pub quality_preference: String,  // "low_latency" | "high_quality"
    pub target_latency_ms: u32,
    pub target_fps: u32,
    pub target_bitrate: usize,
}

/// 停止直通播放请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLiveStreamRequest {
    pub reason: Option<String>,
}

/// 精确定位到关键帧请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekToKeyframeRequest {
    pub target_time: f64,
    pub session_id: uuid::Uuid,
}

/// 精确定位到关键帧响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekToKeyframeResponse {
    pub requested_time: f64,
    pub actual_time: f64,
    pub keyframe_offset: u64,
    pub precision_achieved: f64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// 设置播放速率请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPlaybackSpeedRequest {
    pub speed: f32,
    pub session_id: uuid::Uuid,
}

/// 设置播放速率响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPlaybackSpeedResponse {
    pub speed: f32,
    pub success: bool,
    pub error_message: Option<String>,
}

/// 获取关键帧索引请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetKeyframeIndexRequest {
    pub file_path: String,
}

/// 关键帧条目（简化版，用于传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeEntry {
    pub timestamp: f64,
    pub file_offset: u64,
    pub frame_size: u32,
}

/// 获取关键帧索引响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetKeyframeIndexResponse {
    pub file_path: String,
    pub keyframes: Vec<KeyframeEntry>,
    pub total_duration: f64,
    pub success: bool,
    pub error_message: Option<String>,
}
