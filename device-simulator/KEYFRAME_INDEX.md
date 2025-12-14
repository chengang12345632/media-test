# 关键帧索引系统

## 概述

关键帧索引系统为视频文件提供精确的定位能力，支持亚秒级精度的 seek 操作。系统通过解析 H.264 NAL 单元识别关键帧（I 帧），并构建索引以实现快速定位。

## 核心功能

### 1. 关键帧检测

系统自动解析 H.264 视频流，识别关键帧位置：
- 解析 NAL 单元头部
- 识别 I 帧（独立解码帧）
- 记录文件偏移和时间戳

### 2. 索引优化策略

支持多种索引优化策略以平衡内存使用和定位精度：

#### Full（完整索引）
- 索引所有关键帧
- 最高精度
- 适用于短视频（< 10分钟）
- 内存使用：约 1MB / 小时

#### Sparse（稀疏索引）
- 定期采样关键帧（每 N 个）
- 中等精度
- 适用于中等长度视频（10-60分钟）
- 内存使用：约 500KB / 小时

#### Adaptive（自适应）**（默认）**
- 根据内存限制动态调整
- 平衡精度和内存
- 适用于大多数场景
- 内存使用：约 50MB 上限

#### Hierarchical（分层索引）
- 多级精度索引
- 最低内存占用
- 适用于长视频（> 1小时）
- 内存使用：约 200KB / 小时

### 3. Seek 操作

精确定位到指定时间点：

```rust
// 使用关键帧索引定位
let result = file_reader.seek_to_time(
    &mut file,
    target_time,
    &keyframe_index
).await?;

// 返回 SeekResult
// - requested_time: 请求的时间
// - actual_time: 实际定位到的时间
// - keyframe_offset: 文件偏移位置
// - precision_achieved: 达到的精度
// - execution_time: 执行时间
```

## 性能指标

- **索引构建时间**: < 5秒（1小时视频）
- **Seek 响应时间**: < 100ms
- **内存占用**: < 100MB（自适应策略）
- **定位精度**: ≤ 0.1秒

## 配置选项

在 `config.rs` 中配置：

```rust
// 索引策略
keyframe_index_strategy: IndexOptimizationStrategy::Adaptive

// 内存限制（MB）
keyframe_index_memory_limit_mb: 50
```

环境变量：
```bash
KEYFRAME_INDEX_STRATEGY=adaptive  # full, sparse, adaptive, hierarchical
KEYFRAME_INDEX_MEMORY_LIMIT_MB=50
```

## 使用示例

### 构建索引

```rust
let file_reader = DefaultFileStreamReader::new();
let mut file = tokio::fs::File::open("video.h264").await?;

// 使用默认策略
let index = file_reader.build_keyframe_index(&mut file).await?;

// 使用特定策略
let index = file_reader.build_keyframe_index_with_strategy(
    &mut file,
    IndexOptimizationStrategy::Sparse
).await?;

// 使用内存限制
let index = file_reader.build_keyframe_index_with_memory_limit(
    &mut file,
    30  // 30MB
).await?;
```

### 执行 Seek

```rust
// 定位到 30 秒位置
let offset = file_reader.seek_to_time(
    &mut file,
    30.0,
    &index
).await?;

// 从新位置读取数据
let mut buffer = vec![0u8; 8192];
file.read(&mut buffer).await?;
```

## 技术细节

### NAL 单元解析

系统解析 H.264 NAL 单元以识别帧类型：

```
NAL Unit Header (1 byte):
+---+---+---+---+---+---+---+---+
| F | NRI   | Type              |
+---+---+---+---+---+---+---+---+

Type = 5: IDR (I 帧，关键帧)
Type = 1: Non-IDR (P/B 帧)
```

### 索引数据结构

```rust
pub struct KeyframeIndex {
    pub entries: Vec<KeyframeEntry>,
    pub total_duration: f64,
    pub index_precision: f64,
    pub memory_optimized: bool,
    pub optimization_strategy: IndexOptimizationStrategy,
    pub memory_usage: usize,
}

pub struct KeyframeEntry {
    pub timestamp: f64,        // 时间戳（秒）
    pub file_offset: u64,      // 文件偏移位置
    pub frame_size: u32,       // 关键帧大小
    pub gop_size: u32,         // GOP大小
    pub frame_type: FrameType, // 帧类型
}
```

## 故障排除

### 问题：索引构建时间过长

**原因**: 视频文件过大或使用 Full 策略

**解决方案**:
- 使用 Adaptive 或 Sparse 策略
- 降低内存限制以触发更激进的优化
- 使用 Timeline 缓存避免重复构建

### 问题：Seek 精度不足

**原因**: 使用了过于激进的优化策略

**解决方案**:
- 使用 Full 或 Adaptive 策略
- 增加内存限制
- 检查视频的 GOP 大小

### 问题：内存占用过高

**原因**: 使用 Full 策略处理长视频

**解决方案**:
- 切换到 Adaptive 或 Hierarchical 策略
- 设置内存限制
- 使用 Timeline 缓存减少内存占用

## 最佳实践

1. **默认使用 Adaptive 策略**: 适用于大多数场景
2. **启用 Timeline 缓存**: 避免重复构建索引
3. **根据视频长度选择策略**:
   - < 10分钟: Full
   - 10-60分钟: Adaptive
   - > 1小时: Hierarchical
4. **监控内存使用**: 根据实际情况调整策略
5. **使用 FFmpeg 辅助**: 提高索引准确性

## 相关文档

- [Timeline 缓存系统](TIMELINE_CACHE.md)
- [播放控制功能](PLAYBACK_CONTROL.md)
- [配置选项](README.md#配置)
