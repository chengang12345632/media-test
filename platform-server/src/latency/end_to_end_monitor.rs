// 端到端延迟监控系统
//
// 本模块实现了完整的端到端延迟监控，追踪视频分片从设备端到前端播放的全过程。
//
// # 延迟监控架构
//
// ```
// 设备端时间戳 → 平台端接收时间戳 → 平台端转发时间戳 → 前端播放时间戳
// ↓              ↓                ↓                ↓
// T1(发送)      T2(接收)         T3(转发)         T4(播放)
//
// 延迟计算:
// - 传输延迟 = T2 - T1
// - 处理延迟 = T3 - T2
// - 分发延迟 = T4 - T3
// - 端到端延迟 = T4 - T1
// ```

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 延迟阈值配置
#[derive(Debug, Clone)]
pub struct LatencyThresholds {
    /// 传输延迟阈值（设备→平台）
    pub transmission_ms: u64,
    /// 处理延迟阈值（平台接收→转发）
    pub processing_ms: u64,
    /// 分发延迟阈值（平台→前端）
    pub distribution_ms: u64,
    /// 端到端延迟阈值
    pub end_to_end_ms: u64,
}

impl Default for LatencyThresholds {
    fn default() -> Self {
        Self {
            transmission_ms: 100,  // 传输延迟阈值: 100ms
            processing_ms: 50,     // 处理延迟阈值: 50ms
            distribution_ms: 50,   // 分发延迟阈值: 50ms
            end_to_end_ms: 200,    // 端到端延迟阈值: 200ms
        }
    }
}

/// 延迟告警类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatencyAlertType {
    /// 传输延迟超标
    TransmissionLatency {
        segment_id: Uuid,
        latency_ms: u64,
        threshold_ms: u64,
    },
    /// 处理延迟超标
    ProcessingLatency {
        segment_id: Uuid,
        latency_ms: u64,
        threshold_ms: u64,
    },
    /// 分发延迟超标
    DistributionLatency {
        segment_id: Uuid,
        latency_ms: u64,
        threshold_ms: u64,
    },
    /// 端到端延迟超标
    EndToEndLatency {
        segment_id: Uuid,
        latency_ms: u64,
        threshold_ms: u64,
    },
}

/// 延迟告警管理器
#[derive(Clone)]
pub struct LatencyAlertManager {
    alerts: Arc<DashMap<Uuid, Vec<LatencyAlertType>>>,
    thresholds: LatencyThresholds,
}

impl LatencyAlertManager {
    pub fn new(thresholds: LatencyThresholds) -> Self {
        Self {
            alerts: Arc::new(DashMap::new()),
            thresholds,
        }
    }

    /// 触发传输延迟告警
    pub fn trigger_transmission_alert(&self, segment_id: Uuid, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        warn!(
            "Transmission latency alert: segment={}, latency={}ms, threshold={}ms",
            segment_id, latency_ms, self.thresholds.transmission_ms
        );

        let alert = LatencyAlertType::TransmissionLatency {
            segment_id,
            latency_ms,
            threshold_ms: self.thresholds.transmission_ms,
        };

        self.alerts
            .entry(segment_id)
            .or_insert_with(Vec::new)
            .push(alert);
    }

    /// 触发处理延迟告警
    pub fn trigger_processing_alert(&self, segment_id: Uuid, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        warn!(
            "Processing latency alert: segment={}, latency={}ms, threshold={}ms",
            segment_id, latency_ms, self.thresholds.processing_ms
        );

        let alert = LatencyAlertType::ProcessingLatency {
            segment_id,
            latency_ms,
            threshold_ms: self.thresholds.processing_ms,
        };

        self.alerts
            .entry(segment_id)
            .or_insert_with(Vec::new)
            .push(alert);
    }

    /// 触发分发延迟告警
    pub fn trigger_distribution_alert(&self, segment_id: Uuid, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        warn!(
            "Distribution latency alert: segment={}, latency={}ms, threshold={}ms",
            segment_id, latency_ms, self.thresholds.distribution_ms
        );

        let alert = LatencyAlertType::DistributionLatency {
            segment_id,
            latency_ms,
            threshold_ms: self.thresholds.distribution_ms,
        };

        self.alerts
            .entry(segment_id)
            .or_insert_with(Vec::new)
            .push(alert);
    }

    /// 触发端到端延迟告警
    pub fn trigger_end_to_end_alert(&self, segment_id: Uuid, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        warn!(
            "End-to-end latency alert: segment={}, latency={}ms, threshold={}ms",
            segment_id, latency_ms, self.thresholds.end_to_end_ms
        );

        let alert = LatencyAlertType::EndToEndLatency {
            segment_id,
            latency_ms,
            threshold_ms: self.thresholds.end_to_end_ms,
        };

        self.alerts
            .entry(segment_id)
            .or_insert_with(Vec::new)
            .push(alert);
    }

    /// 获取分片的所有告警
    pub fn get_alerts(&self, segment_id: &Uuid) -> Option<Vec<LatencyAlertType>> {
        self.alerts.get(segment_id).map(|entry| entry.clone())
    }

    /// 清除分片的告警
    pub fn clear_alerts(&self, segment_id: &Uuid) {
        self.alerts.remove(segment_id);
    }
}

/// 延迟测量数据
#[derive(Debug, Clone)]
struct LatencyMeasurement {
    /// 传输延迟（设备→平台）
    transmission_latency: Option<Duration>,
    /// 处理延迟（平台接收→转发）
    processing_latency: Option<Duration>,
    /// 分发延迟（平台→前端）
    distribution_latency: Option<Duration>,
    /// 端到端延迟
    end_to_end_latency: Option<Duration>,
}

/// 端到端延迟监控器
///
/// 追踪视频分片从设备端到前端播放的完整延迟链路。
#[derive(Clone)]
pub struct EndToEndLatencyMonitor {
    /// 设备端发送时间戳 (T1)
    device_timestamps: Arc<DashMap<Uuid, SystemTime>>,
    /// 平台端接收时间戳 (T2)
    platform_receive_timestamps: Arc<DashMap<Uuid, SystemTime>>,
    /// 平台端转发时间戳 (T3)
    platform_forward_timestamps: Arc<DashMap<Uuid, SystemTime>>,
    /// 客户端播放时间戳 (T4)
    client_play_timestamps: Arc<DashMap<Uuid, SystemTime>>,
    /// 延迟测量结果
    measurements: Arc<DashMap<Uuid, LatencyMeasurement>>,
    /// 延迟告警管理器
    latency_alerts: LatencyAlertManager,
    /// 延迟阈值配置
    thresholds: LatencyThresholds,
}

impl EndToEndLatencyMonitor {
    /// 创建新的端到端延迟监控器
    pub fn new(thresholds: LatencyThresholds) -> Self {
        Self {
            device_timestamps: Arc::new(DashMap::new()),
            platform_receive_timestamps: Arc::new(DashMap::new()),
            platform_forward_timestamps: Arc::new(DashMap::new()),
            client_play_timestamps: Arc::new(DashMap::new()),
            measurements: Arc::new(DashMap::new()),
            latency_alerts: LatencyAlertManager::new(thresholds.clone()),
            thresholds,
        }
    }

    /// 使用默认阈值创建监控器
    pub fn with_defaults() -> Self {
        Self::new(LatencyThresholds::default())
    }

    /// 记录设备端发送时间戳 (T1)
    pub fn record_device_send(&self, segment_id: Uuid, timestamp: SystemTime) {
        debug!("Recording device send time for segment {}", segment_id);
        self.device_timestamps.insert(segment_id, timestamp);
    }

    /// 记录平台端接收时间戳 (T2)
    pub fn record_platform_receive(&self, segment_id: Uuid, timestamp: SystemTime) {
        debug!("Recording platform receive time for segment {}", segment_id);
        self.platform_receive_timestamps
            .insert(segment_id, timestamp);

        // 计算传输延迟 (T2 - T1)
        if let Some(device_time) = self.device_timestamps.get(&segment_id) {
            if let Ok(transmission_latency) = timestamp.duration_since(*device_time) {
                info!(
                    "Transmission latency for segment {}: {}ms",
                    segment_id,
                    transmission_latency.as_millis()
                );

                // 更新测量数据
                self.measurements
                    .entry(segment_id)
                    .or_insert_with(|| LatencyMeasurement {
                        transmission_latency: None,
                        processing_latency: None,
                        distribution_latency: None,
                        end_to_end_latency: None,
                    })
                    .transmission_latency = Some(transmission_latency);

                // 检查传输延迟阈值
                if transmission_latency.as_millis() as u64 > self.thresholds.transmission_ms {
                    self.latency_alerts
                        .trigger_transmission_alert(segment_id, transmission_latency);
                }
            }
        }
    }

    /// 记录平台端转发时间戳 (T3)
    pub fn record_platform_forward(&self, segment_id: Uuid, timestamp: SystemTime) {
        debug!("Recording platform forward time for segment {}", segment_id);
        self.platform_forward_timestamps
            .insert(segment_id, timestamp);

        // 计算处理延迟 (T3 - T2)
        if let Some(receive_time) = self.platform_receive_timestamps.get(&segment_id) {
            if let Ok(processing_latency) = timestamp.duration_since(*receive_time) {
                info!(
                    "Processing latency for segment {}: {}ms",
                    segment_id,
                    processing_latency.as_millis()
                );

                // 更新测量数据
                if let Some(mut measurement) = self.measurements.get_mut(&segment_id) {
                    measurement.processing_latency = Some(processing_latency);
                }

                // 检查处理延迟阈值
                if processing_latency.as_millis() as u64 > self.thresholds.processing_ms {
                    self.latency_alerts
                        .trigger_processing_alert(segment_id, processing_latency);
                }
            }
        }
    }

    /// 记录客户端播放时间戳 (T4)
    pub fn record_client_play(&self, segment_id: Uuid, timestamp: SystemTime) {
        debug!("Recording client play time for segment {}", segment_id);
        self.client_play_timestamps.insert(segment_id, timestamp);

        // 计算分发延迟 (T4 - T3)
        if let Some(forward_time) = self.platform_forward_timestamps.get(&segment_id) {
            if let Ok(distribution_latency) = timestamp.duration_since(*forward_time) {
                info!(
                    "Distribution latency for segment {}: {}ms",
                    segment_id,
                    distribution_latency.as_millis()
                );

                // 更新测量数据
                if let Some(mut measurement) = self.measurements.get_mut(&segment_id) {
                    measurement.distribution_latency = Some(distribution_latency);
                }

                // 检查分发延迟阈值
                if distribution_latency.as_millis() as u64 > self.thresholds.distribution_ms {
                    self.latency_alerts
                        .trigger_distribution_alert(segment_id, distribution_latency);
                }
            }
        }

        // 计算端到端延迟 (T4 - T1)
        if let Some(device_time) = self.device_timestamps.get(&segment_id) {
            if let Ok(end_to_end_latency) = timestamp.duration_since(*device_time) {
                info!(
                    "End-to-end latency for segment {}: {}ms",
                    segment_id,
                    end_to_end_latency.as_millis()
                );

                // 更新测量数据
                if let Some(mut measurement) = self.measurements.get_mut(&segment_id) {
                    measurement.end_to_end_latency = Some(end_to_end_latency);
                }

                // 检查端到端延迟阈值
                if end_to_end_latency.as_millis() as u64 > self.thresholds.end_to_end_ms {
                    self.latency_alerts
                        .trigger_end_to_end_alert(segment_id, end_to_end_latency);
                }

                // 记录成功的端到端测量
                self.record_successful_measurement(segment_id, end_to_end_latency);
            }
        }
    }

    /// 记录成功的端到端测量
    fn record_successful_measurement(&self, segment_id: Uuid, latency: Duration) {
        debug!(
            "Successfully measured end-to-end latency for segment {}: {}ms",
            segment_id,
            latency.as_millis()
        );
    }

    /// 获取分片的延迟测量数据
    pub fn get_measurement(&self, segment_id: &Uuid) -> Option<LatencyBreakdown> {
        self.measurements.get(segment_id).map(|m| LatencyBreakdown {
            transmission_latency_ms: m
                .transmission_latency
                .map(|d| d.as_millis() as u64),
            processing_latency_ms: m.processing_latency.map(|d| d.as_millis() as u64),
            distribution_latency_ms: m.distribution_latency.map(|d| d.as_millis() as u64),
            end_to_end_latency_ms: m.end_to_end_latency.map(|d| d.as_millis() as u64),
        })
    }

    /// 获取分片的告警
    pub fn get_alerts(&self, segment_id: &Uuid) -> Option<Vec<LatencyAlertType>> {
        self.latency_alerts.get_alerts(segment_id)
    }

    /// 清理分片的监控数据
    pub fn cleanup_segment(&self, segment_id: &Uuid) {
        self.device_timestamps.remove(segment_id);
        self.platform_receive_timestamps.remove(segment_id);
        self.platform_forward_timestamps.remove(segment_id);
        self.client_play_timestamps.remove(segment_id);
        self.measurements.remove(segment_id);
        self.latency_alerts.clear_alerts(segment_id);
    }
}

/// 延迟分解数据（用于API返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBreakdown {
    /// 传输延迟（毫秒）
    pub transmission_latency_ms: Option<u64>,
    /// 处理延迟（毫秒）
    pub processing_latency_ms: Option<u64>,
    /// 分发延迟（毫秒）
    pub distribution_latency_ms: Option<u64>,
    /// 端到端延迟（毫秒）
    pub end_to_end_latency_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_latency_thresholds_default() {
        let thresholds = LatencyThresholds::default();
        assert_eq!(thresholds.transmission_ms, 100);
        assert_eq!(thresholds.processing_ms, 50);
        assert_eq!(thresholds.distribution_ms, 50);
        assert_eq!(thresholds.end_to_end_ms, 200);
    }

    #[test]
    fn test_end_to_end_monitor_creation() {
        let monitor = EndToEndLatencyMonitor::with_defaults();
        let segment_id = Uuid::new_v4();

        // 测试记录时间戳
        let t1 = SystemTime::now();
        monitor.record_device_send(segment_id, t1);

        thread::sleep(Duration::from_millis(10));
        let t2 = SystemTime::now();
        monitor.record_platform_receive(segment_id, t2);

        // 验证测量数据
        let measurement = monitor.get_measurement(&segment_id);
        assert!(measurement.is_some());
        let m = measurement.unwrap();
        assert!(m.transmission_latency_ms.is_some());
        assert!(m.transmission_latency_ms.unwrap() >= 10);
    }

    #[test]
    fn test_latency_alert_manager() {
        let thresholds = LatencyThresholds {
            transmission_ms: 50,
            processing_ms: 10,
            distribution_ms: 10,
            end_to_end_ms: 100,
        };
        let alert_manager = LatencyAlertManager::new(thresholds);
        let segment_id = Uuid::new_v4();

        // 触发告警
        alert_manager.trigger_transmission_alert(segment_id, Duration::from_millis(100));

        // 验证告警
        let alerts = alert_manager.get_alerts(&segment_id);
        assert!(alerts.is_some());
        assert_eq!(alerts.unwrap().len(), 1);

        // 清除告警
        alert_manager.clear_alerts(&segment_id);
        assert!(alert_manager.get_alerts(&segment_id).is_none());
    }

    #[test]
    fn test_complete_latency_chain() {
        let monitor = EndToEndLatencyMonitor::with_defaults();
        let segment_id = Uuid::new_v4();

        // 模拟完整的延迟链路
        let t1 = SystemTime::now();
        monitor.record_device_send(segment_id, t1);

        thread::sleep(Duration::from_millis(20));
        let t2 = SystemTime::now();
        monitor.record_platform_receive(segment_id, t2);

        thread::sleep(Duration::from_millis(5));
        let t3 = SystemTime::now();
        monitor.record_platform_forward(segment_id, t3);

        thread::sleep(Duration::from_millis(10));
        let t4 = SystemTime::now();
        monitor.record_client_play(segment_id, t4);

        // 验证所有延迟都被记录
        let measurement = monitor.get_measurement(&segment_id);
        assert!(measurement.is_some());
        let m = measurement.unwrap();
        assert!(m.transmission_latency_ms.is_some());
        assert!(m.processing_latency_ms.is_some());
        assert!(m.distribution_latency_ms.is_some());
        assert!(m.end_to_end_latency_ms.is_some());

        // 验证延迟值的合理性
        assert!(m.transmission_latency_ms.unwrap() >= 20);
        assert!(m.processing_latency_ms.unwrap() >= 5);
        assert!(m.distribution_latency_ms.unwrap() >= 10);
        assert!(m.end_to_end_latency_ms.unwrap() >= 35);
    }
}
