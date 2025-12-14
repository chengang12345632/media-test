// 延迟告警广播器
//
// 本模块实现了延迟告警的广播功能，通过WebSocket将告警推送到前端。

use crate::latency::{LatencyAlertType, LatencyStatistics};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 告警消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertMessage {
    /// 延迟告警
    LatencyAlert {
        session_id: Uuid,
        alert: LatencyAlertType,
        timestamp: u64,
    },
    /// 统计更新
    StatisticsUpdate {
        session_id: Uuid,
        statistics: LatencyStatistics,
        timestamp: u64,
    },
    /// 会话开始
    SessionStarted { session_id: Uuid, timestamp: u64 },
    /// 会话结束
    SessionEnded { session_id: Uuid, timestamp: u64 },
}

/// 告警广播器
///
/// 管理告警消息的广播，支持多个客户端订阅。
#[derive(Clone)]
pub struct AlertBroadcaster {
    /// 广播通道发送端
    tx: broadcast::Sender<AlertMessage>,
    /// 会话订阅者计数
    subscribers: Arc<DashMap<Uuid, usize>>,
}

impl AlertBroadcaster {
    /// 创建新的告警广播器
    ///
    /// # 参数
    ///
    /// - `capacity`: 广播通道容量
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            subscribers: Arc::new(DashMap::new()),
        }
    }

    /// 使用默认容量创建广播器
    pub fn with_defaults() -> Self {
        Self::new(1000)
    }

    /// 订阅告警消息
    ///
    /// # 返回
    ///
    /// 返回一个接收端，用于接收告警消息
    pub fn subscribe(&self) -> broadcast::Receiver<AlertMessage> {
        debug!("New subscriber connected to alert broadcaster");
        self.tx.subscribe()
    }

    /// 订阅特定会话的告警
    pub fn subscribe_session(&self, session_id: Uuid) -> broadcast::Receiver<AlertMessage> {
        info!("New subscriber for session {}", session_id);
        self.subscribers
            .entry(session_id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        self.tx.subscribe()
    }

    /// 取消订阅会话
    pub fn unsubscribe_session(&self, session_id: &Uuid) {
        if let Some(mut entry) = self.subscribers.get_mut(session_id) {
            *entry -= 1;
            if *entry == 0 {
                drop(entry);
                self.subscribers.remove(session_id);
                info!("All subscribers unsubscribed from session {}", session_id);
            }
        }
    }

    /// 广播延迟告警
    pub fn broadcast_latency_alert(&self, session_id: Uuid, alert: LatencyAlertType) {
        let message = AlertMessage::LatencyAlert {
            session_id,
            alert,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match self.tx.send(message) {
            Ok(count) => {
                debug!(
                    "Broadcasted latency alert for session {} to {} subscribers",
                    session_id, count
                );
            }
            Err(e) => {
                warn!("Failed to broadcast latency alert: {}", e);
            }
        }
    }

    /// 广播统计更新
    pub fn broadcast_statistics_update(&self, session_id: Uuid, statistics: LatencyStatistics) {
        let message = AlertMessage::StatisticsUpdate {
            session_id,
            statistics,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match self.tx.send(message) {
            Ok(count) => {
                debug!(
                    "Broadcasted statistics update for session {} to {} subscribers",
                    session_id, count
                );
            }
            Err(e) => {
                warn!("Failed to broadcast statistics update: {}", e);
            }
        }
    }

    /// 广播会话开始消息
    pub fn broadcast_session_started(&self, session_id: Uuid) {
        let message = AlertMessage::SessionStarted {
            session_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match self.tx.send(message) {
            Ok(count) => {
                info!(
                    "Broadcasted session started for {} to {} subscribers",
                    session_id, count
                );
            }
            Err(e) => {
                warn!("Failed to broadcast session started: {}", e);
            }
        }
    }

    /// 广播会话结束消息
    pub fn broadcast_session_ended(&self, session_id: Uuid) {
        let message = AlertMessage::SessionEnded {
            session_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match self.tx.send(message) {
            Ok(count) => {
                info!(
                    "Broadcasted session ended for {} to {} subscribers",
                    session_id, count
                );
            }
            Err(e) => {
                warn!("Failed to broadcast session ended: {}", e);
            }
        }
    }

    /// 获取订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// 获取特定会话的订阅者数量
    pub fn session_subscriber_count(&self, session_id: &Uuid) -> usize {
        self.subscribers
            .get(session_id)
            .map(|count| *count)
            .unwrap_or(0)
    }
}

/// 告警过滤器
///
/// 用于过滤特定会话或类型的告警消息
pub struct AlertFilter {
    session_id: Option<Uuid>,
}

impl AlertFilter {
    /// 创建新的告警过滤器
    pub fn new() -> Self {
        Self { session_id: None }
    }

    /// 设置会话ID过滤
    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// 检查消息是否通过过滤器
    pub fn matches(&self, message: &AlertMessage) -> bool {
        if let Some(filter_session_id) = self.session_id {
            match message {
                AlertMessage::LatencyAlert { session_id, .. }
                | AlertMessage::StatisticsUpdate { session_id, .. }
                | AlertMessage::SessionStarted { session_id, .. }
                | AlertMessage::SessionEnded { session_id, .. } => {
                    *session_id == filter_session_id
                }
            }
        } else {
            true
        }
    }
}

impl Default for AlertFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latency::LatencyAlertType;

    #[tokio::test]
    async fn test_broadcaster_creation() {
        let broadcaster = AlertBroadcaster::with_defaults();
        assert_eq!(broadcaster.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_subscribe() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let _rx = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_latency_alert() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let mut rx = broadcaster.subscribe();

        let session_id = Uuid::new_v4();
        let segment_id = Uuid::new_v4();
        let alert = LatencyAlertType::TransmissionLatency {
            segment_id,
            latency_ms: 150,
            threshold_ms: 100,
        };

        broadcaster.broadcast_latency_alert(session_id, alert.clone());

        let message = rx.recv().await.unwrap();
        match message {
            AlertMessage::LatencyAlert {
                session_id: msg_session_id,
                alert: msg_alert,
                ..
            } => {
                assert_eq!(msg_session_id, session_id);
                match (alert, msg_alert) {
                    (
                        LatencyAlertType::TransmissionLatency {
                            segment_id: id1, ..
                        },
                        LatencyAlertType::TransmissionLatency {
                            segment_id: id2, ..
                        },
                    ) => {
                        assert_eq!(id1, id2);
                    }
                    _ => panic!("Alert type mismatch"),
                }
            }
            _ => panic!("Expected LatencyAlert message"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_statistics_update() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let mut rx = broadcaster.subscribe();

        let session_id = Uuid::new_v4();
        let statistics = LatencyStatistics {
            session_id,
            start_time: std::time::Instant::now(),
            last_update: std::time::Instant::now(),
            total_segments: 100,
            total_bytes: 102400,
            average_latency_ms: 50.0,
            current_latency_ms: 45.0,
            min_latency_ms: 30,
            max_latency_ms: 80,
            p50_latency_ms: 50,
            p95_latency_ms: 75,
            p99_latency_ms: 78,
            throughput_mbps: 5.2,
            packet_loss_rate: 0.01,
        };

        broadcaster.broadcast_statistics_update(session_id, statistics.clone());

        let message = rx.recv().await.unwrap();
        match message {
            AlertMessage::StatisticsUpdate {
                session_id: msg_session_id,
                statistics: msg_stats,
                ..
            } => {
                assert_eq!(msg_session_id, session_id);
                assert_eq!(msg_stats.total_segments, 100);
            }
            _ => panic!("Expected StatisticsUpdate message"),
        }
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let mut rx = broadcaster.subscribe();

        let session_id = Uuid::new_v4();

        // 广播会话开始
        broadcaster.broadcast_session_started(session_id);
        let message = rx.recv().await.unwrap();
        match message {
            AlertMessage::SessionStarted {
                session_id: msg_session_id,
                ..
            } => {
                assert_eq!(msg_session_id, session_id);
            }
            _ => panic!("Expected SessionStarted message"),
        }

        // 广播会话结束
        broadcaster.broadcast_session_ended(session_id);
        let message = rx.recv().await.unwrap();
        match message {
            AlertMessage::SessionEnded {
                session_id: msg_session_id,
                ..
            } => {
                assert_eq!(msg_session_id, session_id);
            }
            _ => panic!("Expected SessionEnded message"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();
        let mut rx3 = broadcaster.subscribe();

        assert_eq!(broadcaster.subscriber_count(), 3);

        let session_id = Uuid::new_v4();
        broadcaster.broadcast_session_started(session_id);

        // 所有订阅者都应该收到消息
        let _ = rx1.recv().await.unwrap();
        let _ = rx2.recv().await.unwrap();
        let _ = rx3.recv().await.unwrap();
    }

    #[tokio::test]
    async fn test_session_subscription() {
        let broadcaster = AlertBroadcaster::with_defaults();
        let session_id = Uuid::new_v4();

        let _rx1 = broadcaster.subscribe_session(session_id);
        assert_eq!(broadcaster.session_subscriber_count(&session_id), 1);

        let _rx2 = broadcaster.subscribe_session(session_id);
        assert_eq!(broadcaster.session_subscriber_count(&session_id), 2);

        broadcaster.unsubscribe_session(&session_id);
        assert_eq!(broadcaster.session_subscriber_count(&session_id), 1);

        broadcaster.unsubscribe_session(&session_id);
        assert_eq!(broadcaster.session_subscriber_count(&session_id), 0);
    }

    #[test]
    fn test_alert_filter() {
        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();

        let filter = AlertFilter::new().with_session(session_id);

        let message1 = AlertMessage::SessionStarted {
            session_id,
            timestamp: 0,
        };
        let message2 = AlertMessage::SessionStarted {
            session_id: other_session_id,
            timestamp: 0,
        };

        assert!(filter.matches(&message1));
        assert!(!filter.matches(&message2));
    }

    #[test]
    fn test_alert_filter_no_session() {
        let filter = AlertFilter::new();

        let message = AlertMessage::SessionStarted {
            session_id: Uuid::new_v4(),
            timestamp: 0,
        };

        // 没有会话过滤时，所有消息都应该通过
        assert!(filter.matches(&message));
    }
}
