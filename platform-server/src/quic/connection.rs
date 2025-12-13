use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::recording::RecordingManager;
use common::{
    DeviceCapabilities, DeviceInfo, DeviceType, ConnectionStatus, MessageType, 
    ProtocolMessage, VideoSegment, Result, VideoStreamError,
};
use quinn::Connection;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub async fn handle_connection(
    connection: Connection,
    device_manager: DeviceManager,
    _recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
) -> Result<()> {
    let session_id = Uuid::new_v4();
    info!("Handling connection with session: {}", session_id);

    // 处理双向流（控制信令）
    let conn_clone = connection.clone();
    let device_mgr_clone = device_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = handle_bi_streams(conn_clone, device_mgr_clone, session_id).await {
            error!("Bi-stream error: {}", e);
        }
    });

    // 处理单向流（视频数据）
    handle_uni_streams(connection, device_manager, distribution_manager, session_id).await
}

async fn handle_bi_streams(
    connection: Connection,
    device_manager: DeviceManager,
    session_id: Uuid,
) -> Result<()> {
    loop {
        match connection.accept_bi().await {
            Ok((mut send, mut recv)) => {
                let device_mgr = device_manager.clone();
                let conn = connection.clone();
                tokio::spawn(async move {
                    // 读取消息
                    let buf = match recv.read_to_end(1024 * 1024).await {
                        Ok(data) => data,
                        Err(e) => {
                            error!("Failed to read message: {}", e);
                            return;
                        }
                    };

                    // 解析消息
                    match bincode::deserialize::<ProtocolMessage>(&buf) {
                        Ok(msg) => {
                            debug!("Received message: {:?}", msg.message_type);
                            
                            // 处理消息
                            match msg.message_type {
                                MessageType::SessionStart => {
                                    // 从 payload 中反序列化设备信息
                                    match bincode::deserialize::<DeviceInfo>(&msg.payload) {
                                        Ok(mut device) => {
                                            // 更新连接状态和时间
                                            device.connection_status = ConnectionStatus::Online;
                                            device.connection_time = SystemTime::now();
                                            device.last_heartbeat = SystemTime::now();
                                            
                                            let device_id = device.device_id.clone();
                                            
                                            // 注册设备
                                            if let Err(e) = device_mgr.register_device(device) {
                                                error!("Failed to register device: {}", e);
                                            } else {
                                                info!("✓ Device registered: {}", device_id);
                                            }
                                            
                                            // 保存连接
                                            device_mgr.store_connection(device_id, conn);
                                            
                                            // 发送响应
                                            let response = b"OK";
                                            let _ = send.write_all(response).await;
                                        }
                                        Err(e) => {
                                            error!("Failed to deserialize device info: {}", e);
                                            // 使用默认设备信息作为后备
                                            let device_id = format!("device_{}", session_id);
                                            let device = DeviceInfo {
                                                device_id: device_id.clone(),
                                                device_name: "Unknown Device".to_string(),
                                                device_type: DeviceType::Simulator,
                                                connection_status: ConnectionStatus::Online,
                                                connection_time: SystemTime::now(),
                                                last_heartbeat: SystemTime::now(),
                                                capabilities: DeviceCapabilities {
                                                    max_resolution: "1920x1080".to_string(),
                                                    supported_formats: vec!["h264".to_string(), "mp4".to_string()],
                                                    max_bitrate: 10_000_000,
                                                    supports_playback_control: true,
                                                    supports_recording: true,
                                                },
                                            };
                                            
                                            if let Err(e) = device_mgr.register_device(device) {
                                                error!("Failed to register fallback device: {}", e);
                                            }
                                            
                                            device_mgr.store_connection(device_id, conn);
                                            let response = b"OK";
                                            let _ = send.write_all(response).await;
                                        }
                                    }
                                }
                                _ => {
                                    debug!("Unhandled message type: {:?}", msg.message_type);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize message: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                warn!("Accept bi-stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

async fn handle_uni_streams(
    connection: Connection,
    device_manager: DeviceManager,
    distribution_manager: DistributionManager,
    session_id: Uuid,
) -> Result<()> {
    // 不在这里创建会话，会话由 start_playback 创建
    let device_id = format!("device_{}", session_id);

    loop {
        match connection.accept_uni().await {
            Ok(mut recv) => {
                let dist_mgr = distribution_manager.clone();
                let dev_mgr = device_manager.clone();
                let dev_id = device_id.clone();
                tokio::spawn(async move {
                    match recv.read_to_end(10 * 1024 * 1024).await {
                        Ok(buf) => {
                            debug!("Received {} bytes", buf.len());
                            
                            // 尝试解析为协议消息（心跳等）
                            if let Ok(msg) = bincode::deserialize::<ProtocolMessage>(&buf) {
                                match msg.message_type {
                                    MessageType::Heartbeat => {
                                        debug!("Received heartbeat from device: {}", dev_id);
                                        // 更新设备心跳时间
                                        let _ = dev_mgr.update_heartbeat(&dev_id);
                                        return;
                                    }
                                    _ => {
                                        debug!("Received protocol message: {:?}", msg.message_type);
                                    }
                                }
                            }
                            
                            // 尝试解析为视频分片
                            match bincode::deserialize::<VideoSegment>(&buf) {
                                Ok(segment) => {
                                    let seg_session_id = segment.session_id;
                                    debug!("Received segment: {} for session: {}", segment.segment_id, seg_session_id);
                                    // 使用分片中的 session_id 来分发（而不是连接的 session_id）
                                    let _ = dist_mgr.distribute_segment(&seg_session_id, segment);
                                }
                                Err(e) => {
                                    debug!("Failed to deserialize as segment: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read stream: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                warn!("Accept uni-stream error: {}", e);
                break;
            }
        }
    }

    distribution_manager.close_session(&session_id);
    Ok(())
}
