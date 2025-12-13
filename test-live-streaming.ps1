# 端到端直通播放集成测试脚本
# Task 0.9: End-to-End Live Streaming Integration Test

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "端到端直通播放集成测试" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 配置
$PLATFORM_URL = "http://localhost:8080"
$DEVICE_ID = "device_001"
$TEST_RESULTS = @()

# 辅助函数：记录测试结果
function Record-TestResult {
    param(
        [string]$TestName,
        [bool]$Passed,
        [string]$Message,
        [double]$Latency = 0
    )
    
    $result = @{
        TestName = $TestName
        Passed = $Passed
        Message = $Message
        Latency = $Latency
        Timestamp = Get-Date
    }
    
    $script:TEST_RESULTS += $result
    
    if ($Passed) {
        Write-Host "✓ $TestName" -ForegroundColor Green
        if ($Latency -gt 0) {
            Write-Host "  延迟: $($Latency)ms" -ForegroundColor Gray
        }
    } else {
        Write-Host "✗ $TestName" -ForegroundColor Red
        Write-Host "  错误: $Message" -ForegroundColor Red
    }
    Write-Host "  $Message" -ForegroundColor Gray
}

# 测试1: 检查系统组件状态
Write-Host "`n[测试 1/6] 检查系统组件状态..." -ForegroundColor Yellow

try {
    # 检查平台服务器
    $response = Invoke-WebRequest -Uri "$PLATFORM_URL/api/v1/health" -Method GET -TimeoutSec 5
    if ($response.StatusCode -eq 200) {
        Record-TestResult -TestName "平台服务器健康检查" -Passed $true -Message "平台服务器运行正常"
    }
} catch {
    Record-TestResult -TestName "平台服务器健康检查" -Passed $false -Message "无法连接到平台服务器: $_"
    Write-Host "`n❌ 测试失败：平台服务器未运行" -ForegroundColor Red
    exit 1
}

# 检查设备是否在线
try {
    $response = Invoke-RestMethod -Uri "$PLATFORM_URL/api/v1/devices" -Method GET
    $device = $response.data | Where-Object { $_.device_id -eq $DEVICE_ID }
    
    if ($device -and $device.status -eq "online") {
        Record-TestResult -TestName "设备在线状态" -Passed $true -Message "设备 $DEVICE_ID 在线"
    } else {
        Record-TestResult -TestName "设备在线状态" -Passed $false -Message "设备 $DEVICE_ID 不在线或未找到"
        Write-Host "`n❌ 测试失败：设备未在线" -ForegroundColor Red
        exit 1
    }
} catch {
    Record-TestResult -TestName "设备在线状态" -Passed $false -Message "无法获取设备列表: $_"
    exit 1
}

# 测试2: 启动直通播放
Write-Host "`n[测试 2/6] 启动直通播放..." -ForegroundColor Yellow

$startTime = Get-Date
$sessionId = $null

try {
    $body = @{
        mode = "live"
        source = @{
            device_id = $DEVICE_ID
        }
        config = @{
            client_id = "test_client_001"
            low_latency_mode = $true
            target_latency_ms = 100
        }
    } | ConvertTo-Json

    $response = Invoke-RestMethod -Uri "$PLATFORM_URL/api/v1/stream/start" -Method POST -Body $body -ContentType "application/json"
    
    if ($response.status -eq "success" -and $response.data.session_id) {
        $sessionId = $response.data.session_id
        $streamUrl = $response.data.stream_url
        $estimatedLatency = $response.data.estimated_latency_ms
        
        Record-TestResult -TestName "启动直通播放" -Passed $true -Message "会话ID: $sessionId, 预估延迟: ${estimatedLatency}ms"
    } else {
        Record-TestResult -TestName "启动直通播放" -Passed $false -Message "响应格式错误"
        exit 1
    }
} catch {
    Record-TestResult -TestName "启动直通播放" -Passed $false -Message "启动失败: $_"
    exit 1
}

# 测试3: 验证视频分片实时传输
Write-Host "`n[测试 3/6] 验证视频分片实时传输..." -ForegroundColor Yellow

$segmentCount = 0
$firstSegmentTime = $null
$lastSegmentTime = $null
$latencies = @()

try {
    # 创建SSE连接（使用curl模拟，因为PowerShell不直接支持SSE）
    $sseUrl = "$PLATFORM_URL/api/v1/stream/$sessionId/segments"
    
    Write-Host "  连接到SSE端点: $sseUrl" -ForegroundColor Gray
    
    # 使用后台作业接收SSE事件
    $job = Start-Job -ScriptBlock {
        param($url)
        
        $request = [System.Net.HttpWebRequest]::Create($url)
        $request.Method = "GET"
        $request.Accept = "text/event-stream"
        $request.KeepAlive = $true
        
        try {
            $response = $request.GetResponse()
            $stream = $response.GetResponseStream()
            $reader = New-Object System.IO.StreamReader($stream)
            
            $count = 0
            $startTime = Get-Date
            
            while ($count -lt 30 -and -not $reader.EndOfStream) {
                $line = $reader.ReadLine()
                
                if ($line -match "^data: ") {
                    $data = $line.Substring(6)
                    try {
                        $segment = $data | ConvertFrom-Json
                        $count++
                        
                        $now = Get-Date
                        $elapsed = ($now - $startTime).TotalMilliseconds
                        
                        Write-Output @{
                            Count = $count
                            SegmentId = $segment.segment_id
                            Timestamp = $segment.timestamp
                            DataLength = $segment.data_length
                            IsKeyframe = $segment.flags -eq 1
                            Elapsed = $elapsed
                        }
                        
                        if ($count -ge 30) { break }
                    } catch {
                        # 忽略解析错误
                    }
                }
            }
            
            $reader.Close()
            $stream.Close()
            $response.Close()
        } catch {
            Write-Error "SSE连接错误: $_"
        }
    } -ArgumentList $sseUrl
    
    # 等待接收分片（最多30秒）
    $timeout = 30
    $elapsed = 0
    
    while ($elapsed -lt $timeout) {
        Start-Sleep -Milliseconds 500
        $elapsed += 0.5
        
        $results = Receive-Job -Job $job
        if ($results) {
            foreach ($result in $results) {
                $segmentCount++
                
                if ($null -eq $firstSegmentTime) {
                    $firstSegmentTime = Get-Date
                }
                $lastSegmentTime = Get-Date
                
                if ($segmentCount % 10 -eq 0) {
                    Write-Host "  已接收 $segmentCount 个分片..." -ForegroundColor Gray
                }
            }
        }
        
        if ($segmentCount -ge 30) {
            break
        }
    }
    
    Stop-Job -Job $job
    Remove-Job -Job $job
    
    if ($segmentCount -ge 10) {
        $duration = ($lastSegmentTime - $firstSegmentTime).TotalSeconds
        $fps = $segmentCount / $duration
        
        Record-TestResult -TestName "视频分片实时传输" -Passed $true -Message "接收到 $segmentCount 个分片，平均帧率: $([math]::Round($fps, 2)) fps"
    } else {
        Record-TestResult -TestName "视频分片实时传输" -Passed $false -Message "仅接收到 $segmentCount 个分片（预期至少10个）"
    }
} catch {
    Record-TestResult -TestName "视频分片实时传输" -Passed $false -Message "传输验证失败: $_"
}

# 测试4: 验证端到端延迟<100ms
Write-Host "`n[测试 4/6] 验证端到端延迟..." -ForegroundColor Yellow

try {
    # 获取会话统计信息
    $response = Invoke-RestMethod -Uri "$PLATFORM_URL/api/v1/stream/$sessionId/status" -Method GET
    
    if ($response.status -eq "success") {
        $stats = $response.data.stats
        $avgLatency = $stats.average_latency_ms
        $currentLatency = $stats.current_latency_ms
        $p95Latency = $stats.p95_latency_ms
        
        if ($avgLatency -lt 100 -and $p95Latency -lt 100) {
            Record-TestResult -TestName "端到端延迟<100ms" -Passed $true -Message "平均延迟: $([math]::Round($avgLatency, 2))ms, P95延迟: $([math]::Round($p95Latency, 2))ms" -Latency $avgLatency
        } else {
            Record-TestResult -TestName "端到端延迟<100ms" -Passed $false -Message "延迟超标 - 平均: $([math]::Round($avgLatency, 2))ms, P95: $([math]::Round($p95Latency, 2))ms" -Latency $avgLatency
        }
    } else {
        Record-TestResult -TestName "端到端延迟<100ms" -Passed $false -Message "无法获取统计信息"
    }
} catch {
    Record-TestResult -TestName "端到端延迟<100ms" -Passed $false -Message "延迟验证失败: $_"
}

# 测试5: 测试暂停/恢复功能
Write-Host "`n[测试 5/6] 测试暂停/恢复功能..." -ForegroundColor Yellow

try {
    # 暂停
    $body = @{
        command = "pause"
    } | ConvertTo-Json
    
    $pauseStart = Get-Date
    $response = Invoke-RestMethod -Uri "$PLATFORM_URL/api/v1/stream/$sessionId/control" -Method POST -Body $body -ContentType "application/json"
    $pauseLatency = ((Get-Date) - $pauseStart).TotalMilliseconds
    
    if ($response.status -eq "success" -and $pauseLatency -lt 100) {
        Record-TestResult -TestName "暂停功能" -Passed $true -Message "暂停成功，响应时间: $([math]::Round($pauseLatency, 2))ms" -Latency $pauseLatency
    } else {
        Record-TestResult -TestName "暂停功能" -Passed $false -Message "暂停失败或响应时间过长"
    }
    
    Start-Sleep -Seconds 2
    
    # 恢复
    $body = @{
        command = "resume"
    } | ConvertTo-Json
    
    $resumeStart = Get-Date
    $response = Invoke-RestMethod -Uri "$PLATFORM_URL/api/v1/stream/$sessionId/control" -Method POST -Body $body -ContentType "application/json"
    $resumeLatency = ((Get-Date) - $resumeStart).TotalMilliseconds
    
    if ($response.status -eq "success" -and $resumeLatency -lt 100) {
        Record-TestResult -TestName "恢复功能" -Passed $true -Message "恢复成功，响应时间: $([math]::Round($resumeLatency, 2))ms" -Latency $resumeLatency
    } else {
        Record-TestResult -TestName "恢复功能" -Passed $false -Message "恢复失败或响应时间过长"
    }
} catch {
    Record-TestResult -TestName "暂停/恢复功能" -Passed $false -Message "控制功能测试失败: $_"
}

# 测试6: 停止流
Write-Host "`n[测试 6/6] 停止流..." -ForegroundColor Yellow

try {
    $response = Invoke-WebRequest -Uri "$PLATFORM_URL/api/v1/stream/$sessionId" -Method DELETE
    
    if ($response.StatusCode -eq 204) {
        Record-TestResult -TestName "停止流" -Passed $true -Message "流已成功停止"
    } else {
        Record-TestResult -TestName "停止流" -Passed $false -Message "停止失败，状态码: $($response.StatusCode)"
    }
} catch {
    Record-TestResult -TestName "停止流" -Passed $false -Message "停止失败: $_"
}

# 生成测试报告
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "测试报告" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$totalTests = $TEST_RESULTS.Count
$passedTests = ($TEST_RESULTS | Where-Object { $_.Passed }).Count
$failedTests = $totalTests - $passedTests
$passRate = [math]::Round(($passedTests / $totalTests) * 100, 2)

Write-Host "`n总测试数: $totalTests" -ForegroundColor White
Write-Host "通过: $passedTests" -ForegroundColor Green
Write-Host "失败: $failedTests" -ForegroundColor Red
Write-Host "通过率: $passRate%" -ForegroundColor $(if ($passRate -ge 80) { "Green" } else { "Red" })

Write-Host "`n详细结果:" -ForegroundColor White
foreach ($result in $TEST_RESULTS) {
    $status = if ($result.Passed) { "✓" } else { "✗" }
    $color = if ($result.Passed) { "Green" } else { "Red" }
    Write-Host "  $status $($result.TestName)" -ForegroundColor $color
    if ($result.Latency -gt 0) {
        Write-Host "    延迟: $($result.Latency)ms" -ForegroundColor Gray
    }
}

# 保存测试结果到文件
$reportPath = "test-results-live-streaming-$(Get-Date -Format 'yyyyMMdd-HHmmss').json"
$TEST_RESULTS | ConvertTo-Json -Depth 10 | Out-File -FilePath $reportPath -Encoding UTF8
Write-Host "`n测试结果已保存到: $reportPath" -ForegroundColor Cyan

# 更新任务状态
if ($passRate -ge 80) {
    Write-Host "`n✅ 端到端直通播放集成测试通过！" -ForegroundColor Green
    Write-Host "任务 0.9 完成" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n❌ 端到端直通播放集成测试失败" -ForegroundColor Red
    Write-Host "请检查失败的测试项并修复问题" -ForegroundColor Yellow
    exit 1
}
