# 延迟监控系统测试指南

## 问题修复 ✅

**问题**: 前端延迟监控组件连接失败（ERR_CONNECTION_REFUSED）

**原因**: 前端默认连接到 `http://localhost:8443`，但后端 HTTP3 服务器实际运行在 `http://localhost:8080`

**修复**: 
- ✅ 修改 `UnifiedMSEPlayer.tsx` - 传递正确的 `apiBaseUrl="http://localhost:8080"`
- ✅ 修改 `WebCodecsPlayer.tsx` - 传递正确的 `apiBaseUrl="http://localhost:8080"`

## 当前状态

### 后端服务器 ✅
- **状态**: 正在运行
- **QUIC 端口**: 8443
- **HTTP3 端口**: 8080
- **延迟监控**: 已启动

### 前端配置 ✅
- **API URL**: 已修复为 `http://localhost:8080`
- **组件**: UnifiedMSEPlayer 和 WebCodecsPlayer 都已更新

## 测试步骤

### 1. 确认后端运行

后端已经在运行，你可以看到以下日志：
```
✓ QUIC server listening on 0.0.0.0:8443
✓ HTTP3 server listening on 0.0.0.0:8080
✓ Latency monitoring statistics update task started
✅ Platform server ready!
```

### 2. 测试延迟监控 API

打开新的终端，测试 API 端点：

```bash
# 健康检查
curl http://localhost:8080/api/v1/latency/health

# 应该返回:
# {"status":"success","data":"Latency monitoring is healthy","error":null}
```

### 3. 刷新前端页面

1. 在浏览器中刷新前端页面（通常是 `http://localhost:5173`）
2. 前端会重新加载，使用新的 API URL
3. 延迟监控组件应该能够成功连接

### 4. 验证延迟监控显示

启动一个流会话（直通播放或录像回放），你应该看到：

**延迟监控面板**:
- ✅ 连接状态显示为"已连接"
- ✅ 显示实时延迟指标
- ✅ 显示统计数据（平均、P50/P95/P99、吞吐量）
- ✅ 如果有延迟告警，会显示在告警列表中

**颜色编码**:
- 🟢 优秀 (< 50ms): 绿色
- 🟡 良好 (50-100ms): 黄绿色
- 🟠 一般 (100-200ms): 橙色
- 🔴 较差 (> 200ms): 红色

### 5. 测试 SSE 连接

打开浏览器开发者工具（F12），查看网络标签：

1. 应该看到一个持久的 SSE 连接：
   ```
   GET http://localhost:8080/api/v1/latency/sessions/{session_id}/alerts
   ```

2. 连接状态应该是 "pending"（保持打开）

3. 在控制台中应该看到：
   ```
   SSE connected successfully
   ```

### 6. 测试延迟告警

如果系统检测到延迟超过阈值，你会看到：

1. 告警列表中出现新的告警
2. 控制台输出告警信息
3. 延迟指标变为红色（如果超过200ms）

## 故障排查

### 问题：前端仍然显示"连接失败"

**解决方案**:
1. 确认后端正在运行（检查端口8080）
2. 刷新浏览器页面（Ctrl+F5 强制刷新）
3. 清除浏览器缓存
4. 检查浏览器控制台是否有错误

### 问题：SSE 连接断开

**解决方案**:
1. 延迟监控组件会自动重连（最多5次）
2. 检查后端日志是否有错误
3. 确认会话ID是否有效

### 问题：没有显示延迟数据

**解决方案**:
1. 确认流会话已启动
2. 确认有视频分片正在传输
3. 检查后端日志中的延迟记录
4. 使用 curl 测试 API 端点

## API 端点测试

### 获取所有统计
```bash
curl http://localhost:8080/api/v1/latency/statistics
```

### 获取特定会话统计
```bash
curl http://localhost:8080/api/v1/latency/sessions/{session_id}/statistics
```

### 订阅告警（SSE）
```bash
curl -N http://localhost:8080/api/v1/latency/alerts
```

## 预期结果

### 成功的延迟监控显示

当一切正常工作时，你应该看到：

1. **连接状态**: "✅ 已连接"
2. **延迟指标**: 
   - 当前延迟: ~5-50ms（取决于系统性能）
   - 平均延迟: ~10-30ms
   - P50/P95/P99: 合理的百分位数值
3. **吞吐量**: 根据视频码率显示（例如 2-5 Mbps）
4. **丢包率**: 应该接近 0%
5. **告警**: 如果延迟正常，应该没有告警

### 延迟阈值

系统配置的延迟阈值：
- 传输延迟: 100ms
- 处理延迟: 50ms
- 分发延迟: 50ms
- 端到端延迟: 200ms

如果任何延迟超过这些阈值，系统会生成告警。

## 下一步

如果延迟监控正常工作：

1. ✅ 测试直通播放的延迟显示
2. ✅ 测试录像回放的延迟显示
3. ✅ 测试多个并发会话
4. ✅ 测试延迟告警功能
5. ✅ 验证 SSE 自动重连

## 参考

- 后端集成完成报告: `platform-server/LATENCY_INTEGRATION_COMPLETE.md`
- 延迟监控完成总结: `LATENCY_MONITORING_COMPLETE.md`
- 前端使用指南: `web-frontend/src/components/LATENCY_MONITOR_GUIDE.md`
