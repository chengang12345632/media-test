# Release 模式启动脚本
# 启动所有服务（Release 构建）

param(
    [switch]$SkipBuild
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "启动所有服务 (Release 模式)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

function Stop-ExistingProcesses {
    Write-Host "[1/4] 检查并停止现有进程..." -ForegroundColor Yellow
    $stopped = $false
    if (Test-Path ".process-ids.json") {
        try {
            $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
            if ($processIds.Platform) {
                $proc = Get-Process -Id $processIds.Platform -ErrorAction SilentlyContinue
                if ($proc) {
                    Write-Host "  停止 Platform Server (PID: $($processIds.Platform))..." -ForegroundColor Gray
                    Stop-Process -Id $processIds.Platform -Force -ErrorAction SilentlyContinue
                    $stopped = $true
                }
            }
            if ($processIds.Device) {
                $proc = Get-Process -Id $processIds.Device -ErrorAction SilentlyContinue
                if ($proc) {
                    Write-Host "  停止 Device Simulator (PID: $($processIds.Device))..." -ForegroundColor Gray
                    Stop-Process -Id $processIds.Device -Force -ErrorAction SilentlyContinue
                    $stopped = $true
                }
            }
            if ($processIds.Frontend) {
                $proc = Get-Process -Id $processIds.Frontend -ErrorAction SilentlyContinue
                if ($proc) {
                    Write-Host "  停止 Frontend (PID: $($processIds.Frontend))..." -ForegroundColor Gray
                    Stop-Process -Id $processIds.Frontend -Force -ErrorAction SilentlyContinue
                    $stopped = $true
                }
            }
        } catch {
            Write-Host "  警告: 读取进程ID文件失败" -ForegroundColor Yellow
        }
    }
    $processes = @("platform-server", "device-simulator", "node")
    foreach ($procName in $processes) {
        $procs = Get-Process -Name $procName -ErrorAction SilentlyContinue
        if ($procs) {
            foreach ($proc in $procs) {
                Write-Host "  停止 $procName (PID: $($proc.Id))..." -ForegroundColor Gray
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                $stopped = $true
            }
        }
    }
    if ($stopped) {
        Write-Host "  等待进程完全退出..." -ForegroundColor Gray
        Start-Sleep -Seconds 3
        Write-Host "  已停止现有进程" -ForegroundColor Green
    } else {
        Write-Host "  没有发现运行中的进程" -ForegroundColor Green
    }
    Write-Host ""
}

function Build-Projects {
    if ($SkipBuild) {
        Write-Host "[2/4] 跳过编译步骤" -ForegroundColor Gray
        Write-Host ""
        return
    }
    Write-Host "[2/4] 编译项目 (Release 模式)..." -ForegroundColor Yellow
    Write-Host "  注意: Release 编译需要较长时间，请耐心等待..." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  编译 platform-server (Release)..." -ForegroundColor Gray
    Push-Location platform-server
    cargo build --release 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "   platform-server 编译失败" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Write-Host "   platform-server 编译成功" -ForegroundColor Green
    Pop-Location
    Write-Host "  编译 device-simulator (Release)..." -ForegroundColor Gray
    Push-Location device-simulator
    cargo build --release 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "   device-simulator 编译失败" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Write-Host "   device-simulator 编译成功" -ForegroundColor Green
    Pop-Location
    Write-Host "  检查前端依赖..." -ForegroundColor Gray
    Push-Location web-frontend
    if (-not (Test-Path "node_modules")) {
        Write-Host "  安装前端依赖..." -ForegroundColor Gray
        npm install 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   前端依赖安装失败" -ForegroundColor Red
            Pop-Location
            exit 1
        }
    }
    Write-Host "   前端依赖就绪" -ForegroundColor Green
    Pop-Location
    Write-Host ""
}

function Test-Executables {
    Write-Host "[3/4] 检查可执行文件..." -ForegroundColor Yellow
    $platformExe = "target\release\platform-server.exe"
    $deviceExe = "target\release\device-simulator.exe"
    if (-not (Test-Path $platformExe)) {
        Write-Host "   未找到 platform-server.exe (Release)" -ForegroundColor Red
        Write-Host "  请先运行编译: .\start-release.ps1" -ForegroundColor Yellow
        exit 1
    }
    if (-not (Test-Path $deviceExe)) {
        Write-Host "   未找到 device-simulator.exe (Release)" -ForegroundColor Red
        Write-Host "  请先运行编译: .\start-release.ps1" -ForegroundColor Yellow
        exit 1
    }
    Write-Host "   所有可执行文件就绪" -ForegroundColor Green
    Write-Host ""
}

function Start-Services {
    Write-Host "[4/4] 启动服务..." -ForegroundColor Yellow
    Write-Host "  启动 Platform Server..." -ForegroundColor Gray
    $platformProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; `$env:RUST_LOG='info'; .\target\release\platform-server.exe" -PassThru
    Write-Host "   Platform Server 已启动 (PID: $($platformProcess.Id))" -ForegroundColor Green
    Start-Sleep -Seconds 3
    Write-Host "  启动 Device Simulator..." -ForegroundColor Gray
    $deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\device-simulator'; `$env:RUST_LOG='info'; ..\target\release\device-simulator.exe --device-id device_001 --server-addr 127.0.0.1:8443" -PassThru
    Write-Host "   Device Simulator 已启动 (PID: $($deviceProcess.Id))" -ForegroundColor Green
    Start-Sleep -Seconds 3
    Write-Host "  启动 Frontend..." -ForegroundColor Gray
    $frontendProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\web-frontend'; npm run dev" -PassThru
    Write-Host "   Frontend 已启动 (PID: $($frontendProcess.Id))" -ForegroundColor Green
    @{
        Platform = $platformProcess.Id
        Device = $deviceProcess.Id
        Frontend = $frontendProcess.Id
        Mode = "release"
        StartTime = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    } | ConvertTo-Json | Out-File -FilePath ".process-ids.json" -Encoding UTF8
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "所有服务已启动！" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "服务信息:" -ForegroundColor White
    Write-Host "  Platform Server: http://localhost:8080" -ForegroundColor Gray
    Write-Host "  Frontend:        http://localhost:5173" -ForegroundColor Gray
    Write-Host "  Device ID:       device_001" -ForegroundColor Gray
    Write-Host "  构建模式:        Release (优化性能)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "进程 ID:" -ForegroundColor White
    Write-Host "  Platform: $($platformProcess.Id)" -ForegroundColor Gray
    Write-Host "  Device:   $($deviceProcess.Id)" -ForegroundColor Gray
    Write-Host "  Frontend: $($frontendProcess.Id)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "停止所有服务:" -ForegroundColor Yellow
    Write-Host "  Stop-Process -Id $($platformProcess.Id),$($deviceProcess.Id),$($frontendProcess.Id)" -ForegroundColor White
    Write-Host ""
    Write-Host "等待 10-20 秒后访问: http://localhost:5173" -ForegroundColor Cyan
    Write-Host ""
}

Stop-ExistingProcesses
Build-Projects
Test-Executables
Start-Services
