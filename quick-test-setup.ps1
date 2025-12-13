# Quick Setup Script

Write-Host "Compiling all components..." -ForegroundColor Cyan
Write-Host ""

# Compile platform-server
Write-Host "[1/3] Compiling platform-server..." -ForegroundColor Yellow
Push-Location platform-server
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to compile platform-server" -ForegroundColor Red
    Pop-Location
    exit 1
}
Write-Host "Success" -ForegroundColor Green
Pop-Location

# Compile device-simulator
Write-Host "[2/3] Compiling device-simulator..." -ForegroundColor Yellow
Push-Location device-simulator
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to compile device-simulator" -ForegroundColor Red
    Pop-Location
    exit 1
}
Write-Host "Success" -ForegroundColor Green
Pop-Location

# Install frontend dependencies
Write-Host "[3/3] Installing frontend dependencies..." -ForegroundColor Yellow
Push-Location web-frontend
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to install dependencies" -ForegroundColor Red
        Pop-Location
        exit 1
    }
}
Write-Host "Success" -ForegroundColor Green
Pop-Location

Write-Host ""
Write-Host "All components compiled successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Next step:" -ForegroundColor Yellow
Write-Host "  powershell -ExecutionPolicy Bypass -File .\start-all-simple.ps1" -ForegroundColor White
Write-Host ""
