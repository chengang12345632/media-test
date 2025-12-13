use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::recording::RecordingManager;
use common::{Result, VideoStreamError};
use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

pub struct QuicServer {
    endpoint: Endpoint,
    device_manager: DeviceManager,
    recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
}

impl QuicServer {
    pub fn new(
        addr: SocketAddr,
        device_manager: DeviceManager,
        recording_manager: RecordingManager,
        distribution_manager: DistributionManager,
    ) -> Result<Self> {
        // 创建自签名证书
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .map_err(|e| VideoStreamError::ProtocolError(e.to_string()))?;
        
        let cert_der = cert.serialize_der()
            .map_err(|e| VideoStreamError::ProtocolError(e.to_string()))?;
        let key_der = cert.serialize_private_key_der();

        let cert_chain = vec![rustls::Certificate(cert_der)];
        let key = rustls::PrivateKey(key_der);

        let mut server_config = ServerConfig::with_single_cert(cert_chain, key)
            .map_err(|e| VideoStreamError::ProtocolError(e.to_string()))?;

        // 配置传输参数
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(100_u32.into());
        transport_config.max_concurrent_bidi_streams(10_u32.into());
        transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(300).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
        server_config.transport_config(Arc::new(transport_config));

        let endpoint = Endpoint::server(server_config, addr)
            .map_err(|e| VideoStreamError::QuicError(e.to_string()))?;

        Ok(Self {
            endpoint,
            device_manager,
            recording_manager,
            distribution_manager,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("QUIC server running...");

        while let Some(conn) = self.endpoint.accept().await {
            let device_manager = self.device_manager.clone();
            let recording_manager = self.recording_manager.clone();
            let distribution_manager = self.distribution_manager.clone();

            tokio::spawn(async move {
                match conn.await {
                    Ok(connection) => {
                        info!("New QUIC connection from: {}", connection.remote_address());
                        if let Err(e) = super::connection::handle_connection(
                            connection,
                            device_manager,
                            recording_manager,
                            distribution_manager,
                        )
                        .await
                        {
                            error!("Connection error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                    }
                }
            });
        }

        Ok(())
    }
}
