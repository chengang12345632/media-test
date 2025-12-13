mod config;
mod quic;
mod video;
mod uploader;
mod device_service;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志 - 使用环境变量 RUST_LOG 控制级别
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .init();

    info!("🎥 Device simulator starting...");

    // 加载配置
    let config = config::Config::load()?;
    info!("✓ Configuration loaded");
    info!("  Device ID: {}", config.device_id);
    info!("  Device Name: {}", config.device_name);

    // 连接到平台
    info!("Connecting to platform: {}:{}", config.platform_host, config.platform_port);
    let mut client = quic::QuicClient::new(config.clone()).await?;
    
    // 尝试初始连接，失败也不退出
    match client.connect().await {
        Ok(_) => {
            info!("✓ QUIC connection established");
        }
        Err(e) => {
            info!("⚠️  Initial connection failed: {}", e);
            info!("   Will retry in background...");
        }
    }

    // 扫描测试视频
    let video_files = video::scan_video_files(&config.video_dir)?;
    info!("✓ Found {} test video(s)", video_files.len());

    if video_files.is_empty() {
        info!("⚠️  No test videos found in {:?}", config.video_dir);
        info!("   Please add .h264 or .mp4 files to the test-videos directory");
        return Ok(());
    }

    info!("✓ Device service initialized");
    info!("✅ Device simulator ready!");
    info!("   Press Ctrl+C to stop");

    // 启动设备服务（支持重连、录像列表查询、回放）
    let video_dir = config.video_dir.clone();
    let service = device_service::DeviceService::new(client, video_files, config.device_id, video_dir);
    service.run().await?;

    Ok(())
}
