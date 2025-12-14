# 回放功能修复总结

## 修复内容

### 1. 后端修复 (platform-server/src/http3/streaming.rs)

**问题：**
- 文件路径使用相对路径 `../device-simulator/test-videos`，在不同工作目录下会失败
- Content-Type 固定为 `video/mp4`，不支持 H.264 文件

**修复：**
- ✅ 支持多个可能的路径查找
- ✅ 根据文件扩展名自动设置 Content-Type
- ✅ 添加详细的日志输出

### 2. 前端修复 (web-frontend/src/components/VideoPlayer.tsx)

**问题：**
- 文件类型检测不够准确
- 没有正确处理 file_id 中包含的文件名

**修复：**
- ✅ 改进文件类型检测逻辑
- ✅ 支持 `.mp4`, `.h264`, `.264` 等多种扩展名
- ✅ 添加详细的日志输出

## 支持的播放模式

### 直通播放（Live Streaming）
- **格式：** H.264 裸流
- **播放器：** WebCodecsPlayer
- **特点：** 实时流式播放，低延迟

### 录像回放（Playback）

#### MP4 文件
- **播放器：** 浏览器原生 `<video>` 标签
- **传输方式：** HTTP Range 请求（直接流式传输）
- **特点：**
  - ✅ 可以直接播放
  - ✅ 支持拖动进度条
  - ✅ 支持快进快退
  - ✅ 支持倍速播放
  - ✅ 无需转换

#### H.264 文件
- **播放器：** H264Player (使用 MSE + mux.js)
- **传输方式：** SSE (Server-Sent Events)
- **特点：**
  - ✅ 实时转换为 fMP4
  - ✅ 支持流式播放
  - ⚠️ 需要标准 Annex B 格式
  - ⚠️ 必须包含 SPS/PPS

## 使用流程

### 1. 重新编译并启动

```powershell
# 方式1：使用重启脚本（推荐）
powershell -ExecutionPolicy Bypass -File .\rebuild-and-restart.ps1

# 方式2：手动操作
# 停止现有进程
# 重新编译
cd platform-server
cargo build
cd ..

# 启动服务
powershell -ExecutionPolicy Bypass -File .\start-services.ps1
```

### 2. 测试 MP4 回放

1. 访问 http://localhost:5173
2. 选择设备 `device_001`
3. 点击"查看录像"
4. 选择任意 MP4 文件（如 `oceans.mp4`）
5. 点击"播放"
6. 应该可以看到视频直接播放，支持进度条拖动

### 3. 测试 H.264 回放

1. 访问 http://localhost:5173
2. 选择设备 `device_001`
3. 点击"查看录像"
4. 选择 H.264 文件（如 `sample_720p_60fps.h264`）
5. 点击"播放"
6. 应该可以看到视频通过 SSE 流式播放

## 故障排查

### MP4 播放失败

**检查项：**
1. 浏览器控制台是否有错误
2. Network 标签中 `/api/v1/recordings/*/stream` 请求状态
3. platform-server 日志中的文件路径

**常见问题：**
- 404 错误 → 文件路径不正确
- 403 错误 → 权限问题
- 视频无法播放 → 编码格式不兼容

**解决方案：**
```powershell
# 检查文件是否存在
Test-Path "device-simulator\test-videos\oceans.mp4"

# 查看 platform-server 日志
# 在 platform-server 窗口中查看输出
```

### H.264 播放失败

**检查项：**
1. 是否成功启动回放会话（查看 Network 标签中的 `/api/v1/playback/start` 请求）
2. SSE 连接是否建立（查看 `/api/v1/stream/*/segments` 请求）
3. 是否收到视频分片数据

**常见问题：**
- "H.264 data does not have NAL start code" → 文件格式不正确
- "No data received from transmuxer" → 文件无法转换
- SSE 连接失败 → 会话未创建或已过期

**解决方案：**
```powershell
# 检查 H.264 文件格式
ffprobe device-simulator\test-videos\sample_720p_60fps.h264

# 重新生成标准格式的 H.264 文件
ffmpeg -i input.mp4 -c:v copy -bsf:v h264_mp4toannexb -an output.h264
```

## 文件路径说明

### 后端查找顺序
1. `device-simulator/test-videos/filename`
2. `../device-simulator/test-videos/filename`
3. `./test-videos/filename`

### 推荐配置
- 从项目根目录启动 platform-server
- 视频文件放在 `device-simulator/test-videos/` 目录

## 测试清单

- [ ] MP4 文件可以播放
- [ ] MP4 文件可以拖动进度条
- [ ] MP4 文件可以快进快退
- [ ] H.264 文件可以通过 SSE 播放
- [ ] H.264 直通播放正常工作
- [ ] 多个客户端可以同时观看

## 下一步优化

1. **性能优化**
   - 添加视频文件缓存
   - 优化 H.264 转换性能
   - 支持多码率自适应

2. **功能增强**
   - 支持更多视频格式（WebM, AV1）
   - 添加字幕支持
   - 添加播放列表功能

3. **用户体验**
   - 添加缩略图预览
   - 显示加载进度
   - 优化错误提示

## 相关文档

- [VIDEO-FORMAT-GUIDE.md](./VIDEO-FORMAT-GUIDE.md) - 视频格式详细说明
- [START-HERE.md](./START-HERE.md) - 快速开始指南
- [README.md](./README.md) - 项目总览
