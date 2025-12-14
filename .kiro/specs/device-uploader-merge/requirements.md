# Device-Uploader 功能合并需求文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | Device-Uploader 高级功能合并到 Device-Simulator |
| 创建日期 | 2025-12-14 |
| 状态 | 草稿 |
| 优先级 | 高 |

## 简介

本需求文档定义了将 `device-uploader` 项目中的高级功能合并到 `device-simulator` 项目的需求。通过分析两个项目的差异，识别 `device-uploader` 的亮点功能并制定合并策略，最终保留 `device-simulator` 作为统一的设备端实现。

## 术语表

- **Device-Simulator**: 目标项目，当前的设备模拟器实现
- **Device-Uploader**: 源项目，包含高级功能的视频上传器
- **Keyframe Index**: 关键帧索引，用于精确视频定位
- **Seek**: 视频定位操作，跳转到指定时间位置
- **Playback Controller**: 播放控制器，管理播放速率、定位等操作
- **Performance Monitor**: 性能监控器，实时监控传输性能
- **Timeline File**: 时间线文件，缓存关键帧信息的JSON文件
- **FFmpeg CLI**: FFmpeg命令行工具，用于视频解析

## 项目差异分析

### Device-Simulator 现有功能
1. ✅ 基础QUIC连接和通信
2. ✅ 视频文件扫描和管理
3. ✅ 简单的视频分片上传
4. ✅ 设备注册和心跳
5. ✅ 录像列表查询
6. ✅ 基础回放支持
7. ✅ 自动重连机制
8. ✅ 屏幕录制和实时编码（部分实现）

### Device-Uploader 亮点功能（待合并）
1. 🌟 **精确关键帧定位** - 亚秒级精度的视频seek功能
2. 🌟 **关键帧索引系统** - 多种优化策略（Full, Sparse, Adaptive, Hierarchical）
3. 🌟 **Timeline文件缓存** - JSON格式的关键帧信息持久化
4. 🌟 **FFmpeg命令行集成** - 可靠的视频解析和元数据提取
5. 🌟 **高级播放控制器** - 支持精确seek、倍速播放、帧丢弃策略
6. 🌟 **高性能传输** - 实现1+ Gbps吞吐量

### 功能对比表

| 功能模块 | Device-Simulator | Device-Uploader | 优先级 |
|---------|-----------------|-----------------|--------|
| QUIC连接 | ✅ 基础实现 | ✅ 完整实现 | 中 |
| 视频分片 | ✅ 简单分片 | ✅ 智能分片 | 高 |
| Seek定位 | ❌ 不支持 | ✅ 精确定位 | 高 |
| 关键帧索引 | ❌ 不支持 | ✅ 多策略支持 | 高 |
| 播放控制 | ⚠️ 基础支持 | ✅ 高级控制 | 高 |
| FFmpeg集成 | ❌ 不支持 | ✅ CLI集成 | 中 |

## 需求列表

### 需求 1: 精确关键帧定位系统

**用户故事**: 作为平台端，我希望能够精确定位到视频的任意时间点，以便实现流畅的回放控制和快速定位。

#### 验收标准

1. WHEN 平台请求seek到指定时间 THEN System SHALL使用关键帧索引定位到最近的关键帧
2. WHEN 关键帧索引不存在 THEN System SHALL自动构建关键帧索引
3. WHEN seek到非关键帧位置 THEN System SHALL自动对齐到最近的关键帧确保解码完整性
4. THE System SHALL支持亚秒级精度（≤0.1秒）的seek操作
5. WHEN seek操作完成 THEN System SHALL返回详细的SeekResult（请求位置、实际位置、精度、执行时间）
6. THE System SHALL支持多种索引优化策略（Full, Sparse, Adaptive, Hierarchical）
7. WHEN 视频文件较大 THEN System SHALL使用内存限制自动选择合适的索引策略

### 需求 2: Timeline文件缓存系统

**用户故事**: 作为系统，我希望能够缓存关键帧索引信息，以便避免重复解析视频文件，提高启动速度。

#### 验收标准

1. WHEN 首次解析视频文件 THEN System SHALL生成.timeline JSON文件保存关键帧信息
2. WHEN 再次加载视频文件 THEN System SHALL优先读取.timeline文件而不是重新解析
3. WHEN 视频文件被修改 THEN System SHALL检测到变化并重新生成timeline文件
4. THE Timeline文件 SHALL包含关键帧时间戳、文件偏移、帧大小、GOP大小等信息
5. THE System SHALL支持timeline文件的版本控制和向后兼容

### 需求 3: FFmpeg命令行集成

**用户故事**: 作为开发者，我希望使用FFmpeg命令行工具解析视频文件，以便获得可靠的元数据和关键帧信息。

#### 验收标准

1. THE System SHALL使用FFmpeg命令行工具提取视频元数据（分辨率、帧率、编码格式、时长）
2. THE System SHALL使用FFmpeg提取关键帧位置信息
3. WHEN FFmpeg不可用 THEN System SHALL回退到基础的文件解析方法
4. THE System SHALL验证FFmpeg版本兼容性
5. THE System SHALL缓存FFmpeg解析结果以提高性能

### 需求 4: 高级播放控制器

**用户故事**: 作为平台端，我希望有完整的播放控制功能，以便实现倍速播放、精确定位、帧丢弃等高级功能。

#### 验收标准

1. THE System SHALL支持0.25x到4x的倍速播放
2. WHEN 倍速播放时 THEN System SHALL根据播放速率调整帧丢弃策略
3. THE System SHALL支持多种帧丢弃策略（DropNone, DropNonKeyframes, DropByRate, Adaptive）
4. THE System SHALL维护音视频同步信息
5. WHEN 播放速率改变 THEN System SHALL调整传输队列和缓冲区
6. THE System SHALL支持清除缓冲区操作
7. THE System SHALL记录最后的seek位置用于恢复

### 需求 5: 向后兼容性

**用户故事**: 作为系统维护者，我希望合并后的代码保持向后兼容，以便不影响现有功能。

#### 验收标准

1. THE System SHALL保持现有device-simulator的所有功能不变
2. THE System SHALL保持现有的QUIC协议兼容性
3. THE System SHALL保持现有的配置文件格式兼容
4. THE System SHALL保持现有的API接口不变
5. WHEN 新功能不可用 THEN System SHALL回退到现有实现

## 非功能性需求

### 性能需求

- Seek操作延迟 < 100ms
- 关键帧索引构建时间 < 5秒（对于1小时视频）
- Timeline文件加载时间 < 100ms
- 传输吞吐量 > 100 Mbps（目标1+ Gbps）
- 内存占用 < 100MB（关键帧索引）

### 兼容性需求

- 支持H.264、MP4、AVI、MOV、MKV格式
- 支持FFmpeg 4.0+版本
- 兼容现有的QUIC协议
- 支持Windows、macOS、Linux操作系统

### 可维护性需求

- 代码复用率 > 80%
- 测试覆盖率 > 80%
- 模块耦合度低
- 清晰的错误信息和日志

### 可扩展性需求

- 易于添加新的视频格式支持
- 易于添加新的索引优化策略
- 易于添加新的性能监控指标
- 易于集成新的传输协议

## 合并策略

### 阶段 1: 核心功能合并（高优先级）

1. **关键帧索引系统** (需求1, 2)
   - 复制 `file_reader.rs` 的关键帧索引功能
   - 复制 `types.rs` 中的KeyframeIndex、KeyframeEntry、SeekResult结构
   - 实现timeline文件的读写功能

2. **播放控制器** (需求4)
   - 复制 `controller.rs` 的PlaybackController trait和实现
   - 集成到现有的device_service.rs中
   - 实现seek、倍速播放、帧丢弃功能

### 阶段 2: 增强功能合并（中优先级）

3. **FFmpeg集成** (需求3)
   - 复制 `ffmpeg_cli_parser.rs`
   - 实现FFmpeg命令行调用
   - 添加回退机制

### 阶段 3: 测试和优化

4. **集成测试**
   - 端到端测试所有新功能
   - 性能基准测试
   - 兼容性测试

5. **文档更新**
   - 更新README.md
   - 添加新功能使用指南
   - 更新API文档

## 依赖关系

### 新增依赖

```toml
[dependencies]
# FFmpeg集成（可选，通过命令行调用）
# 无需额外依赖，使用系统FFmpeg命令行工具

# 其他功能使用现有依赖
# tokio, quinn, serde, uuid, tracing等
```

### 现有依赖保持

- tokio (异步运行时)
- quinn (QUIC协议)
- serde (序列化)
- uuid (唯一标识)
- tracing (日志)

## 风险评估

### 高风险

1. **协议兼容性** - 新功能可能影响现有QUIC协议
   - 缓解：保持协议向后兼容，添加版本协商

2. **性能影响** - 关键帧索引可能增加内存占用
   - 缓解：使用优化策略，支持内存限制

### 中风险

3. **FFmpeg依赖** - 需要系统安装FFmpeg
   - 缓解：提供回退机制，不强制依赖

### 低风险

5. **代码复杂度** - 合并后代码量增加
   - 缓解：保持模块化，清晰的文档

## 验收标准总结

系统将被认为满足需求，当且仅当：

1. ✅ 支持精确关键帧定位（亚秒级精度）
2. ✅ 支持timeline文件缓存
3. ✅ 支持FFmpeg命令行集成（可选）
4. ✅ 支持高级播放控制（倍速、帧丢弃）
5. ✅ 保持向后兼容性

## 附录

### 参考文档

- Device-Uploader README.md
- Device-Uploader USAGE.md
- Device-Simulator 现有代码
- QUIC协议规范 RFC 9000

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-14 | 初始版本 | 系统架构团队 |
