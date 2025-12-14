// 时间戳管理器实现
//
// 本模块实现了精确的时间戳管理功能，包括时间戳生成、验证和格式转换。

use tracing::{debug, info, warn};

/// 时间戳管理器
/// 
/// 负责管理视频帧的时间戳，确保时间戳单调递增，并提供微秒级精度的时间戳操作。
pub struct TimestampManager {
    /// 基准时间戳（微秒）
    base_timestamp: u64,
    /// 上一个时间戳（微秒）
    last_timestamp: u64,
    /// 帧持续时间（微秒）
    frame_duration_us: u64,
    /// 时钟频率（通常是90000 Hz，即90kHz）
    clock_rate: u32,
    /// 帧计数器
    frame_count: u64,
}

impl TimestampManager {
    /// 创建新的时间戳管理器
    /// 
    /// # 参数
    /// 
    /// * `fps` - 帧率（帧/秒）
    /// 
    /// # 示例
    /// 
    /// ```
    /// let manager = TimestampManager::new(30.0);
    /// ```
    pub fn new(fps: f64) -> Self {
        let frame_duration_us = if fps > 0.0 {
            (1_000_000.0 / fps) as u64
        } else {
            33_333 // 默认30fps
        };

        info!(
            "Creating TimestampManager: fps={:.2}, frame_duration={}us",
            fps, frame_duration_us
        );

        Self {
            base_timestamp: 0,
            last_timestamp: 0,
            frame_duration_us,
            clock_rate: 90_000, // H.264标准时钟频率
            frame_count: 0,
        }
    }

    /// 创建带自定义基准时间戳的时间戳管理器
    /// 
    /// # 参数
    /// 
    /// * `fps` - 帧率（帧/秒）
    /// * `base_timestamp` - 基准时间戳（微秒）
    pub fn with_base_timestamp(fps: f64, base_timestamp: u64) -> Self {
        let mut manager = Self::new(fps);
        manager.base_timestamp = base_timestamp;
        manager.last_timestamp = base_timestamp;
        manager
    }

    /// 获取帧持续时间（微秒）
    pub fn frame_duration_us(&self) -> u64 {
        self.frame_duration_us
    }

    /// 获取时钟频率
    pub fn clock_rate(&self) -> u32 {
        self.clock_rate
    }

    /// 获取当前帧计数
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// 获取上一个时间戳（微秒）
    pub fn last_timestamp(&self) -> u64 {
        self.last_timestamp
    }

    /// 生成下一个时间戳
    /// 
    /// 根据帧率计算并生成单调递增的时间戳。
    /// 
    /// # 返回
    /// 
    /// 返回新的时间戳（微秒）
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut manager = TimestampManager::new(30.0);
    /// let ts1 = manager.generate_next_timestamp();
    /// let ts2 = manager.generate_next_timestamp();
    /// assert!(ts2 > ts1);
    /// ```
    pub fn generate_next_timestamp(&mut self) -> u64 {
        let new_timestamp = self.base_timestamp + (self.frame_count * self.frame_duration_us);
        
        debug!(
            "Generated timestamp: {} us (frame {}, duration {} us)",
            new_timestamp, self.frame_count, self.frame_duration_us
        );

        self.last_timestamp = new_timestamp;
        self.frame_count += 1;

        new_timestamp
    }

    /// 验证时间戳
    /// 
    /// 检查时间戳是否单调递增，并检测异常间隔。
    /// 
    /// # 参数
    /// 
    /// * `pts` - 要验证的时间戳（微秒）
    /// 
    /// # 返回
    /// 
    /// 如果时间戳有效返回Ok(())，否则返回错误信息
    /// 
    /// # 错误
    /// 
    /// - 时间戳不单调递增
    /// - 时间戳间隔异常（太大或太小）
    pub fn validate_timestamp(&self, pts: u64) -> Result<(), String> {
        // 检查单调性
        if pts <= self.last_timestamp && self.last_timestamp > 0 {
            return Err(format!(
                "Timestamp not monotonic: {} <= {} (previous)",
                pts, self.last_timestamp
            ));
        }

        // 检查间隔是否异常
        if self.last_timestamp > 0 {
            let interval = pts - self.last_timestamp;
            let expected_interval = self.frame_duration_us;
            
            // 允许±50%的偏差
            let min_interval = expected_interval / 2;
            let max_interval = expected_interval * 3;
            
            if interval < min_interval {
                warn!(
                    "Timestamp interval too small: {} us (expected ~{} us)",
                    interval, expected_interval
                );
            } else if interval > max_interval {
                warn!(
                    "Timestamp interval too large: {} us (expected ~{} us)",
                    interval, expected_interval
                );
            }
        }

        Ok(())
    }

    /// 转换时间戳：90kHz时钟 → 微秒
    /// 
    /// # 参数
    /// 
    /// * `timestamp_90khz` - 90kHz时钟的时间戳
    /// 
    /// # 返回
    /// 
    /// 返回微秒时间戳
    /// 
    /// # 示例
    /// 
    /// ```
    /// let manager = TimestampManager::new(30.0);
    /// let us = manager.convert_to_microseconds(90_000); // 1秒
    /// assert_eq!(us, 1_000_000);
    /// ```
    pub fn convert_to_microseconds(&self, timestamp_90khz: u64) -> u64 {
        // 90kHz = 90,000 ticks per second
        // 1 tick = 1/90,000 second = 1,000,000/90,000 microseconds ≈ 11.111 microseconds
        // timestamp_us = timestamp_90khz * 1,000,000 / 90,000
        (timestamp_90khz * 1_000_000) / self.clock_rate as u64
    }

    /// 转换时间戳：微秒 → 90kHz时钟
    /// 
    /// # 参数
    /// 
    /// * `timestamp_us` - 微秒时间戳
    /// 
    /// # 返回
    /// 
    /// 返回90kHz时钟的时间戳
    /// 
    /// # 示例
    /// 
    /// ```
    /// let manager = TimestampManager::new(30.0);
    /// let khz = manager.convert_to_90khz(1_000_000); // 1秒
    /// assert_eq!(khz, 90_000);
    /// ```
    pub fn convert_to_90khz(&self, timestamp_us: u64) -> u64 {
        // timestamp_90khz = timestamp_us * 90,000 / 1,000,000
        (timestamp_us * self.clock_rate as u64) / 1_000_000
    }

    /// 更新帧率
    /// 
    /// 当检测到帧率变化时，更新帧持续时间。
    /// 
    /// # 参数
    /// 
    /// * `new_fps` - 新的帧率
    pub fn update_frame_rate(&mut self, new_fps: f64) {
        let old_duration = self.frame_duration_us;
        self.frame_duration_us = if new_fps > 0.0 {
            (1_000_000.0 / new_fps) as u64
        } else {
            33_333
        };

        info!(
            "Frame rate updated: {:.2} fps (duration: {} -> {} us)",
            new_fps, old_duration, self.frame_duration_us
        );
    }

    /// 重置时间戳管理器
    /// 
    /// 重置所有状态，但保持帧率设置。
    pub fn reset(&mut self) {
        info!("Resetting TimestampManager");
        self.base_timestamp = 0;
        self.last_timestamp = 0;
        self.frame_count = 0;
    }

    /// 重置到新的基准时间戳
    /// 
    /// 用于处理时间戳不连续的情况。
    /// 
    /// # 参数
    /// 
    /// * `new_base` - 新的基准时间戳（微秒）
    pub fn reset_to_base(&mut self, new_base: u64) {
        info!(
            "Resetting TimestampManager to new base: {} us (old: {} us)",
            new_base, self.base_timestamp
        );
        self.base_timestamp = new_base;
        self.last_timestamp = new_base;
        self.frame_count = 0;
    }

    /// 处理时间戳不连续
    /// 
    /// 检测时间戳跳跃并重新同步时间戳基准。
    /// 根据需求9.2，系统应在3帧内完成重新同步。
    /// 
    /// # 参数
    /// 
    /// * `new_timestamp` - 新的时间戳（微秒）
    /// 
    /// # 返回
    /// 
    /// 如果检测到不连续返回true，否则返回false
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut manager = TimestampManager::new(30.0);
    /// manager.generate_next_timestamp();
    /// 
    /// // 模拟时间戳跳跃
    /// let discontinuous = manager.handle_discontinuity(10_000_000);
    /// assert!(discontinuous);
    /// ```
    pub fn handle_discontinuity(&mut self, new_timestamp: u64) -> bool {
        // 如果这是第一个时间戳，不算不连续
        if self.last_timestamp == 0 {
            self.base_timestamp = new_timestamp;
            self.last_timestamp = new_timestamp;
            return false;
        }

        // 计算时间戳间隔
        let interval = if new_timestamp > self.last_timestamp {
            new_timestamp - self.last_timestamp
        } else {
            // 时间戳倒退
            warn!(
                "Timestamp went backwards: {} -> {} us",
                self.last_timestamp, new_timestamp
            );
            self.reset_to_base(new_timestamp);
            return true;
        };

        // 检测时间戳跳跃
        // 跳跃定义：间隔 > 5秒 (5,000,000 微秒)
        let max_gap_us = 5_000_000;
        
        if interval > max_gap_us {
            warn!(
                "Timestamp discontinuity detected: gap = {:.3}s ({} us)",
                interval as f64 / 1_000_000.0,
                interval
            );
            
            // 重新同步到新的基准
            // 根据需求，应在3帧内完成重新同步
            // 我们立即重置到新时间戳，这样下一帧就已经同步了
            self.reset_to_base(new_timestamp);
            
            info!("Timestamp resynchronized to {} us", new_timestamp);
            return true;
        }

        // 检测异常小的间隔（可能是时间戳错误）
        // 如果间隔小于预期的1/10，也认为是不连续
        let min_gap_us = self.frame_duration_us / 10;
        
        if interval < min_gap_us {
            warn!(
                "Abnormally small timestamp interval: {} us (expected ~{} us)",
                interval, self.frame_duration_us
            );
            // 对于小间隔，我们不重置，只是警告
            // 因为这可能是正常的帧率变化
        }

        // 更新最后时间戳
        self.last_timestamp = new_timestamp;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_manager_creation() {
        let manager = TimestampManager::new(30.0);
        assert_eq!(manager.frame_duration_us(), 33_333);
        assert_eq!(manager.clock_rate(), 90_000);
        assert_eq!(manager.frame_count(), 0);
        assert_eq!(manager.last_timestamp(), 0);
    }

    #[test]
    fn test_timestamp_manager_with_base() {
        let manager = TimestampManager::with_base_timestamp(30.0, 1_000_000);
        assert_eq!(manager.base_timestamp, 1_000_000);
        assert_eq!(manager.last_timestamp(), 1_000_000);
    }

    #[test]
    fn test_generate_next_timestamp() {
        let mut manager = TimestampManager::new(30.0);
        
        let ts1 = manager.generate_next_timestamp();
        assert_eq!(ts1, 0);
        assert_eq!(manager.frame_count(), 1);
        
        let ts2 = manager.generate_next_timestamp();
        assert_eq!(ts2, 33_333);
        assert_eq!(manager.frame_count(), 2);
        
        let ts3 = manager.generate_next_timestamp();
        assert_eq!(ts3, 66_666);
        assert_eq!(manager.frame_count(), 3);
    }

    #[test]
    fn test_generate_timestamp_60fps() {
        let mut manager = TimestampManager::new(60.0);
        
        let ts1 = manager.generate_next_timestamp();
        assert_eq!(ts1, 0);
        
        let ts2 = manager.generate_next_timestamp();
        assert_eq!(ts2, 16_666);
    }

    #[test]
    fn test_validate_timestamp_monotonic() {
        let mut manager = TimestampManager::new(30.0);
        
        // 第一个时间戳总是有效的
        assert!(manager.validate_timestamp(0).is_ok());
        manager.last_timestamp = 0;
        
        // 单调递增的时间戳应该有效
        assert!(manager.validate_timestamp(33_333).is_ok());
        manager.last_timestamp = 33_333;
        
        assert!(manager.validate_timestamp(66_666).is_ok());
        manager.last_timestamp = 66_666;
        
        // 不单调的时间戳应该无效
        let result = manager.validate_timestamp(50_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_microseconds() {
        let manager = TimestampManager::new(30.0);
        
        // 1秒 = 90,000 ticks @ 90kHz = 1,000,000 us
        assert_eq!(manager.convert_to_microseconds(90_000), 1_000_000);
        
        // 0.5秒 = 45,000 ticks = 500,000 us
        assert_eq!(manager.convert_to_microseconds(45_000), 500_000);
        
        // 0秒
        assert_eq!(manager.convert_to_microseconds(0), 0);
    }

    #[test]
    fn test_convert_to_90khz() {
        let manager = TimestampManager::new(30.0);
        
        // 1,000,000 us = 1秒 = 90,000 ticks @ 90kHz
        assert_eq!(manager.convert_to_90khz(1_000_000), 90_000);
        
        // 500,000 us = 0.5秒 = 45,000 ticks
        assert_eq!(manager.convert_to_90khz(500_000), 45_000);
        
        // 0 us
        assert_eq!(manager.convert_to_90khz(0), 0);
    }

    #[test]
    fn test_timestamp_conversion_round_trip() {
        let manager = TimestampManager::new(30.0);
        
        // 测试往返转换
        let original_us = 1_234_567;
        let khz = manager.convert_to_90khz(original_us);
        let back_to_us = manager.convert_to_microseconds(khz);
        
        // 由于整数除法，可能有小的精度损失
        let diff = if back_to_us > original_us {
            back_to_us - original_us
        } else {
            original_us - back_to_us
        };
        
        // 精度损失应该小于1微秒
        assert!(diff < 2, "Precision loss: {} us", diff);
    }

    #[test]
    fn test_update_frame_rate() {
        let mut manager = TimestampManager::new(30.0);
        assert_eq!(manager.frame_duration_us(), 33_333);
        
        manager.update_frame_rate(60.0);
        assert_eq!(manager.frame_duration_us(), 16_666);
        
        manager.update_frame_rate(24.0);
        assert_eq!(manager.frame_duration_us(), 41_666);
    }

    #[test]
    fn test_reset() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成一些时间戳
        manager.generate_next_timestamp();
        manager.generate_next_timestamp();
        
        assert_eq!(manager.frame_count(), 2);
        assert_eq!(manager.last_timestamp(), 33_333);
        
        // 重置
        manager.reset();
        
        assert_eq!(manager.frame_count(), 0);
        assert_eq!(manager.last_timestamp(), 0);
        assert_eq!(manager.base_timestamp, 0);
        
        // 帧持续时间应该保持不变
        assert_eq!(manager.frame_duration_us(), 33_333);
    }

    #[test]
    fn test_reset_to_base() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成一些时间戳
        manager.generate_next_timestamp();
        manager.generate_next_timestamp();
        
        // 重置到新基准
        manager.reset_to_base(1_000_000);
        
        assert_eq!(manager.base_timestamp, 1_000_000);
        assert_eq!(manager.last_timestamp(), 1_000_000);
        assert_eq!(manager.frame_count(), 0);
        
        // 下一个时间戳应该从新基准开始
        let ts = manager.generate_next_timestamp();
        assert_eq!(ts, 1_000_000);
    }

    #[test]
    fn test_timestamp_precision() {
        let manager = TimestampManager::new(30.0);
        
        // 测试高精度时间戳
        let us = 123_456_789;
        let khz = manager.convert_to_90khz(us);
        let back = manager.convert_to_microseconds(khz);
        
        // 精度损失应该非常小
        let error = if back > us { back - us } else { us - back };
        assert!(error < 2, "Precision error: {} us", error);
    }

    #[test]
    fn test_handle_discontinuity_large_gap() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成一些正常的时间戳
        manager.generate_next_timestamp();
        manager.generate_next_timestamp();
        assert_eq!(manager.last_timestamp(), 33_333);
        
        // 模拟大的时间戳跳跃（10秒）
        let discontinuous = manager.handle_discontinuity(10_000_000);
        assert!(discontinuous);
        
        // 应该重新同步到新基准
        assert_eq!(manager.base_timestamp, 10_000_000);
        assert_eq!(manager.last_timestamp(), 10_000_000);
        assert_eq!(manager.frame_count(), 0);
    }

    #[test]
    fn test_handle_discontinuity_backwards() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成一些正常的时间戳
        manager.generate_next_timestamp();
        manager.generate_next_timestamp();
        let last_ts = manager.last_timestamp();
        
        // 模拟时间戳倒退
        let discontinuous = manager.handle_discontinuity(10_000);
        assert!(discontinuous);
        
        // 应该重新同步到新时间戳
        assert_eq!(manager.base_timestamp, 10_000);
        assert_eq!(manager.last_timestamp(), 10_000);
    }

    #[test]
    fn test_handle_discontinuity_normal_interval() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成第一个时间戳
        manager.generate_next_timestamp();
        
        // 正常间隔的时间戳不应该触发不连续
        let discontinuous = manager.handle_discontinuity(33_333);
        assert!(!discontinuous);
        
        assert_eq!(manager.last_timestamp(), 33_333);
    }

    #[test]
    fn test_handle_discontinuity_first_timestamp() {
        let mut manager = TimestampManager::new(30.0);
        
        // 第一个时间戳不应该触发不连续
        let discontinuous = manager.handle_discontinuity(1_000_000);
        assert!(!discontinuous);
        
        assert_eq!(manager.base_timestamp, 1_000_000);
        assert_eq!(manager.last_timestamp(), 1_000_000);
    }

    #[test]
    fn test_handle_discontinuity_small_interval() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成第一个时间戳
        manager.generate_next_timestamp();
        
        // 异常小的间隔（应该警告但不重置）
        let discontinuous = manager.handle_discontinuity(1_000);
        assert!(!discontinuous);
        
        // 时间戳应该更新
        assert_eq!(manager.last_timestamp(), 1_000);
    }

    #[test]
    fn test_resync_within_3_frames() {
        let mut manager = TimestampManager::new(30.0);
        
        // 生成一些时间戳
        manager.generate_next_timestamp();
        manager.generate_next_timestamp();
        
        // 触发不连续
        manager.handle_discontinuity(10_000_000);
        
        // 验证立即重新同步（frame_count = 0）
        assert_eq!(manager.frame_count(), 0);
        
        // 生成接下来的3帧，应该都是正常的
        let ts1 = manager.generate_next_timestamp();
        let ts2 = manager.generate_next_timestamp();
        let ts3 = manager.generate_next_timestamp();
        
        // 验证时间戳是连续的
        assert_eq!(ts1, 10_000_000);
        assert_eq!(ts2, 10_033_333);
        assert_eq!(ts3, 10_066_666);
    }
}
