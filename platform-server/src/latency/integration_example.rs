// 延迟监控集成示例
//
// 本文件展示如何在UnifiedStreamHandler中集成延迟监控系统

#![allow(dead_code)]

use crate::latency::{
    AlertBroadcaster, EndToEndLatencyMonitor, LatencyStatisticsManager, LatencyThresholds,
};
use crate::streaming::VideoSegment;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 集成了延迟监控的流处理器示例
pub struct MonitoredStreamHandler {
    /// 端到端延迟监控器
    latency_monitor: Arc<EndToEndLatencyMonitor>,
    /// 延迟统计管理器
    stats_manager: Arc<LatencyStatisticsManager>,
    /// 告警广播器
    alert_broadcaster: Arc<AlertBroadcaster>,
    /// 活动会话
    active_sessions: Arc<RwLock<Vec<Uuid>>>,
}

impl MonitoredStreamHandler {
    /// 创建新的监控流处理器
    pub fn new() -> Self {
        let thresholds = LatencyThresholds {
            transmission_ms: 100,
            processing_ms: 50,
            distribution_ms: 50,
            end_to_end_ms: 200,
        };

        Self {
            latency_monitor: Arc::new(EndToEndLatencyMonitor::new(thresholds)),
            stats_manager: Arc::new(LatencyStatisticsManager::new()),
            alert_broadcaster: Arc::new(AlertBroadcaster::with_defaults()),
            active_sessions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 启动会话监控
    pub async fn start_session_monitoring(&self, session_id: Uuid) {
        // 启动统计
        self.stats_manager.start_session(session_id);

        // 广播会话开始
        self.alert_broadcaster.broadcast_session_started(session_id);

        // 添加到活动会话列表
        let mut sessions = self.active_sessions.write().await;
        sessions.push(session_id);

        tracing::info!("Started monitoring for session {}", session_id);
    }

    /// 处理接收到的分片（从设备端）
    pub async fn handle_received_segment(&self, session_id: Uuid, mut segment: VideoSegment) {
        // 记录平台端接收时间 (T2)
        let receive_time = SystemTime::now();
        segment.receive_time = Some(receive_time);

        // 如果分片有设备端时间戳，记录到监控器
        // 注意：实际实现中，设备端时间戳应该在分片中携带
        // 这里假设segment.timestamp可以转换为SystemTime
        let device_send_time = SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs_f64(segment.timestamp);
        self.latency_monitor
            .record_device_send(segment.segment_id, device_send_time);

        // 记录平台端接收时间
        self.latency_monitor
            .record_platform_receive(segment.segment_id, receive_time);

        // 检查是否有告警
        if let Some(alerts) = self.latency_monitor.get_alerts(&segment.segment_id) {
            for alert in alerts {
                self.alert_broadcaster
                    .broadcast_latency_alert(session_id, alert);
            }
        }

        tracing::debug!(
            "Received segment {} for session {}",
            segment.segment_id,
            session_id
        );
    }

    /// 处理转发分片（到前端）
    pub async fn handle_forward_segment(&self, session_id: Uuid, mut segment: VideoSegment) {
        // 记录平台端转发时间 (T3)
        let forward_time = SystemTime::now();
        segment.forward_time = Some(forward_time);

        self.latency_monitor
            .record_platform_forward(segment.segment_id, forward_time);

        // 计算处理延迟并记录到统计
        if let Some(receive_time) = segment.receive_time {
            if let Ok(processing_latency) = forward_time.duration_since(receive_time) {
                self.stats_manager.record_segment_latency(
                    &session_id,
                    processing_latency,
                    segment.data.len(),
                );
            }
        }

        // 检查是否有新的告警
        if let Some(alerts) = self.latency_monitor.get_alerts(&segment.segment_id) {
            for alert in alerts {
                self.alert_broadcaster
                    .broadcast_latency_alert(session_id, alert);
            }
        }

        tracing::debug!(
            "Forwarded segment {} for session {}",
            segment.segment_id,
            session_id
        );
    }

    /// 处理客户端播放确认
    pub async fn handle_client_play_ack(&self, session_id: Uuid, segment_id: Uuid) {
        // 记录客户端播放时间 (T4)
        let play_time = SystemTime::now();

        self.latency_monitor
            .record_client_play(segment_id, play_time);

        // 检查端到端延迟告警
        if let Some(alerts) = self.latency_monitor.get_alerts(&segment_id) {
            for alert in alerts {
                self.alert_broadcaster
                    .broadcast_latency_alert(session_id, alert);
            }
        }

        // 清理分片的监控数据（可选，取决于是否需要保留历史）
        // self.latency_monitor.cleanup_segment(&segment_id);

        tracing::debug!(
            "Client played segment {} for session {}",
            segment_id,
            session_id
        );
    }

    /// 定期广播统计更新（每秒调用一次）
    pub async fn broadcast_statistics_update(&self, session_id: Uuid) {
        if let Some(statistics) = self.stats_manager.get_statistics(&session_id) {
            self.alert_broadcaster
                .broadcast_statistics_update(session_id, statistics);

            tracing::debug!(
                "Broadcasted statistics update for session {}",
                session_id
            );
        }
    }

    /// 停止会话监控
    pub async fn stop_session_monitoring(&self, session_id: Uuid) {
        // 停止统计
        self.stats_manager.stop_session(&session_id);

        // 广播会话结束
        self.alert_broadcaster.broadcast_session_ended(session_id);

        // 从活动会话列表移除
        let mut sessions = self.active_sessions.write().await;
        sessions.retain(|id| *id != session_id);

        tracing::info!("Stopped monitoring for session {}", session_id);
    }

    /// 获取监控器引用（用于API端点）
    pub fn get_latency_monitor(&self) -> Arc<EndToEndLatencyMonitor> {
        Arc::clone(&self.latency_monitor)
    }

    /// 获取统计管理器引用（用于API端点）
    pub fn get_stats_manager(&self) -> Arc<LatencyStatisticsManager> {
        Arc::clone(&self.stats_manager)
    }

    /// 获取告警广播器引用（用于API端点）
    pub fn get_alert_broadcaster(&self) -> Arc<AlertBroadcaster> {
        Arc::clone(&self.alert_broadcaster)
    }
}

impl Default for MonitoredStreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 启动统计更新任务
///
/// 每秒广播一次统计更新
pub async fn start_statistics_update_task(handler: Arc<MonitoredStreamHandler>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;

            // 获取所有活动会话
            let sessions = handler.active_sessions.read().await;

            // 为每个会话广播统计更新
            for session_id in sessions.iter() {
                handler.broadcast_statistics_update(*session_id).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitored_stream_handler_creation() {
        let handler = MonitoredStreamHandler::new();
        let sessions = handler.active_sessions.read().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let handler = MonitoredStreamHandler::new();
        let session_id = Uuid::new_v4();

        // 启动会话
        handler.start_session_monitoring(session_id).await;
        let sessions = handler.active_sessions.read().await;
        assert_eq!(sessions.len(), 1);
        drop(sessions);

        // 停止会话
        handler.stop_session_monitoring(session_id).await;
        let sessions = handler.active_sessions.read().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_segment_processing() {
        let handler = MonitoredStreamHandler::new();
        let session_id = Uuid::new_v4();

        handler.start_session_monitoring(session_id).await;

        let segment = VideoSegment {
            segment_id: Uuid::new_v4(),
            timestamp: 1.0,
            duration: 0.033,
            data: vec![0u8; 1024],
            is_keyframe: true,
            format: crate::streaming::SegmentFormat::FMP4,
            source_type: crate::streaming::SegmentSourceType::Live,
            receive_time: None,
            forward_time: None,
        };

        // 处理接收
        handler
            .handle_received_segment(session_id, segment.clone())
            .await;

        // 处理转发
        handler
            .handle_forward_segment(session_id, segment.clone())
            .await;

        // 处理播放确认
        handler
            .handle_client_play_ack(session_id, segment.segment_id)
            .await;

        // 验证统计数据
        let stats = handler.stats_manager.get_statistics(&session_id);
        assert!(stats.is_some());
    }
}
