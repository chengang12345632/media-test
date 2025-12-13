# Start Services Script - Opens each service in a new window

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting All Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if compiled
$platformExe = "target\debug\platform-server.exe"
$deviceExe = "target\debug\device-simulator.exe"

if (-not (Test-Path $platformExe)) {
    Write-Host "platform-server not compiled" -ForegroundColor Red
    Write-Host "Please run: powershell -ExecutionPolicy Bypass -File .\quick-test-setup.ps1" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $deviceExe)) {
    Write-Host "device-simulator not compiled" -ForegroundColor Red
    Write-Host "Please run: powershell -ExecutionPolicy Bypass -File .\quick-test-setup.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host "All components compiled" -ForegroundColor Green
Write-Host ""

# Start platform server in new window
Write-Host "[1/3] Starting platform server..." -ForegroundColor Yellow
$platformProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\debug\platform-server.exe" -PassThru
Write-Host "Platform server started (PID: $($platformProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start device simulator in new window
Write-Host "[2/3] Starting device simulator..." -ForegroundColor Yellow
$deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\debug\device-simulator.exe --device-id device_001 --server-addr 127.0.0.1:8443" -PassThru
Write-Host "Device simulator started (PID: $($deviceProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start frontend in new window
Write-Host "[3/3] Starting frontend..." -ForegroundColor Yellow
$frontendProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\web-frontend'; npm run dev" -PassThru
Write-Host "Frontend started (PID: $($frontendProcess.Id))" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All services started!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Service Information:" -ForegroundColor White
Write-Host "  Platform Server: http://localhost:8080" -ForegroundColor Gray
Write-Host "  Frontend:        http://localhost:5173" -ForegroundColor Gray
Write-Host "  Device ID:       device_001" -ForegroundColor Gray
Write-Host ""
Write-Host "Process IDs:" -ForegroundColor White
Write-Host "  Platform: $($platformProcess.Id)" -ForegroundColor Gray
Write-Host "  Device:   $($deviceProcess.Id)" -ForegroundColor Gray
Write-Host "  Frontend: $($frontendProcess.Id)" -ForegroundColor Gray
Write-Host ""
Write-Host "Wait 10-20 seconds for services to start, then:" -ForegroundColor Cyan
Write-Host "  Visit http://localhost:5173" -ForegroundColor White
Write-Host ""
Write-Host "To stop all services, close the PowerShell windows or run:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Id $($platformProcess.Id),$($deviceProcess.Id),$($frontendProcess.Id)" -ForegroundColor White
Write-Host ""

# Save Process IDs to file
@{
    Platform = $platformProcess.Id
    Device = $deviceProcess.Id
    Frontend = $frontendProcess.Id
} | ConvertTo-Json | Out-File -FilePath ".process-ids.json" -Encoding UTF8

Write-Host "Process IDs saved to .process-ids.json" -ForegroundColor Gray
Write-Host ""
