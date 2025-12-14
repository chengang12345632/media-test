# Debug Mode Startup Script
param([switch]$SkipBuild)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting All Services (Debug Mode)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Stop existing processes
Write-Host "[1/4] Checking and stopping existing processes..." -ForegroundColor Yellow
$stopped = $false

if (Test-Path ".process-ids.json") {
    try {
        $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
        if ($processIds.Platform) {
            $proc = Get-Process -Id $processIds.Platform -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Platform Server (PID: $($processIds.Platform))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Platform -Force -ErrorAction SilentlyContinue
                $stopped = $true
            }
        }
        if ($processIds.Device) {
            $proc = Get-Process -Id $processIds.Device -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Device Simulator (PID: $($processIds.Device))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Device -Force -ErrorAction SilentlyContinue
                $stopped = $true
            }
        }
        if ($processIds.Frontend) {
            $proc = Get-Process -Id $processIds.Frontend -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Frontend (PID: $($processIds.Frontend))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Frontend -Force -ErrorAction SilentlyContinue
                $stopped = $true
            }
        }
    } catch {
        Write-Host "  Warning: Failed to read process ID file" -ForegroundColor Yellow
    }
}

$processes = @("platform-server", "device-simulator", "node")
foreach ($procName in $processes) {
    $procs = Get-Process -Name $procName -ErrorAction SilentlyContinue
    if ($procs) {
        foreach ($proc in $procs) {
            Write-Host "  Stopping $procName (PID: $($proc.Id))..." -ForegroundColor Gray
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            $stopped = $true
        }
    }
}

if ($stopped) {
    Write-Host "  Waiting for processes to exit..." -ForegroundColor Gray
    Start-Sleep -Seconds 3
    Write-Host "  Existing processes stopped" -ForegroundColor Green
} else {
    Write-Host "  No running processes found" -ForegroundColor Green
}
Write-Host ""

# Build projects
if (-not $SkipBuild) {
    Write-Host "[2/4] Building projects (Debug mode)..." -ForegroundColor Yellow
    
    Write-Host "  Building platform-server..." -ForegroundColor Gray
    Push-Location platform-server
    cargo build 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  X platform-server build failed" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Write-Host "  + platform-server build successful" -ForegroundColor Green
    Pop-Location
    
    Write-Host "  Building device-simulator..." -ForegroundColor Gray
    Push-Location device-simulator
    cargo build 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  X device-simulator build failed" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Write-Host "  + device-simulator build successful" -ForegroundColor Green
    Pop-Location
    
    Write-Host "  Checking frontend dependencies..." -ForegroundColor Gray
    Push-Location web-frontend
    if (-not (Test-Path "node_modules")) {
        Write-Host "  Installing frontend dependencies..." -ForegroundColor Gray
        npm install 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Host "  X frontend dependencies installation failed" -ForegroundColor Red
            Pop-Location
            exit 1
        }
    }
    Write-Host "  + frontend dependencies ready" -ForegroundColor Green
    Pop-Location
    Write-Host ""
} else {
    Write-Host "[2/4] Skipping build step" -ForegroundColor Gray
    Write-Host ""
}

# Check executables
Write-Host "[3/4] Checking executables..." -ForegroundColor Yellow
$platformExe = "target\debug\platform-server.exe"
$deviceExe = "target\debug\device-simulator.exe"

if (-not (Test-Path $platformExe)) {
    Write-Host "  X platform-server.exe not found" -ForegroundColor Red
    Write-Host "  Please run: .\start-debug.ps1" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $deviceExe)) {
    Write-Host "  X device-simulator.exe not found" -ForegroundColor Red
    Write-Host "  Please run: .\start-debug.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host "  + All executables ready" -ForegroundColor Green
Write-Host ""

# Start services
Write-Host "[4/4] Starting services..." -ForegroundColor Yellow

Write-Host "  Starting Platform Server..." -ForegroundColor Gray
$platformProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\debug\platform-server.exe" -PassThru
Write-Host "  + Platform Server started (PID: $($platformProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

Write-Host "  Starting Device Simulator..." -ForegroundColor Gray
$deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\device-simulator'; `$env:RUST_LOG='info'; ..\target\debug\device-simulator.exe --device-id device_001 --server-addr 127.0.0.1:8443" -PassThru
Write-Host "  + Device Simulator started (PID: $($deviceProcess.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

Write-Host "  Starting Frontend..." -ForegroundColor Gray
$frontendProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\web-frontend'; npm run dev" -PassThru
Write-Host "  + Frontend started (PID: $($frontendProcess.Id))" -ForegroundColor Green

@{
    Platform = $platformProcess.Id
    Device = $deviceProcess.Id
    Frontend = $frontendProcess.Id
    Mode = "debug"
    StartTime = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
} | ConvertTo-Json | Out-File -FilePath ".process-ids.json" -Encoding UTF8

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All services started!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Service Information:" -ForegroundColor White
Write-Host "  Platform Server: http://localhost:8080" -ForegroundColor Gray
Write-Host "  Frontend:        http://localhost:5173" -ForegroundColor Gray
Write-Host "  Device ID:       device_001" -ForegroundColor Gray
Write-Host "  Build Mode:      Debug" -ForegroundColor Gray
Write-Host ""
Write-Host "Process IDs:" -ForegroundColor White
Write-Host "  Platform: $($platformProcess.Id)" -ForegroundColor Gray
Write-Host "  Device:   $($deviceProcess.Id)" -ForegroundColor Gray
Write-Host "  Frontend: $($frontendProcess.Id)" -ForegroundColor Gray
Write-Host ""
Write-Host "Stop all services:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Id $($platformProcess.Id),$($deviceProcess.Id),$($frontendProcess.Id)" -ForegroundColor White
Write-Host ""
Write-Host "Wait 10-20 seconds, then visit: http://localhost:5173" -ForegroundColor Cyan
Write-Host ""
