use common::{RecordingInfo, VideoStreamError, Result};
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use super::scanner::RecordingScanner;

#[derive(Clone)]
pub struct RecordingManager {
    storage_root: PathBuf,
    cache: Arc<DashMap<String, RecordingInfo>>,
    scanner: Arc<RecordingScanner>,
}

impl RecordingManager {
    pub fn new(storage_root: PathBuf) -> Self {
        info!("Initializing recording manager at: {:?}", storage_root);
        
        let scanner = Arc::new(RecordingScanner::new(storage_root.clone()));
        
        Self {
            storage_root,
            cache: Arc::new(DashMap::new()),
            scanner,
        }
    }

    /// 扫描设备录像
    pub async fn scan_device_recordings(&self, device_id: &str) -> Result<Vec<RecordingInfo>> {
        info!("Scanning recordings for device: {}", device_id);
        self.scanner.scan_device_recordings(device_id).await
    }

    /// 获取录像信息
    pub fn get_recording(&self, file_id: &str) -> Result<RecordingInfo> {
        self.cache
            .get(file_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| VideoStreamError::RecordingNotFound(file_id.to_string()))
    }

    /// 添加录像到缓存
    pub fn add_to_cache(&self, recording: RecordingInfo) {
        self.cache.insert(recording.file_id.clone(), recording);
    }

    /// 获取录像文件路径
    pub fn get_recording_path(&self, device_id: &str, file_name: &str) -> PathBuf {
        self.storage_root.join(device_id).join(file_name)
    }
}
