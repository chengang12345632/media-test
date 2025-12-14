use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

// ============================================================================
// Core Video File Information
// ============================================================================

/// Video file information
#[derive(Debug, Clone)]
pub struct VideoFileInfo {
    pub duration: f64,
    pub resolution: Resolution,
    pub codec: String,
    pub frame_rate: f64,
    pub bit_rate: u64,
    pub has_audio: bool,
}

/// Video resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// Keyframe Index Structures
// ============================================================================

/// Keyframe index for precise seek operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeIndex {
    pub entries: Vec<KeyframeEntry>,
    pub total_duration: f64,
    pub index_precision: f64,
    pub memory_optimized: bool,
    pub optimization_strategy: IndexOptimizationStrategy,
    pub memory_usage: usize,
}

/// Individual keyframe entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeEntry {
    pub timestamp: f64,
    pub file_offset: u64,
    pub frame_size: u32,
    pub gop_size: u32,
    pub frame_type: FrameType,
}

/// Frame type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameType {
    I, // Intra frame (keyframe)
    P, // Predicted frame
    B, // Bidirectional frame
}

/// Index optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexOptimizationStrategy {
    Full,         // Complete index with all keyframes
    Sparse,       // Sparse index with periodic sampling
    Adaptive,     // Adaptive strategy based on memory
    Hierarchical, // Hierarchical index with multiple precision levels
}

// ============================================================================
// Seek Operation Structures
// ============================================================================

/// Result of a seek operation
#[derive(Debug, Clone)]
pub struct SeekResult {
    pub requested_time: f64,
    pub actual_time: f64,
    pub keyframe_offset: u64,
    pub precision_achieved: f64,
    pub keyframe_used: KeyframeEntry,
    pub execution_time: Duration,
}

// ============================================================================
// Video Segment Structures
// ============================================================================

/// Video segment for transmission
#[derive(Debug, Clone)]
pub struct VideoSegment {
    pub id: Uuid,
    pub data: Vec<u8>,
    pub timestamp: f64,
    pub duration: f64,
    pub frame_count: usize,
    pub is_key_frame: bool,
}

// ============================================================================
// Playback Control Structures
// ============================================================================

/// Frame dropping strategy for playback control
#[derive(Debug, Clone, PartialEq)]
pub struct DropFrameStrategy {
    pub drop_b_frames: bool,
    pub drop_p_frames: bool,
    pub keep_key_frames_only: bool,
    pub adaptive_dropping: bool,
}

/// Buffer manager for playback
#[derive(Debug)]
pub struct BufferManager {
    pub video_buffers: HashMap<Uuid, Vec<u8>>,
    pub audio_buffers: HashMap<Uuid, Vec<u8>>,
    pub max_buffer_size: usize,
    pub current_buffer_size: usize,
    pub buffer_health: BufferHealth,
}

/// Buffer health status
#[derive(Debug, Clone)]
pub struct BufferHealth {
    pub video_buffer_level: f64,
    pub audio_buffer_level: f64,
    pub underrun_count: u32,
    pub overrun_count: u32,
    pub last_underrun: Option<SystemTime>,
}

// ============================================================================
// Timeline File Structures
// ============================================================================

/// Timeline file format for caching keyframe information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineFile {
    pub version: u32,
    pub video_file_path: PathBuf,
    pub video_file_hash: String,
    pub video_file_size: u64,
    #[serde(with = "systemtime_serde")]
    pub video_file_modified: SystemTime,
    pub duration: f64,
    pub resolution: Resolution,
    pub frame_rate: f64,
    pub keyframe_index: KeyframeIndex,
    #[serde(with = "systemtime_serde")]
    pub created_at: SystemTime,
    pub ffmpeg_version: Option<String>,
}

// ============================================================================
// Audio and Synchronization Structures
// ============================================================================

/// Audio segment for transmission
#[derive(Debug, Clone)]
pub struct AudioSegment {
    pub id: Uuid,
    pub data: Vec<u8>,
    pub timestamp: f64,
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Synchronization information between audio and video
#[derive(Debug, Clone)]
pub struct SyncInfo {
    pub video_timestamp: f64,
    pub audio_timestamp: f64,
    pub offset: f64,
}

// ============================================================================
// Network Condition Structures
// ============================================================================

/// Network conditions for adaptive playback
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    pub bandwidth_estimate: u64,
    pub rtt: Duration,
    pub packet_loss_rate: f64,
    pub jitter: Duration,
    pub congestion_level: CongestionLevel,
}

/// Network congestion level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// FFmpeg Integration Structures
// ============================================================================

/// FFmpeg video information
#[derive(Debug, Clone)]
pub struct FFmpegVideoInfo {
    pub duration: f64,
    pub resolution: Resolution,
    pub codec: String,
    pub frame_rate: f64,
    pub bit_rate: u64,
    pub has_audio: bool,
    pub keyframe_timestamps: Vec<f64>,
}

/// FFmpeg configuration
#[derive(Debug, Clone)]
pub struct FFmpegConfig {
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub timeout: Duration,
    pub min_version: String,
}

// ============================================================================
// Serde helpers for SystemTime
// ============================================================================

mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}
