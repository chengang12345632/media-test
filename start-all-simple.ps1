# Start All Services Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting All Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if compiled
Write-Host "Checking compilation status..." -ForegroundColor Yellow

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

# Start platform server
Write-Host "[1/3] Starting platform server..." -ForegroundColor Yellow
$platformJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:RUST_LOG = "info"
    & ".\target\debug\platform-server.exe"
}
Write-Host "Platform server started (Job ID: $($platformJob.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start device simulator
Write-Host "[2/3] Starting device simulator..." -ForegroundColor Yellow
$deviceJob = Start-Job -ScriptBlock {
    Set-Location "$using:PWD\device-simulator"
    $env:RUST_LOG = "info"
    & "..\target\debug\device-simulator.exe" --device-id device_001 --server-addr 127.0.0.1:8443
}
Write-Host "Device simulator started (Job ID: $($deviceJob.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start frontend
Write-Host "[3/3] Starting frontend..." -ForegroundColor Yellow
$frontendJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD\web-frontend
    npm run dev
}
Write-Host "Frontend started (Job ID: $($frontendJob.Id))" -ForegroundColor Green

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
Write-Host "Job IDs:" -ForegroundColor White
Write-Host "  Platform: $($platformJob.Id)" -ForegroundColor Gray
Write-Host "  Device:   $($deviceJob.Id)" -ForegroundColor Gray
Write-Host "  Frontend: $($frontendJob.Id)" -ForegroundColor Gray
Write-Host ""
Write-Host "View logs:" -ForegroundColor Yellow
Write-Host "  Receive-Job -Id $($platformJob.Id) -Keep" -ForegroundColor White
Write-Host "  Receive-Job -Id $($deviceJob.Id) -Keep" -ForegroundColor White
Write-Host "  Receive-Job -Id $($frontendJob.Id) -Keep" -ForegroundColor White
Write-Host ""
Write-Host "Stop all services:" -ForegroundColor Yellow
Write-Host "  Stop-Job -Id $($platformJob.Id),$($deviceJob.Id),$($frontendJob.Id)" -ForegroundColor White
Write-Host "  Remove-Job -Id $($platformJob.Id),$($deviceJob.Id),$($frontendJob.Id)" -ForegroundColor White
Write-Host ""
Write-Host "Wait 10-20 seconds for services to start, then:" -ForegroundColor Cyan
Write-Host "  1. Visit http://localhost:5173 for manual testing" -ForegroundColor White
Write-Host "  2. Run automated test with .\test-live-streaming.ps1" -ForegroundColor White
Write-Host ""

# Save Job IDs to file
@{
    Platform = $platformJob.Id
    Device = $deviceJob.Id
    Frontend = $frontendJob.Id
} | ConvertTo-Json | Out-File -FilePath ".job-ids.json" -Encoding UTF8

Write-Host "Job IDs saved to .job-ids.json" -ForegroundColor Gray
Write-Host ""
