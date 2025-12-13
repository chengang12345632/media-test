use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::latency::LatencyMonitor;
use crate::recording::RecordingManager;
use crate::streaming::UnifiedStreamHandler;
use axum::{
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn create_router(
    device_manager: DeviceManager,
    recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
    latency_monitor: LatencyMonitor,
    stream_handler: Arc<UnifiedStreamHandler>,
) -> Router {
    Router::new()
        // 设备管理
        .route("/api/v1/devices", get(super::handlers::get_devices))
        .route("/api/v1/devices/:device_id", get(super::handlers::get_device_detail))
        
        // 录像管理
        .route(
            "/api/v1/devices/:device_id/recordings",
            get(super::handlers::get_recordings),
        )
        
        // 统一流启动API（支持直通播放和录像回放）
        .route(
            "/api/v1/stream/start",
            post(super::handlers::unified_stream_start),
        )
        .route(
            "/api/v1/stream/:session_id/segments",
            get(super::handlers::get_playback_segments),
        )
        .route(
            "/api/v1/stream/:session_id/control",
            post(super::handlers::playback_control),
        )
        
        // 直通播放
        .route(
            "/api/v1/devices/:device_id/live-stream",
            post(super::handlers::start_live_stream),
        )
        .route(
            "/api/v1/stream/:session_id",
            delete(super::handlers::stop_stream),
        )
        
        // 录像回放（使用 POST body 传递 file_id）
        .route(
            "/api/v1/playback/start",
            post(super::handlers::start_playback),
        )
        .route(
            "/api/v1/playback/:session_id/control",
            post(super::handlers::playback_control),
        )
        .route(
            "/api/v1/playback/:session_id/segments",
            get(super::handlers::get_playback_segments),
        )
        
        // 直接流式传输（用于 MP4 文件）
        .route(
            "/api/v1/recordings/:file_id/stream",
            get(super::streaming::stream_recording_file),
        )
        
        // 健康检查
        .route("/health", get(super::handlers::health_check))
        
        // 添加状态 - 使用嵌套路由来支持多个状态
        .with_state((
            device_manager,
            recording_manager,
            distribution_manager,
            latency_monitor,
            stream_handler,
        ))
        
        // CORS中间件
        .layer(CorsLayer::permissive())
}
