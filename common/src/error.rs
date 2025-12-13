use thiserror::Error;

#[derive(Error, Debug)]
pub enum VideoStreamError {
    #[error("QUIC connection error: {0}")]
    QuicError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    BincodeError(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Recording not found: {0}")]
    RecordingNotFound(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

pub type Result<T> = std::result::Result<T, VideoStreamError>;
