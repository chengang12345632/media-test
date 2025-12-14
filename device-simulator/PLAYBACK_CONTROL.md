# 播放控制功能

## 概述

播放控制器提供完整的视频播放控制功能，包括精确定位、倍速播放、帧丢弃策略和音视频同步。

## 核心功能

### 1. 精确 Seek

使用关键帧索引实现亚秒级精度的定位：

```rust
// 定位到 30 秒位置
let result = controller.seek_to_keyframe(30.0, &index).await?;

println!("请求时间: {:.2}s", result.requested_time);
println!("实际时间: {:.2}s", result.actual_time);
println!("精度: {:.3}s", result.precision_achieved);
println!("执行时间: {}ms", result.execution_time.as_millis());
```

### 2. 倍速播放

支持 0.25x 到 4x 的播放速率：

```rust
// 设置 2x 倍速播放
controller.set_playback_rate(2.0).await?;

// 支持的速率范围
// 0.25x - 慢速播放
// 0.5x  - 半速播放
// 1.0x  - 正常速度
// 1.5x  - 1.5倍速
// 2.0x  - 2倍速
// 4.0x  - 4倍速
```

### 3. 帧丢弃策略

根据播放速率自动选择帧丢弃策略：

#### DropNone（不丢帧）
- 播放速率: ≤ 1.0x
- 保留所有帧
- 最佳画质

#### DropNonKeyframes（仅保留关键帧）
- 播放速率: 1.5x - 2.0x
- 仅传输关键帧
- 平衡性能和画质

#### DropByRate（按比例丢帧）
- 播放速率: 2.0x - 3.0x
- 按固定比例丢帧
- 保持流畅性

#### Adaptive（自适应丢帧）
- 播放速率: > 3.0x
- 根据网络和缓冲区动态调整
- 最佳性能

### 4. 缓冲区管理

动态管理视频和音频缓冲区：

```rust
// 清除缓冲区（Seek 后）
controller.clear_buffers()?;

// 获取缓冲区健康状态
let health = controller.get_buffer_health();
println!("视频缓冲: {:.1}%", health.video_buffer_level);
println!("音频缓冲: {:.1}%", health.audio_buffer_level);
```

### 5. 音视频同步

自动维护音视频同步：

```rust
// 系统自动调整同步偏移
// Seek 后自动重置为 0.0
// 播放过程中动态调整
```

## QUIC 协议扩展

### 新增命令

#### SeekToKeyframe (0x12)

精确定位到关键帧：

```
请求:
+--------+----------------+----------------+
| 0x12   | target_time    | session_id     |
| 1 byte | 8 bytes (f64)  | 16 bytes (UUID)|
+--------+----------------+----------------+

响应:
+--------+----------------+----------------+----------------+
| 0x15   | requested_time | actual_time    | precision      |
| 1 byte | 8 bytes (f64)  | 8 bytes (f64)  | 8 bytes (f64)  |
+--------+----------------+----------------+----------------+
```

#### SetPlaybackSpeed (0x13)

设置播放速率：

```
请求:
+--------+----------------+----------------+
| 0x13   | speed          | session_id     |
| 1 byte | 4 bytes (f32)  | 16 bytes (UUID)|
+--------+----------------+----------------+

响应:
+--------+----------------+----------------+
| 0x09   | success        | error_message  |
| 1 byte | 1 byte (bool)  | variable       |
+--------+----------------+----------------+
```

#### GetKeyframeIndex (0x14)

获取关键帧索引：

```
请求:
+--------+----------------+
| 0x14   | file_path      |
| 1 byte | variable       |
+--------+----------------+

响应:
+--------+----------------+----------------+
| 0x16   | keyframe_count | keyframe_data  |
| 1 byte | 4 bytes (u32)  | variable       |
+--------+----------------+----------------+
```

## 配置选项

在 `config.rs` 中配置：

```rust
// 播放速率范围
playback_speed_min: 0.25
playback_speed_max: 4.0
```

环境变量：
```bash
PLAYBACK_SPEED_MIN=0.25
PLAYBACK_SPEED_MAX=4.0
```

## 使用示例

### 基础播放控制

```rust
let controller = DefaultPlaybackController::new();

// 开始播放
controller.seek(0.0).await?;

// 暂停（设置速率为 0）
controller.set_playback_rate(0.0).await?;

// 恢复播放
controller.set_playback_rate(1.0).await?;

// 快进到 1分钟
controller.seek_to_keyframe(60.0, &index).await?;

// 2倍速播放
controller.set_playback_rate(2.0).await?;
```

### 高级控制

```rust
// 获取当前位置
let position = controller.get_current_position();

// 获取播放速率
let rate = controller.get_playback_rate();

// 获取最后 Seek 位置
if let Some(last_seek) = controller.get_last_seek_position() {
    println!("Last seek: {:.2}s", last_seek);
}

// 调整传输队列
let segments = vec![/* ... */];
let adjusted = controller.adjust_transmission_queue(
    segments,
    2.0  // 2x 播放速率
);
```

## 性能指标

- **Seek 延迟**: < 100ms
- **速率切换**: 即时生效
- **缓冲区调整**: < 50ms
- **音视频同步**: ± 50ms

## 帧丢弃策略详解

### 策略选择逻辑

```rust
fn get_drop_frame_strategy(rate: f64) -> DropFrameStrategy {
    match rate {
        r if r <= 1.0 => DropFrameStrategy::DropNone,
        r if r <= 2.0 => DropFrameStrategy::DropNonKeyframes,
        r if r <= 3.0 => DropFrameStrategy::DropByRate(0.5),
        _ => DropFrameStrategy::Adaptive,
    }
}
```

### 策略效果对比

| 速率 | 策略 | 帧保留率 | 带宽占用 | 画质 |
|------|------|----------|----------|------|
| 1.0x | DropNone | 100% | 100% | 最佳 |
| 1.5x | DropNonKeyframes | ~10% | ~10% | 良好 |
| 2.0x | DropNonKeyframes | ~10% | ~10% | 可接受 |
| 3.0x | DropByRate(0.5) | ~5% | ~5% | 较差 |
| 4.0x | Adaptive | ~3% | ~3% | 最差 |

## 故障排除

### 问题：Seek 后播放卡顿

**原因**: 未清除缓冲区

**解决方案**:
```rust
controller.seek_to_keyframe(time, &index).await?;
controller.clear_buffers()?;  // 清除旧数据
```

### 问题：倍速播放不流畅

**原因**: 帧丢弃策略不当

**解决方案**:
- 检查网络带宽
- 降低播放速率
- 使用更激进的丢帧策略

### 问题：音视频不同步

**原因**: 同步偏移未正确维护

**解决方案**:
- Seek 后自动重置同步偏移
- 检查时间戳计算
- 调整缓冲区大小

## 最佳实践

1. **Seek 前检查索引**: 确保关键帧索引已加载
2. **Seek 后清除缓冲**: 避免播放旧数据
3. **渐进式速率调整**: 避免突然的速率变化
4. **监控缓冲区健康**: 及时调整策略
5. **记录 Seek 位置**: 便于恢复播放

## 客户端集成示例

### Web 客户端

```javascript
// Seek 到指定位置
async function seekTo(time) {
    const response = await fetch('/api/playback/seek', {
        method: 'POST',
        body: JSON.stringify({
            session_id: sessionId,
            target_time: time
        })
    });
    
    const result = await response.json();
    console.log(`Seeked to ${result.actual_time}s`);
}

// 设置播放速率
async function setSpeed(speed) {
    await fetch('/api/playback/speed', {
        method: 'POST',
        body: JSON.stringify({
            session_id: sessionId,
            speed: speed
        })
    });
}
```

### 移动客户端

```kotlin
// Android 示例
class PlaybackController(private val api: ApiService) {
    suspend fun seekTo(time: Double): SeekResult {
        return api.seekToKeyframe(
            SeekRequest(sessionId, time)
        )
    }
    
    suspend fun setSpeed(speed: Float) {
        api.setPlaybackSpeed(
            SpeedRequest(sessionId, speed)
        )
    }
}
```

## 日志示例

```
INFO  ⏩ Received seek to keyframe request
INFO    Target time: 30.00s
INFO  ✓ Keyframe index loaded: 120 keyframes, 3600.00s duration
INFO  ✓ Seeked to 29.97s (precision: 0.03s, time: 45ms)

INFO  ⚡ Received set playback speed request
INFO    Speed: 2.0x
INFO  ✓ Playback speed set to 2.0x
INFO  ✓ Frame drop strategy: DropNonKeyframes
```

## 相关文档

- [关键帧索引系统](KEYFRAME_INDEX.md)
- [Timeline 缓存系统](TIMELINE_CACHE.md)
- [QUIC 协议规范](../docs/QUIC_PROTOCOL.md)
- [配置选项](README.md#配置)
