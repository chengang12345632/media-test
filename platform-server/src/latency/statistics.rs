// 延迟统计模块
//
// 本模块实现了延迟统计功能，包括：
// - 平均延迟、最小延迟、最大延迟
// - P50、P95、P99延迟百分位数
// - 吞吐量统计
// - 丢包率统计

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use uuid::Uuid;

/// 统计窗口大小（保留最近N个测量值）
const STATS_WINDOW_SIZE: usize = 1000;

/// 延迟统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStatistics {
    /// 会话ID
    pub session_id: Uuid,
    /// 统计开始时间（跳过序列化，使用默认值）
    #[serde(skip, default = "Instant::now")]
    pub start_time: Instant,
    /// 最后更新时间（跳过序列化，使用默认值）
    #[serde(skip, default = "Instant::now")]
    pub last_update: Instant,
    /// 总分片数
    pub total_segments: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 平均延迟（毫秒）
    pub average_latency_ms: f64,
    /// 当前延迟（毫秒）
    pub current_latency_ms: f64,
    /// 最小延迟（毫秒）
    pub min_latency_ms: u64,
    /// 最大延迟（毫秒）
    pub max_latency_ms: u64,
    /// P50延迟（毫秒）
    pub p50_latency_ms: u64,
    /// P95延迟（毫秒）
    pub p95_latency_ms: u64,
    /// P99延迟（毫秒）
    pub p99_latency_ms: u64,
    /// 吞吐量（Mbps）
    pub throughput_mbps: f64,
    /// 丢包率（0.0-1.0）
    pub packet_loss_rate: f64,
}

/// 会话统计数据（内部使用）
struct SessionStats {
    session_id: Uuid,
    start_time: Instant,
    last_update: Instant,
    total_segments: u64,
    total_bytes: u64,
    expected_segments: u64,
    lost_segments: u64,
    latency_history: VecDeque<Duration>,
}

impl SessionStats {
    fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            start_time: Instant::now(),
            last_update: Instant::now(),
            total_segments: 0,
            total_bytes: 0,
            expected_segments: 0,
            lost_segments: 0,
            latency_history: VecDeque::with_capacity(STATS_WINDOW_SIZE),
        }
    }

    /// 添加延迟测量
    fn add_latency(&mut self, latency: Duration) {
        if self.latency_history.len() >= STATS_WINDOW_SIZE {
            self.latency_history.pop_front();
        }
        self.latency_history.push_back(latency);
        self.last_update = Instant::now();
    }

    /// 计算平均延迟
    fn average_latency(&self) -> f64 {
        if self.latency_history.is_empty() {
            return 0.0;
        }

        let sum: Duration = self.latency_history.iter().sum();
        let avg_duration = sum / self.latency_history.len() as u32;
        avg_duration.as_secs_f64() * 1000.0
    }

    /// 计算最小延迟
    fn min_latency(&self) -> u64 {
        self.latency_history
            .iter()
            .map(|d| d.as_millis() as u64)
            .min()
            .unwrap_or(0)
    }

    /// 计算最大延迟
    fn max_latency(&self) -> u64 {
        self.latency_history
            .iter()
            .map(|d| d.as_millis() as u64)
            .max()
            .unwrap_or(0)
    }

    /// 计算百分位延迟
    fn percentile_latency(&self, percentile: f64) -> u64 {
        if self.latency_history.is_empty() {
            return 0;
        }

        let mut sorted: Vec<u64> = self
            .latency_history
            .iter()
            .map(|d| d.as_millis() as u64)
            .collect();
        sorted.sort_unstable();

        let index = ((sorted.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
        sorted.get(index).copied().unwrap_or(0)
    }

    /// 计算吞吐量（Mbps）
    fn throughput_mbps(&self) -> f64 {
        let elapsed = self.last_update.duration_since(self.start_time);
        if elapsed.as_secs() == 0 {
            return 0.0;
        }

        let bits = self.total_bytes as f64 * 8.0;
        let seconds = elapsed.as_secs_f64();
        bits / seconds / 1_000_000.0
    }

    /// 计算丢包率
    fn packet_loss_rate(&self) -> f64 {
        if self.expected_segments == 0 {
            return 0.0;
        }
        self.lost_segments as f64 / self.expected_segments as f64
    }

    /// 转换为公共统计数据
    fn to_statistics(&self) -> LatencyStatistics {
        LatencyStatistics {
            session_id: self.session_id,
            start_time: self.start_time,
            last_update: self.last_update,
            total_segments: self.total_segments,
            total_bytes: self.total_bytes,
            average_latency_ms: self.average_latency(),
            current_latency_ms: self
                .latency_history
                .back()
                .map(|d| d.as_secs_f64() * 1000.0)
                .unwrap_or(0.0),
            min_latency_ms: self.min_latency(),
            max_latency_ms: self.max_latency(),
            p50_latency_ms: self.percentile_latency(0.50),
            p95_latency_ms: self.percentile_latency(0.95),
            p99_latency_ms: self.percentile_latency(0.99),
            throughput_mbps: self.throughput_mbps(),
            packet_loss_rate: self.packet_loss_rate(),
        }
    }
}

/// 延迟统计管理器
///
/// 管理所有会话的延迟统计数据，提供实时性能指标。
#[derive(Clone)]
pub struct LatencyStatisticsManager {
    sessions: Arc<DashMap<Uuid, SessionStats>>,
}

impl LatencyStatisticsManager {
    /// 创建新的统计管理器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// 开始统计会话
    pub fn start_session(&self, session_id: Uuid) {
        info!("Starting latency statistics for session {}", session_id);
        let stats = SessionStats::new(session_id);
        self.sessions.insert(session_id, stats);
    }

    /// 记录分片延迟
    pub fn record_segment_latency(
        &self,
        session_id: &Uuid,
        latency: Duration,
        segment_size: usize,
    ) {
        if let Some(mut stats) = self.sessions.get_mut(session_id) {
            stats.add_latency(latency);
            stats.total_segments += 1;
            stats.total_bytes += segment_size as u64;
            stats.expected_segments += 1;

            debug!(
                "Recorded latency for session {}: {}ms",
                session_id,
                latency.as_millis()
            );
        }
    }

    /// 记录丢失的分片
    pub fn record_lost_segment(&self, session_id: &Uuid) {
        if let Some(mut stats) = self.sessions.get_mut(session_id) {
            stats.lost_segments += 1;
            stats.expected_segments += 1;
            debug!("Recorded lost segment for session {}", session_id);
        }
    }

    /// 获取会话统计数据
    pub fn get_statistics(&self, session_id: &Uuid) -> Option<LatencyStatistics> {
        self.sessions
            .get(session_id)
            .map(|stats| stats.to_statistics())
    }

    /// 获取所有会话的统计数据
    pub fn get_all_statistics(&self) -> Vec<LatencyStatistics> {
        self.sessions
            .iter()
            .map(|entry| entry.value().to_statistics())
            .collect()
    }

    /// 停止统计会话
    pub fn stop_session(&self, session_id: &Uuid) {
        info!("Stopping latency statistics for session {}", session_id);
        self.sessions.remove(session_id);
    }

    /// 清理所有会话
    pub fn clear_all(&self) {
        info!("Clearing all latency statistics");
        self.sessions.clear();
    }
}

impl Default for LatencyStatisticsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_session_stats_creation() {
        let session_id = Uuid::new_v4();
        let stats = SessionStats::new(session_id);

        assert_eq!(stats.session_id, session_id);
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.total_bytes, 0);
        assert!(stats.latency_history.is_empty());
    }

    #[test]
    fn test_add_latency() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        stats.add_latency(Duration::from_millis(50));
        stats.add_latency(Duration::from_millis(60));
        stats.add_latency(Duration::from_millis(70));

        assert_eq!(stats.latency_history.len(), 3);
        assert!(stats.average_latency() > 0.0);
    }

    #[test]
    fn test_latency_calculations() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        // 添加一些测试数据
        for i in 1..=100 {
            stats.add_latency(Duration::from_millis(i));
        }

        // 验证统计计算
        assert!(stats.average_latency() > 0.0);
        assert_eq!(stats.min_latency(), 1);
        assert_eq!(stats.max_latency(), 100);
        assert!(stats.percentile_latency(0.50) > 0);
        assert!(stats.percentile_latency(0.95) > 0);
        assert!(stats.percentile_latency(0.99) > 0);
    }

    #[test]
    fn test_percentile_calculation() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        // 添加已知数据
        for i in 1..=100 {
            stats.add_latency(Duration::from_millis(i));
        }

        let p50 = stats.percentile_latency(0.50);
        let p95 = stats.percentile_latency(0.95);
        let p99 = stats.percentile_latency(0.99);

        // P50应该接近50
        assert!(p50 >= 45 && p50 <= 55);
        // P95应该接近95
        assert!(p95 >= 90 && p95 <= 100);
        // P99应该接近99
        assert!(p99 >= 95 && p99 <= 100);
    }

    #[test]
    fn test_throughput_calculation() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        // 模拟1秒内传输1MB数据
        stats.total_bytes = 1_000_000;
        thread::sleep(Duration::from_millis(100));
        stats.last_update = Instant::now();

        let throughput = stats.throughput_mbps();
        assert!(throughput > 0.0);
    }

    #[test]
    fn test_packet_loss_rate() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        stats.expected_segments = 100;
        stats.lost_segments = 5;

        let loss_rate = stats.packet_loss_rate();
        assert_eq!(loss_rate, 0.05); // 5%丢包率
    }

    #[test]
    fn test_statistics_manager() {
        let manager = LatencyStatisticsManager::new();
        let session_id = Uuid::new_v4();

        // 开始会话
        manager.start_session(session_id);

        // 记录一些延迟
        manager.record_segment_latency(&session_id, Duration::from_millis(50), 1024);
        manager.record_segment_latency(&session_id, Duration::from_millis(60), 1024);
        manager.record_segment_latency(&session_id, Duration::from_millis(70), 1024);

        // 获取统计数据
        let stats = manager.get_statistics(&session_id);
        assert!(stats.is_some());

        let s = stats.unwrap();
        assert_eq!(s.session_id, session_id);
        assert_eq!(s.total_segments, 3);
        assert_eq!(s.total_bytes, 3072);
        assert!(s.average_latency_ms > 0.0);

        // 停止会话
        manager.stop_session(&session_id);
        assert!(manager.get_statistics(&session_id).is_none());
    }

    #[test]
    fn test_lost_segment_tracking() {
        let manager = LatencyStatisticsManager::new();
        let session_id = Uuid::new_v4();

        manager.start_session(session_id);

        // 记录一些正常分片和丢失分片
        manager.record_segment_latency(&session_id, Duration::from_millis(50), 1024);
        manager.record_lost_segment(&session_id);
        manager.record_segment_latency(&session_id, Duration::from_millis(60), 1024);

        let stats = manager.get_statistics(&session_id).unwrap();
        assert_eq!(stats.total_segments, 2);
        assert!(stats.packet_loss_rate > 0.0);
    }

    #[test]
    fn test_window_size_limit() {
        let session_id = Uuid::new_v4();
        let mut stats = SessionStats::new(session_id);

        // 添加超过窗口大小的数据
        for i in 0..(STATS_WINDOW_SIZE + 100) {
            stats.add_latency(Duration::from_millis(i as u64));
        }

        // 验证窗口大小限制
        assert_eq!(stats.latency_history.len(), STATS_WINDOW_SIZE);
    }

    #[test]
    fn test_get_all_statistics() {
        let manager = LatencyStatisticsManager::new();

        // 创建多个会话
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        manager.start_session(session1);
        manager.start_session(session2);
        manager.start_session(session3);

        // 记录一些数据
        manager.record_segment_latency(&session1, Duration::from_millis(50), 1024);
        manager.record_segment_latency(&session2, Duration::from_millis(60), 1024);
        manager.record_segment_latency(&session3, Duration::from_millis(70), 1024);

        // 获取所有统计数据
        let all_stats = manager.get_all_statistics();
        assert_eq!(all_stats.len(), 3);
    }
}
