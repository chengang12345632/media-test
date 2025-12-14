# 视频格式使用指南

## 支持的视频格式

### 1. MP4 格式（推荐）✅

**优点：**
- ✅ 可以直接播放，无需转换
- ✅ 支持拖动进度条
- ✅ 支持快进快退
- ✅ 支持倍速播放
- ✅ 浏览器原生支持

**使用方法：**
直接将 `.mp4` 文件放入 `device-simulator/test-videos/` 目录即可。

**当前可用的 MP4 文件：**
- `20250505_093638.mp4`
- `oceans.mp4`
- `sample_60fps_audio.mp4`
- `sample1.mp4`

### 2. H.264 裸流格式（需要特殊处理）⚠️

**要求：**
- ⚠️ 必须是 Annex B 格式
- ⚠️ 必须包含 NAL 起始码（00 00 00 01 或 00 00 01）
- ⚠️ 必须包含 SPS/PPS 参数集
- ⚠️ 需要实时转换为 fMP4 格式

**当前可用的 H.264 文件：**
- `128x128.264`
- `sample_720p_60fps.h264`

**播放流程：**
1. 前端使用 mux.js 将 H.264 转换为 fMP4
2. 使用 MediaSource Extensions (MSE) 播放
3. 支持实时流式播放

## 如何生成测试视频

### 生成 MP4 文件（推荐）

```bash
# 使用 ffmpeg 生成测试 MP4
ffmpeg -f lavfi -i testsrc=duration=30:size=1920x1080:rate=30 \
       -c:v libx264 -preset fast -crf 23 \
       -pix_fmt yuv420p \
       device-simulator/test-videos/test_30s.mp4
```

### 生成 H.264 裸流文件

```bash
# 从 MP4 提取 H.264 裸流（Annex B 格式）
ffmpeg -i input.mp4 -c:v copy -bsf:v h264_mp4toannexb -an output.h264

# 或者直接生成 H.264 裸流
ffmpeg -f lavfi -i testsrc=duration=30:size=1920x1080:rate=30 \
       -c:v libx264 -preset fast -crf 23 \
       -bsf:v h264_mp4toannexb -an \
       device-simulator/test-videos/test_30s.h264
```

**重要参数说明：**
- `-bsf:v h264_mp4toannexb` - 转换为 Annex B 格式（带起始码）
- `-an` - 移除音频（H.264 裸流不包含音频）
- `-pix_fmt yuv420p` - 确保像素格式兼容

## 验证视频格式

### 检查 MP4 文件

```bash
ffprobe -v error -show_format -show_streams input.mp4
```

### 检查 H.264 文件

```bash
# 查看前 32 字节（应该看到 00 00 00 01 或 00 00 01）
xxd -l 32 input.h264

# 或使用 ffprobe
ffprobe -v error -show_packets input.h264
```

## 常见问题

### Q: 为什么 H.264 文件播放失败？

**A:** 可能的原因：
1. 文件不是 Annex B 格式（缺少 NAL 起始码）
2. 文件缺少 SPS/PPS 参数集
3. 文件编码参数不兼容

**解决方案：**
- 使用 MP4 格式代替
- 或使用 ffmpeg 重新编码为标准格式

### Q: 如何转换现有视频？

**A:** 使用 ffmpeg 转换：

```bash
# 转换为 MP4（推荐）
ffmpeg -i input.avi -c:v libx264 -preset fast -crf 23 -pix_fmt yuv420p output.mp4

# 转换为 H.264 裸流
ffmpeg -i input.avi -c:v libx264 -preset fast -crf 23 -bsf:v h264_mp4toannexb -an output.h264
```

### Q: 哪种格式性能更好？

**A:** 
- **MP4** - 播放性能最好，浏览器原生支持，推荐用于回放
- **H.264** - 适合实时流场景，但需要转换，延迟稍高

## 推荐配置

### 开发测试
- 使用 MP4 格式
- 分辨率：720p 或 1080p
- 帧率：30fps
- 码率：2-5 Mbps

### 生产环境
- 根据实际需求选择格式
- 考虑网络带宽和延迟要求
- 实时流使用 H.264，回放使用 MP4

## 快速测试

```powershell
# 1. 生成测试 MP4
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 `
       -c:v libx264 -preset fast -crf 23 -pix_fmt yuv420p `
       device-simulator/test-videos/test_10s.mp4

# 2. 启动服务
powershell -ExecutionPolicy Bypass -File .\start-services.ps1

# 3. 访问 http://localhost:5173 并选择 test_10s.mp4 播放
```

## 总结

| 格式 | 直接播放 | 进度控制 | 实时流 | 推荐场景 |
|------|---------|---------|--------|---------|
| MP4  | ✅ 是   | ✅ 是   | ❌ 否  | 录像回放 |
| H.264| ❌ 否   | ❌ 否   | ✅ 是  | 实时直播 |

**建议：优先使用 MP4 格式进行测试和开发。**
