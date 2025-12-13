use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
pub struct LatencyMonitor {
    measurements: Arc<DashMap<Uuid, LatencyMeasurement>>,
}

#[derive(Debug, Clone)]
pub struct LatencyMeasurement {
    pub session_id: Uuid,
    pub start_time: Instant,
    pub segment_count: u64,
    pub total_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

impl LatencyMonitor {
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(DashMap::new()),
        }
    }

    /// 开始监控会话
    pub fn start_session(&self, session_id: Uuid) {
        let measurement = LatencyMeasurement {
            session_id,
            start_time: Instant::now(),
            segment_count: 0,
            total_latency_ms: 0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
        };
        self.measurements.insert(session_id, measurement);
    }

    /// 记录延迟
    pub fn record_latency(&self, session_id: &Uuid, latency: Duration) {
        if let Some(mut entry) = self.measurements.get_mut(session_id) {
            let latency_ms = latency.as_millis() as u64;
            entry.segment_count += 1;
            entry.total_latency_ms += latency_ms;
            entry.min_latency_ms = entry.min_latency_ms.min(latency_ms);
            entry.max_latency_ms = entry.max_latency_ms.max(latency_ms);
        }
    }

    /// 获取平均延迟
    pub fn get_average_latency(&self, session_id: &Uuid) -> Option<u64> {
        self.measurements.get(session_id).map(|entry| {
            if entry.segment_count > 0 {
                entry.total_latency_ms / entry.segment_count
            } else {
                0
            }
        })
    }

    /// 停止监控会话
    pub fn stop_session(&self, session_id: &Uuid) {
        self.measurements.remove(session_id);
    }
}
