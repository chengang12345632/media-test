# 简化的启动脚本 - 启动所有服务

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "启动所有服务" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查编译是否完成
Write-Host "检查编译状态..." -ForegroundColor Yellow

$platformExe = "platform-server\target\debug\platform-server.exe"
$deviceExe = "device-simulator\target\debug\device-simulator.exe"

if (-not (Test-Path $platformExe)) {
    Write-Host "✗ platform-server未编译" -ForegroundColor Red
    Write-Host "  请先运行: .\quick-test-setup.ps1" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $deviceExe)) {
    Write-Host "✗ device-simulator未编译" -ForegroundColor Red
    Write-Host "  请先运行: .\quick-test-setup.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host "✓ 所有组件已编译" -ForegroundColor Green
Write-Host ""

# 启动平台服务器
Write-Host "[1/3] 启动平台服务器..." -ForegroundColor Yellow
$platformJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:RUST_LOG = "info"
    & ".\platform-server\target\debug\platform-server.exe"
}
Write-Host "  ✓ 平台服务器已启动 (Job ID: $($platformJob.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# 启动设备模拟器
Write-Host "[2/3] 启动设备模拟器..." -ForegroundColor Yellow
$deviceJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:RUST_LOG = "info"
    & ".\device-simulator\target\debug\device-simulator.exe" --device-id device_001 --server-addr 127.0.0.1:8443
}
Write-Host "  ✓ 设备模拟器已启动 (Job ID: $($deviceJob.Id))" -ForegroundColor Green
Start-Sleep -Seconds 3

# 启动前端
Write-Host "[3/3] 启动前端..." -ForegroundColor Yellow
$frontendJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD\web-frontend
    npm run dev
}
Write-Host "  ✓ 前端已启动 (Job ID: $($frontendJob.Id))" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ 所有服务已启动！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "服务信息：" -ForegroundColor White
Write-Host "  平台服务器: http://localhost:8080" -ForegroundColor Gray
Write-Host "  前端界面:   http://localhost:5173" -ForegroundColor Gray
Write-Host "  设备ID:     device_001" -ForegroundColor Gray
Write-Host ""
Write-Host "Job IDs:" -ForegroundColor White
Write-Host "  平台服务器: $($platformJob.Id)" -ForegroundColor Gray
Write-Host "  设备模拟器: $($deviceJob.Id)" -ForegroundColor Gray
Write-Host "  前端:       $($frontendJob.Id)" -ForegroundColor Gray
Write-Host ""
Write-Host "查看日志：" -ForegroundColor Yellow
Write-Host "  Receive-Job -Id $($platformJob.Id) -Keep" -ForegroundColor White
Write-Host "  Receive-Job -Id $($deviceJob.Id) -Keep" -ForegroundColor White
Write-Host "  Receive-Job -Id $($frontendJob.Id) -Keep" -ForegroundColor White
Write-Host ""
Write-Host "停止所有服务：" -ForegroundColor Yellow
Write-Host "  Stop-Job -Id $($platformJob.Id),$($deviceJob.Id),$($frontendJob.Id)" -ForegroundColor White
Write-Host "  Remove-Job -Id $($platformJob.Id),$($deviceJob.Id),$($frontendJob.Id)" -ForegroundColor White
Write-Host ""
Write-Host "等待服务完全启动（约10-20秒）后，可以：" -ForegroundColor Cyan
Write-Host "  1. 访问 http://localhost:5173 进行手动测试" -ForegroundColor White
Write-Host "  2. 运行 .\test-live-streaming.ps1 进行自动化测试" -ForegroundColor White
Write-Host ""

# 保存Job IDs到文件
@{
    Platform = $platformJob.Id
    Device = $deviceJob.Id
    Frontend = $frontendJob.Id
} | ConvertTo-Json | Out-File -FilePath ".job-ids.json" -Encoding UTF8

Write-Host "提示: Job IDs已保存到 .job-ids.json" -ForegroundColor Gray
Write-Host ""
