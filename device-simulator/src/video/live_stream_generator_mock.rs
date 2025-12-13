// 实时流生成器模块（模拟版本）
//
// 用于测试的模拟实现，不需要FFmpeg依赖
// 生成模拟的H.264数据用于验证信令流程和数据传输

use common::VideoSegment;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use tracing::{debug, info, warn};

/// 实时流生成器（模拟版本）
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
        info!("🎥 Creating live stream generator (MOCK MODE)");
        info!("  Session ID: {}", session_id);
        info!("  FPS: {}", fps);
        info!("  Bitrate: {} Mbps", bitrate / 1_000_000);
        info!("  ⚠️  Using mock data (no real screen capture)");
        
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
        
        info!("🚀 Starting live stream with mock data...");
        
        // 启动模拟数据生成任务
        self.spawn_mock_task(tx).await?;
        
        Ok(rx)
    }

    
    async fn spawn_mock_task(
        &mut self,
        tx: mpsc::Sender<VideoSegment>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session_id = self.session_id;
        let fps = self.fps;
        let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
        let bytes_per_frame = (self.bitrate / fps as usize / 8) as usize; // 每帧字节数
        
        tokio::spawn(async move {
            let mut interval_timer = interval(frame_duration);
            let mut frame_count = 0u64;
            let mut timestamp = 0.0f64;
            let frame_duration_secs = frame_duration.as_secs_f64();
            
            info!("✓ Mock stream generator started");
            info!("  Frame size: {} bytes", bytes_per_frame);
            info!("  Frame interval: {:.2}ms", frame_duration.as_millis());
            
            loop {
                interval_timer.tick().await;
                
                // 生成模拟H.264数据
                let mut mock_data = Vec::new();
                
                // 对于关键帧，添加SPS和PPS
                if frame_count % fps as u64 == 0 {
                    // SPS (Sequence Parameter Set) - 简化版本
                    // 这是一个最小的有效SPS，用于1280x720 baseline profile
                    let sps: Vec<u8> = vec![
                        0x00, 0x00, 0x00, 0x01, // NAL start code
                        0x67, // NAL type 7 (SPS)
                        0x42, 0xC0, 0x1E, // profile_idc, constraints, level_idc
                        0xFF, 0xE1, 0x00, 0x19, // more SPS data
                        0x67, 0x42, 0xC0, 0x1E,
                        0xDA, 0x01, 0x40, 0x16,
                        0xE8, 0x06, 0xD0, 0xA1,
                        0x35, 0x00, 0x00, 0x03,
                        0x00, 0x01, 0x00, 0x00,
                        0x03, 0x00, 0x32, 0x0F,
                        0x16, 0x2D, 0x96,
                    ];
                    mock_data.extend_from_slice(&sps);
                    
                    // PPS (Picture Parameter Set)
                    let pps: Vec<u8> = vec![
                        0x00, 0x00, 0x00, 0x01, // NAL start code
                        0x68, // NAL type 8 (PPS)
                        0xCE, 0x3C, 0x80, // PPS data
                    ];
                    mock_data.extend_from_slice(&pps);
                    
                    // IDR frame
                    mock_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65]); // NAL type 5 (IDR)
                } else {
                    // 非关键帧 (P帧)
                    mock_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41]); // NAL type 1 (P)
                }
                
                // 填充帧数据到目标大小
                let remaining = bytes_per_frame.saturating_sub(mock_data.len());
                for i in 0..remaining {
                    mock_data.push(((frame_count + i as u64) % 256) as u8);
                }
                
                let is_keyframe = frame_count % fps as u64 == 0;
                
                let segment = VideoSegment {
                    stream_type: 0x01, // 视频
                    segment_id: Uuid::new_v4(),
                    session_id,
                    timestamp,
                    duration: frame_duration_secs,
                    frame_count: 1,
                    flags: if is_keyframe { 1 } else { 0 },
                    data_length: mock_data.len() as u32,
                    data: mock_data,
                };
                
                if frame_count % 30 == 0 {
                    debug!(
                        "📤 Sending mock segment #{}: {:.2}s, {} bytes, keyframe: {}",
                        frame_count, timestamp, segment.data.len(), is_keyframe
                    );
                }
                
                if tx.send(segment).await.is_err() {
                    warn!("⚠️ Receiver dropped, stopping mock stream");
                    break;
                }
                
                frame_count += 1;
                timestamp += frame_duration_secs;
            }
            
            info!("✓ Mock stream generator stopped (total frames: {})", frame_count);
        });
        
        Ok(())
    }
    
    /// 停止实时流
    pub fn stop_streaming(&mut self) {
        self.is_running = false;
        info!("⏹️ Stopping mock stream generator");
        // 通道关闭会自动停止任务
    }
}
