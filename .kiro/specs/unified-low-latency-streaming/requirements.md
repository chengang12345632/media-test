# 统一低延迟视频流传输系统需求文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | 统一低延迟视频流传输系统 |
| 创建日期 | 2025-12-13 |
| 状态 | 草稿 |
| 优先级 | 高 |

## 简介

本需求文档定义了统一低延迟视频流传输系统的功能需求。该系统旨在为直通播放（实时监控）和录像回放提供统一的低延迟传输方案，实现端到端延迟小于200ms的目标，并提供一致的用户体验。

## 术语表

- **System**: 统一低延迟视频流传输系统
- **Platform**: 平台端服务器
- **Device**: 设备端（摄像头或模拟器）
- **Frontend**: Web前端应用
- **Live Stream**: 直通播放，实时视频流
- **Playback**: 录像回放
- **MSE**: Media Source Extensions，浏览器媒体源扩展API
- **HTTP3**: 基于QUIC协议的HTTP版本
- **SSE**: Server-Sent Events，服务器推送事件
- **Segment**: 视频分片，视频流的基本传输单元
- **Zero-Buffer Mode**: 零缓冲模式，边接收边转发，无缓冲延迟
- **Latency**: 延迟，从视频采集到前端显示的时间
- **fMP4**: Fragmented MP4，分片MP4格式

## 需求列表

### 需求 1: 统一流处理架构

**用户故事**: 作为系统架构师，我希望直通播放和录像回放使用统一的流处理架构，以便降低系统复杂度和维护成本。

#### 验收标准

1. WHEN Platform接收到直通播放请求 THEN System SHALL使用统一的流处理器创建流会话
2. WHEN Platform接收到录像回放请求 THEN System SHALL使用统一的流处理器创建流会话
3. WHEN 流会话创建完成 THEN System SHALL返回包含session_id和stream_url的响应
4. WHEN 流会话处于活动状态 THEN System SHALL使用相同的分片传输机制传输视频数据
5. THE System SHALL支持同时处理多个直通播放和录像回放会话

### 需求 2: 低延迟视频分片传输

**用户故事**: 作为用户，我希望观看视频时延迟尽可能低，以便获得接近实时的观看体验。

#### 验收标准

1. WHEN 直通播放会话启动 THEN System SHALL实现端到端延迟小于100ms
2. WHEN 录像回放会话启动 THEN System SHALL实现端到端延迟小于200ms
3. WHEN Platform接收到视频分片 THEN System SHALL在5ms内完成处理并转发到Frontend
4. WHEN 使用零缓冲模式 THEN System SHALL边接收边转发，无额外缓冲延迟
5. WHEN 网络条件良好 THEN System SHALL保持稳定的低延迟传输

### 需求 3: HTTP3协议媒体传输

**用户故事**: 作为开发者，我希望使用HTTP3协议传输媒体流，以便利用QUIC协议的低延迟和可靠性优势。

#### 验收标准

1. THE Platform SHALL使用HTTP3协议向Frontend传输视频分片
2. WHEN Platform发送视频分片 THEN System SHALL使用SSE（Server-Sent Events）推送机制
3. WHEN 视频分片传输 THEN System SHALL在HTTP头中包含分片元数据（timestamp, duration, keyframe标志）
4. WHEN 网络出现丢包 THEN System SHALL利用QUIC协议的自动重传机制恢复数据
5. THE System SHALL支持HTTP3的多路复用特性，允许并发传输多个流

### 需求 4: 统一MSE播放器

**用户故事**: 作为前端开发者，我希望使用统一的MSE播放器处理直通和回放，以便简化前端实现和维护。

#### 验收标准

1. THE Frontend SHALL实现基于MSE的统一视频播放器
2. WHEN 播放器初始化 THEN System SHALL根据流类型（直通/回放）配置适当的缓冲策略
3. WHEN 接收到视频分片 THEN 播放器 SHALL立即追加到SourceBuffer
4. WHEN 直通播放模式 THEN 播放器 SHALL使用最小缓冲（100-500ms）
5. WHEN 录像回放模式 THEN 播放器 SHALL使用适中缓冲（500-2000ms）
6. WHEN SourceBuffer缓冲过多 THEN 播放器 SHALL自动移除旧数据以保持低延迟

### 需求 5: 文件流式读取

**用户故事**: 作为平台端开发者，我希望实现高效的文件流式读取，以便录像回放也能达到低延迟。

#### 验收标准

1. WHEN 录像回放启动 THEN Platform SHALL创建FileStreamReader以流式方式读取文件
2. WHEN 读取视频文件 THEN System SHALL使用小分片（8KB-32KB）降低延迟
3. WHEN 文件读取 THEN System SHALL根据播放速率控制分片发送间隔
4. WHEN 支持倍速播放 THEN System SHALL按比例调整分片发送速率
5. WHEN 文件读取完成 THEN System SHALL发送流结束信号并清理资源

### 需求 6: H.264到fMP4转换

**用户故事**: 作为系统集成者，我希望系统能够处理H.264裸流，以便支持更多的视频格式。

#### 验收标准

1. WHEN 视频源为H.264裸流 THEN System SHALL将其转换为fMP4格式
2. WHEN 转换fMP4 THEN System SHALL生成正确的初始化分片（init segment）
3. WHEN 转换fMP4 THEN System SHALL为每个视频分片生成媒体分片（media segment）
4. WHEN fMP4转换 THEN System SHALL保持原始视频的时间戳和关键帧信息
5. WHEN 转换完成 THEN fMP4分片 SHALL能够被MSE播放器正确解析和播放

### 需求 7: 播放控制功能

**用户故事**: 作为用户，我希望能够控制视频播放（暂停、恢复、拖动、倍速），以便灵活观看视频。

#### 验收标准

1. WHEN 用户点击暂停 THEN System SHALL暂停视频分片传输并保持当前位置
2. WHEN 用户点击恢复 THEN System SHALL从当前位置继续传输视频分片
3. WHEN 用户拖动进度条 THEN System SHALL定位到目标时间并从该位置开始传输
4. WHEN 用户选择倍速播放 THEN System SHALL按比例调整分片发送速率
5. WHEN 录像回放 THEN System SHALL支持0.25x到4x的倍速范围
6. WHEN 直通播放 THEN System SHALL仅支持暂停和恢复，不支持拖动和倍速

### 需求 8: 延迟监控和统计

**用户故事**: 作为运维人员，我希望监控系统延迟和性能指标，以便及时发现和解决问题。

#### 验收标准

1. WHEN 视频分片传输 THEN System SHALL记录每个分片的端到端延迟
2. WHEN 延迟超过阈值 THEN System SHALL通过WebSocket推送延迟告警
3. THE System SHALL提供实时性能统计API，包括平均延迟、吞吐量、丢包率
4. WHEN 流会话活动 THEN System SHALL每秒更新一次性能统计数据
5. THE System SHALL在日志中记录详细的延迟分解（采集、传输、处理、渲染）

### 需求 9: 错误处理和恢复

**用户故事**: 作为用户，我希望系统能够优雅处理错误并自动恢复，以便获得稳定的观看体验。

#### 验收标准

1. WHEN 网络连接中断 THEN System SHALL自动尝试重连，最多5次
2. WHEN 视频分片损坏 THEN System SHALL跳过损坏分片并继续播放
3. WHEN 设备端离线 THEN System SHALL通知Frontend并停止流传输
4. WHEN 文件读取失败 THEN System SHALL返回明确的错误信息并清理资源
5. WHEN SSE连接断开 THEN Frontend SHALL自动重新建立连接

### 需求 10: 多客户端支持

**用户故事**: 作为系统管理员，我希望支持多个客户端同时观看同一视频流，以便多人协作监控。

#### 验收标准

1. WHEN 多个Frontend请求同一直通播放流 THEN System SHALL复用设备端连接
2. WHEN 多个Frontend请求同一录像回放 THEN System SHALL为每个客户端创建独立的文件读取器
3. WHEN 向多个客户端转发分片 THEN System SHALL使用并发转发机制
4. WHEN 某个客户端断开 THEN System SHALL不影响其他客户端的播放
5. THE System SHALL支持至少10个并发客户端观看同一视频流

### 需求 11: 缓冲策略优化

**用户故事**: 作为前端开发者，我希望实现智能的缓冲策略，以便在低延迟和播放流畅性之间取得平衡。

#### 验收标准

1. WHEN 直通播放模式 THEN 播放器 SHALL维持100-500ms的目标缓冲
2. WHEN 录像回放模式 THEN 播放器 SHALL维持500-2000ms的目标缓冲
3. WHEN 缓冲不足 THEN 播放器 SHALL显示缓冲指示器并暂停播放
4. WHEN 缓冲充足 THEN 播放器 SHALL自动恢复播放
5. WHEN 网络条件变化 THEN System SHALL动态调整缓冲目标

### 需求 12: 性能优化

**用户故事**: 作为系统架构师，我希望系统具有高性能和低资源占用，以便支持大规模部署。

#### 验收标准

1. WHEN 处理视频分片 THEN Platform SHALL使用零拷贝技术减少内存复制
2. WHEN 转发分片到多个客户端 THEN System SHALL使用异步并发机制
3. THE Platform SHALL支持至少100个并发流会话
4. WHEN 系统运行 THEN 单个流会话的CPU占用 SHALL小于5%
5. WHEN 系统运行 THEN 单个流会话的内存占用 SHALL小于50MB

## 非功能性需求

### 性能需求

- 直通播放端到端延迟 < 100ms
- 录像回放端到端延迟 < 200ms
- 平台端分片处理延迟 < 5ms
- 支持至少100个并发流会话
- 单流会话CPU占用 < 5%
- 单流会话内存占用 < 50MB

### 可靠性需求

- 系统可用性 > 99.9%
- 自动错误恢复成功率 > 95%
- 网络抖动容忍度：2%-5%丢包率

### 兼容性需求

- 支持Chrome 90+、Firefox 88+、Safari 14+、Edge 90+
- 支持H.264和MP4视频格式
- 支持Windows、macOS、Linux操作系统

### 可维护性需求

- 代码复用率 > 80%（直通和回放共享代码）
- 单元测试覆盖率 > 80%
- 详细的日志记录和监控指标

## 约束条件

1. 必须使用HTTP3协议进行媒体传输
2. 必须使用MSE API实现前端播放器
3. 必须保持与现有QUIC信令协议的兼容性
4. Demo版本无需支持认证和权限控制
5. 初期仅支持单音轨视频

## 依赖关系

1. 依赖现有的QUIC设备连接和信令系统
2. 依赖现有的设备管理和录像管理模块
3. 需要浏览器支持MSE和HTTP3

## 验收标准总结

系统将被认为满足需求，当且仅当：

1. ✅ 直通播放和录像回放使用统一的流处理架构
2. ✅ 直通播放延迟 < 100ms，录像回放延迟 < 200ms
3. ✅ 使用HTTP3协议传输媒体流
4. ✅ 前端使用统一的MSE播放器
5. ✅ 支持完整的播放控制功能
6. ✅ 提供实时延迟监控和性能统计
7. ✅ 支持多客户端并发观看
8. ✅ 通过所有功能测试和性能测试

## 附录

### 参考文档

- 系统架构设计文档 v1.0
- QUIC协议规范 RFC 9000
- HTTP/3协议规范 RFC 9114
- MSE API规范 W3C Recommendation

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-13 | 初始版本 | 系统架构团队 |
