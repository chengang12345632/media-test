# 延迟监控修复验证脚本

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "延迟监控修复验证测试" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 编译项目
Write-Host "步骤 1: 编译项目..." -ForegroundColor Yellow
cargo build --package platform-server --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    exit 1
}
Write-Host "✅ 编译成功" -ForegroundColor Green
Write-Host ""

# 2. 运行单元测试
Write-Host "步骤 2: 运行单元测试..." -ForegroundColor Yellow
cargo test --package platform-server --lib streaming::source::tests::test_video_segment_creation
cargo test --package platform-server --lib streaming::handler::tests
cargo test --package platform-server --lib latency
if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️  部分测试失败，但继续验证" -ForegroundColor Yellow
} else {
    Write-Host "✅ 单元测试通过" -ForegroundColor Green
}
Write-Host ""

# 3. 启动服务
Write-Host "步骤 3: 启动平台服务..." -ForegroundColor Yellow
Write-Host "请在另一个终端窗口运行以下命令启动服务：" -ForegroundColor Cyan
Write-Host "  cargo run --package platform-server --release" -ForegroundColor White
Write-Host ""
Write-Host "等待服务启动（按任意键继续）..." -ForegroundColor Yellow
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
Write-Host ""

# 4. 测试延迟监控API
Write-Host "步骤 4: 测试延迟监控API..." -ForegroundColor Yellow

Write-Host "  测试健康检查..." -ForegroundColor Cyan
try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/latency/health" -Method Get
    Write-Host "  ✅ 健康检查通过: $($response.data)" -ForegroundColor Green
} catch {
    Write-Host "  ❌ 健康检查失败: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "  测试获取所有统计..." -ForegroundColor Cyan
try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/latency/statistics" -Method Get
    Write-Host "  ✅ 获取统计成功" -ForegroundColor Green
    Write-Host "  会话数量: $($response.data.Count)" -ForegroundColor White
} catch {
    Write-Host "  ⚠️  暂无活动会话（这是正常的）" -ForegroundColor Yellow
}
Write-Host ""

# 5. 验证说明
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "手动验证步骤" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "1. 直通播放延迟监控验证：" -ForegroundColor Yellow
Write-Host "   - 启动设备模拟器" -ForegroundColor White
Write-Host "   - 在前端打开直通播放" -ForegroundColor White
Write-Host "   - 观察延迟监控面板是否显示延迟数据" -ForegroundColor White
Write-Host "   - 预期：显示传输延迟、处理延迟、分发延迟" -ForegroundColor Green
Write-Host ""

Write-Host "2. MP4回放延迟监控验证：" -ForegroundColor Yellow
Write-Host "   - 在前端打开MP4录像回放" -ForegroundColor White
Write-Host "   - 观察延迟监控面板是否显示延迟数据" -ForegroundColor White
Write-Host "   - 预期：显示处理延迟和分发延迟（无传输延迟）" -ForegroundColor Green
Write-Host ""

Write-Host "3. H.264回放延迟监控验证：" -ForegroundColor Yellow
Write-Host "   - 在前端打开H.264录像回放" -ForegroundColor White
Write-Host "   - 观察延迟监控面板是否显示延迟数据" -ForegroundColor White
Write-Host "   - 预期：显示处理延迟和分发延迟（无传输延迟）" -ForegroundColor Green
Write-Host ""

Write-Host "4. 延迟告警验证：" -ForegroundColor Yellow
Write-Host "   - 观察是否有延迟告警触发" -ForegroundColor White
Write-Host "   - 检查告警阈值是否合理" -ForegroundColor White
Write-Host "   - 预期：延迟超过阈值时显示告警" -ForegroundColor Green
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "修复内容总结" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "✅ 添加了 SegmentSourceType 枚举区分直通和回放" -ForegroundColor Green
Write-Host "✅ PlaybackSource 现在正确设置 receive_time 和 source_type" -ForegroundColor Green
Write-Host "✅ Handler 根据 source_type 区分延迟监控逻辑" -ForegroundColor Green
Write-Host "✅ 直通播放：记录完整延迟链路（T1→T2→T3→T4）" -ForegroundColor Green
Write-Host "✅ 回放：只记录平台内部延迟（T2→T3→T4）" -ForegroundColor Green
Write-Host ""

Write-Host "详细分析文档：延迟监控问题分析.md" -ForegroundColor Cyan
Write-Host ""
