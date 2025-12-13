use common::{RecordingInfo, Result};
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, warn};
use walkdir::WalkDir;

pub struct RecordingScanner {
    storage_root: PathBuf,
}

impl RecordingScanner {
    pub fn new(storage_root: PathBuf) -> Self {
        Self { storage_root }
    }

    pub async fn scan_device_recordings(&self, device_id: &str) -> Result<Vec<RecordingInfo>> {
        let mut recordings = Vec::new();
        
        // 尝试扫描设备专用目录
        let device_path = self.storage_root.join(device_id);
        debug!("Checking device path: {:?}", device_path);
        if device_path.exists() {
            debug!("Device path exists, scanning...");
            self.scan_directory(&device_path, device_id, &mut recordings).await;
        }
        
        // 如果设备专用目录不存在，扫描根目录（用于开发/测试）
        if recordings.is_empty() {
            debug!("No recordings in device path, checking root: {:?}", self.storage_root);
            if self.storage_root.exists() {
                debug!("Root directory exists, scanning...");
                self.scan_directory(&self.storage_root, device_id, &mut recordings).await;
            } else {
                warn!("Storage root does not exist: {:?}", self.storage_root);
            }
        }

        debug!("Found {} recordings for device {}", recordings.len(), device_id);
        recordings.sort_by(|a, b| b.created_time.cmp(&a.created_time));
        Ok(recordings)
    }
    
    async fn scan_directory(&self, path: &std::path::Path, device_id: &str, recordings: &mut Vec<RecordingInfo>) {
        debug!("Scanning directory: {:?}", path);
        let mut file_count = 0;
        for entry in WalkDir::new(path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            file_count += 1;
            let file_path = entry.path();
            debug!("Found file: {:?}", file_path);
            
            if let Some(ext) = file_path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                debug!("File extension: {}", ext_str);
                if ext_str == "h264" || ext_str == "mp4" || ext_str == "264" {
                    debug!("Parsing video file: {:?}", file_path);
                    match self.parse_recording(device_id, file_path).await {
                        Ok(info) => {
                            debug!("Successfully parsed: {}", info.file_name);
                            recordings.push(info);
                        }
                        Err(e) => warn!("Failed to parse recording {:?}: {}", file_path, e),
                    }
                }
            }
        }
        debug!("Scanned {} files in {:?}", file_count, path);
    }

    async fn parse_recording(&self, device_id: &str, path: &std::path::Path) -> Result<RecordingInfo> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_id = format!("{}_{}", device_id, file_name);
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(RecordingInfo {
            file_id,
            device_id: device_id.to_string(),
            file_name,
            file_path: path.to_string_lossy().to_string(),
            file_size: metadata.len(),
            duration: 0.0, // 需要解析视频文件获取
            format,
            resolution: "1920x1080".to_string(), // 默认值
            bitrate: 5000000,
            frame_rate: 30.0,
            created_time: metadata.created().unwrap_or(SystemTime::now()),
            modified_time: metadata.modified().unwrap_or(SystemTime::now()),
        })
    }
}
