# Stop All Services Script

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Stopping All Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$stopped = $false

# Stop from process ID file
if (Test-Path ".process-ids.json") {
    Write-Host "Stopping services from process ID file..." -ForegroundColor Yellow
    
    try {
        $processIds = Get-Content ".process-ids.json" | ConvertFrom-Json
        
        if ($processIds.Platform) {
            $proc = Get-Process -Id $processIds.Platform -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Platform Server (PID: $($processIds.Platform))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Platform -Force -ErrorAction SilentlyContinue
                Write-Host "  + Platform Server stopped" -ForegroundColor Green
                $stopped = $true
            }
        }
        
        if ($processIds.Device) {
            $proc = Get-Process -Id $processIds.Device -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Device Simulator (PID: $($processIds.Device))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Device -Force -ErrorAction SilentlyContinue
                Write-Host "  + Device Simulator stopped" -ForegroundColor Green
                $stopped = $true
            }
        }
        
        if ($processIds.Frontend) {
            $proc = Get-Process -Id $processIds.Frontend -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Host "  Stopping Frontend (PID: $($processIds.Frontend))..." -ForegroundColor Gray
                Stop-Process -Id $processIds.Frontend -Force -ErrorAction SilentlyContinue
                Write-Host "  + Frontend stopped" -ForegroundColor Green
                $stopped = $true
            }
        }
        
        Remove-Item ".process-ids.json" -ErrorAction SilentlyContinue
    } catch {
        Write-Host "  Warning: Failed to read process ID file" -ForegroundColor Yellow
    }
    
    Write-Host ""
}

# Stop by process name
Write-Host "Stopping services by process name..." -ForegroundColor Yellow

$processes = @(
    @{Name="platform-server"; Display="Platform Server"},
    @{Name="device-simulator"; Display="Device Simulator"},
    @{Name="node"; Display="Frontend (Node.js)"}
)

foreach ($proc in $processes) {
    $procs = Get-Process -Name $proc.Name -ErrorAction SilentlyContinue
    if ($procs) {
        foreach ($p in $procs) {
            Write-Host "  Stopping $($proc.Display) (PID: $($p.Id))..." -ForegroundColor Gray
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            $stopped = $true
        }
        Write-Host "  + $($proc.Display) stopped" -ForegroundColor Green
    }
}

Write-Host ""

# Clean up device processes record
if (Test-Path ".device-processes.json") {
    Write-Host "Cleaning device processes record..." -ForegroundColor Yellow
    Remove-Item ".device-processes.json" -ErrorAction SilentlyContinue
    Write-Host "  + Device processes record cleaned" -ForegroundColor Green
    Write-Host ""
}

if ($stopped) {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "All services stopped!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
} else {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "No running services found" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Cyan
}

Write-Host ""
