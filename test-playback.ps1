# Test Playback Script - Start services and test playback

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Test Playback - Start and Verify" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if services are already running
Write-Host "[1/4] Checking existing services..." -ForegroundColor Yellow
if (Test-Path ".process-ids.json") {
    Write-Host "Services may already be running" -ForegroundColor Yellow
    Write-Host "Stopping existing services..." -ForegroundColor Yellow
    $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
    try {
        Stop-Process -Id $processIds.Platform -ErrorAction SilentlyContinue
        Stop-Process -Id $processIds.Device -ErrorAction SilentlyContinue
        Stop-Process -Id $processIds.Frontend -ErrorAction SilentlyContinue
    } catch {
        # Ignore errors
    }
    Start-Sleep -Seconds 2
}

# Start platform server
Write-Host "[2/4] Starting platform server..." -ForegroundColor Yellow
$platformProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\debug\platform-server.exe" -PassThru
Write-Host "Platform server started (PID: $($platformProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start device simulator
Write-Host "[3/4] Starting device simulator..." -ForegroundColor Yellow
$deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\device-simulator'; `$env:RUST_LOG='info'; ..\target\debug\device-simulator.exe --device-id device_001 --server-addr 127.0.0.1:8443" -PassThru
Write-Host "Device simulator started (PID: $($deviceProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start frontend
Write-Host "[4/4] Starting frontend..." -ForegroundColor Yellow
$frontendProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\web-frontend'; npm run dev" -PassThru
Write-Host "Frontend started (PID: $($frontendProcess.Id))" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All services started!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Save process IDs
@{
    Platform = $platformProcess.Id
    Device = $deviceProcess.Id
    Frontend = $frontendProcess.Id
} | ConvertTo-Json | Out-File -FilePath ".process-ids.json" -Encoding UTF8

Write-Host "Waiting for services to initialize (20 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Test Instructions" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Open browser: http://localhost:5173" -ForegroundColor White
Write-Host ""
Write-Host "2. Test Live Streaming (H.264):" -ForegroundColor Yellow
Write-Host "   - Click on device 'device_001'" -ForegroundColor White
Write-Host "   - Click 'Live Streaming' button" -ForegroundColor White
Write-Host "   - Should see real-time H.264 video" -ForegroundColor White
Write-Host ""
Write-Host "3. Test MP4 Playback:" -ForegroundColor Yellow
Write-Host "   - Go back to device list" -ForegroundColor White
Write-Host "   - Click 'View Recordings'" -ForegroundColor White
Write-Host "   - Select any MP4 file (e.g., oceans.mp4)" -ForegroundColor White
Write-Host "   - Click 'Play'" -ForegroundColor White
Write-Host "   - Should play directly with progress bar" -ForegroundColor White
Write-Host ""
Write-Host "4. Test H.264 Playback:" -ForegroundColor Yellow
Write-Host "   - Select H.264 file (e.g., sample_720p_60fps.h264)" -ForegroundColor White
Write-Host "   - Click 'Play'" -ForegroundColor White
Write-Host "   - Should stream via SSE and play" -ForegroundColor White
Write-Host ""
Write-Host "Available test files:" -ForegroundColor Cyan
Get-ChildItem "device-simulator\test-videos" | ForEach-Object {
    $size = "{0:N2} MB" -f ($_.Length / 1MB)
    Write-Host "  - $($_.Name) ($size)" -ForegroundColor Gray
}
Write-Host ""
Write-Host "To stop all services:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Id $($platformProcess.Id),$($deviceProcess.Id),$($frontendProcess.Id)" -ForegroundColor White
Write-Host ""
Write-Host "Press Ctrl+C to exit this script (services will continue running)" -ForegroundColor Gray
Write-Host ""

# Keep script running
Write-Host "Monitoring services... (Press Ctrl+C to exit)" -ForegroundColor Cyan
try {
    while ($true) {
        Start-Sleep -Seconds 10
        # Check if processes are still running
        $platformRunning = Get-Process -Id $platformProcess.Id -ErrorAction SilentlyContinue
        $deviceRunning = Get-Process -Id $deviceProcess.Id -ErrorAction SilentlyContinue
        $frontendRunning = Get-Process -Id $frontendProcess.Id -ErrorAction SilentlyContinue
        
        if (-not $platformRunning) {
            Write-Host "WARNING: Platform server stopped!" -ForegroundColor Red
        }
        if (-not $deviceRunning) {
            Write-Host "WARNING: Device simulator stopped!" -ForegroundColor Red
        }
        if (-not $frontendRunning) {
            Write-Host "WARNING: Frontend stopped!" -ForegroundColor Red
        }
    }
} catch {
    Write-Host ""
    Write-Host "Script terminated. Services are still running." -ForegroundColor Yellow
}
