# 一键执行完整测试流程
# 包括：配置、编译、启动、测试

param(
    [switch]$SkipSetup,  # 跳过配置和编译
    [switch]$SkipTest    # 只启动服务，不运行测试
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "任务 0.9 - 完整测试执行" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 阶段1: 配置和编译
if (-not $SkipSetup) {
    Write-Host "阶段 1: 配置测试环境" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    & .\quick-test-setup.ps1
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`n❌ 配置失败，请检查错误信息" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "`n✅ 配置完成" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "⏭️  跳过配置阶段" -ForegroundColor Gray
    Write-Host ""
}

# 阶段2: 启动服务
Write-Host "阶段 2: 启动所有服务" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

& .\start-all-simple.ps1

if ($LASTEXITCODE -ne 0) {
    Write-Host "`n❌ 服务启动失败" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ 所有服务已启动" -ForegroundColor Green
Write-Host ""

# 等待服务完全启动
Write-Host "等待服务完全启动..." -ForegroundColor Yellow
for ($i = 15; $i -gt 0; $i--) {
    Write-Host "  $i 秒..." -NoNewline -ForegroundColor Gray
    Start-Sleep -Seconds 1
    Write-Host "`r" -NoNewline
}
Write-Host "  ✓ 服务已就绪" -ForegroundColor Green
Write-Host ""

# 阶段3: 验证服务
Write-Host "阶段 3: 验证服务状态" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray

try {
    Write-Host "  检查平台服务器..." -NoNewline -ForegroundColor Gray
    $health = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/health" -TimeoutSec 5
    if ($health.status -eq "success") {
        Write-Host " ✓" -ForegroundColor Green
    } else {
        Write-Host " ✗" -ForegroundColor Red
        throw "平台服务器响应异常"
    }
} catch {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host "`n❌ 平台服务器未响应: $_" -ForegroundColor Red
    Write-Host "提示: 检查服务是否正常启动" -ForegroundColor Yellow
    exit 1
}

try {
    Write-Host "  检查设备状态..." -NoNewline -ForegroundColor Gray
    $devices = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/devices" -TimeoutSec 5
    $device = $devices.data | Where-Object { $_.device_id -eq "device_001" }
    
    if ($device -and $device.status -eq "online") {
        Write-Host " ✓" -ForegroundColor Green
    } else {
        Write-Host " ✗" -ForegroundColor Red
        throw "设备不在线"
    }
} catch {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host "`n❌ 设备未在线: $_" -ForegroundColor Red
    Write-Host "提示: 检查设备模拟器日志" -ForegroundColor Yellow
    exit 1
}

Write-Host "`n✅ 所有服务验证通过" -ForegroundColor Green
Write-Host ""

# 阶段4: 运行测试
if (-not $SkipTest) {
    Write-Host "阶段 4: 运行自动化测试" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    Write-Host ""
    
    & .\test-live-streaming.ps1
    
    $testExitCode = $LASTEXITCODE
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    
    if ($testExitCode -eq 0) {
        Write-Host "✅ 测试完成 - 全部通过！" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "任务 0.9 已完成！" -ForegroundColor Green
        Write-Host ""
        Write-Host "下一步：" -ForegroundColor Yellow
        Write-Host "  1. 查看测试报告: Get-ChildItem test-results-*.json | Sort-Object LastWriteTime -Descending | Select-Object -First 1" -ForegroundColor White
        Write-Host "  2. 更新任务状态: 编辑 .kiro/specs/unified-low-latency-streaming/tasks.md" -ForegroundColor White
        Write-Host "  3. 停止服务: 参考 TEST-EXECUTION-GUIDE.md" -ForegroundColor White
    } else {
        Write-Host "⚠️  测试完成 - 部分失败" -ForegroundColor Yellow
        Write-Host "========================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "请检查失败的测试项并修复" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "故障排查：" -ForegroundColor Yellow
        Write-Host "  1. 查看测试报告: Get-ChildItem test-results-*.json | Sort-Object LastWriteTime -Descending | Select-Object -First 1" -ForegroundColor White
        Write-Host "  2. 查看服务日志: 参考 TEST-EXECUTION-GUIDE.md" -ForegroundColor White
        Write-Host "  3. 参考故障排查指南: LIVE-STREAMING-TEST-GUIDE.md" -ForegroundColor White
    }
    
    Write-Host ""
    exit $testExitCode
} else {
    Write-Host "⏭️  跳过测试阶段" -ForegroundColor Gray
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "✅ 服务已启动并验证" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "手动测试：" -ForegroundColor Yellow
    Write-Host "  访问 http://localhost:5173" -ForegroundColor White
    Write-Host "  选择设备 device_001" -ForegroundColor White
    Write-Host "  点击 '直通播放' 按钮" -ForegroundColor White
    Write-Host ""
    Write-Host "运行自动化测试：" -ForegroundColor Yellow
    Write-Host "  .\test-live-streaming.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "停止服务：" -ForegroundColor Yellow
    Write-Host "  参考 TEST-EXECUTION-GUIDE.md" -ForegroundColor White
    Write-Host ""
}
