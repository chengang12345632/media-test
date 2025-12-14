use std::io;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// File Operation Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum FileError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Unsupported file format: {format}")]
    UnsupportedFormat { format: String },

    #[error("File is corrupted or unreadable")]
    CorruptedFile,

    #[error("Insufficient permissions to access file")]
    PermissionDenied,

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Invalid file metadata")]
    InvalidMetadata,

    #[error("Invalid seek position")]
    InvalidSeekPosition,

    #[error("Seek operation failed")]
    SeekFailed,

    #[error("Seek position beyond end of file")]
    SeekBeyondEnd,

    #[error("No video stream found in file")]
    NoVideoStream,

    #[error("Failed to parse NAL unit")]
    NalParseError,

    #[error("Failed to build keyframe index: {reason}")]
    IndexBuildFailed { reason: String },
}

// ============================================================================
// Timeline File Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum TimelineError {
    #[error("Timeline file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Timeline file corrupted: {reason}")]
    Corrupted { reason: String },

    #[error("Timeline file outdated: video modified after timeline generation")]
    Outdated,

    #[error("Timeline cache full: {current_size} MB used, {limit} MB limit")]
    CacheFull { current_size: u64, limit: u64 },

    #[error("Timeline validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Timeline version incompatible: {version}")]
    IncompatibleVersion { version: u32 },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("File hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("File size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { expected: u64, actual: u64 },
}

// ============================================================================
// Playback Control Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum PlaybackError {
    #[error("Invalid seek position: {position}")]
    InvalidSeekPosition { position: f64 },

    #[error("Invalid playback rate: {rate}")]
    InvalidPlaybackRate { rate: f64 },

    #[error("Seek operation failed: {reason}")]
    SeekFailed { reason: String },

    #[error("Buffer management error: {reason}")]
    BufferError { reason: String },

    #[error("Synchronization lost")]
    SyncLost,

    #[error("Keyframe not found at timestamp: {timestamp}")]
    KeyframeNotFound { timestamp: f64 },

    #[error("Invalid keyframe index: {reason}")]
    InvalidKeyframeIndex { reason: String },

    #[error("Playback controller not initialized")]
    NotInitialized,

    #[error("File error: {0}")]
    FileError(#[from] FileError),
}

// ============================================================================
// FFmpeg Integration Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum FFmpegError {
    #[error("FFmpeg not available on this system")]
    NotAvailable,

    #[error("FFmpeg command failed: {message}")]
    CommandFailed { message: String },

    #[error("Failed to parse FFmpeg output: {reason}")]
    ParseError { reason: String },

    #[error("FFmpeg version incompatible: {version}")]
    IncompatibleVersion { version: String },

    #[error("FFmpeg execution timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Unsupported video format: {format}")]
    UnsupportedFormat { format: String },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("FFmpeg process error: {0}")]
    ProcessError(String),
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl From<TimelineError> for FileError {
    fn from(err: TimelineError) -> Self {
        match err {
            TimelineError::Io(io_err) => FileError::Io(io_err),
            TimelineError::NotFound { path } => FileError::FileNotFound { path },
            other => FileError::IndexBuildFailed {
                reason: other.to_string(),
            },
        }
    }
}

impl From<FFmpegError> for FileError {
    fn from(err: FFmpegError) -> Self {
        match err {
            FFmpegError::Io(io_err) => FileError::Io(io_err),
            FFmpegError::NotAvailable => FileError::IndexBuildFailed {
                reason: "FFmpeg not available".to_string(),
            },
            other => FileError::IndexBuildFailed {
                reason: other.to_string(),
            },
        }
    }
}
