// 屏幕捕获模块
//
// 使用scrap库实现跨平台的屏幕捕获功能

use scrap::{Capturer, Display};
use std::io::ErrorKind;
use std::time::Duration;
use tracing::{debug, error, info};

/// 屏幕捕获器
pub struct ScreenCapturer {
    capturer: Capturer,
    width: usize,
    height: usize,
    frame_interval: Duration,
}

impl ScreenCapturer {
    /// 创建屏幕捕获器
    /// 
    /// # 参数
    /// - fps: 目标帧率（默认30）
    pub fn new(fps: u32) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🎥 Initializing screen capturer ({}fps)", fps);
        
        // 获取主显示器
        let display = Display::primary()?;
        info!("  Display: {}x{}", display.width(), display.height());
        
        let capturer = Capturer::new(display)?;
        
        let width = capturer.width();
        let height = capturer.height();
        let frame_interval = Duration::from_secs_f64(1.0 / fps as f64);
        
        info!("✓ Screen capturer initialized: {}x{} @ {}fps", width, height, fps);
        
        Ok(Self {
            capturer,
            width,
            height,
            frame_interval,
        })
    }
    
    /// 捕获一帧
    /// 
    /// # 返回
    /// - Ok(Some(frame)): 成功捕获帧
    /// - Ok(None): 帧未准备好（需要重试）
    /// - Err: 捕获错误
    pub fn capture_frame(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match self.capturer.frame() {
            Ok(frame) => {
                debug!("📸 Captured frame: {} bytes", frame.len());
                // 转换BGRA到RGB
                let rgb_frame = self.bgra_to_rgb(&frame);
                Ok(Some(rgb_frame))
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // 帧未准备好
                Ok(None)
            }
            Err(e) => {
                error!("Screen capture error: {}", e);
                Err(Box::new(e))
            }
        }
    }

    
    /// 将BGRA格式转换为RGB格式
    /// 
    /// scrap库返回的是BGRA格式，但ffmpeg需要RGB格式
    fn bgra_to_rgb(&self, bgra: &[u8]) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.width * self.height * 3);
        
        for chunk in bgra.chunks(4) {
            rgb.push(chunk[2]); // R
            rgb.push(chunk[1]); // G
            rgb.push(chunk[0]); // B
        }
        
        rgb
    }
    
    /// 获取视频宽度
    pub fn width(&self) -> usize {
        self.width
    }
    
    /// 获取视频高度
    pub fn height(&self) -> usize {
        self.height
    }
    
    /// 获取帧间隔
    pub fn frame_interval(&self) -> Duration {
        self.frame_interval
    }
}
