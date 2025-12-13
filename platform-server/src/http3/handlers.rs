use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::latency::LatencyMonitor;
use crate::recording::RecordingManager;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type AppState = (
    DeviceManager,
    RecordingManager,
    DistributionManager,
    LatencyMonitor,
    std::sync::Arc<crate::streaming::UnifiedStreamHandler>,
);

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

/// 健康检查
pub async fn health_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("OK".to_string()))
}

/// 获取设备列表
pub async fn get_devices(
    State((device_manager, _, _, _, _)): State<AppState>,
) -> Json<ApiResponse<Vec<common::DeviceInfo>>> {
    let devices = device_manager.get_all_devices();
    Json(ApiResponse::success(devices))
}

/// 获取设备详情
pub async fn get_device_detail(
    Path(device_id): Path<String>,
    State((device_manager, _, _, _, _)): State<AppState>,
) -> Result<Json<ApiResponse<common::DeviceInfo>>, StatusCode> {
    match device_manager.get_device(&device_id) {
        Ok(device) => Ok(Json(ApiResponse::success(device))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// 获取录像列表（通过信令从设备获取）
pub async fn get_recordings(
    Path(device_id): Path<String>,
    State((device_manager, _, _, _, _)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<common::RecordingInfo>>>, StatusCode> {
    use common::{FileListResponse, MessageType, ProtocolMessage};
    use std::time::SystemTime;
    
    // 获取设备连接
    let connection = device_manager
        .get_connection(&device_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // 发送文件列表查询
    let query_msg = ProtocolMessage {
        message_type: MessageType::FileListQuery,
        payload: vec![],
        sequence_number: 1,
        timestamp: SystemTime::now(),
        session_id: uuid::Uuid::new_v4(),
    };

    let data = bincode::serialize(&query_msg).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 打开双向流
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 发送请求
    send.write_all(&data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    send.finish()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 接收响应
    let response_buf = recv
        .read_to_end(10 * 1024 * 1024)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 解析响应
    let response_msg: ProtocolMessage =
        bincode::deserialize(&response_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if response_msg.message_type == MessageType::FileListResponse {
        let file_list: FileListResponse = bincode::deserialize(&response_msg.payload)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApiResponse::success(file_list.files)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[derive(Deserialize)]
pub struct StartLiveStreamRequest {
    client_id: String,
}

#[derive(Serialize)]
pub struct StartLiveStreamResponse {
    session_id: String,
    stream_url: String,
}

/// 开始直通播放
pub async fn start_live_stream(
    Path(device_id): Path<String>,
    State((device_manager, _, distribution_manager, _, _)): State<AppState>,
    Json(req): Json<StartLiveStreamRequest>,
) -> Result<Json<ApiResponse<StartLiveStreamResponse>>, StatusCode> {
    // 检查设备是否在线
    if !device_manager.is_device_online(&device_id) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let session_id = Uuid::new_v4();
    let _receiver = distribution_manager.create_session(session_id);

    let response = StartLiveStreamResponse {
        session_id: session_id.to_string(),
        stream_url: format!("/api/v1/stream/{}/segments", session_id),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 停止流
pub async fn stop_stream(
    Path(session_id): Path<String>,
    State((_, _, distribution_manager, _, _)): State<AppState>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&session_id) {
        distribution_manager.close_session(&uuid);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Deserialize)]
pub struct StartPlaybackRequest {
    file_id: String,
    client_id: String,
    start_position: Option<f64>,
}

#[derive(Serialize)]
pub struct StartPlaybackResponse {
    session_id: String,
    playback_url: String,
}

/// 开始录像回放（向设备发送回放请求）
pub async fn start_playback(
    State((device_manager, _, distribution_manager, _, _)): State<AppState>,
    Json(req): Json<StartPlaybackRequest>,
) -> Result<Json<ApiResponse<StartPlaybackResponse>>, StatusCode> {
    use common::{FileRequest, MessageType, ProtocolMessage};
    use std::time::SystemTime;
    
    let file_id = &req.file_id;
    tracing::info!("📹 Start playback request for file_id: {}", file_id);
    
    // 从 file_id 中提取 device_id (格式: device_001_filename)
    // 分割成最多3部分：device, 001, filename
    let parts: Vec<&str> = file_id.splitn(3, '_').collect();
    if parts.len() < 3 {
        tracing::error!("Invalid file_id format: {}", file_id);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let device_id = format!("{}_{}", parts[0], parts[1]);
    tracing::info!("Extracted device_id: {}", device_id);
    
    // 获取设备连接
    let connection = device_manager
        .get_connection(&device_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // 创建播放会话
    let session_id = Uuid::new_v4();
    let _receiver = distribution_manager.create_session(session_id);

    // 构建文件请求
    let file_request = FileRequest {
        file_path: file_id.to_string(),
        priority: 1,
        seek_position: req.start_position,
        playback_rate: 1.0,
    };

    let file_req_data =
        bincode::serialize(&file_request).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 发送回放请求到设备
    let playback_msg = ProtocolMessage {
        message_type: MessageType::FileRequest,
        payload: file_req_data,
        sequence_number: 1,
        timestamp: SystemTime::now(),
        session_id,
    };

    let data =
        bincode::serialize(&playback_msg).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 打开双向流
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 发送请求
    send.write_all(&data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    send.finish()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 等待确认
    let _ = recv
        .read_to_end(1024)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StartPlaybackResponse {
        session_id: session_id.to_string(),
        playback_url: format!("/api/v1/playback/{}/segments", session_id),
    };

    Ok(Json(ApiResponse::success(response)))
}

#[derive(Deserialize)]
pub struct PlaybackControlRequest {
    command: String,
    position: Option<f64>,
    rate: Option<f64>,
}

/// 播放控制
pub async fn playback_control(
    Path(session_id): Path<String>,
    Json(req): Json<PlaybackControlRequest>,
) -> StatusCode {
    // TODO: 实现播放控制逻辑
    StatusCode::OK
}

/// 获取播放分片（SSE流）
pub async fn get_playback_segments(
    Path(session_id): Path<String>,
    State((_, _, distribution_manager, _, _)): State<AppState>,
) -> Result<axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    use futures::stream::{Stream, StreamExt};
    use std::pin::Pin;
    
    tracing::info!("📡 SSE connection request for session: {}", session_id);
    
    let uuid = Uuid::parse_str(&session_id).map_err(|e| {
        tracing::error!("Invalid session_id format: {}", e);
        StatusCode::BAD_REQUEST
    })?;
    
    // 获取接收器
    let mut receiver = distribution_manager
        .get_receiver(&uuid)
        .ok_or_else(|| {
            tracing::error!("Session not found: {}", session_id);
            StatusCode::NOT_FOUND
        })?;
    
    tracing::info!("✓ SSE stream started for session: {}", session_id);

    // 创建 SSE 流
    let stream = async_stream::stream! {
        tracing::info!("📺 SSE stream loop started");
        let mut count = 0;
        loop {
            match receiver.recv().await {
                Ok(segment) => {
                    count += 1;
                    if count % 10 == 0 {
                        tracing::debug!("📦 Sent {} segments via SSE", count);
                    }
                    
                    // 创建包含 base64 编码数据的 JSON 对象
                    let segment_json = serde_json::json!({
                        "segment_id": segment.segment_id,
                        "session_id": segment.session_id,
                        "timestamp": segment.timestamp,
                        "duration": segment.duration,
                        "flags": segment.flags,
                        "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &segment.data),
                        "data_length": segment.data.len()
                    });
                    
                    if let Ok(json) = serde_json::to_string(&segment_json) {
                        yield Ok(axum::response::sse::Event::default().data(json));
                    }
                }
                Err(e) => {
                    tracing::info!("SSE stream ended: {:?}, total segments: {}", e, count);
                    break;
                }
            }
        }
    };

    Ok(axum::response::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive")
    ))
}


// ========== 统一流API端点 ==========

use crate::streaming::{
    LiveStreamSource, PlaybackSource, StreamConfig, UnifiedStreamHandler,
    FileStreamReader, FileReaderConfig,
};
use std::sync::Arc;

/// 统一流启动请求
#[derive(Debug, Deserialize)]
pub struct UnifiedStreamStartRequest {
    /// 流模式：live 或 playback
    pub mode: String,
    /// 数据源配置
    pub source: StreamSourceConfig,
    /// 流配置（可选）
    pub config: Option<StreamConfigRequest>,
}

/// 数据源配置
#[derive(Debug, Deserialize)]
pub struct StreamSourceConfig {
    /// 设备ID（用于直通播放）
    pub device_id: Option<String>,
    /// 文件ID（用于录像回放）
    pub file_id: Option<String>,
    /// 起始位置（秒，用于回放）
    pub start_position: Option<f64>,
    /// 播放速率（用于回放）
    pub playback_rate: Option<f64>,
}

/// 流配置请求
#[derive(Debug, Deserialize)]
pub struct StreamConfigRequest {
    /// 客户端ID
    pub client_id: String,
    /// 是否启用低延迟模式
    pub low_latency_mode: Option<bool>,
    /// 目标延迟（毫秒）
    pub target_latency_ms: Option<u32>,
}

/// 统一流启动响应
#[derive(Debug, Serialize)]
pub struct UnifiedStreamStartResponse {
    /// 会话ID
    pub session_id: String,
    /// 流URL
    pub stream_url: String,
    /// 控制URL
    pub control_url: String,
    /// 预估延迟（毫秒）
    pub estimated_latency_ms: u32,
}

/// 统一流启动API
///
/// POST /api/v1/stream/start
///
/// 支持直通播放和录像回放的统一启动接口。
pub async fn unified_stream_start(
    State((device_manager, _, distribution_manager, _, handler)): State<AppState>,
    Json(req): Json<UnifiedStreamStartRequest>,
) -> Result<Json<ApiResponse<UnifiedStreamStartResponse>>, StatusCode> {
    // 解析流模式
    let mode = req.mode.to_lowercase();
    
    // 创建流配置
    let config = if let Some(cfg) = req.config {
        StreamConfig {
            low_latency: cfg.low_latency_mode.unwrap_or(true),
            target_latency_ms: cfg.target_latency_ms.unwrap_or(100),
            ..Default::default()
        }
    } else {
        StreamConfig::default()
    };

    // 预先生成会话ID（用于live模式）
    let session_id = Uuid::new_v4();
    
    // 根据模式创建数据源
    let source: Box<dyn crate::streaming::StreamSource> = match mode.as_str() {
        "live" => {
            use common::{MessageType, ProtocolMessage, StartLiveStreamRequest};
            use std::time::SystemTime;
            
            // 直通播放模式
            let device_id = req.source.device_id
                .ok_or(StatusCode::BAD_REQUEST)?;
            
            // 检查设备是否在线
            if !device_manager.is_device_online(&device_id) {
                tracing::warn!("Device not online: {}", device_id);
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }

            // 获取设备连接
            let connection = device_manager
                .get_connection(&device_id)
                .ok_or_else(|| {
                    tracing::error!("Device connection not found: {}", device_id);
                    StatusCode::NOT_FOUND
                })?;

            tracing::info!("🎥 Starting live stream for device: {} (session: {})", device_id, session_id);

            // 构建StartLiveStream请求
            let live_request = StartLiveStreamRequest {
                quality_preference: "low_latency".to_string(),
                target_latency_ms: config.target_latency_ms,
                target_fps: 30,
                target_bitrate: 2_000_000, // 2 Mbps
            };

            let request_data = bincode::serialize(&live_request)
                .map_err(|e| {
                    tracing::error!("Failed to serialize live request: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // 发送信令到设备
            let signal_msg = ProtocolMessage {
                message_type: MessageType::StartLiveStream,
                payload: request_data,
                sequence_number: 1,
                timestamp: SystemTime::now(),
                session_id,
            };

            let data = bincode::serialize(&signal_msg)
                .map_err(|e| {
                    tracing::error!("Failed to serialize signal message: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // 打开双向流
            let (mut send, mut recv) = connection
                .open_bi()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to open bi stream: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // 发送请求
            send.write_all(&data)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to write signal: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            send.finish()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to finish send: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // 等待确认
            let _ = recv
                .read_to_end(1024)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to read confirmation: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            tracing::info!("✓ Live stream started on device: {}", device_id);

            // 创建LiveStreamSource
            // 使用DistributionManager创建会话并获取接收器
            let segment_rx = distribution_manager.create_session(session_id);
            
            let live_source = LiveStreamSource::new(device_id, segment_rx);
            Box::new(live_source)
        }
        "playback" => {
            // 录像回放模式
            let file_id = req.source.file_id
                .ok_or(StatusCode::BAD_REQUEST)?;
            
            // 构建文件路径（简化实现）
            let file_path = std::path::PathBuf::from("../device-simulator/test-videos")
                .join(&file_id);

            if !file_path.exists() {
                return Err(StatusCode::NOT_FOUND);
            }

            // 创建PlaybackSource
            let playback_source = PlaybackSource::new(file_id, file_path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // TODO: 如果指定了起始位置，进行定位
            // if let Some(position) = req.source.start_position {
            //     playback_source.seek(position)
            //         .await
            //         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            // }

            Box::new(playback_source)
        }
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 启动流会话（使用预先生成的session_id用于live模式）
    let final_session_id = if mode == "live" {
        // live模式使用预先生成的session_id
        handler
            .start_stream_with_id(session_id, source, config.clone())
            .await
            .map_err(|e| {
                tracing::error!("Failed to start stream: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        session_id
    } else {
        // playback模式让handler生成session_id
        handler
            .start_stream(source, config.clone())
            .await
            .map_err(|e| {
                tracing::error!("Failed to start stream: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    // 构建响应
    let response = UnifiedStreamStartResponse {
        session_id: final_session_id.to_string(),
        stream_url: format!("/api/v1/stream/{}/segments", final_session_id),
        control_url: format!("/api/v1/stream/{}/control", final_session_id),
        estimated_latency_ms: config.target_latency_ms,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 流控制请求
#[derive(Debug, Deserialize)]
pub struct StreamControlRequest {
    /// 控制命令：pause, resume, seek, set_rate, stop
    pub command: String,
    /// 定位位置（秒，用于seek命令）
    pub position: Option<f64>,
    /// 播放速率（用于set_rate命令）
    pub rate: Option<f64>,
}

/// 流控制响应
#[derive(Debug, Serialize)]
pub struct StreamControlResponse {
    /// 操作状态
    pub status: String,
    /// 当前流状态
    pub current_state: String,
}

/// 统一流控制API
///
/// POST /api/v1/stream/{session_id}/control
///
/// 支持暂停、恢复、定位、倍速、停止等控制命令。
pub async fn unified_stream_control(
    Path(session_id): Path<String>,
    State(handler): State<Arc<UnifiedStreamHandler>>,
    Json(req): Json<StreamControlRequest>,
) -> Result<Json<ApiResponse<StreamControlResponse>>, StatusCode> {
    // 解析会话ID
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 执行控制命令
    let result = match req.command.to_lowercase().as_str() {
        "pause" => {
            handler.pause_stream(session_id).await
        }
        "resume" => {
            handler.resume_stream(session_id).await
        }
        "seek" => {
            let position = req.position.ok_or(StatusCode::BAD_REQUEST)?;
            handler.seek_stream(session_id, position).await
        }
        "set_rate" => {
            let rate = req.rate.ok_or(StatusCode::BAD_REQUEST)?;
            handler.set_rate(session_id, rate).await
        }
        "stop" => {
            handler.stop_stream(session_id).await
        }
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取当前状态
    let info = handler.get_session_info(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StreamControlResponse {
        status: "success".to_string(),
        current_state: format!("{:?}", info.state),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 流状态响应
#[derive(Debug, Serialize)]
pub struct StreamStatusResponse {
    /// 会话ID
    pub session_id: String,
    /// 流模式
    pub mode: String,
    /// 流状态
    pub state: String,
    /// 当前位置（秒）
    pub current_position: f64,
    /// 播放速率
    pub playback_rate: f64,
    /// 统计信息
    pub stats: StreamStatsResponse,
}

/// 流统计信息响应
#[derive(Debug, Serialize)]
pub struct StreamStatsResponse {
    /// 平均延迟（毫秒）
    pub average_latency_ms: f64,
    /// 当前延迟（毫秒）
    pub current_latency_ms: f64,
    /// 吞吐量（Mbps）
    pub throughput_mbps: f64,
    /// 丢包率
    pub packet_loss_rate: f64,
}

/// 统一流状态查询API
///
/// GET /api/v1/stream/{session_id}/status
///
/// 查询流会话的当前状态和统计信息。
pub async fn unified_stream_status(
    Path(session_id): Path<String>,
    State(handler): State<Arc<UnifiedStreamHandler>>,
) -> Result<Json<ApiResponse<StreamStatusResponse>>, StatusCode> {
    // 解析会话ID
    let session_id = Uuid::parse_str(&session_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 获取会话信息
    let info = handler.get_session_info(session_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 获取统计信息
    let stats = handler.get_session_stats(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = StreamStatusResponse {
        session_id: session_id.to_string(),
        mode: format!("{:?}", info.mode),
        state: format!("{:?}", info.state),
        current_position: info.current_position,
        playback_rate: info.playback_rate,
        stats: StreamStatsResponse {
            average_latency_ms: stats.average_latency_ms,
            current_latency_ms: stats.current_latency_ms,
            throughput_mbps: stats.throughput_mbps,
            packet_loss_rate: stats.packet_loss_rate,
        },
    };

    Ok(Json(ApiResponse::success(response)))
}
