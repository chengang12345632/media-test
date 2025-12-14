# H.264 回放问题分析 V2

## 问题现状

### 直通播放 ✅
- 使用 `LiveStreamGeneratorFile`
- 按 NAL unit 分割
- **能正常播放**

### 回放播放 ❌
- 现在也使用 `LiveStreamGeneratorFile`
- 按 NAL unit 分割
- **仍然无法播放**

## 根本原因

mux.js 需要**完整的 GOP (Group of Pictures)** 才能开始转换：
- SPS (Sequence Parameter Set)
- PPS (Picture Parameter Set)  
- IDR (Instantaneous Decoder Refresh) 帧
- 后续的 P 帧和 B 帧

当前问题：
1. 第一个分片只有 SPS (23KB)
2. 第二个分片只有 IDR (54KB)
3. **缺少 PPS！**

## 解决方案

### 方案 1：修改前端累积逻辑（推荐）
在前端累积更多分片后再发送给 mux.js：
- 等待收到 SPS + PPS + IDR
- 或者累积前 10-20 个分片
- 然后一次性发送给 transmuxer

### 方案 2：修改后端发送逻辑
在后端累积完整的 GOP 后再发送：
- 检测 NAL unit 类型
- 累积 SPS + PPS + IDR + 若干帧
- 作为一个大分片发送

### 方案 3：使用 MP4 格式（最简单）
- 将 H.264 文件转换为 MP4
- 使用直接流式传输
- 无需 mux.js 转换

## 推荐实现

使用方案 1，修改前端 H264Player.tsx：
- 累积前 30 个分片（约 1 秒的视频）
- 然后一次性发送给 transmuxer
- 后续每收到 10 个分片就处理一次
