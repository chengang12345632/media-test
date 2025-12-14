mod config;
mod device;
mod distribution;
mod http3;
mod latency;
mod protocol;
mod quic;
mod recording;
mod streaming;

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .init();

    info!("🚀 Platform server starting...");

    // 加载配置
    let config = config::Config::load()?;
    info!("✓ Configuration loaded");

    // 创建共享状态
    let device_manager = device::DeviceManager::new();
    let recording_manager = recording::RecordingManager::new(config.storage_root.clone());
    let distribution_manager = distribution::DistributionManager::new();
    let latency_monitor = latency::LatencyMonitor::new();

    info!("✓ Managers initialized");

    // 启动QUIC服务器
    let quic_addr = format!("{}:{}", config.quic_host, config.quic_port);
    let quic_server = quic::QuicServer::new(
        quic_addr.parse()?,
        device_manager.clone(),
        recording_manager.clone(),
        distribution_manager.clone(),
    )?;

    info!("✓ QUIC server listening on {}", quic_addr);

    // 启动HTTP3服务器
    let http3_addr = format!("{}:{}", config.http3_host, config.http3_port);
    let http3_server = http3::Http3Server::new(
        http3_addr.parse()?,
        device_manager.clone(),
        recording_manager.clone(),
        distribution_manager.clone(),
        latency_monitor.clone(),
    );

    info!("✓ HTTP3 server listening on {}", http3_addr);

    // 启动延迟监控统计更新任务
    let stream_handler_for_stats = http3_server.get_stream_handler();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // 获取所有活动会话
            let sessions = stream_handler_for_stats.get_active_sessions();
            
            // 为每个会话广播统计更新
            for session_id in sessions {
                if let Some(statistics) = stream_handler_for_stats
                    .get_stats_manager()
                    .get_statistics(&session_id) 
                {
                    stream_handler_for_stats
                        .get_alert_broadcaster()
                        .broadcast_statistics_update(session_id, statistics);
                }
            }
        }
    });
    
    info!("✓ Latency monitoring statistics update task started");

    info!("✅ Platform server ready!");

    // 并发运行两个服务器
    let quic_handle = tokio::spawn(async move {
        if let Err(e) = quic_server.run().await {
            tracing::error!("QUIC server error: {}", e);
        }
    });

    let http3_handle = tokio::spawn(async move {
        if let Err(e) = http3_server.run().await {
            tracing::error!("HTTP3 server error: {}", e);
        }
    });

    // 等待两个服务器
    let _ = tokio::try_join!(quic_handle, http3_handle);

    Ok(())
}
