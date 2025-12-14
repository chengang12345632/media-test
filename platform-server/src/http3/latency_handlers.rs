// 延迟监控API处理器
//
// 本模块提供延迟监控相关的HTTP API端点

use crate::latency::{
    AlertBroadcaster, AlertMessage, EndToEndLatencyMonitor, LatencyStatistics,
    LatencyStatisticsManager,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Json,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use uuid::Uuid;

/// API响应结构
#[derive(Serialize)]
pub struct ApiResponse<T> {
    status: String,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            status: "success".to_string(),
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            status: "error".to_string(),
            data: None,
            error: Some(message),
        }
    }
}

/// 应用状态
pub type LatencyAppState = (
    Arc<EndToEndLatencyMonitor>,
    Arc<LatencyStatisticsManager>,
    Arc<AlertBroadcaster>,
);

/// 获取会话的延迟统计
///
/// GET /api/v1/latency/sessions/{session_id}/statistics
pub async fn get_session_statistics(
    Path(session_id): Path<Uuid>,
    State((_, stats_manager, _)): State<LatencyAppState>,
) -> Result<Json<ApiResponse<LatencyStatistics>>, StatusCode> {
    info!("Getting statistics for session {}", session_id);

    match stats_manager.get_statistics(&session_id) {
        Some(stats) => Ok(Json(ApiResponse::success(stats))),
        None => {
            error!("Statistics not found for session {}", session_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 获取所有会话的延迟统计
///
/// GET /api/v1/latency/statistics
pub async fn get_all_statistics(
    State((_, stats_manager, _)): State<LatencyAppState>,
) -> Json<ApiResponse<Vec<LatencyStatistics>>> {
    info!("Getting statistics for all sessions");
    let all_stats = stats_manager.get_all_statistics();
    Json(ApiResponse::success(all_stats))
}

/// 获取分片的延迟分解
///
/// GET /api/v1/latency/segments/{segment_id}/breakdown
pub async fn get_segment_breakdown(
    Path(segment_id): Path<Uuid>,
    State((monitor, _, _)): State<LatencyAppState>,
) -> Result<Json<ApiResponse<crate::latency::LatencyBreakdown>>, StatusCode> {
    info!("Getting latency breakdown for segment {}", segment_id);

    match monitor.get_measurement(&segment_id) {
        Some(breakdown) => Ok(Json(ApiResponse::success(breakdown))),
        None => {
            error!("Latency breakdown not found for segment {}", segment_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// 订阅延迟告警（SSE）
///
/// GET /api/v1/latency/alerts
///
/// 返回一个SSE流，实时推送延迟告警和统计更新
pub async fn subscribe_alerts(
    State((_, _, broadcaster)): State<LatencyAppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New client subscribing to latency alerts");

    let rx = broadcaster.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(|result| match result {
        Ok(message) => {
            debug!("Broadcasting alert message: {:?}", message);
            match serde_json::to_string(&message) {
                Ok(json) => Some(Ok(Event::default().data(json))),
                Err(e) => {
                    error!("Failed to serialize alert message: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("Broadcast stream error: {}", e);
            None
        }
    });

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// 订阅特定会话的延迟告警（SSE）
///
/// GET /api/v1/latency/sessions/{session_id}/alerts
pub async fn subscribe_session_alerts(
    Path(session_id): Path<Uuid>,
    State((_, _, broadcaster)): State<LatencyAppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New client subscribing to alerts for session {}", session_id);

    let rx = broadcaster.subscribe_session(session_id);
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(move |result| match result {
        Ok(message) => {
            // 过滤只属于该会话的消息
            let matches = match &message {
                AlertMessage::LatencyAlert {
                    session_id: msg_session_id,
                    ..
                }
                | AlertMessage::StatisticsUpdate {
                    session_id: msg_session_id,
                    ..
                }
                | AlertMessage::SessionStarted {
                    session_id: msg_session_id,
                    ..
                }
                | AlertMessage::SessionEnded {
                    session_id: msg_session_id,
                    ..
                } => *msg_session_id == session_id,
            };

            if matches {
                debug!(
                    "Broadcasting alert message for session {}: {:?}",
                    session_id, message
                );
                match serde_json::to_string(&message) {
                    Ok(json) => Some(Ok(Event::default().data(json))),
                    Err(e) => {
                        error!("Failed to serialize alert message: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            error!("Broadcast stream error: {}", e);
            None
        }
    });

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// 延迟监控配置
#[derive(Debug, Deserialize, Serialize)]
pub struct LatencyConfig {
    /// 传输延迟阈值（毫秒）
    pub transmission_threshold_ms: Option<u64>,
    /// 处理延迟阈值（毫秒）
    pub processing_threshold_ms: Option<u64>,
    /// 分发延迟阈值（毫秒）
    pub distribution_threshold_ms: Option<u64>,
    /// 端到端延迟阈值（毫秒）
    pub end_to_end_threshold_ms: Option<u64>,
}

/// 更新延迟监控配置
///
/// PUT /api/v1/latency/config
///
/// 注意：此端点用于演示，实际实现需要重新创建监控器实例
pub async fn update_latency_config(
    State(_state): State<LatencyAppState>,
    Json(config): Json<LatencyConfig>,
) -> Json<ApiResponse<String>> {
    info!("Updating latency monitoring configuration: {:?}", config);

    // 在实际实现中，这里应该更新监控器的阈值配置
    // 由于当前设计中阈值在创建时设置，这里只返回成功消息
    Json(ApiResponse::success(
        "Configuration update received (note: requires restart to apply)".to_string(),
    ))
}

/// 健康检查端点
///
/// GET /api/v1/latency/health
pub async fn latency_health_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("Latency monitoring is healthy".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latency::{LatencyThresholds, LatencyStatisticsManager};
    use std::time::Duration;

    #[tokio::test]
    async fn test_get_session_statistics() {
        let monitor = Arc::new(EndToEndLatencyMonitor::with_defaults());
        let stats_manager = Arc::new(LatencyStatisticsManager::new());
        let broadcaster = Arc::new(AlertBroadcaster::with_defaults());

        let session_id = Uuid::new_v4();
        stats_manager.start_session(session_id);
        stats_manager.record_segment_latency(&session_id, Duration::from_millis(50), 1024);

        let state = (monitor, stats_manager, broadcaster);

        let result = get_session_statistics(Path(session_id), State(state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_all_statistics() {
        let monitor = Arc::new(EndToEndLatencyMonitor::with_defaults());
        let stats_manager = Arc::new(LatencyStatisticsManager::new());
        let broadcaster = Arc::new(AlertBroadcaster::with_defaults());

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        stats_manager.start_session(session1);
        stats_manager.start_session(session2);

        let state = (monitor, stats_manager, broadcaster);

        let result = get_all_statistics(State(state)).await;
        assert_eq!(result.0.data.as_ref().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_health_check() {
        let result = latency_health_check().await;
        assert_eq!(result.0.status, "success");
    }
}
