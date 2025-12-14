# 前端延迟监控组件使用指南

## 概述

`LatencyMonitor` 组件提供了实时的延迟监控和性能统计显示，已集成到 `UnifiedMSEPlayer` 中。

## 功能特性

### 1. 实时延迟指标

- **平均延迟**: 显示会话的平均延迟
- **当前延迟**: 显示最新分片的延迟
- **吞吐量**: 显示当前的数据传输速率（Mbps）
- **丢包率**: 显示丢包百分比

### 2. 详细统计

- 最小/最大延迟
- P50/P95/P99 百分位延迟
- 总分片数和总字节数

### 3. 实时告警

- 传输延迟告警（设备→平台）
- 处理延迟告警（平台接收→转发）
- 分发延迟告警（平台→前端）
- 端到端延迟告警

### 4. 颜色编码

延迟指标根据性能自动着色：
- 🟢 **优秀** (< 50ms): 绿色
- 🟡 **良好** (50-100ms): 黄绿色
- 🟠 **一般** (100-200ms): 橙色
- 🔴 **较差** (> 200ms): 红色

## 使用方法

### 基本用法

组件已自动集成到 `UnifiedMSEPlayer` 中，无需额外配置：

```tsx
import UnifiedMSEPlayer from './components/UnifiedMSEPlayer';

function App() {
  return (
    <UnifiedMSEPlayer
      sessionId="your-session-id"
      mode="live"
      streamUrl="http://localhost:8443/api/v1/stream/session-id/segments"
    />
  );
}
```

### 独立使用

如果需要单独使用延迟监控组件：

```tsx
import LatencyMonitor from './components/LatencyMonitor';

function MyComponent() {
  return (
    <LatencyMonitor
      sessionId="your-session-id"
      apiBaseUrl="http://localhost:8443"
    />
  );
}
```

### Props

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `sessionId` | `string` | 否 | - | 会话ID，如果提供则只显示该会话的数据 |
| `apiBaseUrl` | `string` | 否 | `http://localhost:8443` | API服务器地址 |

## 数据流

```
后端 SSE 推送
    ↓
LatencyMonitor 组件
    ↓
实时更新 UI
```

### SSE 端点

组件会自动连接到以下SSE端点：

- **所有会话**: `GET /api/v1/latency/alerts`
- **特定会话**: `GET /api/v1/latency/sessions/{session_id}/alerts`

### 消息类型

1. **StatisticsUpdate**: 统计数据更新（每秒一次）
```json
{
  "type": "StatisticsUpdate",
  "session_id": "uuid",
  "statistics": {
    "average_latency_ms": 85.5,
    "p95_latency_ms": 120,
    "throughput_mbps": 5.2,
    ...
  },
  "timestamp": 1702540800
}
```

2. **LatencyAlert**: 延迟告警
```json
{
  "type": "LatencyAlert",
  "session_id": "uuid",
  "alert": {
    "EndToEndLatency": {
      "segment_id": "uuid",
      "latency_ms": 250,
      "threshold_ms": 200
    }
  },
  "timestamp": 1702540800
}
```

## 样式定制

### 修改颜色主题

编辑 `LatencyMonitor.css` 中的渐变背景：

```css
.latency-monitor {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  /* 修改为你喜欢的颜色 */
}
```

### 修改延迟等级阈值

编辑 `LatencyMonitor.tsx` 中的 `getLatencyLevel` 函数：

```typescript
const getLatencyLevel = (latency: number): 'excellent' | 'good' | 'fair' | 'poor' => {
  if (latency < 50) return 'excellent';   // 优秀
  if (latency < 100) return 'good';       // 良好
  if (latency < 200) return 'fair';       // 一般
  return 'poor';                          // 较差
};
```

## 故障排查

### 问题：组件显示"等待延迟数据..."

**原因**: SSE连接未建立或后端未推送数据

**解决方案**:
1. 检查后端服务是否运行
2. 检查 `apiBaseUrl` 是否正确
3. 打开浏览器开发者工具，查看网络请求
4. 确认后端已启动统计更新任务

### 问题：显示"连接失败，正在重试..."

**原因**: SSE连接错误

**解决方案**:
1. 检查CORS配置
2. 确认API端点可访问
3. 查看浏览器控制台错误信息

### 问题：延迟数据不更新

**原因**: 后端未定期广播统计更新

**解决方案**:
1. 确认后端已启动统计更新任务（每秒一次）
2. 检查会话是否处于活动状态
3. 查看后端日志

## 性能考虑

### 内存使用

- 组件只保留最近10条告警
- 统计数据实时更新，不累积历史

### 网络流量

- SSE连接保持长连接
- 每秒接收一次统计更新（约1KB）
- 告警按需推送

### 浏览器兼容性

- ✅ Chrome 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Edge 90+

## 示例截图

### 正常状态
```
┌─────────────────────────────────────────┐
│ 📊 延迟监控              ● 已连接       │
├─────────────────────────────────────────┤
│  平均延迟    当前延迟    吞吐量   丢包率 │
│   85.5ms     87.0ms    5.2Mbps   0.01%  │
├─────────────────────────────────────────┤
│ 详细统计                                │
│ 最小延迟: 30ms    P50延迟: 85ms        │
│ 最大延迟: 120ms   P95延迟: 110ms       │
│ 总分片数: 1,234   P99延迟: 115ms       │
└─────────────────────────────────────────┘
```

### 告警状态
```
┌─────────────────────────────────────────┐
│ ⚠️ 延迟告警                             │
├─────────────────────────────────────────┤
│ 端到端延迟告警: 250ms (阈值: 200ms)    │
│ 处理延迟告警: 60ms (阈值: 50ms)        │
└─────────────────────────────────────────┘
```

## 开发建议

### 添加新的指标

1. 在 `LatencyStatistics` 接口中添加新字段
2. 在后端统计管理器中计算新指标
3. 在组件中显示新指标

### 自定义告警处理

```typescript
// 在 LatencyMonitor 组件中添加自定义处理
useEffect(() => {
  // ... SSE 连接代码
  
  eventSource.onmessage = (event) => {
    const message = JSON.parse(event.data);
    
    if (message.type === 'LatencyAlert') {
      // 自定义告警处理
      if (message.alert.EndToEndLatency) {
        const latency = message.alert.EndToEndLatency.latency_ms;
        if (latency > 300) {
          // 严重告警：显示通知
          showNotification('严重延迟告警', `延迟: ${latency}ms`);
        }
      }
    }
  };
}, []);
```

## 相关文档

- 后端实现: `platform-server/src/latency/README.md`
- API文档: `platform-server/src/latency/IMPLEMENTATION_SUMMARY.md`
- 设计文档: `.kiro/specs/unified-low-latency-streaming/design.md`

## 更新日志

### v1.0.0 (2025-12-14)
- ✨ 初始版本
- ✅ 实时延迟监控
- ✅ 统计数据显示
- ✅ 告警推送
- ✅ 颜色编码
- ✅ 响应式设计
