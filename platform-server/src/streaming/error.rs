// 统一低延迟视频流传输系统 - 错误类型定义
//
// 本模块定义了流处理过程中可能出现的所有错误类型，
// 并提供了错误转换和恢复策略。

use std::fmt;
use std::io;
use std::time::Duration;
use thiserror::Error;

/// 流错误类型
#[derive(Debug, Clone, Error)]
pub enum StreamError {
    // ========== 连接错误 ==========
    /// 设备未连接
    #[error("Device not connected")]
    DeviceNotConnected,
    
    /// 设备离线
    #[error("Device offline")]
    DeviceOffline,
    
    /// 连接丢失
    #[error("Connection lost")]
    ConnectionLost,
    
    // ========== 文件错误 ==========
    /// 文件未找到
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    /// 文件不可访问
    #[error("File not accessible: {0}")]
    FileNotAccessible(String),
    
    /// 文件读取错误
    #[error("File read error: {0}")]
    FileReadError(String),
    
    // ========== 传输错误 ==========
    /// 传输超时
    #[error("Transmission timeout")]
    TransmissionTimeout,
    
    /// 分片损坏
    #[error("Segment corrupted")]
    SegmentCorrupted,
    
    /// 网络错误
    #[error("Network error: {0}")]
    NetworkError(String),
    
    // ========== 播放错误 ==========
    /// 操作不支持
    #[error("Operation not supported")]
    OperationNotSupported,
    
    /// 无效的定位位置
    #[error("Invalid seek position: {0}")]
    InvalidSeekPosition(f64),
    
    /// 无效的播放速率
    #[error("Invalid playback rate: {0}")]
    InvalidPlaybackRate(f64),
    
    // ========== 资源错误 ==========
    /// 会话未找到
    #[error("Session not found")]
    SessionNotFound,
    
    /// 会话过多
    #[error("Too many sessions")]
    TooManySessions,
    
    /// 内存不足
    #[error("Out of memory")]
    OutOfMemory,
    
    // ========== 其他错误 ==========
    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// IO错误
    #[error("IO error: {0}")]
    Io(String),
}

/// 错误恢复策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// 立即重试
    Immediate,
    /// 指数退避
    ExponentialBackoff,
    /// 线性退避
    LinearBackoff,
}

/// 错误恢复策略配置
#[derive(Debug, Clone)]
pub struct ErrorRecoveryPolicy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试策略
    pub retry_strategy: RetryStrategy,
    /// 基础退避时间
    pub backoff_base: Duration,
    /// 最大退避时间
    pub backoff_max: Duration,
}

impl Default for ErrorRecoveryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            retry_strategy: RetryStrategy::ExponentialBackoff,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_secs(30),
        }
    }
}

impl ErrorRecoveryPolicy {
    /// 计算重试延迟
    ///
    /// # 参数
    ///
    /// - `attempt`: 当前重试次数（从0开始）
    ///
    /// # 返回
    ///
    /// 返回应该等待的时间
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self.retry_strategy {
            RetryStrategy::Immediate => Duration::from_millis(0),
            
            RetryStrategy::LinearBackoff => {
                let delay = self.backoff_base * (attempt + 1);
                delay.min(self.backoff_max)
            }
            
            RetryStrategy::ExponentialBackoff => {
                let multiplier = 2u32.pow(attempt);
                let delay = self.backoff_base * multiplier;
                delay.min(self.backoff_max)
            }
        }
    }

    /// 判断是否应该重试
    ///
    /// # 参数
    ///
    /// - `error`: 错误类型
    /// - `attempt`: 当前重试次数
    ///
    /// # 返回
    ///
    /// 如果应该重试返回true
    pub fn should_retry(&self, error: &StreamError, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }

        // 根据错误类型判断是否可以重试
        matches!(
            error,
            StreamError::ConnectionLost
                | StreamError::TransmissionTimeout
                | StreamError::NetworkError(_)
                | StreamError::FileReadError(_)
        )
    }
}

/// 错误转换：从 std::io::Error
impl From<io::Error> for StreamError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => StreamError::FileNotFound(error.to_string()),
            io::ErrorKind::PermissionDenied => StreamError::FileNotAccessible(error.to_string()),
            io::ErrorKind::TimedOut => StreamError::TransmissionTimeout,
            io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe => StreamError::ConnectionLost,
            _ => StreamError::Io(error.to_string()),
        }
    }
}

/// 错误转换：从 tokio::sync::broadcast::error::RecvError
impl From<tokio::sync::broadcast::error::RecvError> for StreamError {
    fn from(error: tokio::sync::broadcast::error::RecvError) -> Self {
        StreamError::Internal(format!("Broadcast receive error: {}", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let errors = vec![
            StreamError::DeviceNotConnected,
            StreamError::FileNotFound("test.mp4".to_string()),
            StreamError::InvalidSeekPosition(10.5),
            StreamError::SessionNotFound,
        ];

        for error in errors {
            let msg = error.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_retry_policy_immediate() {
        let policy = ErrorRecoveryPolicy {
            max_retries: 3,
            retry_strategy: RetryStrategy::Immediate,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_secs(10),
        };

        assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(0));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(0));
    }

    #[test]
    fn test_retry_policy_linear() {
        let policy = ErrorRecoveryPolicy {
            max_retries: 3,
            retry_strategy: RetryStrategy::LinearBackoff,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_secs(10),
        };

        assert_eq!(policy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(300));
    }

    #[test]
    fn test_retry_policy_exponential() {
        let policy = ErrorRecoveryPolicy {
            max_retries: 5,
            retry_strategy: RetryStrategy::ExponentialBackoff,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_secs(10),
        };

        assert_eq!(policy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(800));
        assert_eq!(policy.calculate_delay(4), Duration::from_millis(1600));
    }

    #[test]
    fn test_retry_policy_max_delay() {
        let policy = ErrorRecoveryPolicy {
            max_retries: 10,
            retry_strategy: RetryStrategy::ExponentialBackoff,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_secs(1),
        };

        // 应该被限制在最大值
        assert_eq!(policy.calculate_delay(10), Duration::from_secs(1));
    }

    #[test]
    fn test_should_retry() {
        let policy = ErrorRecoveryPolicy::default();

        // 可以重试的错误
        assert!(policy.should_retry(&StreamError::ConnectionLost, 0));
        assert!(policy.should_retry(&StreamError::TransmissionTimeout, 0));
        assert!(policy.should_retry(&StreamError::NetworkError("test".to_string()), 0));

        // 不可以重试的错误
        assert!(!policy.should_retry(&StreamError::OperationNotSupported, 0));
        assert!(!policy.should_retry(&StreamError::SessionNotFound, 0));

        // 超过最大重试次数
        assert!(!policy.should_retry(&StreamError::ConnectionLost, 5));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let stream_error: StreamError = io_error.into();
        
        match stream_error {
            StreamError::FileNotFound(_) => assert!(true),
            _ => panic!("Expected FileNotFound error"),
        }
    }
}
