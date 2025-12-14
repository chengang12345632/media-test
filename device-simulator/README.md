# Device Simulator - 设备模拟器

## 概述

Device Simulator 是一个功能完整的视频设备模拟器，支持实时流传输和录像回放。集成了高级播放控制功能，包括精确关键帧定位、倍速播放和智能缓存。

## 🌟 核心功能

### 基础功能
- ✅ QUIC 连接和通信
- ✅ 视频文件扫描和管理
- ✅ 实时视频流传输
- ✅ 录像回放支持
- ✅ 设备注册和心跳
- ✅ 自动重连机制

### 高级功能（新增）
- 🌟 **精确关键帧定位** - 亚秒级精度的 seek 操作
- 🌟 **关键帧索引系统** - 多种优化策略（Full, Sparse, Adaptive, Hierarchical）
- 🌟 **Timeline 文件缓存** - JSON 格式的关键帧信息持久化
- 🌟 **FFmpeg CLI 集成** - 可靠的视频解析和元数据提取
- 🌟 **高级播放控制器** - 支持倍速播放（0.25x-4x）和帧丢弃策略
- 🌟 **高性能传输** - 优化的分片和传输策略

## 📚 文档

- [关键帧索引系统](KEYFRAME_INDEX.md) - 精确定位和索引优化
- [Timeline 缓存系统](TIMELINE_CACHE.md) - 缓存机制和性能优化
- [播放控制功能](PLAYBACK_CONTROL.md) - 倍速播放和帧丢弃策略

## 🚀 快速开始

### 安装依赖

```bash
# 确保已安装 Rust 1.70+
rustup --version

# 编译项目
cargo build --release
```

### 准备测试视频

```bash
# 创建视频目录
mkdir -p test-videos

# 将 H.264 或 MP4 视频文件放入目录
cp /path/to/your/video.h264 test-videos/
cp /path/to/your/video.mp4 test-videos/
```

### 启动设备模拟器

```bash
# 使用默认配置
cargo run --release

# 使用环境变量配置
DEVICE_ID=device_002 \
DEVICE_NAME="摄像头-02" \
PLATFORM_HOST=192.168.1.100 \
PLATFORM_PORT=8443 \
cargo run --release
```

## ⚙️ 配置

### 基础配置

```rust
// config.rs
pub struct Config {
    pub device_id: String,              // 设备ID
    pub device_name: String,            // 设备名称
    pub platform_host: String,          // 平台地址
    pub platform_port: u16,             // 平台端口
    pub video_dir: PathBuf,             // 视频目录
}
```

### 高级配置

```rust
// 关键帧索引配置
pub keyframe_index_strategy: IndexOptimizationStrategy,  // 默认: Adaptive
pub keyframe_index_memory_limit_mb: usize,               // 默认: 50MB

// Timeline 缓存配置
pub timeline_cache_enabled: bool,                        // 默认: true

// FFmpeg 配置
pub ffmpeg_enabled: bool,                                // 默认: true
pub ffmpeg_path: Option<PathBuf>,                        // 默认: 自动检测
pub ffmpeg_timeout_seconds: u64,                         // 默认: 30秒

// 播放控制配置
pub playback_speed_min: f32,                             // 默认: 0.25x
pub playback_speed_max: f32,                             // 默认: 4.0x
```

### 环境变量

```bash
# 基础配置
export DEVICE_ID=device_001
export DEVICE_NAME="模拟摄像头-01"
export PLATFORM_HOST=127.0.0.1
export PLATFORM_PORT=8443
export VIDEO_DIR=./test-videos

# 关键帧索引配置
export KEYFRAME_INDEX_STRATEGY=adaptive  # full, sparse, adaptive, hierarchical
export KEYFRAME_INDEX_MEMORY_LIMIT_MB=50

# Timeline 缓存配置
export TIMELINE_CACHE_ENABLED=true

# FFmpeg 配置
export FFMPEG_ENABLED=true
export FFMPEG_PATH=/usr/bin/ffmpeg
export FFMPEG_TIMEOUT_SECONDS=30

# 播放控制配置
export PLAYBACK_SPEED_MIN=0.25
export PLAYBACK_SPEED_MAX=4.0
```

## 📊 性能指标

### 关键帧索引
- **索引构建时间**: < 5秒（1小时视频）
- **Seek 响应时间**: < 100ms
- **内存占用**: < 100MB（自适应策略）
- **定位精度**: ≤ 0.1秒

### Timeline 缓存
- **首次加载**: ~5秒（需构建索引）
- **缓存加载**: < 100ms
- **性能提升**: 50倍
- **缓存文件大小**: ~1MB / 小时

### 播放控制
- **Seek 延迟**: < 100ms
- **速率切换**: 即时生效
- **支持速率**: 0.25x - 4.0x
- **音视频同步**: ± 50ms

## 🏗️ 项目结构

```
device-simulator/
├── src/
│   ├── main.rs                    # 主入口
│   ├── config.rs                  # 配置管理
│   ├── device_service.rs          # 设备服务
│   ├── quic/                      # QUIC 通信
│   │   ├── mod.rs
│   │   └── client.rs
│   ├── video/                     # 视频处理
│   │   ├── mod.rs
│   │   ├── types.rs               # 类型定义
│   │   ├── errors.rs              # 错误类型
│   │   ├── file_reader.rs         # 关键帧索引
│   │   ├── timeline.rs            # Timeline 缓存
│   │   ├── ffmpeg_parser.rs       # FFmpeg 集成
│   │   ├── controller.rs          # 播放控制器
│   │   ├── reader.rs              # 文件读取
│   │   └── live_stream_generator_file.rs  # 实时流生成
│   └── uploader/                  # 上传模块
│       ├── mod.rs
│       └── uploader.rs
├── test-videos/                   # 测试视频目录
├── Cargo.toml
├── README.md
├── KEYFRAME_INDEX.md             # 关键帧索引文档
├── TIMELINE_CACHE.md             # Timeline 缓存文档
└── PLAYBACK_CONTROL.md           # 播放控制文档
```

## 🔧 开发

### 编译

```bash
# 开发模式
cargo build

# 发布模式
cargo build --release

# 检查代码
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```

### 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --test integration_tests

# 显示测试输出
cargo test -- --nocapture
```

### 调试

```bash
# 启用详细日志
export RUST_LOG=debug
cargo run

# 启用特定模块日志
export RUST_LOG=device_simulator::video=debug
cargo run
```

## 📝 使用示例

### 基础播放

```rust
// 1. 连接到平台
let mut client = QuicClient::new(config).await?;
client.connect().await?;

// 2. 扫描视频文件
let video_files = scan_video_files(&config.video_dir)?;

// 3. 启动设备服务
let service = DeviceService::new_with_config(
    client,
    video_files,
    config.device_id,
    config.video_dir,
    Some(config),
);
service.run().await?;
```

### 高级播放控制

```rust
// 1. 加载关键帧索引
let timeline_manager = DefaultTimelineManager::new();
let index = match timeline_manager.load_timeline(&video_path).await? {
    Some(timeline) if timeline_manager.validate_timeline(&timeline, &video_path).await? => {
        timeline.keyframe_index
    }
    _ => {
        // 构建新索引
        let file_reader = DefaultFileStreamReader::new();
        let mut file = tokio::fs::File::open(&video_path).await?;
        file_reader.build_keyframe_index_with_strategy(
            &mut file,
            IndexOptimizationStrategy::Adaptive
        ).await?
    }
};

// 2. 执行 Seek
let result = file_reader.seek_to_time(&mut file, 30.0, &index).await?;
println!("Seeked to {:.2}s (precision: {:.3}s)", 
         result.actual_time, result.precision_achieved);

// 3. 设置播放速率
let controller = DefaultPlaybackController::new();
controller.set_playback_rate(2.0).await?;
```

## 🐛 故障排除

### 问题：连接平台失败

**解决方案**:
1. 检查平台服务是否启动
2. 验证 IP 地址和端口配置
3. 检查防火墙设置
4. 查看日志: `RUST_LOG=debug cargo run`

### 问题：视频文件无法播放

**解决方案**:
1. 确认文件格式（支持 H.264, MP4）
2. 检查文件权限
3. 验证文件完整性
4. 尝试使用 FFmpeg 验证: `ffmpeg -i video.h264`

### 问题：Seek 操作失败

**解决方案**:
1. 确认关键帧索引已构建
2. 检查 Timeline 缓存是否有效
3. 验证 FFmpeg 是否可用
4. 查看错误日志

### 问题：内存占用过高

**解决方案**:
1. 降低内存限制: `KEYFRAME_INDEX_MEMORY_LIMIT_MB=30`
2. 使用更激进的策略: `KEYFRAME_INDEX_STRATEGY=hierarchical`
3. 清理 Timeline 缓存: `rm test-videos/*.timeline`
4. 监控内存使用: `cargo run --release`

## 📈 性能优化建议

1. **启用 Timeline 缓存**: 避免重复构建索引
2. **使用 Adaptive 策略**: 平衡性能和内存
3. **启用 FFmpeg**: 提高索引准确性
4. **发布模式编译**: `cargo build --release`
5. **调整内存限制**: 根据系统资源配置

## 🔗 相关链接

- [主项目 README](../README.md)
- [平台服务器文档](../platform-server/README.md)
- [Web 前端文档](../web-frontend/README.md)
- [API 文档](../docs/API接口文档.md)
- [系统架构](../docs/系统架构设计文档.md)

## 📄 许可证

MIT License - 查看 [LICENSE](../LICENSE) 文件了解详情。

## 🙏 致谢

- [Quinn](https://github.com/quinn-rs/quinn) - Rust QUIC 实现
- [Tokio](https://tokio.rs/) - 异步运行时
- [FFmpeg](https://ffmpeg.org/) - 视频处理工具

---

<div align="center">

**Device Simulator** - 功能完整的视频设备模拟器

Made with ❤️ by 系统架构团队

</div>
