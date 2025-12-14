# Rebuild and Restart Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Rebuild and Restart Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Stop existing processes
Write-Host "[1/3] Stopping existing processes..." -ForegroundColor Yellow
if (Test-Path ".process-ids.json") {
    $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
    try {
        Stop-Process -Id $processIds.Platform -ErrorAction SilentlyContinue
        Stop-Process -Id $processIds.Device -ErrorAction SilentlyContinue
        Stop-Process -Id $processIds.Frontend -ErrorAction SilentlyContinue
        Write-Host "Stopped existing processes" -ForegroundColor Green
    } catch {
        Write-Host "Some processes may have already stopped" -ForegroundColor Yellow
    }
    Start-Sleep -Seconds 2
}

# Rebuild platform-server
Write-Host "[2/3] Rebuilding platform-server..." -ForegroundColor Yellow
Push-Location platform-server
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed" -ForegroundColor Red
    Pop-Location
    exit 1
}
Write-Host "Build successful" -ForegroundColor Green
Pop-Location

# Restart services
Write-Host "[3/3] Restarting services..." -ForegroundColor Yellow
powershell -ExecutionPolicy Bypass -File .\start-services.ps1

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Done!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
