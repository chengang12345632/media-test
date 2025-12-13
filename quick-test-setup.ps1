# 快速测试设置脚本
# 使用模拟数据快速验证直通播放功能

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "快速测试设置 - 使用模拟数据" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 步骤1: 修改device-simulator使用模拟数据生成器
Write-Host "[步骤 1/4] 配置模拟数据生成器..." -ForegroundColor Yellow

$modFilePath = "device-simulator\src\video\mod.rs"

if (Test-Path $modFilePath) {
    $content = Get-Content $modFilePath -Raw
    
    # 检查是否已经使用模拟版本
    if ($content -match "pub use live_stream_generator_mock::LiveStreamGenerator;") {
        Write-Host "  ✓ 已配置为使用模拟数据生成器" -ForegroundColor Green
    } else {
        Write-Host "  正在修改 $modFilePath..." -ForegroundColor Gray
        
        # 备份原文件
        Copy-Item $modFilePath "$modFilePath.backup" -Force
        Write-Host "  ✓ 已备份原文件到 $modFilePath.backup" -ForegroundColor Gray
        
        # 修改导出
        $content = $content -replace "pub use live_stream_generator::LiveStreamGenerator;", "// pub use live_stream_generator::LiveStreamGenerator; // 真实屏幕录制版本`npub use live_stream_generator_mock::LiveStreamGenerator; // 模拟数据版本（用于测试）"
        
        Set-Content $modFilePath $content -NoNewline
        Write-Host "  ✓ 已切换到模拟数据生成器" -ForegroundColor Green
    }
} else {
    Write-Host "  ✗ 未找到 $modFilePath" -ForegroundColor Red
    exit 1
}

# 步骤2: 注释FFmpeg依赖
Write-Host "`n[步骤 2/4] 配置Cargo依赖..." -ForegroundColor Yellow

$cargoFilePath = "device-simulator\Cargo.toml"

if (Test-Path $cargoFilePath) {
    $content = Get-Content $cargoFilePath -Raw
    
    # 检查是否已经注释
    if ($content -match "# ffmpeg-next") {
        Write-Host "  ✓ FFmpeg依赖已注释" -ForegroundColor Green
    } else {
        Write-Host "  正在修改 $cargoFilePath..." -ForegroundColor Gray
        
        # 备份原文件
        Copy-Item $cargoFilePath "$cargoFilePath.backup" -Force
        Write-Host "  ✓ 已备份原文件到 $cargoFilePath.backup" -ForegroundColor Gray
        
        # 注释FFmpeg相关依赖
        $content = $content -replace "scrap = `"0.5`"", "# scrap = `"0.5`"  # 暂时注释，使用模拟数据"
        $content = $content -replace "ffmpeg-next = `"6.0`"", "# ffmpeg-next = `"6.0`"  # 暂时注释，使用模拟数据"
        $content = $content -replace "image = `"0.24`"", "# image = `"0.24`"  # 暂时注释，使用模拟数据"
        
        Set-Content $cargoFilePath $content -NoNewline
        Write-Host "  ✓ 已注释FFmpeg依赖" -ForegroundColor Green
    }
} else {
    Write-Host "  ✗ 未找到 $cargoFilePath" -ForegroundColor Red
    exit 1
}

# 步骤3: 清理并重新编译
Write-Host "`n[步骤 3/4] 清理并重新编译..." -ForegroundColor Yellow

Write-Host "  清理device-simulator..." -ForegroundColor Gray
Push-Location device-simulator
cargo clean | Out-Null
Write-Host "  ✓ 清理完成" -ForegroundColor Green

Write-Host "  编译device-simulator（这可能需要几分钟）..." -ForegroundColor Gray
$buildOutput = cargo build 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "  ✓ device-simulator编译成功" -ForegroundColor Green
} else {
    Write-Host "  ✗ device-simulator编译失败" -ForegroundColor Red
    Write-Host "  错误信息:" -ForegroundColor Red
    Write-Host $buildOutput -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

Write-Host "  清理platform-server..." -ForegroundColor Gray
Push-Location platform-server
cargo clean | Out-Null
Write-Host "  ✓ 清理完成" -ForegroundColor Green

Write-Host "  编译platform-server（这可能需要几分钟）..." -ForegroundColor Gray
$buildOutput = cargo build 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "  ✓ platform-server编译成功" -ForegroundColor Green
} else {
    Write-Host "  ✗ platform-server编译失败" -ForegroundColor Red
    Write-Host "  错误信息:" -ForegroundColor Red
    Write-Host $buildOutput -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# 步骤4: 提示下一步
Write-Host "`n[步骤 4/4] 设置完成！" -ForegroundColor Yellow
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ 快速测试环境配置完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "下一步操作：" -ForegroundColor White
Write-Host ""
Write-Host "1. 启动所有服务：" -ForegroundColor Yellow
Write-Host "   .\start-all.ps1" -ForegroundColor White
Write-Host ""
Write-Host "2. 等待所有服务启动（约10-20秒）" -ForegroundColor Yellow
Write-Host ""
Write-Host "3. 运行自动化测试：" -ForegroundColor Yellow
Write-Host "   .\test-live-streaming.ps1" -ForegroundColor White
Write-Host ""
Write-Host "或者手动测试：" -ForegroundColor Yellow
Write-Host "   访问 http://localhost:5173" -ForegroundColor White
Write-Host "   选择设备 device_001" -ForegroundColor White
Write-Host "   点击 '直通播放' 按钮" -ForegroundColor White
Write-Host ""
Write-Host "注意：" -ForegroundColor Cyan
Write-Host "  - 当前使用模拟数据，不会真实录制屏幕" -ForegroundColor Gray
Write-Host "  - 可以验证信令流程和数据传输" -ForegroundColor Gray
Write-Host "  - 延迟监控和统计功能正常工作" -ForegroundColor Gray
Write-Host ""
Write-Host "恢复真实屏幕录制：" -ForegroundColor Cyan
Write-Host "  1. 恢复备份文件：" -ForegroundColor Gray
Write-Host "     Copy-Item device-simulator\src\video\mod.rs.backup device-simulator\src\video\mod.rs -Force" -ForegroundColor White
Write-Host "     Copy-Item device-simulator\Cargo.toml.backup device-simulator\Cargo.toml -Force" -ForegroundColor White
Write-Host "  2. 配置FFmpeg环境（参考 COMPILATION-FIX.md）" -ForegroundColor Gray
Write-Host "  3. 重新编译" -ForegroundColor Gray
Write-Host ""
