# Start Device Simulator with Random Device Info
param(
    [string]$DeviceId = "",
    [string]$ServerAddr = "127.0.0.1:8443",
    [switch]$Release
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting Device Simulator" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Generate random device info
function Get-RandomDeviceInfo {
    $deviceTypes = @("Camera", "Sensor", "Monitor", "Recorder", "Gateway")
    $locations = @("Office", "Warehouse", "Lobby", "Parking", "Lab", "Factory", "Store")
    
    $type = $deviceTypes | Get-Random
    $location = $locations | Get-Random
    $number = Get-Random -Minimum 100 -Maximum 999
    
    $deviceId = "device_${type}_${location}_${number}".ToLower()
    
    return @{
        DeviceId = $deviceId
        Type = $type
        Location = $location
        Number = $number
    }
}

# Determine device ID
if ([string]::IsNullOrEmpty($DeviceId)) {
    $deviceInfo = Get-RandomDeviceInfo
    $DeviceId = $deviceInfo.DeviceId
    
    Write-Host "Generated random device info:" -ForegroundColor Yellow
    Write-Host "  Device ID:   $($deviceInfo.DeviceId)" -ForegroundColor Gray
    Write-Host "  Type:        $($deviceInfo.Type)" -ForegroundColor Gray
    Write-Host "  Location:    $($deviceInfo.Location)" -ForegroundColor Gray
    Write-Host "  Number:      $($deviceInfo.Number)" -ForegroundColor Gray
    Write-Host ""
} else {
    Write-Host "Using specified device ID: $DeviceId" -ForegroundColor Yellow
    Write-Host ""
}

# Determine build mode
$buildMode = if ($Release) { "release" } else { "debug" }
$exePath = "..\target\$buildMode\device-simulator.exe"

Write-Host "Checking executable..." -ForegroundColor Yellow
if (-not (Test-Path $exePath)) {
    Write-Host "  X device-simulator.exe not found ($buildMode)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please build the project first:" -ForegroundColor Yellow
    if ($Release) {
        Write-Host "  cd device-simulator" -ForegroundColor White
        Write-Host "  cargo build --release" -ForegroundColor White
    } else {
        Write-Host "  cd device-simulator" -ForegroundColor White
        Write-Host "  cargo build" -ForegroundColor White
    }
    Write-Host ""
    exit 1
}
Write-Host "  + Executable ready" -ForegroundColor Green
Write-Host ""

# Check existing devices
Write-Host "Checking existing device processes..." -ForegroundColor Yellow
$existingDevices = Get-Process -Name "device-simulator" -ErrorAction SilentlyContinue
if ($existingDevices) {
    Write-Host "  Found $($existingDevices.Count) device simulator(s) running" -ForegroundColor Yellow
    
    $response = Read-Host "  Continue to start new device? (y/n)"
    if ($response -ne "y" -and $response -ne "Y") {
        Write-Host "  Cancelled" -ForegroundColor Gray
        exit 0
    }
}
Write-Host ""

# Start device simulator
Write-Host "Starting device simulator..." -ForegroundColor Yellow
Write-Host "  Device ID:     $DeviceId" -ForegroundColor Gray
Write-Host "  Server:        $ServerAddr" -ForegroundColor Gray
Write-Host "  Build Mode:    $buildMode" -ForegroundColor Gray
Write-Host ""

$deviceProcess = Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD\device-simulator'; `$env:RUST_LOG='info'; $exePath --device-id $DeviceId --server-addr $ServerAddr" -PassThru

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Device simulator started!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Device Information:" -ForegroundColor White
Write-Host "  Device ID:     $DeviceId" -ForegroundColor Gray
Write-Host "  Server:        $ServerAddr" -ForegroundColor Gray
Write-Host "  Process ID:    $($deviceProcess.Id)" -ForegroundColor Gray
Write-Host "  Build Mode:    $buildMode" -ForegroundColor Gray
Write-Host ""
Write-Host "Stop device:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Id $($deviceProcess.Id)" -ForegroundColor White
Write-Host ""
Write-Host "View device list:" -ForegroundColor Yellow
Write-Host "  Invoke-RestMethod -Uri http://localhost:8080/api/v1/devices" -ForegroundColor White
Write-Host ""

# Save device info
$deviceRecord = @{
    DeviceId = $DeviceId
    ProcessId = $deviceProcess.Id
    ServerAddr = $ServerAddr
    BuildMode = $buildMode
    StartTime = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
}

$devicesFile = ".device-processes.json"
$devices = @()

if (Test-Path $devicesFile) {
    try {
        $devices = Get-Content $devicesFile | ConvertFrom-Json
        $devices = $devices | Where-Object { 
            $proc = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
            $null -ne $proc
        }
    } catch {
        $devices = @()
    }
}

$devices += $deviceRecord
$devices | ConvertTo-Json | Out-File -FilePath $devicesFile -Encoding UTF8

Write-Host "Device info saved to: $devicesFile" -ForegroundColor Gray
Write-Host ""
