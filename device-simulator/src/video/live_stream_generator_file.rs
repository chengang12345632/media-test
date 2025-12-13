// 实时流生成器模块（文件版本）
//
// 从真实的H.264文件读取数据并流式传输

use common::VideoSegment;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use tracing::{debug, info, warn};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

/// 实时流生成器（文件版本）
pub struct LiveStreamGeneratorFile {
    session_id: Uuid,
    fps: u32,
    bitrate: usize,
    file_path: std::path::PathBuf,
    is_running: bool,
    stop_signal: Option<tokio::sync::watch::Sender<bool>>,
}

impl LiveStreamGeneratorFile {
    /// 创建实时流生成器
    pub fn new(
        session_id: Uuid,
        fps: u32,
        bitrate: usize,
        file_path: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = file_path.as_ref().to_path_buf();
        
        if !file_path.exists() {
            return Err(format!("H.264 file not found: {:?}", file_path).into());
        }
        
        info!("🎥 Creating live stream generator (FILE MODE)");
        info!("  Session ID: {}", session_id);
        info!("  FPS: {}", fps);
        info!("  Bitrate: {} Mbps", bitrate / 1_000_000);
        info!("  File: {:?}", file_path);
        
        Ok(Self {
            session_id,
            fps,
            bitrate,
            file_path,
            is_running: false,
            stop_signal: None,
        })
    }
    
    /// 启动实时流
    pub async fn start_streaming(
        &mut self,
    ) -> Result<mpsc::Receiver<VideoSegment>, Box<dyn std::error::Error>> {
        if self.is_running {
            return Err("Stream already running".into());
        }
        
        self.is_running = true;
        let (tx, rx) = mpsc::channel(100);
        
        // 创建停止信号通道
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        self.stop_signal = Some(stop_tx);
        
        info!("🚀 Starting live stream from file...");
        
        // 启动文件读取任务
        self.spawn_file_task(tx, stop_rx).await?;
        
        Ok(rx)
    }
    
    async fn spawn_file_task(
        &mut self,
        tx: mpsc::Sender<VideoSegment>,
        stop_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session_id = self.session_id;
        let fps = self.fps;
        let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
        let file_path = self.file_path.clone();
        
        tokio::spawn(async move {
            match Self::stream_file(session_id, fps, frame_duration, file_path, tx, stop_rx).await {
                Ok(_) => info!("✓ File streaming completed"),
                Err(e) => warn!("⚠️ File streaming error: {}", e),
            }
        });
        
        Ok(())
    }
    
    async fn stream_file(
        session_id: Uuid,
        fps: u32,
        frame_duration: Duration,
        file_path: std::path::PathBuf,
        tx: mpsc::Sender<VideoSegment>,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(&file_path).await?;
        let mut reader = BufReader::new(file);
        
        let mut interval_timer = interval(frame_duration);
        let mut frame_count = 0u64;
        let mut timestamp = 0.0f64;
        let frame_duration_secs = frame_duration.as_secs_f64();
        
        // 读取整个文件到内存（对于小文件）
        let mut file_data = Vec::new();
        reader.read_to_end(&mut file_data).await?;
        
        info!("✓ Loaded H.264 file: {} bytes", file_data.len());
        
        // 查找NAL单元
        let nal_units = Self::find_nal_units(&file_data);
        info!("✓ Found {} NAL units", nal_units.len());
        
        if nal_units.is_empty() {
            return Err("No NAL units found in file".into());
        }
        
        // 循环发送NAL单元
        // 策略：将连续的NAL单元组合成一个分片，直到遇到下一个关键帧或达到目标大小
        let mut nal_index = 0;
        let target_segment_size = 50000; // 目标分片大小：50KB
        
        loop {
            // 检查停止信号
            if *stop_rx.borrow() {
                info!("⏹️ Stop signal received, ending stream");
                break;
            }
            
            interval_timer.tick().await;
            
            // 收集NAL单元直到达到目标大小或遇到关键帧
            let mut segment_data = Vec::new();
            let mut segment_has_keyframe = false;
            let start_nal_index = nal_index;
            
            // 第一个NAL单元
            let first_nal = &nal_units[nal_index];
            let first_nal_type = Self::get_nal_type(first_nal);
            segment_data.extend_from_slice(first_nal);
            segment_has_keyframe = Self::is_keyframe_nal(first_nal);
            nal_index += 1;
            
            // 如果第一个是SPS，继续添加所有SPS、PPS和第一个IDR
            if first_nal_type == 7 {
                // 添加所有SPS
                while nal_index < nal_units.len() && Self::get_nal_type(&nal_units[nal_index]) == 7 {
                    segment_data.extend_from_slice(&nal_units[nal_index]);
                    nal_index += 1;
                }
                // 添加所有PPS
                while nal_index < nal_units.len() && Self::get_nal_type(&nal_units[nal_index]) == 8 {
                    segment_data.extend_from_slice(&nal_units[nal_index]);
                    segment_has_keyframe = true;
                    nal_index += 1;
                }
                // 添加第一个IDR
                if nal_index < nal_units.len() && Self::get_nal_type(&nal_units[nal_index]) == 5 {
                    segment_data.extend_from_slice(&nal_units[nal_index]);
                    segment_has_keyframe = true;
                    nal_index += 1;
                }
                
                info!("📦 Sending SPS+PPS+IDR segment: {} bytes", segment_data.len());
            } else {
                // 继续添加NAL单元直到达到目标大小
                while segment_data.len() < target_segment_size && nal_index < nal_units.len() {
                    let next_nal = &nal_units[nal_index];
                    let next_nal_type = Self::get_nal_type(next_nal);
                    
                    // 如果遇到SPS，停止（下一个分片从SPS开始）
                    if next_nal_type == 7 {
                        break;
                    }
                    
                    segment_data.extend_from_slice(next_nal);
                    if Self::is_keyframe_nal(next_nal) {
                        segment_has_keyframe = true;
                    }
                    nal_index += 1;
                }
            }
            
            // 每30帧（1秒）重新发送SPS+PPS以支持新加入的客户端
            if frame_count > 0 && frame_count % 30 == 0 {
                info!("🔄 Resending SPS/PPS for new clients at frame {}", frame_count);
                // 重置到开头，下一帧将发送SPS+PPS+IDR
                nal_index = 0;
            }
            
            // 记录前几个分片的信息
            if frame_count < 5 {
                info!("  Segment #{}: size={}, keyframe={}, NALs={}-{}", 
                      frame_count, segment_data.len(), segment_has_keyframe, 
                      start_nal_index, nal_index - 1);
            }
            
            let segment = VideoSegment {
                stream_type: 0x01, // 视频
                segment_id: Uuid::new_v4(),
                session_id,
                timestamp,
                duration: frame_duration_secs,
                frame_count: 1,
                flags: if segment_has_keyframe { 1 } else { 0 },
                data_length: segment_data.len() as u32,
                data: segment_data,
            };
            
            if frame_count % 30 == 0 {
                debug!(
                    "📤 Sending segment #{}: {:.2}s, {} bytes, keyframe: {}",
                    frame_count, timestamp, segment.data.len(), segment_has_keyframe
                );
            }
            
            if tx.send(segment).await.is_err() {
                warn!("⚠️ Receiver dropped, stopping stream");
                break;
            }
            
            frame_count += 1;
            timestamp += frame_duration_secs;
            
            // 循环播放
            if nal_index >= nal_units.len() {
                info!("🔄 Looping file playback");
                nal_index = 0;
            }
        }
        
        Ok(())
    }
    
    /// 查找文件中的所有NAL单元，并重新排序确保SPS/PPS在前
    fn find_nal_units(data: &[u8]) -> Vec<Vec<u8>> {
        let mut nal_units = Vec::new();
        let mut sps_units = Vec::new();
        let mut pps_units = Vec::new();
        let mut idr_units = Vec::new();
        let mut other_units = Vec::new();
        
        let mut i = 0;
        
        while i < data.len() {
            // 查找起始码 (0x00 0x00 0x00 0x01 或 0x00 0x00 0x01)
            if i + 3 < data.len() && data[i] == 0x00 && data[i+1] == 0x00 {
                let start_code_len = if data[i+2] == 0x00 && data[i+3] == 0x01 {
                    4
                } else if data[i+2] == 0x01 {
                    3
                } else {
                    i += 1;
                    continue;
                };
                
                // 找到起始码，查找下一个起始码
                let nal_start = i;
                i += start_code_len;
                
                // 查找下一个NAL单元的起始码
                let mut nal_end = data.len();
                let mut j = i;
                while j < data.len() - 3 {
                    if data[j] == 0x00 && data[j+1] == 0x00 {
                        if (data[j+2] == 0x00 && j + 3 < data.len() && data[j+3] == 0x01) ||
                           data[j+2] == 0x01 {
                            nal_end = j;
                            break;
                        }
                    }
                    j += 1;
                }
                
                // 提取NAL单元（包含起始码）
                if nal_end > nal_start {
                    let nal_data = data[nal_start..nal_end].to_vec();
                    
                    // 根据NAL类型分类
                    let nal_type = Self::get_nal_type(&nal_data);
                    match nal_type {
                        7 => sps_units.push(nal_data), // SPS
                        8 => pps_units.push(nal_data), // PPS
                        5 => idr_units.push(nal_data), // IDR
                        _ => other_units.push(nal_data),
                    }
                }
                
                i = nal_end;
            } else {
                i += 1;
            }
        }
        
        // 重新排序：SPS -> PPS -> IDR -> 其他
        // 这样确保第一批数据包含完整的初始化信息
        info!("  NAL unit classification:");
        info!("    SPS: {}", sps_units.len());
        info!("    PPS: {}", pps_units.len());
        info!("    IDR: {}", idr_units.len());
        info!("    Other: {}", other_units.len());
        
        nal_units.extend(sps_units);
        nal_units.extend(pps_units);
        nal_units.extend(idr_units);
        nal_units.extend(other_units);
        
        nal_units
    }
    
    /// 获取NAL单元类型
    fn get_nal_type(nal_data: &[u8]) -> u8 {
        let start = if nal_data.len() >= 4 && nal_data[0] == 0x00 && nal_data[1] == 0x00 && 
                       nal_data[2] == 0x00 && nal_data[3] == 0x01 {
            4
        } else if nal_data.len() >= 3 && nal_data[0] == 0x00 && nal_data[1] == 0x00 && 
                  nal_data[2] == 0x01 {
            3
        } else {
            0
        };
        
        if start < nal_data.len() {
            nal_data[start] & 0x1F
        } else {
            0
        }
    }
    
    /// 检查NAL单元是否是关键帧
    fn is_keyframe_nal(nal_data: &[u8]) -> bool {
        let nal_type = Self::get_nal_type(nal_data);
        // NAL type 5 = IDR (关键帧)
        // NAL type 7 = SPS
        // NAL type 8 = PPS
        nal_type == 5 || nal_type == 7 || nal_type == 8
    }
    
    /// 停止实时流
    pub fn stop_streaming(&mut self) {
        self.is_running = false;
        if let Some(stop_tx) = &self.stop_signal {
            let _ = stop_tx.send(true);
            info!("⏹️ Stop signal sent to streaming task");
        }
    }
}
