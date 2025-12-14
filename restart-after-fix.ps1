# Restart After Fix Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Restart After H.264 Playback Fix" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Stop existing processes
Write-Host "[1/4] Stopping existing processes..." -ForegroundColor Yellow
if (Test-Path ".process-ids.json") {
    $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
    try {
        Write-Host "Stopping Platform (PID: $($processIds.Platform))..." -ForegroundColor Gray
        Stop-Process -Id $processIds.Platform -Force -ErrorAction SilentlyContinue
        Write-Host "Stopping Device (PID: $($processIds.Device))..." -ForegroundColor Gray
        Stop-Process -Id $processIds.Device -Force -ErrorAction SilentlyContinue
        Write-Host "Stopping Frontend (PID: $($processIds.Frontend))..." -ForegroundColor Gray
        Stop-Process -Id $processIds.Frontend -Force -ErrorAction SilentlyContinue
        Write-Host "Processes stopped" -ForegroundColor Green
    } catch {
        Write-Host "Some processes may have already stopped" -ForegroundColor Yellow
    }
    Start-Sleep -Seconds 3
} else {
    Write-Host "No running processes found" -ForegroundColor Yellow
}

# Rebuild device-simulator
Write-Host "[2/4] Rebuilding device-simulator..." -ForegroundColor Yellow
Push-Location device-simulator
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Write-Host "Build successful" -ForegroundColor Green
Pop-Location

Write-Host "[3/4] Waiting for file locks to release..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

# Restart services
Write-Host "[4/4] Restarting services..." -ForegroundColor Yellow

# Start platform server
$platformProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\debug\platform-server.exe" -PassThru
Write-Host "Platform server started (PID: $($platformProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start device simulator
$deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\device-simulator'; `$env:RUST_LOG='info'; ..\target\debug\device-simulator.exe --device-id device_001 --server-addr 127.0.0.1:8443" -PassThru
Write-Host "Device simulator started (PID: $($deviceProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start frontend
$frontendProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\web-frontend'; npm run dev" -PassThru
Write-Host "Frontend started (PID: $($frontendProcess.Id))" -ForegroundColor Green

# Save process IDs
@{
    Platform = $platformProcess.Id
    Device = $deviceProcess.Id
    Frontend = $frontendProcess.Id
} | ConvertTo-Json | Out-File -FilePath ".process-ids.json" -Encoding UTF8

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Services Restarted!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Waiting for services to initialize (20 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Test H.264 Playback" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Open: http://localhost:5173" -ForegroundColor White
Write-Host "2. Click device 'device_001'" -ForegroundColor White
Write-Host "3. Click 'View Recordings'" -ForegroundColor White
Write-Host "4. Select 'sample_720p_60fps.h264'" -ForegroundColor White
Write-Host "5. Click 'Play'" -ForegroundColor White
Write-Host ""
Write-Host "Expected: H.264 video should play via SSE streaming" -ForegroundColor Yellow
Write-Host ""
Write-Host "Fix Applied:" -ForegroundColor Cyan
Write-Host "  - H.264 playback now uses NAL unit streaming" -ForegroundColor Green
Write-Host "  - Same logic as live streaming" -ForegroundColor Green
Write-Host "  - mux.js should receive valid H.264 data" -ForegroundColor Green
Write-Host ""
