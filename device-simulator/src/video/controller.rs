use crate::video::errors::PlaybackError;
use crate::video::types::{
    AudioSegment, BufferHealth, BufferManager, CongestionLevel, DropFrameStrategy,
    KeyframeEntry, KeyframeIndex, NetworkConditions, SeekResult, SyncInfo, VideoSegment,
};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info};

// ============================================================================
// Playback Controller Trait
// ============================================================================

/// Trait for playback control operations
#[async_trait]
pub trait PlaybackController: Send + Sync {
    /// Seek to a specific position
    async fn seek(&mut self, position: f64) -> Result<(), PlaybackError>;

    /// Seek to the nearest keyframe
    async fn seek_to_keyframe(
        &mut self,
        position: f64,
        index: &KeyframeIndex,
    ) -> Result<SeekResult, PlaybackError>;

    /// Set playback rate (0.25x to 4x)
    async fn set_playback_rate(&mut self, rate: f64) -> Result<(), PlaybackError>;

    /// Get frame dropping strategy for a given playback rate
    fn get_drop_frame_strategy(&self, rate: f64) -> DropFrameStrategy;

    /// Adjust transmission queue based on playback rate
    fn adjust_transmission_queue(
        &self,
        segments: Vec<VideoSegment>,
        playback_rate: f64,
    ) -> Vec<VideoSegment>;

    /// Clear all buffers
    fn clear_buffers(&mut self) -> Result<(), PlaybackError>;

    /// Find the nearest keyframe for a given timestamp
    fn find_nearest_keyframe(&self, timestamp: f64, index: &KeyframeIndex)
        -> Option<KeyframeEntry>;

    /// Adjust audio-video synchronization
    fn adjust_audio_video_sync(&mut self, playback_rate: f64) -> SyncInfo;
}

// ============================================================================
// Default Playback Controller Implementation
// ============================================================================

/// Default implementation of playback controller
pub struct DefaultPlaybackController {
    current_position: f64,
    playback_rate: f64,
    transmission_queue: VecDeque<VideoSegment>,
    audio_queue: VecDeque<AudioSegment>,
    buffer_manager: BufferManager,
    sync_offset: f64,
    last_seek_position: Option<f64>,
}

impl DefaultPlaybackController {
    /// Create a new playback controller
    pub fn new() -> Self {
        Self {
            current_position: 0.0,
            playback_rate: 1.0,
            transmission_queue: VecDeque::new(),
            audio_queue: VecDeque::new(),
            buffer_manager: BufferManager {
                video_buffers: HashMap::new(),
                audio_buffers: HashMap::new(),
                max_buffer_size: 10 * 1024 * 1024, // 10MB
                current_buffer_size: 0,
                buffer_health: BufferHealth {
                    video_buffer_level: 0.0,
                    audio_buffer_level: 0.0,
                    underrun_count: 0,
                    overrun_count: 0,
                    last_underrun: None,
                },
            },
            sync_offset: 0.0,
            last_seek_position: None,
        }
    }

    /// Get current playback position
    pub fn get_current_position(&self) -> f64 {
        self.current_position
    }

    /// Get current playback rate
    pub fn get_playback_rate(&self) -> f64 {
        self.playback_rate
    }

    /// Get buffer health status
    pub fn get_buffer_health(&self) -> &BufferHealth {
        &self.buffer_manager.buffer_health
    }

    /// Get last seek position
    pub fn get_last_seek_position(&self) -> Option<f64> {
        self.last_seek_position
    }

    /// Apply intelligent frame dropping based on strategy
    fn apply_intelligent_frame_dropping(
        &self,
        segments: Vec<VideoSegment>,
        strategy: &DropFrameStrategy,
    ) -> Vec<VideoSegment> {
        if !strategy.adaptive_dropping {
            return segments;
        }

        let mut result = Vec::new();
        let mut last_key_frame_index = None;

        for (index, segment) in segments.iter().enumerate() {
            if segment.is_key_frame {
                result.push(segment.clone());
                last_key_frame_index = Some(result.len() - 1);
            } else {
                let should_keep = match (strategy.drop_b_frames, strategy.drop_p_frames) {
                    (false, false) => true,
                    (true, false) => (index - last_key_frame_index.unwrap_or(0)) % 3 != 2,
                    (true, true) => (index - last_key_frame_index.unwrap_or(0)) % 4 == 1,
                    (false, true) => (index - last_key_frame_index.unwrap_or(0)) % 2 == 1,
                };

                if should_keep {
                    result.push(segment.clone());
                }
            }
        }

        result
    }
}

#[async_trait]
impl PlaybackController for DefaultPlaybackController {
    async fn seek(&mut self, position: f64) -> Result<(), PlaybackError> {
        if position < 0.0 {
            return Err(PlaybackError::InvalidSeekPosition { position });
        }

        debug!("Seeking to position: {:.2}s", position);
        self.clear_buffers()?;
        self.current_position = position;
        self.last_seek_position = Some(position);
        self.sync_offset = 0.0;

        info!("Seek completed to position: {:.2}s", position);
        Ok(())
    }

    async fn seek_to_keyframe(
        &mut self,
        position: f64,
        index: &KeyframeIndex,
    ) -> Result<SeekResult, PlaybackError> {
        let start_time = Instant::now();

        if position < 0.0 || position > index.total_duration {
            return Err(PlaybackError::InvalidSeekPosition { position });
        }

        debug!("Seeking to keyframe at position: {:.2}s", position);

        let keyframe_entry = self
            .find_nearest_keyframe(position, index)
            .ok_or(PlaybackError::KeyframeNotFound { timestamp: position })?;

        self.clear_buffers()?;

        let actual_time = keyframe_entry.timestamp;
        self.current_position = actual_time;
        self.last_seek_position = Some(actual_time);
        self.sync_offset = 0.0;

        let precision_achieved = if (position - actual_time).abs() < f64::EPSILON {
            1.0
        } else {
            1.0 - ((position - actual_time).abs() / position.max(1.0)).min(1.0)
        };

        let execution_time = start_time.elapsed();

        info!(
            "Seek to keyframe completed: requested={:.2}s, actual={:.2}s, precision={:.2}%",
            position,
            actual_time,
            precision_achieved * 100.0
        );

        Ok(SeekResult {
            requested_time: position,
            actual_time,
            keyframe_offset: keyframe_entry.file_offset,
            precision_achieved,
            keyframe_used: keyframe_entry,
            execution_time,
        })
    }

    async fn set_playback_rate(&mut self, rate: f64) -> Result<(), PlaybackError> {
        if rate <= 0.0 || rate > 10.0 {
            return Err(PlaybackError::InvalidPlaybackRate { rate });
        }

        debug!("Setting playback rate to: {:.2}x", rate);

        let old_rate = self.playback_rate;
        self.playback_rate = rate;

        if (old_rate - rate).abs() > f64::EPSILON {
            self.adjust_audio_video_sync(rate);
        }

        info!("Playback rate set to: {:.2}x", rate);
        Ok(())
    }

    fn get_drop_frame_strategy(&self, rate: f64) -> DropFrameStrategy {
        match rate {
            r if r <= 1.0 => DropFrameStrategy {
                drop_b_frames: false,
                drop_p_frames: false,
                keep_key_frames_only: false,
                adaptive_dropping: false,
            },
            r if r <= 2.0 => DropFrameStrategy {
                drop_b_frames: true,
                drop_p_frames: false,
                keep_key_frames_only: false,
                adaptive_dropping: true,
            },
            r if r <= 4.0 => DropFrameStrategy {
                drop_b_frames: true,
                drop_p_frames: true,
                keep_key_frames_only: false,
                adaptive_dropping: true,
            },
            _ => DropFrameStrategy {
                drop_b_frames: true,
                drop_p_frames: true,
                keep_key_frames_only: true,
                adaptive_dropping: true,
            },
        }
    }

    fn adjust_transmission_queue(
        &self,
        segments: Vec<VideoSegment>,
        playback_rate: f64,
    ) -> Vec<VideoSegment> {
        let mut segments = segments;
        let strategy = self.get_drop_frame_strategy(playback_rate);

        if strategy.keep_key_frames_only {
            segments.retain(|segment| segment.is_key_frame);
        } else if strategy.drop_b_frames || strategy.drop_p_frames {
            segments = self.apply_intelligent_frame_dropping(segments, &strategy);
        }

        segments.sort_by(|a, b| {
            a.timestamp
                .partial_cmp(&b.timestamp)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        segments
    }

    fn clear_buffers(&mut self) -> Result<(), PlaybackError> {
        debug!("Clearing buffers");

        self.transmission_queue.clear();
        self.audio_queue.clear();
        self.buffer_manager.video_buffers.clear();
        self.buffer_manager.audio_buffers.clear();
        self.buffer_manager.current_buffer_size = 0;
        self.buffer_manager.buffer_health.video_buffer_level = 0.0;
        self.buffer_manager.buffer_health.audio_buffer_level = 0.0;

        Ok(())
    }

    fn find_nearest_keyframe(
        &self,
        timestamp: f64,
        index: &KeyframeIndex,
    ) -> Option<KeyframeEntry> {
        if index.entries.is_empty() {
            return None;
        }

        if timestamp <= 0.0 {
            return index.entries.first().cloned();
        }

        if timestamp >= index.total_duration {
            return index.entries.last().cloned();
        }

        let mut left = 0;
        let mut right = index.entries.len();

        while left < right {
            let mid = left + (right - left) / 2;

            if index.entries[mid].timestamp <= timestamp {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        if left == 0 {
            index.entries.first().cloned()
        } else {
            index.entries.get(left - 1).cloned()
        }
    }

    fn adjust_audio_video_sync(&mut self, playback_rate: f64) -> SyncInfo {
        let base_sync_offset = self.sync_offset;

        let adjusted_offset = match playback_rate {
            r if r <= 1.0 => base_sync_offset,
            r if r <= 2.0 => base_sync_offset * 0.8,
            r if r <= 4.0 => base_sync_offset * 0.6,
            _ => 0.0,
        };

        self.sync_offset = adjusted_offset;

        SyncInfo {
            video_timestamp: self.current_position,
            audio_timestamp: self.current_position + adjusted_offset,
            offset: adjusted_offset,
        }
    }
}

impl Default for DefaultPlaybackController {
    fn default() -> Self {
        Self::new()
    }
}
