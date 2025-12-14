# 实时录屏功能合并需求文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | Device-Uploader 实时录屏功能合并到 Device-Simulator |
| 创建日期 | 2025-12-14 |
| 状态 | 草稿 |
| 优先级 | 高 |
| 父需求 | Device-Uploader 功能合并 (requirements.md) |

## 简介

本需求文档定义了将 `device-uploader` 项目中的**实时录屏和H.264编码功能**合并到 `device-simulator` 项目的需求。目标是让 device-simulator 在直通模式下能够实时录制屏幕并编码为H.264流，而不是从文件中读取预录制的H.264数据。

## 背景

### 当前状态

**Device-Simulator (目标项目)**:
- ✅ 支持从文件读取H.264数据进行直通播放
- ✅ 有基础的屏幕录制框架（但未完全实现）
- ✅ 有QUIC传输机制
- ❌ 缺少完整的FFmpeg实时编码集成
- ❌ 缺少实时屏幕捕获和编码的完整实现

**Device-Uploader (源项目)**:
- ✅ 完整的FFmpeg实时录屏实现 (`live_encoder.rs`)
- ✅ 使用avfoundation (macOS) 进行屏幕捕获
- ✅ 实时H.264编码配置（ultrafast, zerolatency, baseline profile）
- ✅ 时间戳叠加功能（用于延迟测试）
- ✅ 性能统计（FPS、码率、编码时间）
- ✅ 优雅的启动和停止机制

### 目标

将 device-uploader 的实时录屏功能完整迁移到 device-simulator，使其能够：
1. 实时捕获屏幕内容
2. 使用FFmpeg进行H.264编码
3. 通过QUIC传输到平台端
4. 支持时间戳叠加用于延迟测试
5. 提供性能监控和统计

## 术语表

- **Live Encoder**: 实时编码器，使用FFmpeg捕获屏幕并编码为H.264
- **Screen Capture**: 屏幕捕获，从操作系统获取屏幕画面
- **avfoundation**: macOS的屏幕捕获框架
- **H.264 Encoding**: H.264视频编码，使用libx264编码器
- **Timestamp Overlay**: 时间戳叠加，在视频上显示时间戳用于延迟测试
- **Zero Latency**: 零延迟模式，FFmpeg的低延迟编码配置
- **Baseline Profile**: H.264的基础配置文件，兼容性最好
- **GOP**: Group of Pictures，关键帧间隔

## 需求列表

### 需求 1: FFmpeg实时编码器集成

**用户故事**: 作为设备模拟器，我希望使用FFmpeg实时编码屏幕内容，以便生成高质量的H.264视频流。

#### 验收标准

1. THE System SHALL使用FFmpeg命令行工具进行实时屏幕录制和H.264编码
2. WHEN 启动直通播放 THEN System SHALL启动FFmpeg进程并开始屏幕捕获
3. THE System SHALL配置FFmpeg使用以下参数：
   - 编码器: libx264
   - 预设: ultrafast (最快编码速度)
   - 调优: zerolatency (零延迟)
   - 配置文件: baseline (最佳兼容性)
   - 级别: 3.1
   - 像素格式: yuv420p
4. THE System SHALL支持配置视频参数（分辨率、帧率、码率、GOP大小）
5. WHEN FFmpeg进程启动失败 THEN System SHALL返回清晰的错误信息
6. WHEN 停止直通播放 THEN System SHALL优雅地终止FFmpeg进程

### 需求 2: 屏幕捕获配置

**用户故事**: 作为开发者，我希望能够配置屏幕捕获源，以便在不同操作系统上使用合适的捕获方法。

#### 验收标准

1. THE System SHALL支持macOS的avfoundation屏幕捕获
2. THE System SHALL支持配置屏幕捕获源（屏幕序号或窗口ID）
3. THE System SHALL支持配置捕获分辨率（默认1280x720）
4. THE System SHALL支持配置捕获帧率（默认30fps）
5. WHEN 屏幕捕获源不可用 THEN System SHALL返回明确的错误信息
6. THE System SHALL在未来支持扩展到Windows (gdigrab) 和 Linux (x11grab)

### 需求 3: H.264流输出管理

**用户故事**: 作为系统，我希望能够从FFmpeg进程读取H.264数据流，以便通过QUIC传输到平台端。

#### 验收标准

1. THE System SHALL从FFmpeg的stdout读取原始H.264数据流
2. THE System SHALL使用异步IO读取H.264数据，避免阻塞
3. THE System SHALL使用缓冲区管理H.264数据（默认64KB缓冲区）
4. WHEN 读取到H.264数据 THEN System SHALL创建VideoSegment并发送到传输队列
5. THE System SHALL监控FFmpeg的stderr输出用于错误诊断
6. THE System SHALL处理FFmpeg进程的异常终止

### 需求 4: 时间戳叠加功能

**用户故事**: 作为测试人员，我希望在视频上叠加时间戳，以便精确测量端到端延迟。

#### 验收标准

1. THE System SHALL支持在视频上叠加时间戳（可配置开关）
2. WHEN 时间戳叠加启用 THEN System SHALL使用FFmpeg的drawtext滤镜
3. THE 时间戳显示 SHALL包含以下信息：
   - 当前时间（HH:MM:SS格式）
   - 帧编号
   - 视频分辨率
   - 帧率
4. THE 时间戳 SHALL显示在视频左上角，使用黄色文字和半透明黑色背景
5. THE 时间戳字体大小 SHALL为24像素，确保清晰可读
6. WHEN 时间戳叠加禁用 THEN System SHALL不使用drawtext滤镜，提高编码性能

### 需求 5: 性能监控和统计

**用户故事**: 作为开发者，我希望能够监控实时编码的性能，以便优化系统配置和诊断问题。

#### 验收标准

1. THE System SHALL记录以下性能指标：
   - 编码帧率 (FPS)
   - 实际码率 (Kbps)
   - 已编码帧数
   - 已编码字节数
   - 当前时间戳 (毫秒)
2. THE System SHALL每100帧打印一次性能统计到日志
3. THE System SHALL提供API获取实时性能统计
4. THE System SHALL在VideoSegment元数据中包含时间戳和帧编号
5. THE System SHALL计算并报告平均编码FPS和码率

### 需求 6: 编码器生命周期管理

**用户故事**: 作为系统，我希望能够可靠地启动和停止编码器，以便支持多次直通播放会话。

#### 验收标准

1. THE System SHALL支持启动编码器（start_encoding方法）
2. THE System SHALL支持停止编码器（stop_encoding方法）
3. WHEN 启动编码器 THEN System SHALL：
   - 创建输出通道（mpsc channel）
   - 启动FFmpeg进程
   - 启动异步读取任务
   - 初始化性能统计
4. WHEN 停止编码器 THEN System SHALL：
   - 终止FFmpeg进程
   - 终止异步读取任务
   - 清理输出通道
   - 保存最终性能统计
5. THE System SHALL支持多次启动和停止编码器
6. THE System SHALL确保资源正确清理，避免内存泄漏

### 需求 7: 错误处理和恢复

**用户故事**: 作为系统，我希望能够优雅地处理编码错误，以便提供稳定的服务。

#### 验收标准

1. WHEN FFmpeg未安装 THEN System SHALL返回ConfigurationError并提示安装FFmpeg
2. WHEN avfoundation不可用 THEN System SHALL返回ConfigurationError并说明原因
3. WHEN FFmpeg进程崩溃 THEN System SHALL记录错误日志并通知上层
4. WHEN 读取H.264数据失败 THEN System SHALL记录错误并尝试恢复
5. WHEN 输出通道满 THEN System SHALL记录警告并丢弃旧数据
6. THE System SHALL在所有错误情况下确保资源正确清理

### 需求 8: 配置灵活性

**用户故事**: 作为开发者，我希望能够灵活配置编码器参数，以便适应不同的使用场景。

#### 验收标准

1. THE System SHALL支持通过LiveEncoderConfig配置编码器
2. THE LiveEncoderConfig SHALL包含以下配置项：
   - quality: LiveStreamQuality (分辨率、帧率、码率、GOP大小)
   - timestamp_overlay: bool (是否叠加时间戳)
   - screen_capture: bool (是否使用屏幕捕获)
   - output_format: OutputFormat (输出格式)
   - segment_duration_ms: u64 (分片时长)
   - timestamp_format: TimestampFormat (时间戳格式)
3. THE System SHALL提供合理的默认配置（720p, 30fps, 2Mbps）
4. THE System SHALL验证配置参数的合法性
5. THE System SHALL支持运行时修改部分配置（如码率、帧率）

### 需求 9: 与现有代码集成

**用户故事**: 作为系统架构师，我希望新功能能够无缝集成到现有的device-simulator代码中，以便保持代码一致性。

#### 验收标准

1. THE System SHALL将LiveH264Encoder集成到device_service.rs中
2. WHEN 收到StartLiveStream信令 THEN System SHALL启动LiveH264Encoder
3. WHEN 收到StopStream信令 THEN System SHALL停止LiveH264Encoder
4. THE System SHALL使用现有的QUIC传输机制发送编码后的H.264数据
5. THE System SHALL保持现有的VideoSegment数据结构兼容性
6. THE System SHALL复用现有的错误处理和日志机制

### 需求 10: 向后兼容性

**用户故事**: 作为系统维护者，我希望新功能不影响现有的文件播放功能，以便保持系统稳定性。

#### 验收标准

1. THE System SHALL保持现有的文件播放功能不变
2. THE System SHALL支持通过配置选择使用实时录屏或文件播放
3. WHEN 实时录屏不可用 THEN System SHALL回退到文件播放模式
4. THE System SHALL保持现有的QUIC协议兼容性
5. THE System SHALL保持现有的API接口不变

## 平台端需求

### 需求 11: 平台端接收实时H.264流

**用户故事**: 作为平台端，我希望能够接收设备端实时编码的H.264流，以便转发到前端播放。

#### 验收标准

1. THE Platform SHALL通过QUIC接收设备端发送的H.264视频分片
2. THE Platform SHALL使用LiveStreamSource接收实时视频流
3. THE Platform SHALL记录分片接收时间用于延迟监控
4. THE Platform SHALL通过UnifiedStreamHandler管理实时流会话
5. THE Platform SHALL支持零缓冲转发（处理延迟<5ms）
6. THE Platform SHALL通过SSE推送H.264分片到前端
7. THE Platform SHALL在分片元数据中包含时间戳和帧编号

### 需求 12: 平台端帧率检测

**用户故事**: 作为平台端，我希望能够检测实时流的帧率，以便提供准确的流信息给前端。

#### 验收标准

1. THE Platform SHALL使用FrameRateDetector检测实时流的帧率
2. THE Platform SHALL基于时间戳样本计算实际帧率
3. WHEN 检测到帧率变化 THEN Platform SHALL更新流信息
4. THE Platform SHALL在流信息中提供检测到的帧率
5. THE Platform SHALL支持30fps、60fps等常见帧率的检测

### 需求 13: 平台端延迟监控增强

**用户故事**: 作为平台端，我希望能够监控实时流的端到端延迟，以便优化传输性能。

#### 验收标准

1. THE Platform SHALL记录设备端发送时间（从分片元数据获取）
2. THE Platform SHALL记录平台端接收时间
3. THE Platform SHALL记录平台端转发时间
4. THE Platform SHALL计算传输延迟（接收时间 - 发送时间）
5. THE Platform SHALL计算处理延迟（转发时间 - 接收时间）
6. THE Platform SHALL通过LatencyMonitor提供延迟统计
7. WHEN 延迟超过阈值 THEN Platform SHALL发送延迟告警

### 需求 14: 平台端流信息管理

**用户故事**: 作为平台端，我希望能够管理实时流的元数据信息，以便前端正确配置播放器。

#### 验收标准

1. THE Platform SHALL在LiveStreamSource中维护流信息（分辨率、帧率、码率）
2. THE Platform SHALL提供API获取流信息
3. THE Platform SHALL在SSE响应中包含流元数据
4. THE Platform SHALL支持动态更新流信息
5. THE Platform SHALL在流信息中标识流类型（Live/Playback）

## 前端需求

### 需求 15: 前端WebCodecs播放器支持实时流

**用户故事**: 作为前端用户，我希望能够使用WebCodecs播放器播放实时H.264流，以便获得低延迟的观看体验。

#### 验收标准

1. THE Frontend SHALL使用WebCodecs API解码H.264流
2. THE Frontend SHALL通过SSE接收实时H.264分片
3. THE Frontend SHALL支持Annex B格式的H.264流（带起始码）
4. THE Frontend SHALL自动检测并配置解码器（从SPS/PPS）
5. THE Frontend SHALL在接收到SPS/PPS后配置VideoDecoder
6. THE Frontend SHALL缓冲初始分片直到解码器配置完成
7. THE Frontend SHALL使用Canvas渲染解码后的视频帧

### 需求 16: 前端帧率控制

**用户故事**: 作为前端用户，我希望视频能够以正确的帧率播放，以便获得流畅的观看体验。

#### 验收标准

1. THE Frontend SHALL使用FrameScheduler控制帧显示速率
2. THE Frontend SHALL从服务器获取目标帧率（默认30fps）
3. THE Frontend SHALL根据帧的PTS（时间戳）调度显示
4. THE Frontend SHALL计算并显示实际播放帧率
5. THE Frontend SHALL计算并显示帧率误差
6. WHEN 帧率误差超过5% THEN Frontend SHALL显示警告
7. THE Frontend SHALL支持丢帧策略以保持同步

### 需求 17: 前端延迟监控显示

**用户故事**: 作为前端用户，我希望能够看到实时的延迟统计信息，以便了解播放质量。

#### 验收标准

1. THE Frontend SHALL使用LatencyMonitor组件显示延迟信息
2. THE Frontend SHALL通过SSE接收延迟统计数据
3. THE Frontend SHALL显示以下延迟指标：
   - 当前延迟
   - 平均延迟
   - 最小/最大延迟
   - P50/P95/P99延迟
4. THE Frontend SHALL显示丢帧统计
5. THE Frontend SHALL显示帧调度延迟
6. WHEN 延迟超过阈值 THEN Frontend SHALL显示告警
7. THE Frontend SHALL支持延迟数据的实时更新（每秒）

### 需求 18: 前端播放器状态管理

**用户故事**: 作为前端用户，我希望能够看到播放器的状态信息，以便了解播放进度。

#### 验收标准

1. THE Frontend SHALL显示播放器状态（初始化、连接中、播放中、错误）
2. THE Frontend SHALL显示接收的分片数量
3. THE Frontend SHALL显示实际FPS和目标FPS
4. THE Frontend SHALL显示解码方式（WebCodecs硬件加速）
5. THE Frontend SHALL显示会话ID
6. THE Frontend SHALL在出错时显示清晰的错误信息
7. THE Frontend SHALL在浏览器不支持时显示升级提示

### 需求 19: 前端资源清理

**用户故事**: 作为前端开发者，我希望播放器能够正确清理资源，以便避免内存泄漏。

#### 验收标准

1. WHEN 组件卸载 THEN Frontend SHALL关闭SSE连接
2. WHEN 组件卸载 THEN Frontend SHALL关闭VideoDecoder
3. WHEN 组件卸载 THEN Frontend SHALL销毁FrameScheduler
4. WHEN 组件卸载 THEN Frontend SHALL清理所有VideoFrame
5. THE Frontend SHALL在清理过程中捕获并忽略异常
6. THE Frontend SHALL确保所有异步任务被取消

## 非功能性需求

### 性能需求

**设备端**:
- 编码延迟 < 50ms (从屏幕捕获到H.264输出)
- 编码帧率 ≥ 30fps (稳定)
- CPU占用 < 30% (单核)
- 内存占用 < 200MB

**平台端**:
- 接收处理延迟 < 5ms
- 转发延迟 < 5ms
- 支持100+并发流会话
- 单流CPU占用 < 5%
- 单流内存占用 < 50MB

**前端**:
- 解码延迟 < 20ms
- 渲染延迟 < 16ms (60fps)
- 帧调度精度 ± 5ms
- 内存占用 < 100MB

**端到端**:
- 总延迟 < 100ms (从屏幕到前端显示)
- 帧率稳定性 > 95% (误差 < 5%)

### 兼容性需求

- 支持macOS 10.14+ (avfoundation)
- 支持FFmpeg 4.0+
- 未来支持Windows (gdigrab) 和 Linux (x11grab)
- 兼容现有的QUIC协议和VideoSegment格式

### 可靠性需求

- FFmpeg进程崩溃恢复率 > 95%
- 连续运行时间 > 1小时无内存泄漏
- 错误日志覆盖率 100%

### 可维护性需求

- 代码复用率 > 80%
- 测试覆盖率 > 70%
- 清晰的模块划分
- 详细的错误信息和日志

## 技术方案概述

### 核心组件

1. **LiveH264Encoder** (`live_encoder.rs`)
   - 管理FFmpeg进程
   - 读取H.264数据流
   - 生成VideoSegment
   - 性能统计

2. **LiveEncoderConfig** (配置结构)
   - 视频质量参数
   - 时间戳叠加配置
   - 输出格式配置

3. **TimestampGenerator** (时间戳生成器)
   - 生成单调递增的时间戳
   - 计算帧编号
   - 提供毫秒级精度

### FFmpeg命令示例

```bash
ffmpeg \
  -f avfoundation \
  -i "4" \
  -r 30 \
  -s 1280x720 \
  -c:v libx264 \
  -preset ultrafast \
  -tune zerolatency \
  -profile:v baseline \
  -level 3.1 \
  -pix_fmt yuv420p \
  -b:v 2000k \
  -g 30 \
  -vf "drawtext=text='%{pts\:hms} | Frame\: %{n} | 1280x720 | 30fps':fontcolor=yellow:fontsize=24:box=1:boxcolor=black@0.5:x=10:y=10" \
  -f h264 \
  -y \
  -loglevel error \
  -
```

### 数据流

```
屏幕 → avfoundation → FFmpeg (libx264) → stdout (H.264) → 
LiveH264Encoder → VideoSegment → QUIC → Platform Server → Frontend
```

## 合并策略

### 阶段 1: 核心文件复制

1. 复制 `device-uploader/src/live_encoder.rs` 到 `device-simulator/src/video/live_encoder.rs`
2. 复制相关的类型定义到 `device-simulator/src/video/types.rs`
3. 更新 `device-simulator/src/video/mod.rs` 导出新模块

### 阶段 2: 依赖调整

1. 调整 `live_encoder.rs` 中的导入路径
2. 确保使用 device-simulator 的类型定义
3. 调整错误类型和日志宏

### 阶段 3: 集成到 device_service

1. 在 `device_service.rs` 中添加 LiveH264Encoder 字段
2. 实现 StartLiveStream 信令处理
3. 实现 StopStream 信令处理
4. 集成到现有的QUIC传输流程

### 阶段 4: 测试和验证

1. 单元测试 LiveH264Encoder
2. 集成测试直通播放流程
3. 性能测试和优化
4. 端到端延迟测试

### 阶段 5: 文档更新

1. 更新 README.md
2. 添加实时录屏使用指南
3. 更新 API 文档

## 依赖关系

### 新增依赖

```toml
[dependencies]
# 现有依赖保持不变
tokio = { version = "1", features = ["full"] }
quinn = "0.10"
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"

# 无需新增依赖，使用系统FFmpeg命令行工具
```

### 系统依赖

- FFmpeg 4.0+ (命令行工具)
- macOS 10.14+ (avfoundation支持)

## 风险评估

### 高风险

1. **FFmpeg依赖** - 需要系统安装FFmpeg
   - 缓解：提供清晰的安装指南，检测FFmpeg可用性
   - 缓解：提供回退到文件播放模式

2. **平台兼容性** - avfoundation仅支持macOS
   - 缓解：文档中明确说明平台限制
   - 缓解：未来扩展支持Windows和Linux

### 中风险

3. **性能影响** - 实时编码可能占用较多CPU
   - 缓解：使用ultrafast预设，优化编码参数
   - 缓解：提供性能监控和调优指南

4. **进程管理** - FFmpeg进程可能崩溃或僵死
   - 缓解：实现进程监控和自动重启
   - 缓解：确保资源正确清理

### 低风险

5. **代码复杂度** - 新增代码量较大
   - 缓解：保持模块化，清晰的文档
   - 缓解：充分的单元测试和集成测试

## 验收标准总结

系统将被认为满足需求，当且仅当：

### 设备端验收标准

1. ✅ 能够使用FFmpeg实时录制屏幕并编码为H.264
2. ✅ 能够通过QUIC传输H.264流到平台端
3. ✅ 支持时间戳叠加用于延迟测试
4. ✅ 提供性能监控和统计
5. ✅ 能够优雅地启动和停止编码器
6. ✅ 错误处理完善，资源清理正确
7. ✅ 编码帧率稳定在30fps

### 平台端验收标准

8. ✅ 能够通过QUIC接收实时H.264流
9. ✅ 能够检测并报告实时流的帧率
10. ✅ 能够通过SSE转发H.264流到前端
11. ✅ 提供完整的延迟监控和统计
12. ✅ 处理延迟 < 5ms
13. ✅ 支持100+并发流会话

### 前端验收标准

14. ✅ 能够使用WebCodecs解码H.264流
15. ✅ 能够以正确的帧率播放视频（误差 < 5%）
16. ✅ 能够显示实时延迟统计信息
17. ✅ 能够正确清理资源，无内存泄漏
18. ✅ 在不支持的浏览器中显示友好提示

### 端到端验收标准

19. ✅ 端到端延迟 < 100ms
20. ✅ 帧率稳定性 > 95%
21. ✅ 保持向后兼容性，不影响现有功能
22. ✅ 通过所有单元测试和集成测试

## 附录

### 参考文档

- Device-Uploader `live_encoder.rs` 源代码
- Device-Simulator 现有代码
- FFmpeg 文档: https://ffmpeg.org/documentation.html
- avfoundation 文档: https://developer.apple.com/av-foundation/

### 相关 Spec

- `requirements.md` - Device-Uploader 功能合并主需求文档
- `design.md` - 设计文档（待创建）
- `tasks.md` - 任务列表（待更新）

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-14 | 初始版本 | 系统架构团队 |
