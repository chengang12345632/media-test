use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::latency::LatencyMonitor;
use crate::recording::RecordingManager;
use crate::streaming::UnifiedStreamHandler;
use common::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct Http3Server {
    addr: SocketAddr,
    device_manager: DeviceManager,
    recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
    latency_monitor: LatencyMonitor,
    stream_handler: Arc<UnifiedStreamHandler>,
}

impl Http3Server {
    pub fn new(
        addr: SocketAddr,
        device_manager: DeviceManager,
        recording_manager: RecordingManager,
        distribution_manager: DistributionManager,
        latency_monitor: LatencyMonitor,
    ) -> Self {
        Self {
            addr,
            device_manager,
            recording_manager,
            distribution_manager,
            latency_monitor,
            stream_handler: Arc::new(UnifiedStreamHandler::new()),
        }
    }
    
    /// 获取流处理器引用
    pub fn get_stream_handler(&self) -> Arc<UnifiedStreamHandler> {
        Arc::clone(&self.stream_handler)
    }

    pub async fn run(&self) -> Result<()> {
        info!("HTTP3 server running on {}", self.addr);

        let app = super::routes::create_router(
            self.device_manager.clone(),
            self.recording_manager.clone(),
            self.distribution_manager.clone(),
            self.latency_monitor.clone(),
            self.stream_handler.clone(),
        );

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
