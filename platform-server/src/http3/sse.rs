// 统一低延迟视频流传输系统 - SSE端点实现
//
// 本模块实现了基于Server-Sent Events (SSE)的视频分片推送机制。
//
// # 特性
//
// - 实时推送视频分片到前端
// - 支持多客户端并发订阅
// - 自动处理客户端断开
// - 包含完整的分片元数据

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    http::StatusCode,
};
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::streaming::{UnifiedStreamHandler, VideoSegment};

/// SSE视频分片数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseSegmentData {
    /// 分片ID
    pub segment_id: String,
    /// 时间戳（秒）
    pub timestamp: f64,
    /// 时长（秒）
    pub duration: f64,
    /// 是否为关键帧
    pub is_keyframe: bool,
    /// 分片格式
    pub format: String,
    /// Base64编码的数据
    pub data: String,
}

impl From<VideoSegment> for SseSegmentData {
    fn from(segment: VideoSegment) -> Self {
        use base64::{Engine as _, engine::general_purpose};
        
        Self {
            segment_id: segment.segment_id.to_string(),
            timestamp: segment.timestamp,
            duration: segment.duration,
            is_keyframe: segment.is_keyframe,
            format: format!("{:?}", segment.format),
            data: general_purpose::STANDARD.encode(&segment.data),
        }
    }
}

/// SSE流端点
///
/// 实现GET /api/v1/stream/{session_id}/segments
///
/// 客户端通过此端点订阅视频分片流，服务器通过SSE实时推送分片。
///
/// # 参数
///
/// - `session_id`: 流会话ID
/// - `handler`: 统一流处理器
///
/// # 返回
///
/// 返回SSE流或错误状态码
pub async fn stream_segments_sse(
    Path(session_id): Path<String>,
    State(handler): State<Arc<UnifiedStreamHandler>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // 解析会话ID
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| {
            warn!("Invalid session ID format: {}", session_id);
            StatusCode::BAD_REQUEST
        })?;

    debug!("SSE client connecting to session: {}", session_id);

    // 订阅会话的分片流
    let receiver = handler
        .subscribe(session_id)
        .await
        .map_err(|e| {
            error!("Failed to subscribe to session {}: {}", session_id, e);
            StatusCode::NOT_FOUND
        })?;

    // 创建SSE流
    let stream = create_sse_stream(receiver, session_id);

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// 创建SSE流
///
/// 将broadcast接收器转换为SSE事件流。
///
/// # 参数
///
/// - `receiver`: 分片接收器
/// - `session_id`: 会话ID
///
/// # 返回
///
/// 返回SSE事件流
fn create_sse_stream(
    receiver: broadcast::Receiver<VideoSegment>,
    session_id: Uuid,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // 将broadcast接收器转换为Stream
    let broadcast_stream = BroadcastStream::new(receiver);

    // 转换为SSE事件流
    broadcast_stream.filter_map(move |result| async move {
        match result {
            Ok(segment) => {
                debug!(
                    "Sending segment {} to SSE client (session: {})",
                    segment.segment_id, session_id
                );

                // 转换为SSE数据
                let sse_data = SseSegmentData::from(segment);

                // 序列化为JSON
                match serde_json::to_string(&sse_data) {
                    Ok(json) => {
                        // 创建SSE事件
                        let event = Event::default().event("segment").data(json);
                        Some(Ok(event))
                    }
                    Err(e) => {
                        error!("Failed to serialize segment: {}", e);
                        None
                    }
                }
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(skipped)) => {
                warn!(
                    "SSE client lagged, skipped {} segments (session: {})",
                    skipped, session_id
                );
                // 发送警告事件
                let event = Event::default()
                    .event("warning")
                    .data(format!("Lagged: skipped {} segments", skipped));
                Some(Ok(event))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::source::{SegmentFormat, SegmentSourceType, StreamConfig};
    use crate::streaming::handler::tests::TestSource;

    #[tokio::test]
    async fn test_sse_segment_data_conversion() {
        let segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.5,
            duration: 0.033,
            data: vec![1, 2, 3, 4, 5],
            is_keyframe: true,
            format: SegmentFormat::H264Raw,
            source_type: SegmentSourceType::Live,
            receive_time: None,
            forward_time: None,
        };

        let sse_data = SseSegmentData::from(segment.clone());

        assert_eq!(sse_data.segment_id, segment.segment_id.to_string());
        assert_eq!(sse_data.timestamp, 1.5);
        assert_eq!(sse_data.duration, 0.033);
        assert_eq!(sse_data.is_keyframe, true);
        assert!(!sse_data.data.is_empty());
    }

    #[tokio::test]
    async fn test_sse_stream_creation() {
        let handler = Arc::new(UnifiedStreamHandler::new());
        let source = Box::new(TestSource::new(5));

        // 启动会话
        let session_id = handler
            .start_stream(source, StreamConfig::default())
            .await
            .unwrap();

        // 订阅
        let receiver = handler.subscribe(session_id).await.unwrap();

        // 创建SSE流
        let mut stream = create_sse_stream(receiver, session_id);

        // 接收一些事件
        for i in 0..3 {
            match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(event))) => {
                    println!("Received SSE event {}: {:?}", i, event);
                }
                Ok(Some(Err(_))) => {
                    panic!("SSE event error");
                }
                Ok(None) => {
                    println!("Stream ended");
                    break;
                }
                Err(_) => {
                    panic!("Timeout waiting for SSE event");
                }
            }
        }

        handler.stop_stream(session_id).await.unwrap();
    }
}
