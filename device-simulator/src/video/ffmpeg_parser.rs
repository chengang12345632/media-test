use crate::video::errors::FFmpegError;
use crate::video::types::{FFmpegConfig, FFmpegVideoInfo, Resolution};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};

// ============================================================================
// FFmpeg Parser Trait
// ============================================================================

/// FFmpeg parser trait for video file analysis
#[async_trait]
pub trait FFmpegParser: Send + Sync {
    /// Check if FFmpeg is available on the system
    async fn check_availability(&self) -> Result<bool, FFmpegError>;

    /// Get FFmpeg version information
    async fn get_version(&self) -> Result<String, FFmpegError>;

    /// Extract video metadata from file
    async fn extract_metadata(&self, video_path: &Path) -> Result<FFmpegVideoInfo, FFmpegError>;

    /// Extract keyframe timestamps from video
    async fn extract_keyframes(&self, video_path: &Path) -> Result<Vec<f64>, FFmpegError>;

    /// Validate video file format
    async fn validate_video(&self, video_path: &Path) -> Result<bool, FFmpegError>;
}

// ============================================================================
// Default FFmpeg Parser Implementation
// ============================================================================

/// Default implementation of FFmpeg parser
pub struct DefaultFFmpegParser {
    config: FFmpegConfig,
}

impl DefaultFFmpegParser {
    /// Create a new FFmpeg parser with default configuration
    pub fn new() -> Self {
        Self {
            config: FFmpegConfig {
                ffmpeg_path: "ffmpeg".into(),
                ffprobe_path: "ffprobe".into(),
                timeout: Duration::from_secs(30),
                min_version: "4.0".to_string(),
            },
        }
    }

    /// Create a new FFmpeg parser with custom configuration
    pub fn with_config(config: FFmpegConfig) -> Self {
        Self { config }
    }

    /// Extract version string from FFmpeg output
    fn extract_version_string(&self, output: &str) -> Result<String, FFmpegError> {
        if let Some(line) = output.lines().next() {
            if line.starts_with("ffmpeg version") {
                return Ok(line.to_string());
            }
        }
        Err(FFmpegError::ParseError {
            reason: "Could not parse FFmpeg version".to_string(),
        })
    }

    /// Extract duration from FFmpeg output
    fn extract_duration(&self, output: &str) -> Result<f64, FFmpegError> {
        for line in output.lines() {
            if line.contains("Duration:") {
                if let Some(duration_start) = line.find("Duration: ") {
                    let duration_part = &line[duration_start + 10..];
                    if let Some(comma_pos) = duration_part.find(',') {
                        let time_str = &duration_part[..comma_pos].trim();
                        return self.parse_time_to_seconds(time_str);
                    }
                }
            }
        }
        Err(FFmpegError::ParseError {
            reason: "Could not parse video duration".to_string(),
        })
    }

    /// Parse time string (HH:MM:SS.ss) to seconds
    fn parse_time_to_seconds(&self, time_str: &str) -> Result<f64, FFmpegError> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return Err(FFmpegError::ParseError {
                reason: format!("Invalid time format: {}", time_str),
            });
        }

        let hours: f64 = parts[0].parse().map_err(|_| FFmpegError::ParseError {
            reason: format!("Invalid hours: {}", parts[0]),
        })?;
        let minutes: f64 = parts[1].parse().map_err(|_| FFmpegError::ParseError {
            reason: format!("Invalid minutes: {}", parts[1]),
        })?;
        let seconds: f64 = parts[2].parse().map_err(|_| FFmpegError::ParseError {
            reason: format!("Invalid seconds: {}", parts[2]),
        })?;

        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    }

    /// Extract resolution and frame rate from FFmpeg output
    fn extract_video_params(&self, output: &str) -> (Resolution, f64, String, u64) {
        let mut width = 1280;
        let mut height = 720;
        let mut fps = 30.0;
        let mut codec = "unknown".to_string();
        let mut bit_rate = 0u64;

        for line in output.lines() {
            if line.contains("Video:") {
                // Extract codec
                if let Some(video_pos) = line.find("Video:") {
                    let after_video = &line[video_pos + 7..];
                    if let Some(comma_pos) = after_video.find(',') {
                        codec = after_video[..comma_pos].trim().to_string();
                    }
                }

                // Extract resolution (e.g., "1920x1080")
                if let Some(res_match) = line
                    .split_whitespace()
                    .find(|s| s.contains('x') && s.chars().next().unwrap_or('a').is_ascii_digit())
                {
                    let res_parts: Vec<&str> = res_match.split('x').collect();
                    if res_parts.len() == 2 {
                        if let (Ok(w), Ok(h)) = (res_parts[0].parse::<u32>(), res_parts[1].parse::<u32>()) {
                            width = w;
                            height = h;
                        }
                    }
                }

                // Extract frame rate (e.g., "29.97 fps")
                if let Some(fps_pos) = line.find(" fps") {
                    let before_fps = &line[..fps_pos];
                    if let Some(last_space) = before_fps.rfind(' ') {
                        let fps_str = &before_fps[last_space + 1..];
                        if let Ok(parsed_fps) = fps_str.parse::<f64>() {
                            fps = parsed_fps;
                        }
                    }
                }

                // Extract bit rate (e.g., "1500 kb/s")
                if let Some(bitrate_pos) = line.find(" kb/s") {
                    let before_bitrate = &line[..bitrate_pos];
                    if let Some(last_space) = before_bitrate.rfind(' ') {
                        let bitrate_str = &before_bitrate[last_space + 1..];
                        if let Ok(parsed_bitrate) = bitrate_str.parse::<u64>() {
                            bit_rate = parsed_bitrate * 1000; // Convert to bits per second
                        }
                    }
                }
            }
        }

        (Resolution { width, height }, fps, codec, bit_rate)
    }

    /// Check if video has audio stream
    fn has_audio_stream(&self, output: &str) -> bool {
        output.lines().any(|line| line.contains("Audio:"))
    }

    /// Parse keyframe timestamps from ffprobe output
    fn parse_keyframe_timestamps(&self, output: &str) -> Result<Vec<f64>, FFmpegError> {
        let mut timestamps = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                // ffprobe output format: pos,pts_time,flags,size
                let pts_time_str = parts[1];
                let flags_str = parts[2];

                // Only process keyframes (flags contains "K")
                if flags_str.contains('K') {
                    if let Ok(timestamp) = pts_time_str.parse::<f64>() {
                        timestamps.push(timestamp);
                    }
                }
            }
        }

        if timestamps.is_empty() {
            return Err(FFmpegError::ParseError {
                reason: "No keyframes found in video".to_string(),
            });
        }

        // Sort timestamps
        timestamps.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(timestamps)
    }
}

#[async_trait]
impl FFmpegParser for DefaultFFmpegParser {
    async fn check_availability(&self) -> Result<bool, FFmpegError> {
        debug!("Checking FFmpeg availability");

        let result = tokio::time::timeout(
            self.config.timeout,
            AsyncCommand::new(&self.config.ffmpeg_path)
                .arg("-version")
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    info!("FFmpeg is available");
                    Ok(true)
                } else {
                    warn!("FFmpeg command failed");
                    Ok(false)
                }
            }
            Ok(Err(e)) => {
                warn!("Failed to execute FFmpeg: {}", e);
                Ok(false)
            }
            Err(_) => {
                warn!("FFmpeg check timed out");
                Err(FFmpegError::Timeout {
                    duration: self.config.timeout,
                })
            }
        }
    }

    async fn get_version(&self) -> Result<String, FFmpegError> {
        debug!("Getting FFmpeg version");

        let result = tokio::time::timeout(
            self.config.timeout,
            AsyncCommand::new(&self.config.ffmpeg_path)
                .arg("-version")
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    return Err(FFmpegError::CommandFailed {
                        message: "FFmpeg version command failed".to_string(),
                    });
                }

                let version_output = String::from_utf8_lossy(&output.stdout);
                let version = self.extract_version_string(&version_output)?;
                info!("FFmpeg version: {}", version);
                Ok(version)
            }
            Ok(Err(e)) => Err(FFmpegError::CommandFailed {
                message: format!("Failed to execute FFmpeg: {}", e),
            }),
            Err(_) => Err(FFmpegError::Timeout {
                duration: self.config.timeout,
            }),
        }
    }

    async fn extract_metadata(&self, video_path: &Path) -> Result<FFmpegVideoInfo, FFmpegError> {
        debug!("Extracting metadata from: {:?}", video_path);

        if !video_path.exists() {
            return Err(FFmpegError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Video file not found",
            )));
        }

        let result = tokio::time::timeout(
            self.config.timeout,
            AsyncCommand::new(&self.config.ffmpeg_path)
                .args(&[
                    "-i",
                    video_path.to_str().unwrap(),
                    "-f",
                    "null",
                    "-",
                ])
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                let duration = self.extract_duration(&stderr)?;
                let (resolution, frame_rate, codec, bit_rate) = self.extract_video_params(&stderr);
                let has_audio = self.has_audio_stream(&stderr);

                // Extract keyframes
                let keyframe_timestamps = self.extract_keyframes(video_path).await.unwrap_or_default();

                info!(
                    "Extracted metadata: duration={:.2}s, resolution={}x{}, fps={:.2}, codec={}",
                    duration, resolution.width, resolution.height, frame_rate, codec
                );

                Ok(FFmpegVideoInfo {
                    duration,
                    resolution,
                    codec,
                    frame_rate,
                    bit_rate,
                    has_audio,
                    keyframe_timestamps,
                })
            }
            Ok(Err(e)) => Err(FFmpegError::CommandFailed {
                message: format!("Failed to extract metadata: {}", e),
            }),
            Err(_) => Err(FFmpegError::Timeout {
                duration: self.config.timeout,
            }),
        }
    }

    async fn extract_keyframes(&self, video_path: &Path) -> Result<Vec<f64>, FFmpegError> {
        debug!("Extracting keyframes from: {:?}", video_path);

        if !video_path.exists() {
            return Err(FFmpegError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Video file not found",
            )));
        }

        let result = tokio::time::timeout(
            self.config.timeout,
            AsyncCommand::new(&self.config.ffprobe_path)
                .args(&[
                    "-v",
                    "quiet",
                    "-select_streams",
                    "v:0",
                    "-show_entries",
                    "packet=pos,pts_time,flags,size",
                    "-of",
                    "csv=p=0",
                    video_path.to_str().unwrap(),
                ])
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    return Err(FFmpegError::CommandFailed {
                        message: "ffprobe command failed".to_string(),
                    });
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let timestamps = self.parse_keyframe_timestamps(&stdout)?;

                info!("Extracted {} keyframes", timestamps.len());
                Ok(timestamps)
            }
            Ok(Err(e)) => Err(FFmpegError::CommandFailed {
                message: format!("Failed to extract keyframes: {}", e),
            }),
            Err(_) => Err(FFmpegError::Timeout {
                duration: self.config.timeout,
            }),
        }
    }

    async fn validate_video(&self, video_path: &Path) -> Result<bool, FFmpegError> {
        debug!("Validating video file: {:?}", video_path);

        if !video_path.exists() {
            return Ok(false);
        }

        // Try to extract metadata - if successful, video is valid
        match self.extract_metadata(video_path).await {
            Ok(_) => {
                info!("Video file is valid");
                Ok(true)
            }
            Err(e) => {
                warn!("Video validation failed: {}", e);
                Ok(false)
            }
        }
    }
}

impl Default for DefaultFFmpegParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ffmpeg_availability() {
        let parser = DefaultFFmpegParser::new();

        // This test may fail if FFmpeg is not installed
        match parser.check_availability().await {
            Ok(available) => {
                if available {
                    println!("FFmpeg is available");
                } else {
                    println!("FFmpeg is not available - test skipped");
                }
            }
            Err(e) => {
                println!("FFmpeg check error: {:?} - test skipped", e);
            }
        }
    }

    #[test]
    fn test_time_parsing() {
        let parser = DefaultFFmpegParser::new();

        assert_eq!(parser.parse_time_to_seconds("00:00:30.50").unwrap(), 30.5);
        assert_eq!(parser.parse_time_to_seconds("00:01:00.00").unwrap(), 60.0);
        assert_eq!(parser.parse_time_to_seconds("01:30:45.25").unwrap(), 5445.25);
    }

    #[test]
    fn test_version_extraction() {
        let parser = DefaultFFmpegParser::new();
        let version_output = "ffmpeg version 4.4.0 Copyright (c) 2000-2021 the FFmpeg developers";

        let version = parser.extract_version_string(version_output).unwrap();
        assert!(version.contains("ffmpeg version 4.4.0"));
    }

    #[test]
    fn test_resolution_extraction() {
        let parser = DefaultFFmpegParser::new();
        let output = "Stream #0:0: Video: h264, yuv420p, 1920x1080, 30 fps";

        let (resolution, fps, codec, _) = parser.extract_video_params(output);
        assert_eq!(resolution.width, 1920);
        assert_eq!(resolution.height, 1080);
        assert_eq!(fps, 30.0);
        assert_eq!(codec, "h264");
    }

    #[test]
    fn test_audio_detection() {
        let parser = DefaultFFmpegParser::new();
        let output_with_audio = "Stream #0:0: Video: h264\nStream #0:1: Audio: aac";
        let output_without_audio = "Stream #0:0: Video: h264";

        assert!(parser.has_audio_stream(output_with_audio));
        assert!(!parser.has_audio_stream(output_without_audio));
    }
}
