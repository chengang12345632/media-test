use anyhow::Result;
use std::path::PathBuf;
use crate::video::IndexOptimizationStrategy;

#[derive(Debug, Clone)]
pub struct Config {
    pub device_id: String,
    pub device_name: String,
    pub platform_host: String,
    pub platform_port: u16,
    pub video_dir: PathBuf,
    
    // 关键帧索引配置
    pub keyframe_index_strategy: IndexOptimizationStrategy,
    pub keyframe_index_memory_limit_mb: usize,
    
    // Timeline 缓存配置
    pub timeline_cache_enabled: bool,
    
    // FFmpeg 配置
    pub ffmpeg_enabled: bool,
    pub ffmpeg_path: Option<PathBuf>,
    pub ffmpeg_timeout_seconds: u64,
    
    // 播放控制配置
    pub playback_speed_min: f32,
    pub playback_speed_max: f32,
}

impl Config {
    pub fn load() -> Result<Self> {
        Ok(Self {
            device_id: "device_001".to_string(),
            device_name: "模拟摄像头-01".to_string(),
            platform_host: "127.0.0.1".to_string(),
            platform_port: 8443,
            video_dir: PathBuf::from("./test-videos"),
            
            // 关键帧索引配置（默认值）
            keyframe_index_strategy: IndexOptimizationStrategy::Adaptive,
            keyframe_index_memory_limit_mb: 50,
            
            // Timeline 缓存配置（默认启用）
            timeline_cache_enabled: true,
            
            // FFmpeg 配置（默认启用，自动检测路径）
            ffmpeg_enabled: true,
            ffmpeg_path: None, // None 表示自动检测
            ffmpeg_timeout_seconds: 30,
            
            // 播放控制配置（默认范围 0.25x - 4.0x）
            playback_speed_min: 0.25,
            playback_speed_max: 4.0,
        })
    }
    
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证播放速率范围
        if self.playback_speed_min <= 0.0 {
            anyhow::bail!("playback_speed_min must be greater than 0");
        }
        
        if self.playback_speed_max <= self.playback_speed_min {
            anyhow::bail!("playback_speed_max must be greater than playback_speed_min");
        }
        
        // 验证内存限制
        if self.keyframe_index_memory_limit_mb == 0 {
            anyhow::bail!("keyframe_index_memory_limit_mb must be greater than 0");
        }
        
        // 验证 FFmpeg 超时
        if self.ffmpeg_timeout_seconds == 0 {
            anyhow::bail!("ffmpeg_timeout_seconds must be greater than 0");
        }
        
        // 验证视频目录存在
        if !self.video_dir.exists() {
            anyhow::bail!("video_dir does not exist: {:?}", self.video_dir);
        }
        
        Ok(())
    }
    
    /// 从环境变量加载配置（可选）
    pub fn from_env() -> Result<Self> {
        let mut config = Self::load()?;
        
        // 从环境变量覆盖配置
        if let Ok(device_id) = std::env::var("DEVICE_ID") {
            config.device_id = device_id;
        }
        
        if let Ok(device_name) = std::env::var("DEVICE_NAME") {
            config.device_name = device_name;
        }
        
        if let Ok(platform_host) = std::env::var("PLATFORM_HOST") {
            config.platform_host = platform_host;
        }
        
        if let Ok(platform_port) = std::env::var("PLATFORM_PORT") {
            config.platform_port = platform_port.parse()?;
        }
        
        if let Ok(video_dir) = std::env::var("VIDEO_DIR") {
            config.video_dir = PathBuf::from(video_dir);
        }
        
        // 关键帧索引配置
        if let Ok(strategy) = std::env::var("KEYFRAME_INDEX_STRATEGY") {
            config.keyframe_index_strategy = match strategy.to_lowercase().as_str() {
                "full" => IndexOptimizationStrategy::Full,
                "sparse" => IndexOptimizationStrategy::Sparse,
                "adaptive" => IndexOptimizationStrategy::Adaptive,
                "hierarchical" => IndexOptimizationStrategy::Hierarchical,
                _ => IndexOptimizationStrategy::Adaptive,
            };
        }
        
        if let Ok(memory_limit) = std::env::var("KEYFRAME_INDEX_MEMORY_LIMIT_MB") {
            config.keyframe_index_memory_limit_mb = memory_limit.parse()?;
        }
        
        // Timeline 缓存配置
        if let Ok(enabled) = std::env::var("TIMELINE_CACHE_ENABLED") {
            config.timeline_cache_enabled = enabled.parse()?;
        }
        
        // FFmpeg 配置
        if let Ok(enabled) = std::env::var("FFMPEG_ENABLED") {
            config.ffmpeg_enabled = enabled.parse()?;
        }
        
        if let Ok(path) = std::env::var("FFMPEG_PATH") {
            config.ffmpeg_path = Some(PathBuf::from(path));
        }
        
        if let Ok(timeout) = std::env::var("FFMPEG_TIMEOUT_SECONDS") {
            config.ffmpeg_timeout_seconds = timeout.parse()?;
        }
        
        // 播放控制配置
        if let Ok(min_speed) = std::env::var("PLAYBACK_SPEED_MIN") {
            config.playback_speed_min = min_speed.parse()?;
        }
        
        if let Ok(max_speed) = std::env::var("PLAYBACK_SPEED_MAX") {
            config.playback_speed_max = max_speed.parse()?;
        }
        
        // 验证配置
        config.validate()?;
        
        Ok(config)
    }
    
    /// 打印配置信息
    pub fn print_info(&self) {
        use tracing::info;
        
        info!("=== Device Configuration ===");
        info!("Device ID: {}", self.device_id);
        info!("Device Name: {}", self.device_name);
        info!("Platform: {}:{}", self.platform_host, self.platform_port);
        info!("Video Directory: {:?}", self.video_dir);
        info!("");
        info!("=== Keyframe Index Configuration ===");
        info!("Strategy: {:?}", self.keyframe_index_strategy);
        info!("Memory Limit: {} MB", self.keyframe_index_memory_limit_mb);
        info!("");
        info!("=== Timeline Cache Configuration ===");
        info!("Enabled: {}", self.timeline_cache_enabled);
        info!("");
        info!("=== FFmpeg Configuration ===");
        info!("Enabled: {}", self.ffmpeg_enabled);
        info!("Path: {:?}", self.ffmpeg_path.as_ref().map(|p| p.to_string_lossy()).unwrap_or("auto-detect".into()));
        info!("Timeout: {}s", self.ffmpeg_timeout_seconds);
        info!("");
        info!("=== Playback Control Configuration ===");
        info!("Speed Range: {}x - {}x", self.playback_speed_min, self.playback_speed_max);
        info!("============================");
    }
}
