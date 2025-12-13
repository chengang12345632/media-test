use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub device_id: String,
    pub device_name: String,
    pub platform_host: String,
    pub platform_port: u16,
    pub video_dir: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        Ok(Self {
            device_id: "device_001".to_string(),
            device_name: "模拟摄像头-01".to_string(),
            platform_host: "127.0.0.1".to_string(),
            platform_port: 8443,
            video_dir: PathBuf::from("./test-videos"),
        })
    }
}
