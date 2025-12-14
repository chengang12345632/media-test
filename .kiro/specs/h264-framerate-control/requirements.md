# H264帧率控制需求文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | H264帧率控制 |
| 创建日期 | 2025-12-14 |
| 状态 | 草稿 |
| 优先级 | 高 |

## 简介

本需求文档定义了H264视频流播放时的帧率控制功能。当前系统在直通H264和回放H264播放时没有控制帧率，导致接收多少就播放多少，可能造成播放速度不正确、缓冲不稳定等问题。本功能旨在添加精确的帧率控制机制，确保视频以正确的速度播放。

## 术语表

- **System**: H264帧率控制系统
- **Platform**: 平台端服务器
- **Frontend**: Web前端应用
- **Frame Rate**: 帧率，每秒显示的视频帧数（FPS）
- **Frame Interval**: 帧间隔，两帧之间的时间间隔（毫秒）
- **Live Stream**: 直通播放，实时视频流
- **Playback**: 录像回放
- **Timestamp**: 时间戳，视频帧的显示时间
- **PTS**: Presentation Timestamp，显示时间戳
- **DTS**: Decoding Timestamp，解码时间戳
- **Target Frame Rate**: 目标帧率，期望的播放帧率
- **Actual Frame Rate**: 实际帧率，当前实际播放帧率
- **Frame Drop**: 丢帧，跳过某些帧以保持同步
- **Frame Duplication**: 重复帧，重复显示某些帧以填充时间

## 需求列表

### 需求 1: 帧率检测和解析

**用户故事**: 作为系统开发者，我希望系统能够自动检测视频流的帧率，以便正确控制播放速度。

#### 验收标准

1. WHEN Platform接收到H264视频流 THEN System SHALL从SPS（Sequence Parameter Set）中解析帧率信息
2. WHEN SPS中没有帧率信息 THEN System SHALL通过分析连续帧的时间戳计算实际帧率
3. WHEN 帧率检测完成 THEN System SHALL将帧率信息包含在流元数据中
4. WHEN 帧率发生变化 THEN System SHALL更新帧率信息并通知Frontend
5. THE System SHALL支持常见帧率（24, 25, 30, 50, 60 FPS）

### 需求 2: 平台端帧率控制

**用户故事**: 作为平台端开发者，我希望在发送视频分片时控制发送速率，以便匹配目标帧率。

#### 验收标准

1. WHEN Platform发送视频分片 THEN System SHALL根据目标帧率计算分片发送间隔
2. WHEN 直通播放模式 THEN System SHALL使用实时帧率控制，保持低延迟
3. WHEN 录像回放模式 THEN System SHALL使用精确帧率控制，支持倍速播放
4. WHEN 分片发送间隔计算 THEN System SHALL考虑分片包含的帧数
5. WHEN 网络拥塞 THEN System SHALL动态调整发送速率以避免缓冲溢出

### 需求 3: 前端播放帧率控制

**用户故事**: 作为前端开发者，我希望播放器能够按照正确的帧率显示视频帧，以便获得流畅的播放体验。

#### 验收标准

1. WHEN Frontend接收到视频分片 THEN 播放器 SHALL根据帧时间戳控制显示时机
2. WHEN 使用MSE播放器 THEN System SHALL依赖浏览器的时间戳处理机制
3. WHEN 使用WebCodecs播放器 THEN System SHALL手动控制帧显示时机
4. WHEN 帧到达过早 THEN 播放器 SHALL延迟显示直到正确时间
5. WHEN 帧到达过晚 THEN 播放器 SHALL立即显示或跳过以保持同步

### 需求 4: 时间戳管理

**用户故事**: 作为系统架构师，我希望系统能够正确管理视频帧的时间戳，以便实现精确的帧率控制。

#### 验收标准

1. WHEN 直通播放启动 THEN System SHALL使用设备端提供的原始时间戳
2. WHEN 录像回放启动 THEN System SHALL从文件中读取时间戳或根据帧率生成时间戳
3. WHEN 生成时间戳 THEN System SHALL确保时间戳单调递增
4. WHEN 转换为fMP4格式 THEN System SHALL保持原始时间戳信息
5. THE System SHALL使用微秒级精度的时间戳

### 需求 5: 帧率同步机制

**用户故事**: 作为用户，我希望视频播放速度稳定，不会出现卡顿或加速现象。

#### 验收标准

1. WHEN 播放器缓冲不足 THEN System SHALL暂停播放等待数据
2. WHEN 播放器缓冲过多 THEN System SHALL跳到最新位置（直通模式）或保持正常播放（回放模式）
3. WHEN 检测到播放速度偏差 THEN System SHALL调整播放速率以恢复同步
4. WHEN 偏差超过阈值 THEN System SHALL执行跳帧或重复帧操作
5. THE System SHALL维持播放速度误差在±5%以内

### 需求 6: 倍速播放支持

**用户故事**: 作为用户，我希望在录像回放时能够调整播放速度，以便快速浏览或慢速查看。

#### 验收标准

1. WHEN 用户选择倍速播放 THEN System SHALL调整帧发送间隔以匹配新速率
2. WHEN 倍速播放 THEN System SHALL保持时间戳的相对关系
3. WHEN 2倍速播放 THEN System SHALL将帧间隔减半
4. WHEN 0.5倍速播放 THEN System SHALL将帧间隔加倍
5. THE System SHALL支持0.25x到4x的倍速范围

### 需求 7: 帧率监控和统计

**用户故事**: 作为运维人员，我希望监控实际播放帧率，以便发现和解决性能问题。

#### 验收标准

1. WHEN 视频播放 THEN System SHALL实时计算并显示当前FPS
2. WHEN 帧率统计 THEN System SHALL记录平均FPS、最小FPS、最大FPS
3. WHEN 帧率低于目标值 THEN System SHALL记录丢帧次数和原因
4. WHEN 帧率高于目标值 THEN System SHALL记录重复帧次数
5. THE System SHALL提供帧率统计API供前端查询

### 需求 8: 自适应帧率调整

**用户故事**: 作为系统架构师，我希望系统能够根据网络和设备性能自适应调整帧率，以便在不同条件下保持流畅播放。

#### 验收标准

1. WHEN 网络带宽不足 THEN System SHALL降低帧率以减少数据传输
2. WHEN CPU负载过高 THEN System SHALL降低帧率以减少解码负担
3. WHEN 条件改善 THEN System SHALL逐步恢复到目标帧率
4. WHEN 调整帧率 THEN System SHALL通知用户当前帧率状态
5. THE System SHALL在保持流畅性和质量之间取得平衡

### 需求 9: 错误处理和恢复

**用户故事**: 作为用户，我希望系统能够处理帧率相关的错误，以便获得稳定的播放体验。

#### 验收标准

1. WHEN 无法检测帧率 THEN System SHALL使用默认帧率（30 FPS）
2. WHEN 时间戳不连续 THEN System SHALL重新同步播放时钟
3. WHEN 帧率突变 THEN System SHALL平滑过渡到新帧率
4. WHEN 播放卡顿 THEN System SHALL自动调整缓冲策略
5. WHEN 错误恢复 THEN System SHALL记录详细日志以便调试

### 需求 10: 配置和调优

**用户故事**: 作为系统管理员，我希望能够配置帧率控制参数，以便针对不同场景优化性能。

#### 验收标准

1. THE System SHALL提供配置接口设置目标帧率
2. THE System SHALL提供配置接口设置帧率容差范围
3. THE System SHALL提供配置接口设置同步策略（跳帧/重复帧）
4. THE System SHALL提供配置接口设置自适应调整阈值
5. WHEN 配置更新 THEN System SHALL在不中断播放的情况下应用新配置

## 非功能性需求

### 性能需求

- 帧率检测延迟 < 1秒
- 时间戳计算精度 ≥ 1微秒
- 帧率控制误差 < ±5%
- 帧显示时机误差 < ±16ms（1帧@60fps）
- CPU占用增加 < 2%

### 可靠性需求

- 帧率检测准确率 > 99%
- 播放速度稳定性 > 95%（误差<5%的时间占比）
- 自动恢复成功率 > 90%

### 兼容性需求

- 支持H.264所有Profile（Baseline, Main, High）
- 支持常见帧率（24, 25, 30, 50, 60 FPS）
- 支持可变帧率（VFR）和固定帧率（CFR）
- 兼容现有的MSE和WebCodecs播放器

### 可维护性需求

- 帧率控制逻辑模块化，易于测试
- 详细的帧率统计和日志
- 配置参数可调，便于调优

## 约束条件

1. 必须保持与现有统一低延迟流系统的兼容性
2. 不能显著增加端到端延迟（增加<10ms）
3. 必须支持现有的直通播放和录像回放模式
4. 初期仅支持H.264视频格式
5. 不改变现有的HTTP3/SSE传输协议

## 依赖关系

1. 依赖统一低延迟视频流传输系统
2. 依赖H.264解析库（用于SPS解析）
3. 依赖现有的MSE和WebCodecs播放器
4. 需要浏览器支持高精度时间API（Performance API）

## 验收标准总结

系统将被认为满足需求，当且仅当：

1. ✅ 能够自动检测视频流帧率
2. ✅ 平台端能够按照目标帧率控制分片发送
3. ✅ 前端播放器能够按照正确帧率显示视频
4. ✅ 时间戳管理准确，精度达到微秒级
5. ✅ 播放速度稳定，误差<±5%
6. ✅ 支持倍速播放（0.25x-4x）
7. ✅ 提供实时帧率监控和统计
8. ✅ 能够自适应调整帧率
9. ✅ 错误处理完善，播放稳定
10. ✅ 通过所有功能测试和性能测试

## 附录

### 参考文档

- H.264/AVC标准 ITU-T H.264
- 统一低延迟视频流传输系统设计文档
- MSE API规范 W3C Recommendation
- WebCodecs API规范

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-14 | 初始版本 | 系统架构团队 |
