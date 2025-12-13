use crate::config::Config;
use common::{
    DeviceCapabilities, DeviceType, MessageType, ProtocolMessage, VideoSegment, Result,
    VideoStreamError,
};
use quinn::{ClientConfig, Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info};
use uuid::Uuid;

pub struct QuicClient {
    endpoint: Endpoint,
    connection: Option<Connection>,
    config: Config,
    session_id: Uuid,
}

impl QuicClient {
    pub async fn new(config: Config) -> Result<Self> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;

        // 配置客户端（跳过证书验证，仅用于Demo）
        let crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let mut client_config = ClientConfig::new(Arc::new(crypto));
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(100_u32.into());
        transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
        client_config.transport_config(Arc::new(transport_config));

        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            connection: None,
            config,
            session_id: Uuid::new_v4(),
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        let server_addr: SocketAddr = format!("{}:{}", self.config.platform_host, self.config.platform_port)
            .parse()
            .map_err(|e| VideoStreamError::NetworkError(format!("Invalid address: {}", e)))?;

        let connection = self
            .endpoint
            .connect(server_addr, "localhost")
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        info!("Connected to platform at {}", server_addr);
        self.connection = Some(connection);

        // 发送SessionStart消息
        self.send_session_start().await?;

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        if let Some(conn) = &self.connection {
            // 检查连接是否真的还活着
            conn.close_reason().is_none()
        } else {
            false
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close(0u32.into(), b"client disconnect");
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        info!("Attempting to reconnect...");
        self.disconnect();
        self.connect().await
    }

    async fn send_session_start(&mut self) -> Result<()> {
        // 构造设备信息
        let device_info = common::DeviceInfo {
            device_id: self.config.device_id.clone(),
            device_name: self.config.device_name.clone(),
            device_type: common::DeviceType::Simulator,
            connection_status: common::ConnectionStatus::Online,
            connection_time: SystemTime::now(),
            last_heartbeat: SystemTime::now(),
            capabilities: common::DeviceCapabilities {
                max_resolution: "1920x1080".to_string(),
                supported_formats: vec!["h264".to_string(), "mp4".to_string()],
                max_bitrate: 10_000_000,
                supports_playback_control: true,
                supports_recording: true,
            },
        };

        // 序列化设备信息作为 payload
        let payload = bincode::serialize(&device_info)
            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;

        let message = ProtocolMessage {
            message_type: MessageType::SessionStart,
            payload,
            sequence_number: 1,
            timestamp: SystemTime::now(),
            session_id: self.session_id,
        };

        let data = bincode::serialize(&message)
            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| VideoStreamError::ProtocolError("Not connected".to_string()))?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        send.write_all(&data)
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;
        send.finish()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        // 等待响应
        let mut response: Vec<u8> = Vec::new();
        recv.read_to_end(1024)
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        debug!("Session start response received");
        Ok(())
    }

    pub async fn send_segment(&mut self, segment: VideoSegment) -> Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| VideoStreamError::ProtocolError("Not connected".to_string()))?;

        let mut stream = conn
            .open_uni()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        let data = bincode::serialize(&segment)
            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;

        stream
            .write_all(&data)
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;
        stream
            .finish()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        debug!("Sent segment: {}", segment.segment_id);
        Ok(())
    }

    pub async fn send_heartbeat(&mut self) -> Result<()> {
        let message = ProtocolMessage {
            message_type: MessageType::Heartbeat,
            payload: vec![],
            sequence_number: 0,
            timestamp: SystemTime::now(),
            session_id: self.session_id,
        };

        let data = bincode::serialize(&message)
            .map_err(|e| VideoStreamError::BincodeError(e.to_string()))?;

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| VideoStreamError::ProtocolError("Not connected".to_string()))?;

        let mut stream = conn
            .open_uni()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        stream
            .write_all(&data)
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;
        stream
            .finish()
            .await
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        debug!("Heartbeat sent");
        Ok(())
    }

    pub fn get_connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    pub fn get_session_id(&self) -> Uuid {
        self.session_id
    }
}

// 跳过服务器证书验证（仅用于Demo）
struct SkipServerVerification;

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> std::result::Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
