// H.264编码器模块
//
// 使用ffmpeg实现低延迟H.264编码

use ffmpeg_next as ffmpeg;
use ffmpeg::codec;
use ffmpeg::format::Pixel;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use tracing::{debug, error, info, warn};

/// H.264编码器
pub struct H264Encoder {
    encoder: ffmpeg::encoder::Video,
    scaler: Context,
    frame_count: i64,
    time_base: ffmpeg::Rational,
}

impl H264Encoder {
    /// 创建H.264编码器
    /// 
    /// # 参数
    /// - width: 视频宽度
    /// - height: 视频高度
    /// - fps: 帧率
    /// - bitrate: 目标码率（bps）
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        bitrate: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🎬 Initializing H.264 encoder");
        info!("  Resolution: {}x{}", width, height);
        info!("  FPS: {}", fps);
        info!("  Bitrate: {} Mbps", bitrate / 1_000_000);
        
        // 初始化ffmpeg
        ffmpeg::init()?;
        
        // 创建编码器
        let codec = ffmpeg::encoder::find(codec::Id::H264)
            .ok_or("H264 codec not found")?;
        
        let mut encoder = codec.video()?;
        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(Pixel::YUV420P);
        encoder.set_bit_rate(bitrate);
        encoder.set_time_base(ffmpeg::Rational::new(1, fps as i32));
        encoder.set_frame_rate(Some(ffmpeg::Rational::new(fps as i32, 1)));
        
        // 低延迟配置
        encoder.set_gop(fps); // GOP = 1秒
        encoder.set_max_b_frames(0); // 禁用B帧

        
        // 设置编码参数（低延迟优化）
        let mut dict = ffmpeg::Dictionary::new();
        dict.set("preset", "ultrafast"); // 最快编码速度
        dict.set("tune", "zerolatency"); // 零延迟调优
        dict.set("profile", "baseline"); // baseline profile
        
        info!("  Preset: ultrafast");
        info!("  Tune: zerolatency");
        info!("  Profile: baseline");
        
        let encoder = encoder.open_with(dict)?;
        
        // 创建缩放器（RGB -> YUV420P）
        let scaler = Context::get(
            Pixel::RGB24,
            width,
            height,
            Pixel::YUV420P,
            width,
            height,
            Flags::BILINEAR,
        )?;
        
        info!("✓ H.264 encoder initialized");
        
        Ok(Self {
            encoder,
            scaler,
            frame_count: 0,
            time_base: ffmpeg::Rational::new(1, fps as i32),
        })
    }
    
    /// 编码一帧
    /// 
    /// # 参数
    /// - rgb_data: RGB24格式的帧数据
    /// 
    /// # 返回
    /// - 编码后的H.264数据包（可能为空，因为编码器可能缓冲）
    pub fn encode_frame(
        &mut self,
        rgb_data: &[u8],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        // 创建RGB帧
        let mut rgb_frame = Video::new(Pixel::RGB24, self.encoder.width(), self.encoder.height());
        rgb_frame.data_mut(0).copy_from_slice(rgb_data);
        
        // 转换为YUV420P
        let mut yuv_frame = Video::new(Pixel::YUV420P, self.encoder.width(), self.encoder.height());
        self.scaler.run(&rgb_frame, &mut yuv_frame)?;
        
        // 设置时间戳
        yuv_frame.set_pts(Some(self.frame_count));
        self.frame_count += 1;
        
        // 编码
        self.encoder.send_frame(&yuv_frame)?;
        
        // 接收编码后的数据包
        let mut packets = Vec::new();
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(_) => {
                    if let Some(data) = packet.data() {
                        packets.push(data.to_vec());
                        debug!("📦 Encoded packet: {} bytes", data.len());
                    }
                }
                Err(ffmpeg::Error::Other { errno: 11 }) => break, // EAGAIN
                Err(e) => {
                    warn!("Encoding error: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
        
        Ok(packets)
    }
    
    /// 刷新编码器（获取缓冲的帧）
    pub fn flush(&mut self) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        self.encoder.send_eof()?;
        
        let mut packets = Vec::new();
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(_) => {
                    if let Some(data) = packet.data() {
                        packets.push(data.to_vec());
                    }
                }
                Err(_) => break,
            }
        }
        
        Ok(packets)
    }
}
