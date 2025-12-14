use std::path::{Path, PathBuf};
use tokio::fs::{File, metadata};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use sha2::{Sha256, Digest};
use std::time::SystemTime;

use crate::video::types::{TimelineFile, KeyframeIndex};
use crate::video::errors::TimelineError;

/// Timeline manager trait for managing timeline file caching
#[async_trait]
pub trait TimelineManager {
    /// Load timeline from file
    async fn load_timeline(&self, video_path: &Path) -> Result<Option<TimelineFile>, TimelineError>;
    
    /// Save timeline to file
    async fn save_timeline(&self, timeline: &TimelineFile) -> Result<(), TimelineError>;
    
    /// Validate timeline file against video file
    async fn validate_timeline(&self, timeline: &TimelineFile, video_path: &Path) -> Result<bool, TimelineError>;
    
    /// Delete timeline file
    async fn delete_timeline(&self, video_path: &Path) -> Result<(), TimelineError>;
    
    /// Get timeline file path for a video file
    fn get_timeline_path(&self, video_path: &Path) -> PathBuf;
}

/// Default implementation of TimelineManager
pub struct DefaultTimelineManager {
    /// Timeline file version
    version: u32,
}

impl DefaultTimelineManager {
    pub fn new() -> Self {
        Self {
            version: 1,
        }
    }

    /// Calculate SHA-256 hash of a file
    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String, TimelineError> {
        let mut file = File::open(file_path).await
            .map_err(|e| TimelineError::Io(e))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer).await
                .map_err(|e| TimelineError::Io(e))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Get file metadata
    async fn get_file_metadata(&self, file_path: &Path) -> Result<(u64, SystemTime), TimelineError> {
        let meta = metadata(file_path).await
            .map_err(|e| TimelineError::Io(e))?;
        
        let size = meta.len();
        let modified = meta.modified()
            .map_err(|e| TimelineError::Io(e))?;
        
        Ok((size, modified))
    }

    /// Check if timeline file exists and is newer than video file
    async fn is_timeline_valid_by_time(&self, video_path: &Path, timeline_path: &Path) -> bool {
        match (metadata(video_path).await, metadata(timeline_path).await) {
            (Ok(video_meta), Ok(timeline_meta)) => {
                match (video_meta.modified(), timeline_meta.modified()) {
                    (Ok(video_modified), Ok(timeline_modified)) => {
                        timeline_modified >= video_modified
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl Default for DefaultTimelineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TimelineManager for DefaultTimelineManager {
    async fn load_timeline(&self, video_path: &Path) -> Result<Option<TimelineFile>, TimelineError> {
        let timeline_path = self.get_timeline_path(video_path);
        
        // Check if timeline file exists
        if !timeline_path.exists() {
            return Ok(None);
        }
        
        // Check if timeline is newer than video file
        if !self.is_timeline_valid_by_time(video_path, &timeline_path).await {
            return Ok(None);
        }
        
        // Read timeline file
        let mut file = File::open(&timeline_path).await
            .map_err(|e| TimelineError::Io(e))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents).await
            .map_err(|e| TimelineError::Io(e))?;
        
        // Parse JSON
        let timeline: TimelineFile = serde_json::from_str(&contents)
            .map_err(|e| TimelineError::JsonSerialization(e))?;
        
        // Validate version
        if timeline.version != self.version {
            return Err(TimelineError::IncompatibleVersion { 
                version: timeline.version 
            });
        }
        
        Ok(Some(timeline))
    }

    async fn save_timeline(&self, timeline: &TimelineFile) -> Result<(), TimelineError> {
        let timeline_path = self.get_timeline_path(&timeline.video_file_path);
        
        // Serialize to JSON
        let json = serde_json::to_string_pretty(timeline)
            .map_err(|e| TimelineError::JsonSerialization(e))?;
        
        // Write to file
        let mut file = File::create(&timeline_path).await
            .map_err(|e| TimelineError::Io(e))?;
        
        file.write_all(json.as_bytes()).await
            .map_err(|e| TimelineError::Io(e))?;
        
        file.flush().await
            .map_err(|e| TimelineError::Io(e))?;
        
        Ok(())
    }

    async fn validate_timeline(&self, timeline: &TimelineFile, video_path: &Path) -> Result<bool, TimelineError> {
        // Check if video file exists
        if !video_path.exists() {
            return Err(TimelineError::NotFound { 
                path: video_path.to_path_buf() 
            });
        }
        
        // Get current file metadata
        let (current_size, current_modified) = self.get_file_metadata(video_path).await?;
        
        // Check file size
        if current_size != timeline.video_file_size {
            return Ok(false);
        }
        
        // Check modification time
        if current_modified > timeline.video_file_modified {
            return Ok(false);
        }
        
        // Calculate and check file hash
        let current_hash = self.calculate_file_hash(video_path).await?;
        if current_hash != timeline.video_file_hash {
            return Ok(false);
        }
        
        Ok(true)
    }

    async fn delete_timeline(&self, video_path: &Path) -> Result<(), TimelineError> {
        let timeline_path = self.get_timeline_path(video_path);
        
        if timeline_path.exists() {
            tokio::fs::remove_file(&timeline_path).await
                .map_err(|e| TimelineError::Io(e))?;
        }
        
        Ok(())
    }

    fn get_timeline_path(&self, video_path: &Path) -> PathBuf {
        let mut timeline_path = video_path.to_path_buf();
        timeline_path.set_extension("timeline");
        timeline_path
    }
}

/// Builder for creating TimelineFile instances
pub struct TimelineFileBuilder {
    video_file_path: PathBuf,
    keyframe_index: KeyframeIndex,
    ffmpeg_version: Option<String>,
}

impl TimelineFileBuilder {
    pub fn new(video_file_path: PathBuf, keyframe_index: KeyframeIndex) -> Self {
        Self {
            video_file_path,
            keyframe_index,
            ffmpeg_version: None,
        }
    }

    pub fn with_ffmpeg_version(mut self, version: String) -> Self {
        self.ffmpeg_version = Some(version);
        self
    }

    pub async fn build(self, manager: &DefaultTimelineManager) -> Result<TimelineFile, TimelineError> {
        // Get file metadata
        let (size, modified) = manager.get_file_metadata(&self.video_file_path).await?;
        
        // Calculate file hash
        let hash = manager.calculate_file_hash(&self.video_file_path).await?;
        
        // Extract video information from keyframe index
        let duration = self.keyframe_index.total_duration;
        let resolution = if !self.keyframe_index.entries.is_empty() {
            // Default resolution - should be extracted from actual video
            crate::video::types::Resolution {
                width: 1920,
                height: 1080,
            }
        } else {
            crate::video::types::Resolution {
                width: 0,
                height: 0,
            }
        };
        
        // Estimate frame rate from keyframe index
        let frame_rate = if self.keyframe_index.entries.len() > 1 {
            let time_span = self.keyframe_index.entries.last().unwrap().timestamp 
                - self.keyframe_index.entries.first().unwrap().timestamp;
            if time_span > 0.0 {
                (self.keyframe_index.entries.len() as f64 / time_span).min(60.0).max(15.0)
            } else {
                30.0
            }
        } else {
            30.0
        };
        
        Ok(TimelineFile {
            version: manager.version,
            video_file_path: self.video_file_path,
            video_file_hash: hash,
            video_file_size: size,
            video_file_modified: modified,
            duration,
            resolution,
            frame_rate,
            keyframe_index: self.keyframe_index,
            created_at: SystemTime::now(),
            ffmpeg_version: self.ffmpeg_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_timeline_path() {
        let manager = DefaultTimelineManager::new();
        let video_path = PathBuf::from("test_video.mp4");
        let timeline_path = manager.get_timeline_path(&video_path);
        
        assert_eq!(timeline_path, PathBuf::from("test_video.timeline"));
    }

    #[test]
    fn test_get_timeline_path_with_directory() {
        let manager = DefaultTimelineManager::new();
        let video_path = PathBuf::from("videos/test_video.h264");
        let timeline_path = manager.get_timeline_path(&video_path);
        
        assert_eq!(timeline_path, PathBuf::from("videos/test_video.timeline"));
    }

    #[tokio::test]
    async fn test_timeline_manager_creation() {
        let manager = DefaultTimelineManager::new();
        assert_eq!(manager.version, 1);
    }
}
