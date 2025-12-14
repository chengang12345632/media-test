use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;

/// 流式传输录像文件（支持 HTTP Range 请求）
pub async fn stream_recording_file(
    Path(file_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    tracing::info!("📹 Stream request for file_id: {}", file_id);
    
    // 从 file_id 中提取文件名（格式: device_001_filename）
    // 分割成最多3部分：device, 001, filename
    let parts: Vec<&str> = file_id.splitn(3, '_').collect();
    if parts.len() < 3 {
        tracing::error!("Invalid file_id format: {}", file_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    let _device_id = format!("{}_{}", parts[0], parts[1]);
    let file_name = parts[2];
    
    tracing::info!("Extracted file_name: {}", file_name);
    
    // 获取设备连接以查询文件路径
    // 简化实现：直接从 test-videos 目录读取
    // 尝试多个可能的路径
    let possible_paths = vec![
        std::path::PathBuf::from("device-simulator/test-videos").join(file_name),
        std::path::PathBuf::from("../device-simulator/test-videos").join(file_name),
        std::path::PathBuf::from("./test-videos").join(file_name),
    ];
    
    let file_path = possible_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            tracing::error!("File not found in any path: {}", file_name);
            tracing::error!("Tried paths: {:?}", possible_paths);
            StatusCode::NOT_FOUND
        })?;
    
    tracing::info!("Found file at: {:?}", file_path);

    // 获取文件元数据
    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let file_size = metadata.len();

    // 检查是否有 Range 请求
    if let Some(range_header) = headers.get(header::RANGE) {
        // 解析 Range 头
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range(range_str, file_size) {
                return serve_range(&file_path, range, file_size).await;
            }
        }
    }

    // 没有 Range 请求，返回完整文件
    serve_full_file(&file_path, file_size).await
}

/// 解析 Range 头（格式: bytes=start-end）
fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range_part = &range_str[6..];
    let parts: Vec<&str> = range_part.split('-').collect();

    if parts.len() != 2 {
        return None;
    }

    let start = parts[0].parse::<u64>().ok()?;
    let end = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1].parse::<u64>().ok()?
    };

    if start > end || end >= file_size {
        return None;
    }

    Some((start, end))
}

/// 获取文件的 Content-Type
fn get_content_type(file_path: &std::path::Path) -> &'static str {
    match file_path.extension().and_then(|s| s.to_str()) {
        Some("mp4") => "video/mp4",
        Some("h264") | Some("264") => "video/h264",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

/// 返回部分内容（206 Partial Content）
async fn serve_range(
    file_path: &std::path::Path,
    range: (u64, u64),
    file_size: u64,
) -> Result<Response, StatusCode> {
    let (start, end) = range;
    let content_length = end - start + 1;

    let mut file = File::open(file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 跳到起始位置
    use tokio::io::AsyncSeekExt;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 创建限制读取长度的 reader
    let limited_reader = file.take(content_length);
    let stream = ReaderStream::new(limited_reader);
    let body = Body::from_stream(stream);

    let content_type = get_content_type(file_path);

    let response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, content_length)
        .header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, file_size),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

/// 返回完整文件
async fn serve_full_file(
    file_path: &std::path::Path,
    file_size: u64,
) -> Result<Response, StatusCode> {
    let file = File::open(file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_type = get_content_type(file_path);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}
