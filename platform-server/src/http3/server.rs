use crate::device::DeviceManager;
use crate::distribution::DistributionManager;
use crate::latency::LatencyMonitor;
use crate::recording::RecordingManager;
use common::Result;
use std::net::SocketAddr;
use tracing::info;

#[derive(Clone)]
pub struct Http3Server {
    addr: SocketAddr,
    device_manager: DeviceManager,
    recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
    latency_monitor: LatencyMonitor,
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
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("HTTP3 server running on {}", self.addr);

        // 创建统一流处理器
        let stream_handler = std::sync::Arc::new(
            crate::streaming::UnifiedStreamHandler::new()
        );

        let app = super::routes::create_router(
            self.device_manager.clone(),
            self.recording_manager.clone(),
            self.distribution_manager.clone(),
            self.latency_monitor.clone(),
            stream_handler,
        );

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
