# 延迟监控系统

本模块实现了完整的端到端延迟监控系统，用于追踪视频分片从设备端到前端播放的全过程。

## 架构概览

```
设备端时间戳 → 平台端接收时间戳 → 平台端转发时间戳 → 前端播放时间戳
↓              ↓                ↓                ↓
T1(发送)      T2(接收)         T3(转发)         T4(播放)

延迟计算:
- 传输延迟 = T2 - T1  (设备→平台)
- 处理延迟 = T3 - T2  (平台接收→转发)
- 分发延迟 = T4 - T3  (平台→前端)
- 端到端延迟 = T4 - T1
```

## 核心组件

### 1. EndToEndLatencyMonitor

端到端延迟监控器，追踪完整的延迟链路。

```rust
use crate::latency::{EndToEndLatencyMonitor, LatencyThresholds};

// 创建监控器
let thresholds = LatencyThresholds {
    transmission_ms: 100,
    processing_ms: 50,
    distribution_ms: 50,
    end_to_end_ms: 200,
};
let monitor = EndToEndLatencyMonitor::new(thresholds);

// 记录时间戳
monitor.record_device_send(segment_id, t1);
monitor.record_platform_receive(segment_id, t2);
monitor.record_platform_forward(segment_id, t3);
monitor.record_client_play(segment_id, t4);

// 获取延迟分解
let breakdown = monitor.get_measurement(&segment_id);
```

### 2. LatencyStatisticsManager

延迟统计管理器，计算平均延迟、百分位数等统计指标。

```rust
use crate::latency::LatencyStatisticsManager;

let stats_manager = LatencyStatisticsManager::new();

// 开始会话统计
stats_manager.start_session(session_id);

// 记录分片延迟
stats_manager.record_segment_latency(&session_id, latency, segment_size);

// 获取统计数据
let statistics = stats_manager.get_statistics(&session_id);
println!("Average latency: {}ms", statistics.average_latency_ms);
println!("P95 latency: {}ms", statistics.p95_latency_ms);
println!("Throughput: {} Mbps", statistics.throughput_mbps);
```

### 3. AlertBroadcaster

告警广播器，通过SSE实时推送延迟告警和统计更新。

```rust
use crate::latency::AlertBroadcaster;

let broadcaster = AlertBroadcaster::with_defaults();

// 订阅告警
let mut rx = broadcaster.subscribe();

// 广播告警
broadcaster.broadcast_latency_alert(session_id, alert);
broadcaster.broadcast_statistics_update(session_id, statistics);

// 接收告警
tokio::spawn(async move {
    while let Ok(message) = rx.recv().await {
        println!("Received alert: {:?}", message);
    }
});
```

## HTTP API端点

### 获取会话统计

```http
GET /api/v1/latency/sessions/{session_id}/statistics

Response:
{
  "status": "success",
  "data": {
    "session_id": "uuid",
    "total_segments": 1000,
    "average_latency_ms": 85.5,
    "p95_latency_ms": 120,
    "throughput_mbps": 5.2,
    "packet_loss_rate": 0.01
  }
}
```

### 获取分片延迟分解

```http
GET /api/v1/latency/segments/{segment_id}/breakdown

Response:
{
  "status": "success",
  "data": {
    "transmission_latency_ms": 45,
    "processing_latency_ms": 3,
    "distribution_latency_ms": 25,
    "end_to_end_latency_ms": 73
  }
}
```

### 订阅延迟告警（SSE）

```http
GET /api/v1/latency/alerts

Response (SSE Stream):
event: message
data: {"type":"LatencyAlert","session_id":"uuid","alert":{...}}

event: message
data: {"type":"StatisticsUpdate","session_id":"uuid","statistics":{...}}
```

### 订阅特定会话的告警

```http
GET /api/v1/latency/sessions/{session_id}/alerts
```

## 集成示例

### 在UnifiedStreamHandler中集成

```rust
use crate::latency::{
    EndToEndLatencyMonitor, LatencyStatisticsManager, AlertBroadcaster,
    LatencyThresholds,
};
use std::sync::Arc;

pub struct UnifiedStreamHandler {
    // ... 其他字段
    latency_monitor: Arc<EndToEndLatencyMonitor>,
    stats_manager: Arc<LatencyStatisticsManager>,
    alert_broadcaster: Arc<AlertBroadcaster>,
}

impl UnifiedStreamHandler {
    pub fn new() -> Self {
        let thresholds = LatencyThresholds::default();
        
        Self {
            // ... 其他字段初始化
            latency_monitor: Arc::new(EndToEndLatencyMonitor::new(thresholds)),
            stats_manager: Arc::new(LatencyStatisticsManager::new()),
            alert_broadcaster: Arc::new(AlertBroadcaster::with_defaults()),
        }
    }

    // 启动会话时
    pub async fn start_stream(&self, session_id: Uuid) {
        self.stats_manager.start_session(session_id);
        self.alert_broadcaster.broadcast_session_started(session_id);
    }

    // 接收分片时
    pub async fn on_segment_received(&self, session_id: Uuid, segment: &mut VideoSegment) {
        let receive_time = SystemTime::now();
        segment.receive_time = Some(receive_time);
        
        // 记录设备端发送时间（从分片元数据获取）
        let device_time = /* 从分片获取 */;
        self.latency_monitor.record_device_send(segment.segment_id, device_time);
        self.latency_monitor.record_platform_receive(segment.segment_id, receive_time);
    }

    // 转发分片时
    pub async fn on_segment_forward(&self, session_id: Uuid, segment: &mut VideoSegment) {
        let forward_time = SystemTime::now();
        segment.forward_time = Some(forward_time);
        
        self.latency_monitor.record_platform_forward(segment.segment_id, forward_time);
        
        // 记录统计
        if let Some(receive_time) = segment.receive_time {
            if let Ok(latency) = forward_time.duration_since(receive_time) {
                self.stats_manager.record_segment_latency(
                    &session_id,
                    latency,
                    segment.data.len(),
                );
            }
        }
        
        // 检查并广播告警
        if let Some(alerts) = self.latency_monitor.get_alerts(&segment.segment_id) {
            for alert in alerts {
                self.alert_broadcaster.broadcast_latency_alert(session_id, alert);
            }
        }
    }

    // 停止会话时
    pub async fn stop_stream(&self, session_id: Uuid) {
        self.stats_manager.stop_session(&session_id);
        self.alert_broadcaster.broadcast_session_ended(session_id);
    }
}
```

### 启动定期统计更新任务

```rust
// 每秒广播一次统计更新
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        // 获取所有活动会话
        for session_id in active_sessions.iter() {
            if let Some(stats) = stats_manager.get_statistics(session_id) {
                alert_broadcaster.broadcast_statistics_update(*session_id, stats);
            }
        }
    }
});
```

## 前端集成

### 订阅延迟告警

```typescript
// 创建SSE连接
const eventSource = new EventSource('/api/v1/latency/alerts');

eventSource.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch (message.type) {
    case 'LatencyAlert':
      console.warn('Latency alert:', message.alert);
      showLatencyWarning(message);
      break;
      
    case 'StatisticsUpdate':
      updateLatencyDisplay(message.statistics);
      break;
      
    case 'SessionStarted':
      console.log('Session started:', message.session_id);
      break;
      
    case 'SessionEnded':
      console.log('Session ended:', message.session_id);
      break;
  }
};

eventSource.onerror = (error) => {
  console.error('SSE connection error:', error);
  // 实现重连逻辑
};
```

### 显示延迟统计

```typescript
function updateLatencyDisplay(statistics: LatencyStatistics) {
  document.getElementById('avg-latency').textContent = 
    `${statistics.average_latency_ms.toFixed(1)}ms`;
  
  document.getElementById('p95-latency').textContent = 
    `${statistics.p95_latency_ms}ms`;
  
  document.getElementById('throughput').textContent = 
    `${statistics.throughput_mbps.toFixed(2)} Mbps`;
  
  document.getElementById('packet-loss').textContent = 
    `${(statistics.packet_loss_rate * 100).toFixed(2)}%`;
}
```

## 配置

### 延迟阈值配置

```rust
use crate::latency::LatencyThresholds;

let thresholds = LatencyThresholds {
    transmission_ms: 100,   // 传输延迟阈值
    processing_ms: 50,      // 处理延迟阈值
    distribution_ms: 50,    // 分发延迟阈值
    end_to_end_ms: 200,     // 端到端延迟阈值
};
```

### 统计窗口大小

统计管理器默认保留最近1000个测量值。可以在`statistics.rs`中修改`STATS_WINDOW_SIZE`常量。

## 性能考虑

1. **内存使用**: 每个分片的时间戳数据会保留在内存中，直到调用`cleanup_segment`。建议定期清理已完成的分片数据。

2. **并发性能**: 使用`DashMap`实现无锁并发访问，支持高并发场景。

3. **广播容量**: 告警广播器默认容量为1000条消息。如果消费速度慢，旧消息会被丢弃。

## 测试

运行单元测试：

```bash
cargo test --package platform-server --lib latency
```

运行集成测试：

```bash
cargo test --package platform-server --test latency_integration
```

## 日志

延迟监控系统使用`tracing`记录详细日志：

- `INFO`: 会话生命周期事件
- `DEBUG`: 延迟测量和统计更新
- `WARN`: 延迟告警触发

启用调试日志：

```bash
RUST_LOG=platform_server::latency=debug cargo run
```

## 故障排查

### 问题：统计数据不更新

检查是否调用了`start_session`和`record_segment_latency`。

### 问题：告警未收到

1. 检查SSE连接是否建立
2. 检查延迟是否真的超过阈值
3. 检查广播器是否有订阅者

### 问题：内存占用过高

定期调用`cleanup_segment`清理已完成的分片数据。

## 参考

- 需求文档: `.kiro/specs/unified-low-latency-streaming/requirements.md` (需求8)
- 设计文档: `.kiro/specs/unified-low-latency-streaming/design.md`
- 任务列表: `.kiro/specs/unified-low-latency-streaming/tasks.md` (任务8)
