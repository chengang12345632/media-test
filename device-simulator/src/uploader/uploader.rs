use crate::quic::QuicClient;
use crate::video::{VideoFile, VideoFileReader};
use common::{Result, VideoSegment};
use std::time::Instant;
use tracing::{info, warn};

pub struct Uploader {
    client: QuicClient,
    video_files: Vec<VideoFile>,
}

impl Uploader {
    pub fn new(client: QuicClient, video_files: Vec<VideoFile>) -> Self {
        Self {
            client,
            video_files,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let video_files = self.video_files.clone();
        for video_file in &video_files {
            info!("📹 Uploading: {}", video_file.name);
            
            match self.upload_file(video_file).await {
                Ok(stats) => {
                    info!("✓ Upload completed:");
                    info!("  Segments: {}", stats.segment_count);
                    info!("  Duration: {:.2}s", stats.duration.as_secs_f64());
                    info!("  Throughput: {:.2} Mbps", stats.throughput_mbps());
                }
                Err(e) => {
                    warn!("✗ Upload failed: {}", e);
                }
            }
        }

        // 上传完成后，保持连接并定期发送心跳
        info!("✓ All uploads completed, keeping connection alive...");
        info!("  Sending heartbeat every 10 seconds");
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            
            match self.client.send_heartbeat().await {
                Ok(_) => {
                    info!("💓 Heartbeat sent");
                }
                Err(e) => {
                    warn!("✗ Heartbeat failed: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn upload_file(&mut self, video_file: &VideoFile) -> Result<UploadStats> {
        let mut reader = VideoFileReader::new(&video_file.path).await?;
        let start_time = Instant::now();
        let mut segment_count = 0;
        let mut total_bytes = 0u64;
        let mut timestamp = 0.0;

        while let Some(chunk) = reader.read_chunk().await? {
            let segment = VideoSegment::new(chunk.clone(), timestamp, segment_count % 30 == 0);
            
            self.client.send_segment(segment).await?;
            
            segment_count += 1;
            total_bytes += chunk.len() as u64;
            timestamp += 0.033; // ~30fps

            // 模拟实时传输速率
            tokio::time::sleep(tokio::time::Duration::from_millis(33)).await;
        }

        let duration = start_time.elapsed();

        Ok(UploadStats {
            segment_count,
            total_bytes,
            duration,
        })
    }
}

struct UploadStats {
    segment_count: u64,
    total_bytes: u64,
    duration: std::time::Duration,
}

impl UploadStats {
    fn throughput_mbps(&self) -> f64 {
        let bits = self.total_bytes as f64 * 8.0;
        let seconds = self.duration.as_secs_f64();
        if seconds > 0.0 {
            bits / seconds / 1_000_000.0
        } else {
            0.0
        }
    }
}
