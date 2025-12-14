# Timeline 文件缓存系统

## 概述

Timeline 缓存系统通过将关键帧索引信息持久化到 JSON 文件，避免重复解析视频文件，显著提高启动速度和性能。

## 核心功能

### 1. 自动缓存

系统在首次解析视频文件后自动生成 `.timeline` 文件：

```
video.h264          # 原始视频文件
video.h264.timeline # Timeline 缓存文件
```

### 2. 智能验证

每次加载时自动验证缓存有效性：
- 文件大小检查
- 修改时间检查
- SHA-256 哈希验证

### 3. 版本控制

支持向后兼容的版本升级：
- 当前版本: v1
- 自动检测版本不兼容
- 失败时自动重建

## Timeline 文件格式

```json
{
  "version": 1,
  "video_file_path": "/path/to/video.h264",
  "video_file_hash": "sha256:abc123...",
  "video_file_size": 1048576000,
  "video_file_modified": "2025-12-14T10:00:00Z",
  "duration": 3600.0,
  "resolution": {
    "width": 1920,
    "height": 1080
  },
  "frame_rate": 30.0,
  "keyframe_index": {
    "entries": [
      {
        "timestamp": 0.0,
        "file_offset": 0,
        "frame_size": 65536,
        "gop_size": 30,
        "frame_type": "I"
      }
    ],
    "total_duration": 3600.0,
    "index_precision": 0.033,
    "memory_optimized": true,
    "optimization_strategy": "Adaptive",
    "memory_usage": 524288
  },
  "created_at": "2025-12-14T10:05:00Z",
  "ffmpeg_version": "4.4.2"
}
```

## 性能提升

### 首次加载（无缓存）
```
1. 打开视频文件
2. 解析 H.264 NAL 单元
3. 构建关键帧索引 (3-5秒)
4. 保存 Timeline 文件
5. 开始播放
总时间: ~5秒
```

### 后续加载（有缓存）
```
1. 加载 Timeline 文件 (< 100ms)
2. 验证文件哈希
3. 使用缓存的索引
4. 开始播放
总时间: ~100ms
```

**性能提升: 50倍**

## 配置选项

在 `config.rs` 中配置：

```rust
// 启用/禁用 Timeline 缓存
timeline_cache_enabled: true
```

环境变量：
```bash
TIMELINE_CACHE_ENABLED=true
```

## 使用示例

### 加载 Timeline

```rust
let timeline_manager = DefaultTimelineManager::new();
let video_path = Path::new("video.h264");

// 尝试加载缓存
match timeline_manager.load_timeline(video_path).await? {
    Some(timeline) => {
        // 验证缓存
        if timeline_manager.validate_timeline(&timeline, video_path).await? {
            println!("✓ Using cached index");
            let index = timeline.keyframe_index;
        } else {
            println!("⚠ Cache invalid, rebuilding");
            // 重建索引...
        }
    }
    None => {
        println!("📋 No cache found, building index");
        // 构建索引...
    }
}
```

### 保存 Timeline

```rust
// 构建索引
let index = file_reader.build_keyframe_index(&mut file).await?;

// 创建 Timeline
let timeline = TimelineFileBuilder::new(video_path.to_path_buf(), index)
    .build(&timeline_manager).await?;

// 保存到文件
timeline_manager.save_timeline(&timeline).await?;
```

### 删除 Timeline

```rust
// 删除缓存文件
timeline_manager.delete_timeline(video_path).await?;
```

## 缓存失效场景

Timeline 缓存在以下情况下会失效：

1. **文件被修改**: 修改时间或大小变化
2. **文件内容变化**: SHA-256 哈希不匹配
3. **版本不兼容**: Timeline 文件版本过旧
4. **文件损坏**: JSON 解析失败

失效时系统会自动重建索引并更新缓存。

## 缓存管理

### 查看缓存状态

```bash
# 查找所有 Timeline 文件
find test-videos -name "*.timeline"

# 查看文件大小
ls -lh test-videos/*.timeline
```

### 清理缓存

```bash
# 删除所有 Timeline 文件
rm test-videos/*.timeline

# 删除特定文件的缓存
rm test-videos/video.h264.timeline
```

### 缓存统计

```rust
// 获取缓存命中率
let total_loads = 100;
let cache_hits = 95;
let hit_rate = (cache_hits as f64 / total_loads as f64) * 100.0;
println!("Cache hit rate: {:.1}%", hit_rate);
```

## 技术细节

### 文件哈希计算

使用 SHA-256 计算文件哈希：

```rust
use sha2::{Sha256, Digest};

async fn calculate_hash(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];
    
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
```

### 验证逻辑

```rust
async fn validate_timeline(
    &self,
    timeline: &TimelineFile,
    video_path: &Path
) -> Result<bool> {
    // 1. 检查文件大小
    let metadata = tokio::fs::metadata(video_path).await?;
    if metadata.len() != timeline.video_file_size {
        return Ok(false);
    }
    
    // 2. 检查修改时间
    let modified = metadata.modified()?;
    if modified != timeline.video_file_modified {
        return Ok(false);
    }
    
    // 3. 验证文件哈希
    let hash = self.calculate_file_hash(video_path).await?;
    if hash != timeline.video_file_hash {
        return Ok(false);
    }
    
    Ok(true)
}
```

## 故障排除

### 问题：缓存总是失效

**原因**: 文件系统时间戳不稳定

**解决方案**:
- 检查文件系统挂载选项
- 使用哈希验证而非时间戳
- 禁用修改时间检查

### 问题：Timeline 文件过大

**原因**: 使用 Full 索引策略

**解决方案**:
- 切换到 Adaptive 或 Sparse 策略
- 设置内存限制
- 定期清理旧缓存

### 问题：加载缓存失败

**原因**: JSON 格式错误或版本不兼容

**解决方案**:
- 删除损坏的 Timeline 文件
- 系统会自动重建
- 检查磁盘空间

## 最佳实践

1. **始终启用缓存**: 显著提升性能
2. **定期清理**: 删除不再使用的视频的缓存
3. **监控缓存命中率**: 优化缓存策略
4. **备份重要缓存**: 避免重复构建大文件索引
5. **使用版本控制**: 跟踪 Timeline 文件格式变化

## 日志示例

```
INFO  ✓ Loaded keyframe index from timeline cache
INFO  ⚠ Timeline file invalid, rebuilding index
INFO  📋 No timeline cache found, building index
INFO  ✓ Timeline cache saved: video.h264.timeline
```

## 相关文档

- [关键帧索引系统](KEYFRAME_INDEX.md)
- [播放控制功能](PLAYBACK_CONTROL.md)
- [配置选项](README.md#配置)
