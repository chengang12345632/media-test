use common::{VideoSegment, PlaybackControl, Result};
use tracing::debug;

pub struct ProtocolConverter;

impl ProtocolConverter {
    pub fn new() -> Self {
        Self
    }

    /// 转换QUIC视频分片到HTTP3格式
    pub async fn convert_quic_to_http3(&self, segment: VideoSegment) -> Result<Vec<u8>> {
        debug!("Converting QUIC segment to HTTP3: {}", segment.segment_id);
        // 简单序列化为二进制
        bincode::serialize(&segment)
            .map_err(|e| common::VideoStreamError::BincodeError(e.to_string()))
    }

    /// 转换HTTP3控制命令到QUIC格式
    pub async fn convert_http3_to_quic(&self, control: PlaybackControl) -> Result<Vec<u8>> {
        debug!("Converting HTTP3 control to QUIC: {:?}", control.command);
        serde_json::to_vec(&control)
            .map_err(|e| common::VideoStreamError::SerdeError(e))
    }
}
