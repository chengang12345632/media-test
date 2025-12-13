# 生成测试用的H.264文件
# 使用FFmpeg从屏幕录制或生成测试视频

Write-Host "生成测试H.264文件..." -ForegroundColor Green

# 检查FFmpeg是否安装
$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if (-not $ffmpeg) {
    Write-Host "错误: 未找到FFmpeg。请先安装FFmpeg。" -ForegroundColor Red
    Write-Host "可以运行 .\setup-ffmpeg-windows.ps1 安装" -ForegroundColor Yellow
    exit 1
}

$outputDir = "device-simulator\test-videos"
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

Write-Host "生成选项:" -ForegroundColor Cyan
Write-Host "1. 生成彩色测试图案 (10秒)" -ForegroundColor White
Write-Host "2. 录制屏幕 (10秒)" -ForegroundColor White
Write-Host "3. 生成简单动画 (10秒)" -ForegroundColor White

$choice = Read-Host "请选择 (1-3)"

$outputFile = "$outputDir\test-live-$(Get-Date -Format 'yyyyMMdd-HHmmss').h264"

switch ($choice) {
    "1" {
        Write-Host "生成彩色测试图案..." -ForegroundColor Yellow
        ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 `
            -c:v libx264 -preset ultrafast -tune zerolatency `
            -profile:v baseline -level 3.0 `
            -g 30 -keyint_min 30 `
            -b:v 2M -maxrate 2M -bufsize 4M `
            -pix_fmt yuv420p `
            -an `
            -f h264 $outputFile
    }
    "2" {
        Write-Host "录制屏幕 (10秒)..." -ForegroundColor Yellow
        Write-Host "3秒后开始录制..." -ForegroundColor Cyan
        Start-Sleep -Seconds 3
        
        ffmpeg -f gdigrab -framerate 30 -i desktop -t 10 `
            -c:v libx264 -preset ultrafast -tune zerolatency `
            -profile:v baseline -level 3.0 `
            -g 30 -keyint_min 30 `
            -b:v 2M -maxrate 2M -bufsize 4M `
            -pix_fmt yuv420p `
            -an `
            -f h264 $outputFile
    }
    "3" {
        Write-Host "生成简单动画..." -ForegroundColor Yellow
        ffmpeg -f lavfi -i "color=c=blue:s=1280x720:d=10:r=30,drawtext=text='Live Stream Test':fontsize=60:fontcolor=white:x=(w-text_w)/2:y=(h-text_h)/2" `
            -c:v libx264 -preset ultrafast -tune zerolatency `
            -profile:v baseline -level 3.0 `
            -g 30 -keyint_min 30 `
            -b:v 2M -maxrate 2M -bufsize 4M `
            -pix_fmt yuv420p `
            -an `
            -f h264 $outputFile
    }
    default {
        Write-Host "无效选择" -ForegroundColor Red
        exit 1
    }
}

if (Test-Path $outputFile) {
    $fileSize = (Get-Item $outputFile).Length / 1KB
    Write-Host "`n✅ 成功生成H.264文件!" -ForegroundColor Green
    Write-Host "文件: $outputFile" -ForegroundColor Cyan
    Write-Host "大小: $([math]::Round($fileSize, 2)) KB" -ForegroundColor Cyan
    Write-Host "`n现在可以使用这个文件测试直通播放功能" -ForegroundColor Yellow
} else {
    Write-Host "`n❌ 生成失败" -ForegroundColor Red
    exit 1
}
