use common::Result;
use std::path::{Path, PathBuf};
use tracing::debug;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct VideoFile {
    pub path: PathBuf,
    pub name: String,
    pub format: VideoFormat,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoFormat {
    H264,
    MP4,
}

pub struct VideoFileReader {
    file: tokio::fs::File,
    format: VideoFormat,
    chunk_size: usize,
}

impl VideoFileReader {
    pub async fn new(path: &Path) -> Result<Self> {
        let file = tokio::fs::File::open(path).await?;
        let format = detect_format(path);
        
        Ok(Self {
            file,
            format,
            chunk_size: 256 * 1024, // 256KB chunks
        })
    }

    pub async fn read_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        use tokio::io::AsyncReadExt;
        
        let mut buffer = vec![0u8; self.chunk_size];
        let n = self.file.read(&mut buffer).await?;
        
        if n == 0 {
            return Ok(None);
        }
        
        buffer.truncate(n);
        Ok(Some(buffer))
    }

    pub fn format(&self) -> &VideoFormat {
        &self.format
    }
}

pub fn scan_video_files(dir: &Path) -> Result<Vec<VideoFile>> {
    let mut files = Vec::new();

    if !dir.exists() {
        debug!("Video directory does not exist: {:?}", dir);
        return Ok(files);
    }

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "h264" || ext_str == "264" || ext_str == "mp4" {
                let metadata = std::fs::metadata(path)?;
                let format = detect_format(path);
                
                files.push(VideoFile {
                    path: path.to_path_buf(),
                    name: path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    format,
                    size: metadata.len(),
                });
            }
        }
    }

    Ok(files)
}

fn detect_format(path: &Path) -> VideoFormat {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "h264" | "264" => VideoFormat::H264,
            "mp4" => VideoFormat::MP4,
            _ => VideoFormat::H264,
        }
    } else {
        VideoFormat::H264
    }
}
