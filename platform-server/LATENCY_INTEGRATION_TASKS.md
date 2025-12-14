# 延迟监控系统集成任务清单

## 当前状态

### ✅ 已完成
1. **后端核心模块** - 所有延迟监控核心功能已实现
   - `EndToEndLatencyMonitor` - 端到端延迟监控
   - `LatencyStatisticsManager` - 统计管理
   - `AlertBroadcaster` - 告警广播
   - HTTP API处理器 (`latency_handlers.rs`)

2. **前端显示组件** - 延迟监控UI已实现
   - `LatencyMonitor.tsx` - 延迟监控组件
   - 已集成到 `UnifiedMSEPlayer`
   - 已集成到 `WebCodecsPlayer`

### ⏳ 待完成

## 任务列表

### 任务 1: 在 UnifiedStreamHandler 中集成延迟监控 ⭐ 高优先级

**目标**: 让流处理器能够记录和追踪延迟

**步骤**:

1. **修改 `platform-server/src/streaming/handler.rs`**

```rust
// 在文件顶部添加导入
use crate::latency::{
    EndToEndLatencyMonitor, LatencyStatisticsManager, 
    AlertBroadcaster, LatencyThresholds,
};

// 在 UnifiedStreamHandler 结构体中添加字段
pub struct UnifiedStreamHandler {
    // ... 现有字段
    
    /// 端到端延迟监控器
    latency_monitor: Arc<EndToEndLatencyMonitor>,
    /// 延迟统计管理器
    stats_manager: Arc<LatencyStatisticsManager>,
    /// 告警广播器
    alert_broadcaster: Arc<AlertBroadcaster>,
}

// 在 new() 方法中初始化
impl UnifiedStreamHandler {
    pub fn new() -> Self {
        let thresholds = LatencyThresholds {
            transmission_ms: 100,
            processing_ms: 50,
            distribution_ms: 50,
            end_to_end_ms: 200,
        };
        
        Self {
            // ... 现有字段初始化
            latency_monitor: Arc::new(EndToEndLatencyMonitor::new(thresholds)),
            stats_manager: Arc::new(LatencyStatisticsManager::new()),
            alert_broadcaster: Arc::new(AlertBroadcaster::with_defaults()),
        }
    }
    
    // 添加获取器方法
    pub fn get_latency_monitor(&self) -> Arc<EndToEndLatencyMonitor> {
        Arc::clone(&self.latency_monitor)
    }
    
    pub fn get_stats_manager(&self) -> Arc<LatencyStatisticsManager> {
        Arc::clone(&self.stats_manager)
    }
    
    pub fn get_alert_broadcaster(&self) -> Arc<AlertBroadcaster> {
        Arc::clone(&self.alert_broadcaster)
    }
}
```

2. **在流会话启动时开始监控**

```rust
// 在 start_stream 或类似方法中
pub async fn start_stream(&self, session_id: Uuid, source: Box<dyn StreamSource>) {
    // 启动统计
    self.stats_manager.start_session(session_id);
    
    // 广播会话开始
    self.alert_broadcaster.broadcast_session_started(session_id);
    
    // ... 其他启动逻辑
}
```

3. **在接收分片时记录时间戳**

```rust
// 在接收到分片时
pub async fn on_segment_received(&self, session_id: Uuid, segment: &mut VideoSegment) {
    let receive_time = SystemTime::now();
    segment.receive_time = Some(receive_time);
    
    // 记录平台端接收时间
    self.latency_monitor.record_platform_receive(
        segment.segment_id, 
        receive_time
    );
    
    // 如果分片有设备端时间戳，也记录
    // 注意：需要在VideoSegment中添加device_send_time字段
}
```

4. **在转发分片时记录时间戳和统计**

```rust
// 在转发分片时
pub async fn on_segment_forward(&self, session_id: Uuid, segment: &mut VideoSegment) {
    let forward_time = SystemTime::now();
    segment.forward_time = Some(forward_time);
    
    // 记录平台端转发时间
    self.latency_monitor.record_platform_forward(
        segment.segment_id, 
        forward_time
    );
    
    // 计算处理延迟并记录统计
    if let Some(receive_time) = segment.receive_time {
        if let Ok(processing_latency) = forward_time.duration_since(receive_time) {
            self.stats_manager.record_segment_latency(
                &session_id,
                processing_latency,
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
```

5. **在停止流时清理**

```rust
pub async fn stop_stream(&self, session_id: Uuid) {
    // 停止统计
    self.stats_manager.stop_session(&session_id);
    
    // 广播会话结束
    self.alert_broadcaster.broadcast_session_ended(session_id);
    
    // ... 其他清理逻辑
}
```

---

### 任务 2: 添加延迟监控 HTTP 路由 ⭐ 高优先级

**目标**: 暴露延迟监控API端点

**步骤**:

1. **修改 `platform-server/src/http3/routes.rs`**

```rust
use crate::http3::latency_handlers;

pub fn create_router(
    device_manager: DeviceManager,
    recording_manager: RecordingManager,
    distribution_manager: DistributionManager,
    latency_monitor: LatencyMonitor,
    stream_handler: Arc<UnifiedStreamHandler>,
) -> Router {
    // 创建延迟监控状态
    let latency_state = (
        stream_handler.get_latency_monitor(),
        stream_handler.get_stats_manager(),
        stream_handler.get_alert_broadcaster(),
    );
    
    Router::new()
        // ... 现有路由
        
        // 延迟监控API
        .route(
            "/api/v1/latency/health",
            get(latency_handlers::latency_health_check),
        )
        .route(
            "/api/v1/latency/statistics",
            get(latency_handlers::get_all_statistics),
        )
        .route(
            "/api/v1/latency/sessions/:session_id/statistics",
            get(latency_handlers::get_session_statistics),
        )
        .route(
            "/api/v1/latency/segments/:segment_id/breakdown",
            get(latency_handlers::get_segment_breakdown),
        )
        .route(
            "/api/v1/latency/alerts",
            get(latency_handlers::subscribe_alerts),
        )
        .route(
            "/api/v1/latency/sessions/:session_id/alerts",
            get(latency_handlers::subscribe_session_alerts),
        )
        .route(
            "/api/v1/latency/config",
            put(latency_handlers::update_latency_config),
        )
        
        // 主状态
        .with_state((
            device_manager,
            recording_manager,
            distribution_manager,
            latency_monitor,
            stream_handler.clone(),
        ))
        
        // 延迟监控状态（嵌套路由）
        .nest(
            "/api/v1/latency",
            Router::new()
                .with_state(latency_state)
        )
        
        .layer(CorsLayer::permissive())
}
```

**注意**: 由于Axum的状态管理限制，可能需要调整路由结构或使用Extension。

---

### 任务 3: 启动统计更新任务 ⭐ 高优先级

**目标**: 每秒广播一次统计更新

**步骤**:

1. **在 `platform-server/src/main.rs` 或服务器启动代码中添加**

```rust
// 启动统计更新任务
let stream_handler_clone = stream_handler.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        // 获取所有活动会话
        let sessions = stream_handler_clone.get_active_sessions();
        
        // 为每个会话广播统计更新
        for session_id in sessions {
            if let Some(stats) = stream_handler_clone
                .get_stats_manager()
                .get_statistics(&session_id) 
            {
                stream_handler_clone
                    .get_alert_broadcaster()
                    .broadcast_statistics_update(session_id, stats);
            }
        }
    }
});
```

2. **在 UnifiedStreamHandler 中添加获取活动会话的方法**

```rust
impl UnifiedStreamHandler {
    pub fn get_active_sessions(&self) -> Vec<Uuid> {
        self.sessions
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }
}
```

---

### 任务 4: 在 VideoSegment 中添加设备端时间戳 🔧 可选

**目标**: 支持完整的端到端延迟测量（T1→T4）

**步骤**:

1. **修改 `platform-server/src/streaming/source.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSegment {
    pub segment_id: Uuid,
    pub timestamp: f64,
    pub duration: f64,
    pub data: Vec<u8>,
    pub is_keyframe: bool,
    pub format: SegmentFormat,
    
    /// 设备端发送时间 (T1)
    #[serde(skip)]
    pub device_send_time: Option<SystemTime>,
    
    /// 平台端接收时间 (T2)
    #[serde(skip)]
    pub receive_time: Option<SystemTime>,
    
    /// 平台端转发时间 (T3)
    #[serde(skip)]
    pub forward_time: Option<SystemTime>,
}
```

2. **在设备端发送分片时记录时间戳**

```rust
// 在 device-simulator 或设备端代码中
let segment = VideoSegment {
    segment_id: Uuid::new_v4(),
    timestamp: current_timestamp,
    duration: frame_duration,
    data: encoded_data,
    is_keyframe: is_keyframe,
    format: SegmentFormat::H264Raw,
    device_send_time: Some(SystemTime::now()),
    receive_time: None,
    forward_time: None,
};
```

---

### 任务 5: 测试和验证 ✅ 必需

**目标**: 确保延迟监控系统正常工作

**测试步骤**:

1. **启动后端服务**
   ```bash
   cd platform-server
   cargo run
   ```

2. **启动前端服务**
   ```bash
   cd web-frontend
   npm run dev
   ```

3. **测试直通播放**
   - 打开浏览器访问前端
   - 选择设备，启动直通播放
   - 检查延迟监控组件是否显示数据
   - 检查浏览器控制台的SSE连接

4. **测试录像回放**
   - 选择录像文件，启动回放
   - 检查延迟监控组件是否显示数据

5. **测试API端点**
   ```bash
   # 健康检查
   curl http://localhost:8443/api/v1/latency/health
   
   # 获取所有统计
   curl http://localhost:8443/api/v1/latency/statistics
   
   # 获取特定会话统计
   curl http://localhost:8443/api/v1/latency/sessions/{session_id}/statistics
   ```

6. **测试SSE告警**
   ```bash
   # 使用curl订阅告警
   curl -N http://localhost:8443/api/v1/latency/alerts
   ```

---

### 任务 6: 性能优化 🚀 可选

**目标**: 优化延迟监控的性能开销

**优化项**:

1. **定期清理旧的分片数据**
   ```rust
   // 在统计更新任务中添加清理逻辑
   tokio::spawn(async move {
       let mut interval = tokio::time::interval(Duration::from_secs(60));
       
       loop {
           interval.tick().await;
           
           // 清理超过5分钟的分片数据
           let cutoff_time = SystemTime::now() - Duration::from_secs(300);
           latency_monitor.cleanup_old_segments(cutoff_time);
       }
   });
   ```

2. **限制统计窗口大小**
   - 已在 `statistics.rs` 中实现（STATS_WINDOW_SIZE = 1000）

3. **使用采样减少开销**
   - 对于高吞吐量场景，可以只监控部分分片

---

## 优先级总结

### 🔴 必须完成（核心功能）
1. ✅ 任务 1: 在 UnifiedStreamHandler 中集成延迟监控
2. ✅ 任务 2: 添加延迟监控 HTTP 路由
3. ✅ 任务 3: 启动统计更新任务
4. ✅ 任务 5: 测试和验证

### 🟡 建议完成（增强功能）
5. 任务 4: 在 VideoSegment 中添加设备端时间戳
6. 任务 6: 性能优化

---

## 快速开始指南

### 最小可行集成（15分钟）

1. **修改 handler.rs** (5分钟)
   - 添加延迟监控字段
   - 在 new() 中初始化
   - 添加获取器方法

2. **修改 routes.rs** (5分钟)
   - 添加延迟监控路由
   - 配置状态

3. **启动统计任务** (5分钟)
   - 在 main.rs 中添加定时任务

4. **测试** (5分钟)
   - 启动服务
   - 打开前端
   - 验证延迟数据显示

---

## 故障排查

### 问题：前端显示"等待延迟数据..."

**检查清单**:
- [ ] 后端是否启动？
- [ ] 延迟监控路由是否添加？
- [ ] 统计更新任务是否启动？
- [ ] CORS是否配置正确？
- [ ] 浏览器控制台是否有错误？

### 问题：SSE连接失败

**检查清单**:
- [ ] API端点是否正确？
- [ ] 服务器是否支持SSE？
- [ ] 防火墙是否阻止？
- [ ] 检查服务器日志

---

## 参考文档

- 后端实现: `platform-server/src/latency/README.md`
- 实现总结: `platform-server/src/latency/IMPLEMENTATION_SUMMARY.md`
- 集成示例: `platform-server/src/latency/integration_example.rs`
- 前端指南: `web-frontend/src/components/LATENCY_MONITOR_GUIDE.md`

---

## 更新日志

- 2025-12-14: 创建集成任务清单
- 前端延迟监控组件已完成
- 后端核心模块已完成
- 待完成：后端集成和路由配置
