# MSE 播放器实现方案

## 为什么选择 MSE？

基于当前的录像回放需求，**Media Source Extensions (MSE)** 是最佳选择：

### MSE vs WebRTC 对比

| 特性 | MSE | WebRTC |
|------|-----|--------|
| **延迟** | 1-3秒 | < 500ms |
| **复杂度** | ⭐⭐ 中等 | ⭐⭐⭐⭐⭐ 很高 |
| **播放控制** | ✅ 完整支持 | ❌ 有限 |
| **适用场景** | 录像回放、VOD | 实时通话、直播 |
| **浏览器支持** | ✅ 原生支持 | ⚠️ 需要额外库 |
| **实现成本** | 低 | 高 |

**结论**: 对于录像回放场景，MSE 是最合适的选择。

---

## 实现方案

### 方案A: fMP4 流式传输（推荐）⭐⭐⭐⭐⭐

#### 架构流程
```
设备端 (H.264裸流) 
  ↓ 
平台端 (转换为fMP4) 
  ↓ SSE
前端 (MSE播放)
```

#### 优点
- ✅ 标准化格式，浏览器原生支持
- ✅ 可以流式传输，无需等待完整文件
- ✅ 支持精确的播放控制
- ✅ 可以实现自适应码率

#### 实现步骤

##### 1. 后端改造（Rust）

需要在平台服务器添加 H.264 到 fMP4 的转换：

```rust
// 添加依赖到 platform-server/Cargo.toml
mp4 = "0.14"  // MP4 封装库

// 创建 platform-server/src/video/mp4_muxer.rs
pub struct Mp4Muxer {
    // 初始化段（只发送一次）
    init_segment: Vec<u8>,
    // 序列号
    sequence: u32,
}

impl Mp4Muxer {
    pub fn new(width: u32, height: u32, fps: f64) -> Self {
        // 生成 fMP4 初始化段（ftyp + moov）
        let init_segment = Self::create_init_segment(width, height, fps);
        Self {
            init_segment,
            sequence: 0,
        }
    }

    pub fn get_init_segment(&self) -> &[u8] {
        &self.init_segment
    }

    pub fn mux_segment(&mut self, h264_data: &[u8], timestamp: f64, is_keyframe: bool) -> Vec<u8> {
        // 将 H.264 NAL 单元封装为 fMP4 moof + mdat
        self.sequence += 1;
        Self::create_media_segment(h264_data, timestamp, is_keyframe, self.sequence)
    }

    fn create_init_segment(width: u32, height: u32, fps: f64) -> Vec<u8> {
        // 创建 ftyp box
        // 创建 moov box (包含 mvhd, trak, mdia, minf, stbl)
        // 返回完整的初始化段
        todo!()
    }

    fn create_media_segment(data: &[u8], timestamp: f64, is_keyframe: bool, sequence: u32) -> Vec<u8> {
        // 创建 moof box (包含 mfhd, traf)
        // 创建 mdat box (包含实际的 H.264 数据)
        // 返回完整的媒体段
        todo!()
    }
}
```

##### 2. 修改 SSE 处理器

```rust
// platform-server/src/http3/handlers.rs

pub async fn get_playback_segments(
    Path(session_id): Path<String>,
    State((_, _, distribution_manager, _)): State<AppState>,
) -> Result<axum::response::Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let uuid = Uuid::parse_str(&session_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut receiver = distribution_manager.get_receiver(&uuid).ok_or(StatusCode::NOT_FOUND)?;

    // 创建 MP4 封装器
    let mut muxer = Mp4Muxer::new(1280, 720, 60.0);
    let mut init_sent = false;

    let stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(segment) => {
                    // 首次发送初始化段
                    if !init_sent {
                        let init_data = base64::encode(muxer.get_init_segment());
                        yield Ok(Event::default()
                            .event("init")
                            .data(init_data));
                        init_sent = true;
                    }

                    // 转换为 fMP4 并发送
                    let fmp4_data = muxer.mux_segment(
                        &segment.data,
                        segment.timestamp,
                        segment.is_keyframe()
                    );
                    let encoded = base64::encode(&fmp4_data);
                    yield Ok(Event::default()
                        .event("segment")
                        .data(encoded));
                }
                Err(_) => break,
            }
        }
    };

    Ok(Sse::new(stream))
}
```

##### 3. 前端实现（TypeScript + React）

```typescript
// web-frontend/src/components/VideoPlayer.tsx

import React, { useEffect, useRef, useState } from 'react'

interface VideoPlayerProps {
  sessionId: string
}

function VideoPlayer({ sessionId }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const mediaSourceRef = useRef<MediaSource | null>(null)
  const sourceBufferRef = useRef<SourceBuffer | null>(null)
  const queueRef = useRef<Uint8Array[]>([])

  useEffect(() => {
    if (!MediaSource.isTypeSupported('video/mp4; codecs="avc1.64001f"')) {
      console.error('Browser does not support H.264 in MP4')
      return
    }

    const mediaSource = new MediaSource()
    mediaSourceRef.current = mediaSource
    
    if (videoRef.current) {
      videoRef.current.src = URL.createObjectURL(mediaSource)
    }

    mediaSource.addEventListener('sourceopen', () => {
      console.log('MediaSource opened')
      
      // 创建 SourceBuffer
      const sourceBuffer = mediaSource.addSourceBuffer('video/mp4; codecs="avc1.64001f"')
      sourceBufferRef.current = sourceBuffer

      sourceBuffer.addEventListener('updateend', () => {
        // 处理队列中的下一个分片
        if (queueRef.current.length > 0 && !sourceBuffer.updating) {
          const nextSegment = queueRef.current.shift()!
          sourceBuffer.appendBuffer(nextSegment)
        }
      })

      // 开始接收视频流
      startReceiving()
    })

    return () => {
      if (mediaSource.readyState === 'open') {
        mediaSource.endOfStream()
      }
    }
  }, [sessionId])

  const startReceiving = () => {
    const eventSource = new EventSource(`/api/v1/playback/${sessionId}/segments`)

    eventSource.addEventListener('init', (event) => {
      console.log('Received init segment')
      const data = base64ToUint8Array(event.data)
      appendSegment(data)
    })

    eventSource.addEventListener('segment', (event) => {
      console.log('Received media segment')
      const data = base64ToUint8Array(event.data)
      appendSegment(data)
    })

    eventSource.onerror = (err) => {
      console.error('SSE error:', err)
      eventSource.close()
    }
  }

  const appendSegment = (data: Uint8Array) => {
    const sourceBuffer = sourceBufferRef.current
    if (!sourceBuffer) return

    if (sourceBuffer.updating) {
      // 如果正在更新，加入队列
      queueRef.current.push(data)
    } else {
      // 直接添加
      sourceBuffer.appendBuffer(data)
    }
  }

  const base64ToUint8Array = (base64: string): Uint8Array => {
    const binary = atob(base64)
    const bytes = new Uint8Array(binary.length)
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i)
    }
    return bytes
  }

  return (
    <div className="video-player">
      <video ref={videoRef} controls autoPlay />
    </div>
  )
}

export default VideoPlayer
```

---

### 方案B: 使用现有 MP4 文件（快速实现）⭐⭐⭐

如果测试视频已经是 MP4 格式，可以直接使用 HTTP Range 请求：

#### 后端实现
```rust
// 添加文件流式传输端点
pub async fn stream_recording(
    Path(file_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response<Body>, StatusCode> {
    // 解析 Range 请求头
    let range = headers.get("range");
    
    // 打开文件
    let file = tokio::fs::File::open(file_path).await?;
    
    // 根据 Range 返回部分内容
    // 返回 206 Partial Content
}
```

#### 前端实现
```typescript
// 直接使用 video 标签
<video src={`/api/v1/recordings/${fileId}/stream`} controls autoPlay />
```

**优点**: 实现简单，立即可用  
**缺点**: 需要完整的 MP4 文件，不支持实时流

---

### 方案C: 使用第三方库（最快实现）⭐⭐⭐⭐

使用 `hls.js` 或 `dash.js` 等成熟的流媒体库：

#### 使用 hls.js
```bash
npm install hls.js
```

```typescript
import Hls from 'hls.js'

const hls = new Hls()
hls.loadSource(`/api/v1/playback/${sessionId}/playlist.m3u8`)
hls.attachMedia(videoRef.current)
```

**优点**: 成熟稳定，功能完整  
**缺点**: 需要后端支持 HLS 协议

---

## 推荐实施路线

### 阶段1: 快速验证（1-2小时）
使用**方案B**，如果测试视频是 MP4 格式：
1. 添加文件流式传输端点
2. 前端直接使用 `<video>` 标签
3. 验证基本播放功能

### 阶段2: 完整实现（1-2天）
实现**方案A**，支持实时流传输：
1. 集成 MP4 封装库（或使用 FFmpeg）
2. 实现 H.264 到 fMP4 的转换
3. 前端实现 MSE 播放器
4. 添加播放控制功能

### 阶段3: 优化增强（可选）
1. 添加自适应码率
2. 实现缓冲管理
3. 添加播放统计
4. 支持多种编码格式

---

## 技术栈建议

### 后端（Rust）
- `mp4` crate - MP4 封装
- 或 `ffmpeg-next` - 使用 FFmpeg 进行转码
- `base64` - Base64 编码

### 前端（TypeScript）
- 原生 MSE API
- 或 `hls.js` / `dash.js` - 成熟的流媒体库

---

## 常见问题

### Q: 为什么不直接发送 H.264 裸流？
A: 浏览器的 MSE 不支持裸流，必须封装为容器格式（fMP4、WebM 等）。

### Q: fMP4 和普通 MP4 有什么区别？
A: fMP4 (Fragmented MP4) 是流式友好的格式，可以边接收边播放，不需要完整文件。

### Q: 如何处理不同的视频编码？
A: 可以在后端使用 FFmpeg 进行转码，统一转换为 H.264。

### Q: 延迟能做到多低？
A: MSE 通常延迟在 1-3 秒，通过优化缓冲策略可以降到 1 秒以内。

---

## 下一步行动

1. **确认测试视频格式**: 检查 `test-videos` 目录中的文件格式
2. **选择实施方案**: 根据时间和需求选择合适的方案
3. **实现后端转换**: 添加视频格式转换逻辑
4. **完善前端播放器**: 实现 MSE 播放和控制

需要我帮你实现具体的方案吗？
