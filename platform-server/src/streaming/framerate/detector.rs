// 帧率检测器实现
//
// 本模块实现了自动帧率检测功能，支持从SPS解析和时间戳分析两种方式。

use std::collections::VecDeque;
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// 帧率检测方法
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionMethod {
    /// 从H.264 SPS解析
    FromSPS,
    /// 从时间戳分析计算
    FromTimestamp,
    /// 使用默认值
    Default,
}

/// 帧率信息
#[derive(Debug, Clone)]
pub struct FrameRateInfo {
    /// 帧率（帧/秒）
    pub fps: f64,
    /// 帧持续时间（微秒）
    pub frame_duration_us: u64,
    /// 是否可变帧率
    pub is_variable: bool,
    /// 检测方法
    pub detection_method: DetectionMethod,
    /// 检测置信度 (0.0-1.0)
    pub confidence: f32,
}

impl FrameRateInfo {
    /// 创建新的帧率信息
    pub fn new(fps: f64, detection_method: DetectionMethod, confidence: f32) -> Self {
        let frame_duration_us = if fps > 0.0 {
            (1_000_000.0 / fps) as u64
        } else {
            33_333 // 默认30fps
        };

        Self {
            fps,
            frame_duration_us,
            is_variable: false,
            detection_method,
            confidence,
        }
    }

    /// 创建默认帧率信息（30 FPS）
    pub fn default_fps() -> Self {
        Self::new(30.0, DetectionMethod::Default, 0.5)
    }
}

/// 时间戳样本
#[derive(Debug, Clone)]
struct TimestampSample {
    /// PTS时间戳（微秒）
    pts: u64,
    /// 接收时间
    receive_time: SystemTime,
}

/// 帧率检测器
pub struct FrameRateDetector {
    /// 检测到的帧率
    detected_fps: Option<f64>,
    /// 时间戳历史记录（用于时间戳分析）
    timestamp_history: VecDeque<TimestampSample>,
    /// 检测置信度
    confidence: f32,
    /// 最大样本数
    max_samples: usize,
    /// 上一次检测到的帧率（用于变化检测）
    previous_fps: Option<f64>,
}

impl FrameRateDetector {
    /// 创建新的帧率检测器
    pub fn new() -> Self {
        Self {
            detected_fps: None,
            timestamp_history: VecDeque::new(),
            confidence: 0.0,
            max_samples: 20,
            previous_fps: None,
        }
    }

    /// 创建带自定义样本数的帧率检测器
    pub fn with_max_samples(max_samples: usize) -> Self {
        Self {
            detected_fps: None,
            timestamp_history: VecDeque::new(),
            confidence: 0.0,
            max_samples,
            previous_fps: None,
        }
    }

    /// 获取当前检测到的帧率
    pub fn get_frame_rate(&self) -> Option<FrameRateInfo> {
        self.detected_fps.map(|fps| {
            let method = if self.timestamp_history.is_empty() {
                DetectionMethod::Default
            } else {
                DetectionMethod::FromTimestamp
            };
            FrameRateInfo::new(fps, method, self.confidence)
        })
    }

    /// 获取当前检测到的FPS值
    pub fn get_fps(&self) -> Option<f64> {
        self.detected_fps
    }

    /// 获取检测置信度
    pub fn get_confidence(&self) -> f32 {
        self.confidence
    }

    /// 检测帧率是否发生变化
    /// 
    /// 如果帧率变化超过10%，返回true
    pub fn has_frame_rate_changed(&self) -> bool {
        match (self.previous_fps, self.detected_fps) {
            (Some(prev), Some(current)) => {
                let change_percent = ((current - prev).abs() / prev) * 100.0;
                if change_percent > 10.0 {
                    info!(
                        "Frame rate changed: {:.2} -> {:.2} fps ({:.1}% change)",
                        prev, current, change_percent
                    );
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 重置检测器状态
    pub fn reset(&mut self) {
        self.timestamp_history.clear();
        self.detected_fps = None;
        self.confidence = 0.0;
    }

    /// 添加时间戳样本
    /// 
    /// # 参数
    /// 
    /// * `pts` - 显示时间戳（微秒）
    /// * `receive_time` - 接收时间
    pub fn add_timestamp_sample(&mut self, pts: u64, receive_time: SystemTime) {
        let sample = TimestampSample { pts, receive_time };
        
        self.timestamp_history.push_back(sample);
        
        // 保持样本数量在限制内
        while self.timestamp_history.len() > self.max_samples {
            self.timestamp_history.pop_front();
        }
        
        // 如果有足够的样本，尝试检测帧率
        if self.timestamp_history.len() >= 10 {
            if let Ok(info) = self.detect_from_timestamps() {
                self.update_detected_fps(info.fps, info.confidence);
            }
        }
    }

    /// 从时间戳序列检测帧率
    /// 
    /// 通过分析连续帧的时间间隔来计算帧率
    /// 
    /// # 返回
    /// 
    /// 返回检测到的帧率信息，如果样本不足或检测失败则返回错误
    pub fn detect_from_timestamps(&self) -> Result<FrameRateInfo, String> {
        // 检查样本数量
        if self.timestamp_history.len() < 10 {
            return Err(format!(
                "Insufficient samples: {} (need at least 10)",
                self.timestamp_history.len()
            ));
        }

        // 计算相邻帧的时间间隔（使用PTS）
        let mut intervals = Vec::new();
        for i in 1..self.timestamp_history.len() {
            let prev_pts = self.timestamp_history[i - 1].pts;
            let curr_pts = self.timestamp_history[i].pts;
            
            // 跳过时间戳倒退的情况
            if curr_pts <= prev_pts {
                debug!(
                    "Skipping backwards timestamp: {} -> {}",
                    prev_pts, curr_pts
                );
                continue;
            }
            
            let interval_us = curr_pts - prev_pts;
            let interval_sec = interval_us as f64 / 1_000_000.0;
            
            // 过滤异常间隔（太小或太大）
            // 假设帧率在5-120 FPS之间
            if interval_sec > 0.008 && interval_sec < 0.2 {
                intervals.push(interval_sec);
            } else {
                debug!(
                    "Skipping abnormal interval: {:.6}s ({:.2} fps)",
                    interval_sec,
                    1.0 / interval_sec
                );
            }
        }

        // 检查有效间隔数量
        if intervals.is_empty() {
            return Err("No valid intervals found".to_string());
        }

        // 计算平均间隔
        let sum: f64 = intervals.iter().sum();
        let avg_interval = sum / intervals.len() as f64;

        // 计算帧率
        let fps = 1.0 / avg_interval;

        // 计算方差以评估置信度
        let variance: f64 = intervals
            .iter()
            .map(|&interval| {
                let diff = interval - avg_interval;
                diff * diff
            })
            .sum::<f64>()
            / intervals.len() as f64;

        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / avg_interval;

        // 置信度评估：变异系数越小，置信度越高
        // CV < 0.05: 高置信度 (0.9-1.0)
        // CV < 0.10: 中等置信度 (0.7-0.9)
        // CV < 0.20: 低置信度 (0.5-0.7)
        // CV >= 0.20: 很低置信度 (0.3-0.5)
        let confidence = if coefficient_of_variation < 0.05 {
            0.95
        } else if coefficient_of_variation < 0.10 {
            0.80
        } else if coefficient_of_variation < 0.20 {
            0.60
        } else {
            0.40
        };

        info!(
            "Detected frame rate: {:.2} fps (avg interval: {:.6}s, CV: {:.4}, confidence: {:.2})",
            fps, avg_interval, coefficient_of_variation, confidence
        );

        Ok(FrameRateInfo::new(
            fps,
            DetectionMethod::FromTimestamp,
            confidence,
        ))
    }

    /// 从H.264 SPS解析帧率
    /// 
    /// 注意：此功能需要H.264解析库，当前为占位实现
    /// 
    /// # 参数
    /// 
    /// * `sps_data` - SPS NAL单元数据
    /// 
    /// # 返回
    /// 
    /// 返回检测到的帧率信息，如果解析失败则返回错误
    pub fn detect_from_sps(&self, _sps_data: &[u8]) -> Result<FrameRateInfo, String> {
        // TODO: 实现SPS解析
        // 需要添加h264-reader或类似的crate
        // 
        // 基本步骤：
        // 1. 解析SPS NAL单元
        // 2. 提取VUI参数中的time_scale和num_units_in_tick
        // 3. 计算帧率：fps = time_scale / (2 * num_units_in_tick)
        
        Err("SPS parsing not implemented yet".to_string())
    }

    /// 获取帧率或使用默认值
    /// 
    /// 如果无法检测帧率，返回默认的30 FPS
    /// 
    /// # 返回
    /// 
    /// 返回检测到的帧率信息或默认帧率信息
    pub fn get_frame_rate_or_default(&self) -> FrameRateInfo {
        match self.get_frame_rate() {
            Some(info) => {
                info!("Using detected frame rate: {:.2} fps", info.fps);
                info
            }
            None => {
                warn!("Frame rate detection failed, using default 30 FPS");
                FrameRateInfo::default_fps()
            }
        }
    }

    /// 更新检测到的帧率
    /// 
    /// 内部方法，用于更新帧率并检测变化
    fn update_detected_fps(&mut self, new_fps: f64, confidence: f32) {
        // 检查帧率是否发生变化
        if self.has_frame_rate_changed() {
            self.previous_fps = self.detected_fps;
        }
        
        self.detected_fps = Some(new_fps);
        self.confidence = confidence;
    }
}

impl Default for FrameRateDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_frame_rate_info_creation() {
        let info = FrameRateInfo::new(30.0, DetectionMethod::FromTimestamp, 0.9);
        assert_eq!(info.fps, 30.0);
        assert_eq!(info.frame_duration_us, 33_333);
        assert_eq!(info.detection_method, DetectionMethod::FromTimestamp);
        assert_eq!(info.confidence, 0.9);
    }

    #[test]
    fn test_default_frame_rate() {
        let info = FrameRateInfo::default_fps();
        assert_eq!(info.fps, 30.0);
        assert_eq!(info.detection_method, DetectionMethod::Default);
    }

    #[test]
    fn test_detector_creation() {
        let detector = FrameRateDetector::new();
        assert!(detector.get_fps().is_none());
        assert_eq!(detector.get_confidence(), 0.0);
    }

    #[test]
    fn test_detector_with_max_samples() {
        let detector = FrameRateDetector::with_max_samples(10);
        assert_eq!(detector.max_samples, 10);
    }

    #[test]
    fn test_add_timestamp_sample() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        
        // 添加样本
        detector.add_timestamp_sample(0, now);
        assert_eq!(detector.timestamp_history.len(), 1);
        
        detector.add_timestamp_sample(33_333, now + Duration::from_micros(33_333));
        assert_eq!(detector.timestamp_history.len(), 2);
    }

    #[test]
    fn test_detect_from_timestamps_30fps() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        let frame_interval_us = 33_333; // ~30 fps
        
        // 添加15个样本（30fps）
        for i in 0..15 {
            let pts = i * frame_interval_us;
            let time = now + Duration::from_micros(pts);
            detector.add_timestamp_sample(pts, time);
        }
        
        // 检测帧率
        let result = detector.detect_from_timestamps();
        assert!(result.is_ok());
        
        let info = result.unwrap();
        // 允许5%的误差
        assert!((info.fps - 30.0).abs() < 1.5, "FPS: {}", info.fps);
        assert_eq!(info.detection_method, DetectionMethod::FromTimestamp);
        assert!(info.confidence > 0.5);
    }

    #[test]
    fn test_detect_from_timestamps_60fps() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        let frame_interval_us = 16_667; // ~60 fps
        
        // 添加15个样本（60fps）
        for i in 0..15 {
            let pts = i * frame_interval_us;
            let time = now + Duration::from_micros(pts);
            detector.add_timestamp_sample(pts, time);
        }
        
        // 检测帧率
        let result = detector.detect_from_timestamps();
        assert!(result.is_ok());
        
        let info = result.unwrap();
        // 允许5%的误差
        assert!((info.fps - 60.0).abs() < 3.0, "FPS: {}", info.fps);
        assert!(info.confidence > 0.5);
    }

    #[test]
    fn test_detect_from_timestamps_insufficient_samples() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        
        // 只添加5个样本（不足10个）
        for i in 0..5 {
            let pts = i * 33_333;
            let time = now + Duration::from_micros(pts);
            detector.add_timestamp_sample(pts, time);
        }
        
        // 检测应该失败
        let result = detector.detect_from_timestamps();
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_from_timestamps_with_jitter() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        let base_interval_us = 33_333; // ~30 fps
        
        // 添加带抖动的样本
        for i in 0..15 {
            let jitter = if i % 2 == 0 { 500 } else { -500 }; // ±500us抖动
            let pts = i * base_interval_us + jitter;
            let time = now + Duration::from_micros(pts as u64);
            detector.add_timestamp_sample(pts as u64, time);
        }
        
        // 检测帧率
        let result = detector.detect_from_timestamps();
        assert!(result.is_ok());
        
        let info = result.unwrap();
        // 允许5%的误差
        assert!((info.fps - 30.0).abs() < 1.5, "FPS: {}", info.fps);
        // 抖动会降低置信度
        assert!(info.confidence > 0.3);
    }

    #[test]
    fn test_max_samples_limit() {
        let mut detector = FrameRateDetector::with_max_samples(10);
        let now = SystemTime::now();
        
        // 添加20个样本
        for i in 0..20 {
            let pts = i * 33_333;
            let time = now + Duration::from_micros(pts);
            detector.add_timestamp_sample(pts, time);
        }
        
        // 应该只保留最后10个
        assert_eq!(detector.timestamp_history.len(), 10);
    }

    #[test]
    fn test_frame_rate_change_detection() {
        let mut detector = FrameRateDetector::new();
        
        // 设置初始帧率
        detector.detected_fps = Some(30.0);
        detector.previous_fps = Some(30.0);
        
        // 变化小于10%，不应该检测到变化
        detector.detected_fps = Some(32.0);
        assert!(!detector.has_frame_rate_changed());
        
        // 变化大于10%，应该检测到变化
        detector.detected_fps = Some(40.0);
        assert!(detector.has_frame_rate_changed());
    }

    #[test]
    fn test_reset() {
        let mut detector = FrameRateDetector::new();
        let now = SystemTime::now();
        
        // 添加样本
        for i in 0..15 {
            let pts = i * 33_333;
            let time = now + Duration::from_micros(pts);
            detector.add_timestamp_sample(pts, time);
        }
        
        assert!(!detector.timestamp_history.is_empty());
        assert!(detector.detected_fps.is_some());
        
        // 重置
        detector.reset();
        
        assert!(detector.timestamp_history.is_empty());
        assert!(detector.detected_fps.is_none());
        assert_eq!(detector.confidence, 0.0);
    }
}
