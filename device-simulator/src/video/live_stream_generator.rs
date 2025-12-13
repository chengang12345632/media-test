// 实时流生成器模块
//
// 整合屏幕捕获和H.264编码，生成实时视频流

use super::screen_capture::ScreenCapturer;
use super::h264_encoder::H264Encoder;
use common::VideoSegment;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use tracing::{debug, error, info, warn};

/// 实时流生成器
pub struct LiveStreamGenerator {
    session_id: Uuid,
    fps: u32,
    bitrate: usize,
    is_running: bool,
}

impl LiveStreamGenerator {
    /// 创建实时流生成器
    pub fn new(
        session_id: Uuid,
        fps: u32,
        bitrate: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🎥 Creating live stream generator");
        info!("  Session ID: {}", session_id);
        info!("  FPS: {}", fps);
        info!("  Bitrate: {} Mbps", bitrate / 1_000_000);
        
        Ok(Self {
            session_id,
            fps,
            bitrate,
            is_running: false,
        })
    }
    
    /// 启动实时流
    /// 
    /// # 返回
    /// - 视频分片接收器
    pub async fn start_streaming(
        &mut self,
    ) -> Result<mpsc::Receiver<VideoSegment>, Box<dyn std::error::Error>> {
        if self.is_running {
            return Err("Stream already running".into());
        }
        
        self.is_running = true;
        let (tx, rx) = mpsc::channel(100);
        
        info!("🚀 Starting live stream capture and encoding...");
        
        // 启动捕获和编码任务
        self.spawn_capture_task(tx).await?;
        
        Ok(rx)
    }

    
    async fn spawn_capture_task(
        &mut self,
        tx: mpsc::Sender<VideoSegment>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 创建屏幕捕获器
        let mut capturer = ScreenCapturer::new(self.fps)?;
        let width = capturer.width() as u32;
        let height = capturer.height() as u32;
        let frame_interval = capturer.frame_interval();
        
        // 创建H.264编码器
        let mut encoder = H264Encoder::new(width, height, self.fps, self.bitrate)?;
        
        let session_id = self.session_id;
        let fps = self.fps;
        
        tokio::spawn(async move {
            let mut interval_timer = interval(frame_interval);
            let mut frame_count = 0u64;
            let mut timestamp = 0.0f64;
            let frame_duration = frame_interval.as_secs_f64();
            
            info!("✓ Live stream capture loop started");
            
            loop {
                interval_timer.tick().await;
                
                // 捕获屏幕帧
                let rgb_frame = match capturer.capture_frame() {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        // 帧未准备好，跳过
                        continue;
                    }
                    Err(e) => {
                        error!("❌ Screen capture error: {}", e);
                        break;
                    }
                };
                
                // 编码帧
                let packets = match encoder.encode_frame(&rgb_frame) {
                    Ok(packets) => packets,
                    Err(e) => {
                        error!("❌ Encoding error: {}", e);
                        continue;
                    }
                };
                
                // 发送编码后的数据包
                for packet in packets {
                    let is_keyframe = frame_count % fps as u64 == 0; // 每秒一个关键帧
                    
                    let segment = VideoSegment {
                        segment_id: Uuid::new_v4(),
                        session_id,
                        timestamp,
                        duration: frame_duration,
                        data: packet,
                        flags: if is_keyframe { 1 } else { 0 },
                    };
                    
                    if frame_count % 30 == 0 {
                        debug!(
                            "📤 Sending segment #{}: {:.2}s, {} bytes, keyframe: {}",
                            frame_count, timestamp, segment.data.len(), is_keyframe
                        );
                    }
                    
                    if tx.send(segment).await.is_err() {
                        warn!("⚠️ Receiver dropped, stopping stream");
                        break;
                    }
                }
                
                frame_count += 1;
                timestamp += frame_duration;
            }
            
            info!("✓ Live stream generator stopped (total frames: {})", frame_count);
        });
        
        Ok(())
    }
    
    /// 停止实时流
    pub fn stop_streaming(&mut self) {
        self.is_running = false;
        info!("⏹️ Stopping live stream generator");
        // 通道关闭会自动停止任务
    }
}
