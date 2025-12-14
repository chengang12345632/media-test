use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use async_trait::async_trait;
use crate::video::types::{
    VideoFileInfo, Resolution, KeyframeIndex, KeyframeEntry, FrameType, 
    IndexOptimizationStrategy, SeekResult
};
use crate::video::errors::FileError;
use std::time::Instant;

/// File stream reader trait for video file operations
#[async_trait]
pub trait FileStreamReader {
    /// Open a video file
    async fn open_file(&self, file_path: &Path) -> Result<File, FileError>;
    
    /// Read a chunk of data from the file
    async fn read_chunk(&self, handle: &mut File, size: usize) -> Result<Vec<u8>, FileError>;
    
    /// Get video file information
    async fn get_file_info(&self, handle: &mut File) -> Result<VideoFileInfo, FileError>;
    
    /// Seek to a specific file position
    async fn seek_to_position(&self, handle: &mut File, position: u64) -> Result<(), FileError>;
    
    /// Seek to a specific time using keyframe index
    async fn seek_to_time(
        &self, 
        handle: &mut File, 
        time_seconds: f64, 
        index: &KeyframeIndex
    ) -> Result<u64, FileError>;
    
    /// Build keyframe index for the video file
    async fn build_keyframe_index(&self, handle: &mut File) -> Result<KeyframeIndex, FileError>;
}

/// Default implementation of FileStreamReader
pub struct DefaultFileStreamReader;

impl DefaultFileStreamReader {
    pub fn new() -> Self {
        Self
    }

    /// Build keyframe index with specific optimization strategy
    pub async fn build_keyframe_index_with_strategy(
        &self, 
        handle: &mut File, 
        strategy: IndexOptimizationStrategy
    ) -> Result<KeyframeIndex, FileError> {
        self.build_keyframe_index_impl(handle, strategy).await
    }

    /// Build keyframe index with memory limit
    pub async fn build_keyframe_index_with_memory_limit(
        &self, 
        handle: &mut File, 
        memory_limit_mb: usize
    ) -> Result<KeyframeIndex, FileError> {
        // Choose strategy based on memory limit
        let strategy = if memory_limit_mb >= 50 {
            IndexOptimizationStrategy::Full
        } else if memory_limit_mb >= 20 {
            IndexOptimizationStrategy::Adaptive
        } else if memory_limit_mb >= 10 {
            IndexOptimizationStrategy::Sparse
        } else {
            IndexOptimizationStrategy::Hierarchical
        };
        
        self.build_keyframe_index_impl(handle, strategy).await
    }

    /// Check if the buffer contains H.264 format data
    fn is_h264_format(&self, buffer: &[u8]) -> Result<bool, FileError> {
        if buffer.len() < 4 {
            return Ok(false);
        }
        
        // Look for H.264 start codes (0x00000001 or 0x000001)
        for i in 0..buffer.len().saturating_sub(4) {
            // Check for 4-byte start code
            if buffer[i..i+4] == [0x00, 0x00, 0x00, 0x01] {
                if i + 4 < buffer.len() {
                    let nal_type = buffer[i + 4] & 0x1F;
                    // Common H.264 NAL unit types: SPS (7), PPS (8), IDR (5), Non-IDR (1)
                    if matches!(nal_type, 1 | 5 | 7 | 8) {
                        return Ok(true);
                    }
                }
            }
            // Check for 3-byte start code
            if i + 3 < buffer.len() && buffer[i..i+3] == [0x00, 0x00, 0x01] {
                if i + 3 < buffer.len() {
                    let nal_type = buffer[i + 3] & 0x1F;
                    if matches!(nal_type, 1 | 5 | 7 | 8) {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Extract metadata from H.264 file
    async fn extract_h264_info(&self, handle: &mut File) -> Result<VideoFileInfo, FileError> {
        let _file_size = handle.metadata().await?.len();
        
        // Default resolution
        let resolution = Resolution {
            width: 1920,
            height: 1080,
        };
        
        // Calculate duration based on keyframe analysis
        let duration_info = self.calculate_h264_duration(handle).await?;
        
        Ok(VideoFileInfo {
            duration: duration_info.duration,
            resolution,
            codec: "h264".to_string(),
            frame_rate: duration_info.estimated_frame_rate,
            bit_rate: duration_info.estimated_bitrate,
            has_audio: false, // Raw H.264 typically doesn't have audio
        })
    }

    /// Calculate H.264 duration based on keyframe analysis
    async fn calculate_h264_duration(&self, handle: &mut File) -> Result<H264DurationInfo, FileError> {
        let current_pos = handle.stream_position().await?;
        let file_size = handle.metadata().await?.len();
        
        // Seek to beginning for analysis
        handle.seek(SeekFrom::Start(0)).await?;
        
        let mut keyframe_count = 0u32;
        let mut total_frames = 0u32;
        let mut bytes_analyzed = 0u64;
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        
        // Analyze up to 10MB of the file
        let max_analysis_size = (10 * 1024 * 1024).min(file_size);
        
        while bytes_analyzed < max_analysis_size {
            let bytes_read = handle.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            
            // Count keyframes and total frames
            let frame_info = self.analyze_h264_frames_in_buffer(&buffer[..bytes_read]);
            keyframe_count += frame_info.keyframes;
            total_frames += frame_info.total_frames;
            
            bytes_analyzed += bytes_read as u64;
        }
        
        // Restore original position
        handle.seek(SeekFrom::Start(current_pos)).await?;
        
        // Calculate duration (assume 1 keyframe per second)
        let duration = if keyframe_count > 0 {
            keyframe_count as f64
        } else {
            // Fallback: estimate based on file size
            let estimated_bitrate = self.estimate_h264_bitrate(file_size);
            (file_size as f64 * 8.0) / estimated_bitrate
        };
        
        // Estimate frame rate
        let estimated_frame_rate = if duration > 0.0 && total_frames > 0 {
            let analysis_ratio = bytes_analyzed as f64 / file_size as f64;
            let estimated_total_frames = total_frames as f64 / analysis_ratio;
            (estimated_total_frames / duration).min(60.0).max(15.0)
        } else {
            30.0
        };
        
        // Calculate bitrate
        let estimated_bitrate = if duration > 0.0 {
            ((file_size as f64 * 8.0) / duration).max(500_000.0) as u64
        } else {
            2_000_000
        };
        
        Ok(H264DurationInfo {
            duration: duration.max(0.1),
            estimated_frame_rate,
            estimated_bitrate,
            keyframe_count,
            total_frames,
        })
    }

    /// Analyze H.264 frames in a buffer
    fn analyze_h264_frames_in_buffer(&self, buffer: &[u8]) -> H264FrameAnalysis {
        let mut keyframes = 0u32;
        let mut total_frames = 0u32;
        
        let mut i = 0;
        while i < buffer.len().saturating_sub(5) {
            // Look for NAL unit start codes
            if buffer[i..i+4] == [0x00, 0x00, 0x00, 0x01] {
                if i + 4 < buffer.len() {
                    let nal_type = buffer[i + 4] & 0x1F;
                    match nal_type {
                        5 => {
                            keyframes += 1;
                            total_frames += 1;
                        },
                        1 => {
                            total_frames += 1;
                        },
                        _ => {}
                    }
                }
                i += 4;
            } else if buffer[i..i+3] == [0x00, 0x00, 0x01] {
                if i + 3 < buffer.len() {
                    let nal_type = buffer[i + 3] & 0x1F;
                    match nal_type {
                        5 => {
                            keyframes += 1;
                            total_frames += 1;
                        },
                        1 => {
                            total_frames += 1;
                        },
                        _ => {}
                    }
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        
        H264FrameAnalysis {
            keyframes,
            total_frames,
        }
    }

    /// Estimate H.264 bitrate based on file size
    fn estimate_h264_bitrate(&self, file_size: u64) -> f64 {
        match file_size {
            0..=1_000_000 => 500_000.0,
            1_000_001..=10_000_000 => 1_500_000.0,
            10_000_001..=100_000_000 => 3_000_000.0,
            100_000_001..=500_000_000 => 5_000_000.0,
            _ => 8_000_000.0,
        }
    }

    /// Build keyframe index implementation
    async fn build_keyframe_index_impl(
        &self, 
        handle: &mut File, 
        strategy: IndexOptimizationStrategy
    ) -> Result<KeyframeIndex, FileError> {
        let current_pos = handle.stream_position().await?;
        let file_size = handle.metadata().await?.len();
        
        // Seek to beginning
        handle.seek(SeekFrom::Start(0)).await?;
        
        let mut entries = Vec::new();
        let mut current_offset = 0u64;
        let mut memory_usage = 0usize;
        
        // Determine buffer size based on strategy
        let buffer_size = match strategy {
            IndexOptimizationStrategy::Full => 64 * 1024,
            IndexOptimizationStrategy::Sparse => 128 * 1024,
            IndexOptimizationStrategy::Adaptive => 96 * 1024,
            IndexOptimizationStrategy::Hierarchical => 32 * 1024,
        };
        
        let mut buffer = vec![0u8; buffer_size];
        let mut frame_count = 0u64;
        let estimated_frame_rate = 30.0;
        
        // Memory limit (10MB)
        let memory_limit = 10 * 1024 * 1024;
        
        while current_offset < file_size {
            let bytes_read = handle.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            
            // Find keyframes in current buffer
            let keyframes = self.find_keyframes_in_buffer(&buffer[..bytes_read], current_offset)?;
            
            for keyframe in keyframes {
                let timestamp = frame_count as f64 / estimated_frame_rate;
                
                // Apply optimization strategy
                let should_include = match strategy {
                    IndexOptimizationStrategy::Full => true,
                    IndexOptimizationStrategy::Sparse => frame_count % 30 == 0,
                    IndexOptimizationStrategy::Adaptive => {
                        memory_usage < memory_limit || frame_count % 60 == 0
                    },
                    IndexOptimizationStrategy::Hierarchical => {
                        timestamp < 10.0 || frame_count % 30 == 0
                    },
                };
                
                if should_include {
                    let entry = KeyframeEntry {
                        timestamp,
                        file_offset: keyframe.offset,
                        frame_size: keyframe.size,
                        gop_size: keyframe.gop_size,
                        frame_type: FrameType::I,
                    };
                    
                    memory_usage += std::mem::size_of::<KeyframeEntry>();
                    entries.push(entry);
                }
                
                frame_count += 1;
                
                if matches!(strategy, IndexOptimizationStrategy::Adaptive) && memory_usage > memory_limit {
                    break;
                }
            }
            
            current_offset += bytes_read as u64;
            
            // Limit scanning to 100MB
            if current_offset > 100 * 1024 * 1024 {
                break;
            }
        }
        
        // Restore original position
        handle.seek(SeekFrom::Start(current_pos)).await?;
        
        // Calculate index precision
        let index_precision = if entries.len() > 1 {
            let time_span = entries.last().unwrap().timestamp - entries.first().unwrap().timestamp;
            (time_span / entries.len() as f64).max(1.0 / estimated_frame_rate)
        } else {
            1.0 / estimated_frame_rate
        };
        
        // Calculate duration
        let final_duration = if !entries.is_empty() {
            let keyframe_based_duration = entries.len() as f64;
            let timestamp_based_duration = entries.last().unwrap().timestamp + (1.0 / estimated_frame_rate);
            keyframe_based_duration.max(timestamp_based_duration).max(1.0)
        } else {
            let estimated_bitrate = match file_size {
                0..=10_000_000 => 1_500_000.0,
                10_000_001..=100_000_000 => 3_000_000.0,
                _ => 5_000_000.0,
            };
            ((file_size as f64 * 8.0) / estimated_bitrate).max(1.0)
        };
        
        Ok(KeyframeIndex {
            entries,
            total_duration: final_duration,
            index_precision,
            memory_optimized: !matches!(strategy, IndexOptimizationStrategy::Full),
            optimization_strategy: strategy,
            memory_usage,
        })
    }

    /// Find keyframes in a buffer
    fn find_keyframes_in_buffer(&self, buffer: &[u8], base_offset: u64) -> Result<Vec<KeyframeInfo>, FileError> {
        let mut keyframes = Vec::new();
        
        for i in 0..buffer.len().saturating_sub(5) {
            // Check for 4-byte start code
            if buffer[i..i+4] == [0x00, 0x00, 0x00, 0x01] {
                if i + 4 < buffer.len() {
                    let nal_type = buffer[i + 4] & 0x1F;
                    if nal_type == 5 {
                        let keyframe = KeyframeInfo {
                            offset: base_offset + i as u64,
                            size: self.estimate_frame_size(&buffer[i..]).unwrap_or(1024),
                            gop_size: 1,
                        };
                        keyframes.push(keyframe);
                    }
                }
            }
            // Check for 3-byte start code
            else if buffer[i..i+3] == [0x00, 0x00, 0x01] {
                if i + 3 < buffer.len() {
                    let nal_type = buffer[i + 3] & 0x1F;
                    if nal_type == 5 {
                        let keyframe = KeyframeInfo {
                            offset: base_offset + i as u64,
                            size: self.estimate_frame_size(&buffer[i..]).unwrap_or(1024),
                            gop_size: 1,
                        };
                        keyframes.push(keyframe);
                    }
                }
            }
        }
        
        Ok(keyframes)
    }

    /// Estimate frame size from buffer
    fn estimate_frame_size(&self, buffer: &[u8]) -> Option<u32> {
        for i in 4..buffer.len().saturating_sub(4) {
            if buffer[i..i+4] == [0x00, 0x00, 0x00, 0x01] || 
               (i + 3 <= buffer.len() && buffer[i..i+3] == [0x00, 0x00, 0x01]) {
                return Some(i as u32);
            }
        }
        None
    }

    /// Find nearest keyframe entry (binary search)
    fn find_nearest_keyframe<'a>(&self, timestamp: f64, index: &'a KeyframeIndex) -> Option<&'a KeyframeEntry> {
        if index.entries.is_empty() {
            return None;
        }
        
        // Binary search for the nearest keyframe at or before the timestamp
        let mut left = 0;
        let mut right = index.entries.len();
        
        while left < right {
            let mid = (left + right) / 2;
            if index.entries[mid].timestamp <= timestamp {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        // Return the keyframe at or before the timestamp
        if left > 0 {
            Some(&index.entries[left - 1])
        } else {
            Some(&index.entries[0])
        }
    }

    /// Seek to time with detailed result
    pub async fn seek_to_time_with_result(
        &self, 
        handle: &mut File, 
        time_seconds: f64, 
        index: &KeyframeIndex
    ) -> Result<SeekResult, FileError> {
        let start_time = Instant::now();
        
        // Validate input
        if time_seconds < 0.0 {
            return Err(FileError::InvalidSeekPosition);
        }
        
        if time_seconds > index.total_duration {
            return Err(FileError::SeekBeyondEnd);
        }
        
        // Find nearest keyframe
        if let Some(keyframe) = self.find_nearest_keyframe(time_seconds, index) {
            // Seek to keyframe position
            handle.seek(SeekFrom::Start(keyframe.file_offset)).await?;
            
            // Verify seek
            let actual_position = handle.stream_position().await?;
            if actual_position != keyframe.file_offset {
                return Err(FileError::SeekFailed);
            }
            
            let execution_time = start_time.elapsed();
            let precision_achieved = (time_seconds - keyframe.timestamp).abs();
            
            Ok(SeekResult {
                requested_time: time_seconds,
                actual_time: keyframe.timestamp,
                keyframe_offset: keyframe.file_offset,
                precision_achieved,
                keyframe_used: keyframe.clone(),
                execution_time,
            })
        } else {
            // Fallback to beginning
            handle.seek(SeekFrom::Start(0)).await?;
            let execution_time = start_time.elapsed();
            
            let beginning_keyframe = KeyframeEntry {
                timestamp: 0.0,
                file_offset: 0,
                frame_size: 0,
                gop_size: 0,
                frame_type: FrameType::I,
            };
            
            Ok(SeekResult {
                requested_time: time_seconds,
                actual_time: 0.0,
                keyframe_offset: 0,
                precision_achieved: time_seconds,
                keyframe_used: beginning_keyframe,
                execution_time,
            })
        }
    }
}

/// Helper structures
#[derive(Debug, Clone)]
struct KeyframeInfo {
    offset: u64,
    size: u32,
    gop_size: u32,
}

#[derive(Debug, Clone)]
struct H264DurationInfo {
    duration: f64,
    estimated_frame_rate: f64,
    estimated_bitrate: u64,
    keyframe_count: u32,
    total_frames: u32,
}

#[derive(Debug, Clone)]
struct H264FrameAnalysis {
    keyframes: u32,
    total_frames: u32,
}

#[async_trait]
impl FileStreamReader for DefaultFileStreamReader {
    async fn open_file(&self, file_path: &Path) -> Result<File, FileError> {
        match File::open(file_path).await {
            Ok(file) => Ok(file),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Err(FileError::FileNotFound {
                    path: file_path.to_path_buf(),
                }),
                std::io::ErrorKind::PermissionDenied => Err(FileError::PermissionDenied),
                _ => Err(FileError::Io(e)),
            }
        }
    }

    async fn read_chunk(&self, handle: &mut File, size: usize) -> Result<Vec<u8>, FileError> {
        let chunk_size = size.min(1024 * 1024); // Max 1MB chunks
        let mut buffer = vec![0u8; chunk_size];
        let bytes_read = handle.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    async fn get_file_info(&self, handle: &mut File) -> Result<VideoFileInfo, FileError> {
        let original_position = handle.stream_position().await?;
        
        // Seek to beginning
        handle.seek(SeekFrom::Start(0)).await?;
        
        // Read initial chunk
        let mut buffer = vec![0u8; 8192];
        let bytes_read = handle.read(&mut buffer).await?;
        
        if bytes_read < 32 {
            return Err(FileError::CorruptedFile);
        }
        
        // Check if H.264 format
        let file_info = if self.is_h264_format(&buffer[..bytes_read])? {
            self.extract_h264_info(handle).await?
        } else {
            return Err(FileError::UnsupportedFormat { 
                format: "unknown".to_string() 
            });
        };
        
        // Restore position
        handle.seek(SeekFrom::Start(original_position)).await?;
        
        Ok(file_info)
    }

    async fn seek_to_position(&self, handle: &mut File, position: u64) -> Result<(), FileError> {
        handle.seek(SeekFrom::Start(position)).await?;
        Ok(())
    }

    async fn seek_to_time(
        &self, 
        handle: &mut File, 
        time_seconds: f64, 
        index: &KeyframeIndex
    ) -> Result<u64, FileError> {
        // Validate input
        if time_seconds < 0.0 {
            return Err(FileError::InvalidSeekPosition);
        }
        
        if time_seconds > index.total_duration {
            return Err(FileError::SeekBeyondEnd);
        }
        
        // Find nearest keyframe
        if let Some(keyframe) = self.find_nearest_keyframe(time_seconds, index) {
            handle.seek(SeekFrom::Start(keyframe.file_offset)).await?;
            
            let actual_position = handle.stream_position().await?;
            if actual_position != keyframe.file_offset {
                return Err(FileError::SeekFailed);
            }
            
            Ok(keyframe.file_offset)
        } else {
            handle.seek(SeekFrom::Start(0)).await?;
            Ok(0)
        }
    }

    async fn build_keyframe_index(&self, handle: &mut File) -> Result<KeyframeIndex, FileError> {
        self.build_keyframe_index_impl(handle, IndexOptimizationStrategy::Adaptive).await
    }
}
