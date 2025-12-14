#[cfg(test)]
mod tests {
    use super::super::types::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_keyframe_entry_creation() {
        let entry = KeyframeEntry {
            timestamp: 1.5,
            file_offset: 1024,
            frame_size: 2048,
            gop_size: 30,
            frame_type: FrameType::I,
        };

        assert_eq!(entry.timestamp, 1.5);
        assert_eq!(entry.file_offset, 1024);
        assert_eq!(entry.frame_size, 2048);
        assert_eq!(entry.gop_size, 30);
        assert_eq!(entry.frame_type, FrameType::I);
    }

    #[test]
    fn test_keyframe_index_creation() {
        let entries = vec![
            KeyframeEntry {
                timestamp: 0.0,
                file_offset: 0,
                frame_size: 2048,
                gop_size: 30,
                frame_type: FrameType::I,
            },
            KeyframeEntry {
                timestamp: 1.0,
                file_offset: 2048,
                frame_size: 2048,
                gop_size: 30,
                frame_type: FrameType::I,
            },
        ];

        let index = KeyframeIndex {
            entries: entries.clone(),
            total_duration: 10.0,
            index_precision: 0.033,
            memory_optimized: true,
            optimization_strategy: IndexOptimizationStrategy::Adaptive,
            memory_usage: 1024,
        };

        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.total_duration, 10.0);
        assert_eq!(index.index_precision, 0.033);
        assert!(index.memory_optimized);
        assert_eq!(index.optimization_strategy, IndexOptimizationStrategy::Adaptive);
    }

    #[test]
    fn test_seek_result_creation() {
        let keyframe = KeyframeEntry {
            timestamp: 1.0,
            file_offset: 2048,
            frame_size: 2048,
            gop_size: 30,
            frame_type: FrameType::I,
        };

        let result = SeekResult {
            requested_time: 1.5,
            actual_time: 1.0,
            keyframe_offset: 2048,
            precision_achieved: 0.5,
            keyframe_used: keyframe.clone(),
            execution_time: Duration::from_millis(50),
        };

        assert_eq!(result.requested_time, 1.5);
        assert_eq!(result.actual_time, 1.0);
        assert_eq!(result.precision_achieved, 0.5);
        assert_eq!(result.execution_time, Duration::from_millis(50));
    }

    #[test]
    fn test_frame_type_equality() {
        assert_eq!(FrameType::I, FrameType::I);
        assert_eq!(FrameType::P, FrameType::P);
        assert_eq!(FrameType::B, FrameType::B);
        assert_ne!(FrameType::I, FrameType::P);
    }

    #[test]
    fn test_optimization_strategy_equality() {
        assert_eq!(IndexOptimizationStrategy::Full, IndexOptimizationStrategy::Full);
        assert_eq!(IndexOptimizationStrategy::Sparse, IndexOptimizationStrategy::Sparse);
        assert_eq!(IndexOptimizationStrategy::Adaptive, IndexOptimizationStrategy::Adaptive);
        assert_eq!(IndexOptimizationStrategy::Hierarchical, IndexOptimizationStrategy::Hierarchical);
        assert_ne!(IndexOptimizationStrategy::Full, IndexOptimizationStrategy::Sparse);
    }

    #[test]
    fn test_drop_frame_strategy() {
        let strategy1 = DropFrameStrategy::DropNone;
        let strategy2 = DropFrameStrategy::DropNonKeyframes;
        let strategy3 = DropFrameStrategy::DropByRate(0.5);
        let strategy4 = DropFrameStrategy::Adaptive;

        assert_eq!(strategy1, DropFrameStrategy::DropNone);
        assert_eq!(strategy2, DropFrameStrategy::DropNonKeyframes);
        assert_ne!(strategy1, strategy2);
    }

    #[test]
    fn test_resolution_creation() {
        let resolution = Resolution {
            width: 1920,
            height: 1080,
        };

        assert_eq!(resolution.width, 1920);
        assert_eq!(resolution.height, 1080);
    }

    #[test]
    fn test_timeline_file_serialization() {
        use std::path::PathBuf;

        let timeline = TimelineFile {
            version: 1,
            video_file_path: PathBuf::from("/path/to/video.mp4"),
            video_file_hash: "abc123".to_string(),
            video_file_size: 1024000,
            video_file_modified: SystemTime::now(),
            duration: 60.0,
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
            frame_rate: 30.0,
            keyframe_index: KeyframeIndex {
                entries: vec![],
                total_duration: 60.0,
                index_precision: 0.033,
                memory_optimized: true,
                optimization_strategy: IndexOptimizationStrategy::Adaptive,
                memory_usage: 1024,
            },
            created_at: SystemTime::now(),
            ffmpeg_version: Some("4.4.2".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&timeline).expect("Failed to serialize");
        assert!(!json.is_empty());

        // Test deserialization
        let deserialized: TimelineFile = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.duration, 60.0);
        assert_eq!(deserialized.resolution.width, 1920);
    }
}
