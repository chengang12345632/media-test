use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub quic_host: String,
    pub quic_port: u16,
    pub http3_host: String,
    pub http3_port: u16,
    pub storage_root: PathBuf,
    pub max_connections: usize,
    pub buffer_size: usize,
}

impl Config {
    pub fn load() -> Result<Self> {
        Ok(Self {
            quic_host: "0.0.0.0".to_string(),
            quic_port: 8443,  // QUIC端口
            http3_host: "0.0.0.0".to_string(),
            http3_port: 8080,  // HTTP端口
            storage_root: PathBuf::from("../device-simulator/test-videos"),
            max_connections: 1000,
            buffer_size: 1024 * 1024, // 1MB
        })
    }
}
