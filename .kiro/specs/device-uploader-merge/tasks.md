# Device-Uploader 功能合并实现任务

## 任务概述

本任务列表将 device-uploader 的高级功能合并到 device-simulator 项目中。任务按照依赖关系组织，每个任务都包含明确的目标和验收标准。

## 任务列表

- [x] 1. 设置项目结构和核心类型定义





  - 在 device-simulator/src/video/ 目录下创建新模块
  - 定义核心数据结构：KeyframeIndex、KeyframeEntry、SeekResult、FrameType、IndexOptimizationStrategy
  - 定义错误类型：FileError、TimelineError、PlaybackError、FFmpegError
  - 从 device-uploader/src/types.rs 复制并适配类型定义
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 1.6, 2.4, 4.2, 4.3_

- [x] 2. 实现关键帧索引系统





  - 创建 video/file_reader.rs 模块
  - 实现 FileStreamReader trait 定义
  - 实现 DefaultFileStreamReader 结构体
  - 实现 H.264 NAL 单元解析逻辑
  - 实现关键帧检测（识别 I 帧）
  - 实现 build_keyframe_index 方法
  - 实现 build_keyframe_index_with_strategy 方法（支持 Full、Sparse、Adaptive、Hierarchical 策略）
  - 实现 build_keyframe_index_with_memory_limit 方法
  - 实现 seek_to_time 方法（使用关键帧索引定位）
  - 实现 find_nearest_keyframe 辅助方法（二分查找）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

- [ ]* 2.1 编写关键帧索引系统的属性测试
  - **Property 1: Seek 定位到最近关键帧**
  - **Validates: Requirements 1.1**
  - **Property 2: 非关键帧位置自动对齐**
  - **Validates: Requirements 1.3**
  - **Property 3: Seek 精度保证**
  - **Validates: Requirements 1.4**
  - **Property 4: SeekResult 完整性**
  - **Validates: Requirements 1.5**

- [x] 3. 实现 Timeline 文件缓存系统


  - 创建 video/timeline.rs 模块
  - 定义 TimelineFile 数据结构（包含版本、视频信息、关键帧索引）
  - 实现 TimelineManager trait 定义
  - 实现 DefaultTimelineManager 结构体
  - 实现 load_timeline 方法（从 JSON 文件加载）
  - 实现 save_timeline 方法（保存为 JSON 文件）
  - 实现 validate_timeline 方法（验证文件哈希、大小、修改时间）
  - 实现 get_timeline_path 方法（生成 .timeline 文件路径）
  - 实现文件哈希计算（使用 SHA-256）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ]* 3.1 编写 Timeline 缓存系统的属性测试
  - **Property 5: Timeline 文件生成**
  - **Validates: Requirements 2.1**
  - **Property 6: Timeline 文件结构完整性**
  - **Validates: Requirements 2.4**





- [x] 4. 实现 FFmpeg CLI 集成

  - 创建 video/ffmpeg_parser.rs 模块
  - 定义 FFmpegVideoInfo 数据结构
  - 定义 FFmpegConfig 配置结构
  - 实现 FFmpegParser trait 定义
  - 实现 DefaultFFmpegParser 结构体
  - 实现 check_availability 方法（检查 FFmpeg 是否安装）
  - 实现 get_version 方法（获取 FFmpeg 版本）
  - 实现 extract_metadata 方法（使用 ffprobe 提取元数据）
  - 实现 extract_keyframes 方法（使用 ffprobe 提取关键帧时间戳）
  - 实现 validate_video 方法（验证视频文件格式）
  - 实现命令行调用和输出解析
  - 实现超时控制机制
  - 实现回退机制（FFmpeg 不可用时使用基础解析器）
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ]* 4.1 编写 FFmpeg 集成的属性测试
  - **Property 7: FFmpeg 元数据提取完整性**
  - **Validates: Requirements 3.1**
  - **Property 8: FFmpeg 关键帧提取**
  - **Validates: Requirements 3.2**

- [x] 5. 实现播放控制器




  - 创建 video/controller.rs 模块
  - 定义 DropFrameStrategy 枚举（DropNone、DropNonKeyframes、DropByRate、Adaptive）
  - 定义 BufferManager 结构体
  - 实现 PlaybackController trait 定义
  - 实现 DefaultPlaybackController 结构体
  - 实现 seek 方法（基础定位）
  - 实现 seek_to_keyframe 方法（使用关键帧索引精确定位）
  - 实现 set_playback_rate 方法（设置播放速率 0.25x-4x）
  - 实现 get_drop_frame_strategy 方法（根据播放速率选择帧丢弃策略）
  - 实现 adjust_transmission_queue 方法（根据播放速率调整传输队列）
  - 实现 clear_buffers 方法（清除缓冲区）
  - 实现 find_nearest_keyframe 方法（查找最近关键帧）
  - 实现 adjust_audio_video_sync 方法（调整音视频同步）
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [ ]* 5.1 编写播放控制器的属性测试
  - **Property 9: 播放速率调整帧丢弃策略**
  - **Validates: Requirements 4.2**
  - **Property 10: 音视频同步维护**
  - **Validates: Requirements 4.4**
  - **Property 11: 播放速率改变时队列调整**
  - **Validates: Requirements 4.5**
  - **Property 12: Seek 位置记录**
  - **Validates: Requirements 4.7**

- [x] 6. 集成到 device_service


  - 修改 device_service.rs，添加新模块的引用
  - 在 DeviceService 结构体中添加 PlaybackController 字段
  - 在 DeviceService 结构体中添加 TimelineManager 字段
  - 在 DeviceService 结构体中添加 FFmpegParser 字段（可选）
  - 修改录像回放启动逻辑，加载或构建关键帧索引
  - 实现 Timeline 文件的自动加载和保存
  - 实现 FFmpeg 解析的集成（优先使用 FFmpeg，失败时回退）
  - 保持现有功能不变，确保向后兼容
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 7. 扩展 QUIC 协议


  - 修改 quic/protocol.rs，添加新的命令定义
  - 定义 PlaybackCommand 枚举（Seek、SetPlaybackSpeed、GetKeyframeIndex）
  - 定义命令码常量（CMD_SEEK = 0x10、CMD_SET_PLAYBACK_SPEED = 0x11、CMD_GET_KEYFRAME_INDEX = 0x12）
  - 实现 Seek 命令的编码和解码
  - 实现 SetPlaybackSpeed 命令的编码和解码
  - 实现 GetKeyframeIndex 命令的编码和解码
  - 实现 Seek 响应的编码和解码（包含 SeekResult）
  - 在 device_service.rs 中添加命令处理逻辑
  - 实现 handle_seek_command 方法
  - 实现 handle_set_playback_speed_command 方法
  - 实现 handle_get_keyframe_index_command 方法
  - _Requirements: 1.1, 1.5, 4.1, 4.2_


- [x] 8. 添加配置选项


  - 修改 config.rs，添加新功能的配置项
  - 添加 keyframe_index_strategy 配置（默认 Adaptive）
  - 添加 keyframe_index_memory_limit_mb 配置（默认 50MB）
  - 添加 timeline_cache_enabled 配置（默认 true）
  - 添加 ffmpeg_enabled 配置（默认 true）
  - 添加 ffmpeg_path 配置（默认自动检测）
  - 添加 ffmpeg_timeout_seconds 配置（默认 30 秒）
  - 添加 playback_speed_range 配置（默认 0.25-4.0）
  - 实现配置验证逻辑
  - _Requirements: 1.6, 1.7, 3.3, 4.1_

- [x] 9. Checkpoint - 确保所有测试通过


  - 确保所有测试通过，如有问题请询问用户

- [ ] 10. 性能优化
  - 实现零拷贝文件读取（使用 mmap 或 sendfile）
  - 优化关键帧索引的内存布局
  - 实现索引的延迟加载（按需加载部分索引）
  - 优化 Timeline 文件的序列化性能
  - 实现缓冲区池，减少内存分配
  - 添加性能监控日志
  - _Requirements: 非功能性需求 - 性能_

- [ ]* 10.1 编写性能基准测试
  - 测试索引构建时间（目标：1小时视频 < 5秒）
  - 测试 Seek 响应时间（目标：< 100ms）
  - 测试 Timeline 文件加载时间（目标：< 100ms）
  - 测试内存使用（目标：< 100MB）

- [x] 11. 错误处理和日志


  - 为所有新模块添加详细的错误日志
  - 实现关键操作的 tracing 日志
  - 添加索引构建进度日志
  - 添加 Seek 操作日志（请求时间、实际时间、精度、执行时间）
  - 添加 Timeline 缓存命中/未命中日志
  - 添加 FFmpeg 调用日志
  - 实现错误恢复机制
  - _Requirements: 非功能性需求 - 可维护性_

- [ ] 12. 集成测试
  - 创建端到端测试：加载视频 -> 构建索引 -> Seek -> 播放
  - 测试 Timeline 缓存：首次加载 -> 再次加载（使用缓存）
  - 测试 FFmpeg 集成：FFmpeg 可用 -> FFmpeg 不可用（回退）
  - 测试倍速播放：不同播放速率 -> 验证帧丢弃策略
  - 测试 QUIC 协议：发送 Seek 命令 -> 接收响应 -> 验证结果
  - 测试向后兼容性：旧配置文件 -> 新功能可选
  - 测试多种视频格式：H.264、MP4、AVI、MOV、MKV
  - _Requirements: 所有需求_

- [ ]* 12.1 编写集成测试用例
  - 端到端 Seek 测试
  - Timeline 缓存测试
  - FFmpeg 回退测试
  - 倍速播放测试
  - 协议兼容性测试

- [x] 13. 文档更新



  - 更新 device-simulator/README.md，添加新功能说明
  - 创建 KEYFRAME_INDEX.md 文档，说明关键帧索引系统
  - 创建 TIMELINE_CACHE.md 文档，说明 Timeline 缓存机制
  - 创建 PLAYBACK_CONTROL.md 文档，说明播放控制功能
  - 更新 API 文档，添加新的 QUIC 命令说明
  - 添加配置示例和最佳实践
  - 添加故障排除指南
  - _Requirements: 非功能性需求 - 可维护性_

- [ ] 14. Final Checkpoint - 确保所有测试通过
  - 确保所有测试通过，如有问题请询问用户

## 任务依赖关系

```
1 (类型定义)
├── 2 (关键帧索引)
│   ├── 2.1* (属性测试)
│   └── 3 (Timeline 缓存)
│       ├── 3.1* (属性测试)
│       └── 6 (集成到 device_service)
├── 4 (FFmpeg 集成)
│   ├── 4.1* (属性测试)
│   └── 6 (集成到 device_service)
└── 5 (播放控制器)
    ├── 5.1* (属性测试)
    └── 6 (集成到 device_service)

6 (集成到 device_service)
└── 7 (QUIC 协议扩展)
    └── 8 (配置选项)
        └── 9 (Checkpoint)
            └── 10 (性能优化)
                ├── 10.1* (性能测试)
                └── 11 (错误处理和日志)
                    └── 12 (集成测试)
                        ├── 12.1* (集成测试用例)
                        └── 13 (文档更新)
                            └── 14 (Final Checkpoint)
```

## 实现注意事项

### 代码复用

1. 从 device-uploader 复制以下文件作为起点：
   - `types.rs` -> `video/types.rs`
   - `file_reader.rs` -> `video/file_reader.rs`
   - `controller.rs` -> `video/controller.rs`
   - `ffmpeg_cli_parser.rs` -> `video/ffmpeg_parser.rs`
   - `timeline_manager.rs` -> `video/timeline.rs`

2. 适配代码以匹配 device-simulator 的架构：
   - 调整模块路径和导入
   - 适配错误类型
   - 集成到现有的 DeviceService

### 向后兼容性

1. 所有新功能都是可选的
2. 配置文件保持向后兼容
3. QUIC 协议使用新的命令码，不影响现有命令
4. 现有的基础播放功能保持不变

### 测试策略

1. 单元测试：测试每个模块的核心功能
2. 属性测试：使用 proptest 验证正确性属性
3. 集成测试：测试端到端流程
4. 性能测试：验证性能指标

### 性能目标

- Seek 操作延迟 < 100ms
- 关键帧索引构建时间 < 5秒（1小时视频）
- Timeline 文件加载时间 < 100ms
- 内存占用 < 100MB（关键帧索引）

## 验收标准

任务完成的标准：

1. ✅ 所有代码编译通过，无警告
2. ✅ 所有单元测试通过
3. ✅ 所有属性测试通过（至少 100 次迭代）
4. ✅ 所有集成测试通过
5. ✅ 性能指标达标
6. ✅ 向后兼容性验证通过
7. ✅ 文档完整且准确

