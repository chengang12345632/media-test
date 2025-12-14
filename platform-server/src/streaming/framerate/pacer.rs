// 帧率控制器实现
//
// 本模块实现了精确的发送速率控制功能，支持倍速播放和网络自适应调整。

use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// 帧率控制器
/// 
/// 负责控制视频分片的发送速率，确保播放速度正确。
/// 支持倍速播放（0.25x-4x）和网络自适应调整。
pub struct FrameRatePacer {
    /// 目标帧率（帧/秒）
    target_fps: f64,
    /// 播放速率（倍速）
    playback_rate: f64,
    /// 上次发送时间
    last_send_time: Option<Instant>,
    /// 基础帧间隔（微秒）
    base_frame_interval_us: u64,
}

impl FrameRatePacer {
    /// 创建新的帧率控制器
    /// 
    /// # 参数
    /// 
    /// * `target_fps` - 目标帧率（帧/秒）
    /// 
    /// # 示例
    /// 
    /// ```
    /// let pacer = FrameRatePacer::new(30.0);
    /// ```
    pub fn new(target_fps: f64) -> Self {
        let base_frame_interval_us = if target_fps > 0.0 {
            (1_000_000.0 / target_fps) as u64
        } else {
            33_333 // 默认30fps
        };

        info!(
            "Creating FrameRatePacer: target_fps={:.2}, base_interval={}us",
            target_fps, base_frame_interval_us
        );

        Self {
            target_fps,
            playback_rate: 1.0,
            last_send_time: None,
            base_frame_interval_us,
        }
    }

    /// 获取目标帧率
    pub fn target_fps(&self) -> f64 {
        self.target_fps
    }

    /// 获取播放速率
    pub fn playback_rate(&self) -> f64 {
        self.playback_rate
    }

    /// 获取基础帧间隔（微秒）
    pub fn base_frame_interval_us(&self) -> u64 {
        self.base_frame_interval_us
    }

    /// 计算下一帧的发送延迟
    /// 
    /// 根据目标帧率、分片包含的帧数和播放速率计算发送延迟。
    /// 公式：delay = (frames_in_segment / target_fps) / playback_rate 秒
    /// 
    /// # 参数
    /// 
    /// * `frames_in_segment` - 分片包含的帧数（如果未知则为1）
    /// 
    /// # 返回
    /// 
    /// 返回应该等待的时间（Duration）
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut pacer = FrameRatePacer::new(30.0);
    /// let delay = pacer.calculate_send_delay(1);
    /// // delay ≈ 33.33ms (1/30秒)
    /// ```
    pub fn calculate_send_delay(&self, frames_in_segment: u32) -> Duration {
        // 计算基础间隔（秒）
        let base_interval_sec = frames_in_segment as f64 / self.target_fps;
        
        // 应用倍速调整
        let adjusted_interval_sec = base_interval_sec / self.playback_rate;
        
        // 转换为Duration
        let delay = Duration::from_secs_f64(adjusted_interval_sec);
        
        debug!(
            "Calculated send delay: {:.3}ms (frames={}, fps={:.2}, rate={:.2}x)",
            delay.as_secs_f64() * 1000.0,
            frames_in_segment,
            self.target_fps,
            self.playback_rate
        );
        
        delay
    }

    /// 设置播放速率（倍速）
    /// 
    /// # 参数
    /// 
    /// * `rate` - 播放速率（0.25x - 4.0x）
    /// 
    /// # 返回
    /// 
    /// 如果速率有效返回Ok(())，否则返回错误信息
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut pacer = FrameRatePacer::new(30.0);
    /// pacer.set_playback_rate(2.0)?; // 2倍速
    /// ```
    pub fn set_playback_rate(&mut self, rate: f64) -> Result<(), String> {
        // 验证速率范围
        if rate < 0.25 || rate > 4.0 {
            return Err(format!(
                "Invalid playback rate: {:.2} (must be between 0.25 and 4.0)",
                rate
            ));
        }

        info!(
            "Setting playback rate: {:.2}x -> {:.2}x",
            self.playback_rate, rate
        );

        self.playback_rate = rate;
        Ok(())
    }

    /// 等待直到可以发送下一帧
    /// 
    /// 根据上次发送时间和计算的延迟，等待到正确的发送时间。
    /// 如果已经超时（延迟已过），则不等待。
    /// 
    /// # 参数
    /// 
    /// * `frames_in_segment` - 分片包含的帧数
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut pacer = FrameRatePacer::new(30.0);
    /// pacer.wait_for_next_frame(1).await;
    /// // 发送分片
    /// ```
    pub async fn wait_for_next_frame(&mut self, frames_in_segment: u32) {
        let target_delay = self.calculate_send_delay(frames_in_segment);
        
        if let Some(last_send) = self.last_send_time {
            let elapsed = last_send.elapsed();
            
            if elapsed < target_delay {
                let wait_time = target_delay - elapsed;
                
                debug!(
                    "Waiting {:.3}ms before next frame (elapsed: {:.3}ms, target: {:.3}ms)",
                    wait_time.as_secs_f64() * 1000.0,
                    elapsed.as_secs_f64() * 1000.0,
                    target_delay.as_secs_f64() * 1000.0
                );
                
                tokio::time::sleep(wait_time).await;
            } else {
                // 已经超时，不等待
                let overtime = elapsed - target_delay;
                if overtime.as_millis() > 10 {
                    warn!(
                        "Frame send is late by {:.3}ms (elapsed: {:.3}ms, target: {:.3}ms)",
                        overtime.as_secs_f64() * 1000.0,
                        elapsed.as_secs_f64() * 1000.0,
                        target_delay.as_secs_f64() * 1000.0
                    );
                }
            }
        }
        
        // 记录发送时间
        self.last_send_time = Some(Instant::now());
    }

    /// 根据网络条件调整发送速率
    /// 
    /// 这是一个可选功能，用于在网络拥塞时动态调整发送速率。
    /// 当前为占位实现，可在后续迭代中完善。
    /// 
    /// # 参数
    /// 
    /// * `bandwidth_mbps` - 可用带宽（Mbps）
    /// * `buffer_ms` - 缓冲区大小（毫秒）
    /// 
    /// # 示例
    /// 
    /// ```
    /// let mut pacer = FrameRatePacer::new(30.0);
    /// pacer.adjust_for_network(5.0, 100);
    /// ```
    pub fn adjust_for_network(&mut self, bandwidth_mbps: f64, buffer_ms: u64) {
        // TODO: 实现网络自适应调整逻辑
        // 
        // 基本思路：
        // 1. 如果带宽不足，降低发送速率
        // 2. 如果缓冲区过低，增加发送速率
        // 3. 如果缓冲区过高，降低发送速率
        
        debug!(
            "Network adjustment (placeholder): bandwidth={:.2}Mbps, buffer={}ms",
            bandwidth_mbps, buffer_ms
        );
        
        // 占位实现：仅记录日志
        // 实际实现应该根据网络条件调整playback_rate或target_fps
    }

    /// 更新目标帧率
    /// 
    /// 当检测到帧率变化时，更新目标帧率和基础帧间隔。
    /// 
    /// # 参数
    /// 
    /// * `new_fps` - 新的目标帧率
    pub fn update_target_fps(&mut self, new_fps: f64) {
        let old_fps = self.target_fps;
        self.target_fps = new_fps;
        self.base_frame_interval_us = if new_fps > 0.0 {
            (1_000_000.0 / new_fps) as u64
        } else {
            33_333
        };

        info!(
            "Target FPS updated: {:.2} -> {:.2} fps (interval: {}us)",
            old_fps, new_fps, self.base_frame_interval_us
        );
    }

    /// 重置帧率控制器
    /// 
    /// 重置发送时间记录，但保持帧率和倍速设置。
    pub fn reset(&mut self) {
        info!("Resetting FrameRatePacer");
        self.last_send_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacer_creation() {
        let pacer = FrameRatePacer::new(30.0);
        assert_eq!(pacer.target_fps(), 30.0);
        assert_eq!(pacer.playback_rate(), 1.0);
        assert_eq!(pacer.base_frame_interval_us(), 33_333);
        assert!(pacer.last_send_time.is_none());
    }

    #[test]
    fn test_pacer_creation_60fps() {
        let pacer = FrameRatePacer::new(60.0);
        assert_eq!(pacer.target_fps(), 60.0);
        assert_eq!(pacer.base_frame_interval_us(), 16_666);
    }

    #[test]
    fn test_calculate_send_delay_1x() {
        let pacer = FrameRatePacer::new(30.0);
        
        // 1帧 @ 30fps = 33.33ms
        let delay = pacer.calculate_send_delay(1);
        let expected_ms = 1000.0 / 30.0;
        let actual_ms = delay.as_secs_f64() * 1000.0;
        
        // 允许1ms误差
        assert!((actual_ms - expected_ms).abs() < 1.0, 
                "Expected ~{:.2}ms, got {:.2}ms", expected_ms, actual_ms);
    }

    #[test]
    fn test_calculate_send_delay_multiple_frames() {
        let pacer = FrameRatePacer::new(30.0);
        
        // 3帧 @ 30fps = 100ms
        let delay = pacer.calculate_send_delay(3);
        let expected_ms = 3000.0 / 30.0;
        let actual_ms = delay.as_secs_f64() * 1000.0;
        
        // 允许1ms误差
        assert!((actual_ms - expected_ms).abs() < 1.0,
                "Expected ~{:.2}ms, got {:.2}ms", expected_ms, actual_ms);
    }

    #[test]
    fn test_calculate_send_delay_2x_speed() {
        let mut pacer = FrameRatePacer::new(30.0);
        pacer.set_playback_rate(2.0).unwrap();
        
        // 1帧 @ 30fps @ 2x = 16.67ms
        let delay = pacer.calculate_send_delay(1);
        let expected_ms = 1000.0 / 30.0 / 2.0;
        let actual_ms = delay.as_secs_f64() * 1000.0;
        
        // 允许1ms误差
        assert!((actual_ms - expected_ms).abs() < 1.0,
                "Expected ~{:.2}ms, got {:.2}ms", expected_ms, actual_ms);
    }

    #[test]
    fn test_calculate_send_delay_half_speed() {
        let mut pacer = FrameRatePacer::new(30.0);
        pacer.set_playback_rate(0.5).unwrap();
        
        // 1帧 @ 30fps @ 0.5x = 66.67ms
        let delay = pacer.calculate_send_delay(1);
        let expected_ms = 1000.0 / 30.0 / 0.5;
        let actual_ms = delay.as_secs_f64() * 1000.0;
        
        // 允许1ms误差
        assert!((actual_ms - expected_ms).abs() < 1.0,
                "Expected ~{:.2}ms, got {:.2}ms", expected_ms, actual_ms);
    }

    #[test]
    fn test_set_playback_rate_valid() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 测试有效速率
        assert!(pacer.set_playback_rate(0.25).is_ok());
        assert_eq!(pacer.playback_rate(), 0.25);
        
        assert!(pacer.set_playback_rate(1.0).is_ok());
        assert_eq!(pacer.playback_rate(), 1.0);
        
        assert!(pacer.set_playback_rate(2.0).is_ok());
        assert_eq!(pacer.playback_rate(), 2.0);
        
        assert!(pacer.set_playback_rate(4.0).is_ok());
        assert_eq!(pacer.playback_rate(), 4.0);
    }

    #[test]
    fn test_set_playback_rate_invalid() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 测试无效速率
        assert!(pacer.set_playback_rate(0.1).is_err());
        assert!(pacer.set_playback_rate(5.0).is_err());
        assert!(pacer.set_playback_rate(-1.0).is_err());
        
        // 速率应该保持不变
        assert_eq!(pacer.playback_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_wait_for_next_frame_first_call() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 第一次调用应该不等待（没有上次发送时间）
        let start = Instant::now();
        pacer.wait_for_next_frame(1).await;
        let elapsed = start.elapsed();
        
        // 应该几乎立即返回（<5ms）
        assert!(elapsed.as_millis() < 5);
        assert!(pacer.last_send_time.is_some());
    }

    #[tokio::test]
    async fn test_wait_for_next_frame_timing() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 第一次调用
        pacer.wait_for_next_frame(1).await;
        
        // 立即第二次调用，应该等待约33ms
        let start = Instant::now();
        pacer.wait_for_next_frame(1).await;
        let elapsed = start.elapsed();
        
        // 应该等待约33ms（允许±5ms误差）
        let expected_ms = 1000.0 / 30.0;
        let actual_ms = elapsed.as_secs_f64() * 1000.0;
        
        assert!(
            (actual_ms - expected_ms).abs() < 5.0,
            "Expected ~{:.2}ms, got {:.2}ms",
            expected_ms,
            actual_ms
        );
    }

    #[tokio::test]
    async fn test_wait_for_next_frame_no_wait_if_late() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 第一次调用
        pacer.wait_for_next_frame(1).await;
        
        // 等待超过帧间隔
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // 第二次调用应该不等待（已经超时）
        let start = Instant::now();
        pacer.wait_for_next_frame(1).await;
        let elapsed = start.elapsed();
        
        // 应该几乎立即返回（<5ms）
        assert!(elapsed.as_millis() < 5);
    }

    #[test]
    fn test_update_target_fps() {
        let mut pacer = FrameRatePacer::new(30.0);
        assert_eq!(pacer.target_fps(), 30.0);
        assert_eq!(pacer.base_frame_interval_us(), 33_333);
        
        pacer.update_target_fps(60.0);
        assert_eq!(pacer.target_fps(), 60.0);
        assert_eq!(pacer.base_frame_interval_us(), 16_666);
        
        pacer.update_target_fps(24.0);
        assert_eq!(pacer.target_fps(), 24.0);
        assert_eq!(pacer.base_frame_interval_us(), 41_666);
    }

    #[test]
    fn test_reset() {
        let mut pacer = FrameRatePacer::new(30.0);
        
        // 设置一些状态
        pacer.last_send_time = Some(Instant::now());
        pacer.set_playback_rate(2.0).unwrap();
        
        // 重置
        pacer.reset();
        
        // 发送时间应该被清除
        assert!(pacer.last_send_time.is_none());
        
        // 但帧率和倍速应该保持
        assert_eq!(pacer.target_fps(), 30.0);
        assert_eq!(pacer.playback_rate(), 2.0);
    }

    #[test]
    fn test_delay_calculation_precision() {
        let pacer = FrameRatePacer::new(30.0);
        
        // 测试多种帧数和倍速组合
        let test_cases = vec![
            (1, 1.0, 33.33),
            (2, 1.0, 66.67),
            (1, 2.0, 16.67),
            (1, 0.5, 66.67),
            (3, 2.0, 50.0),
        ];
        
        for (frames, rate, expected_ms) in test_cases {
            let mut pacer = FrameRatePacer::new(30.0);
            pacer.set_playback_rate(rate).unwrap();
            
            let delay = pacer.calculate_send_delay(frames);
            let actual_ms = delay.as_secs_f64() * 1000.0;
            
            // 允许1ms误差
            assert!(
                (actual_ms - expected_ms).abs() < 1.0,
                "frames={}, rate={:.1}x: expected {:.2}ms, got {:.2}ms",
                frames,
                rate,
                expected_ms,
                actual_ms
            );
        }
    }
}
