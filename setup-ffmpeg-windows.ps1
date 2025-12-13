# FFmpeg环境配置脚本 (Windows)
# 用于配置直通播放功能所需的FFmpeg开发库

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "FFmpeg环境配置向导" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$INSTALL_DIR = "C:\ffmpeg-dev"
$PKG_CONFIG_DIR = "C:\pkg-config"

# 检查是否以管理员身份运行
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "⚠️  建议以管理员身份运行此脚本以设置系统环境变量" -ForegroundColor Yellow
    Write-Host "   当前将只设置用户级环境变量" -ForegroundColor Yellow
    Write-Host ""
}

# 步骤1: 检查Chocolatey
Write-Host "[步骤 1/5] 检查包管理器..." -ForegroundColor Yellow

$chocoInstalled = Get-Command choco -ErrorAction SilentlyContinue

if ($chocoInstalled) {
    Write-Host "✓ Chocolatey已安装" -ForegroundColor Green
    
    Write-Host "`n是否使用Chocolatey安装pkg-config? (推荐) [Y/n]: " -NoNewline -ForegroundColor Cyan
    $useChoco = Read-Host
    
    if ($useChoco -ne 'n' -and $useChoco -ne 'N') {
        Write-Host "`n正在安装pkg-config..." -ForegroundColor Yellow
        choco install pkgconfiglite -y
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ pkg-config安装成功" -ForegroundColor Green
        } else {
            Write-Host "✗ pkg-config安装失败" -ForegroundColor Red
        }
    }
} else {
    Write-Host "✗ Chocolatey未安装" -ForegroundColor Yellow
    Write-Host "  可以访问 https://chocolatey.org/install 安装Chocolatey" -ForegroundColor Gray
}

# 步骤2: 下载FFmpeg开发库
Write-Host "`n[步骤 2/5] 下载FFmpeg开发库..." -ForegroundColor Yellow

$ffmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip"
$ffmpegZip = "$env:TEMP\ffmpeg-dev.zip"

Write-Host "  下载地址: $ffmpegUrl" -ForegroundColor Gray
Write-Host "  这可能需要几分钟..." -ForegroundColor Gray

try {
    # 使用.NET WebClient下载（更可靠）
    $webClient = New-Object System.Net.WebClient
    $webClient.DownloadFile($ffmpegUrl, $ffmpegZip)
    Write-Host "✓ FFmpeg下载完成" -ForegroundColor Green
} catch {
    Write-Host "✗ FFmpeg下载失败: $_" -ForegroundColor Red
    Write-Host "`n请手动下载FFmpeg:" -ForegroundColor Yellow
    Write-Host "  1. 访问: https://github.com/BtbN/FFmpeg-Builds/releases" -ForegroundColor Gray
    Write-Host "  2. 下载: ffmpeg-master-latest-win64-gpl-shared.zip" -ForegroundColor Gray
    Write-Host "  3. 解压到: $INSTALL_DIR" -ForegroundColor Gray
    Write-Host "  4. 重新运行此脚本" -ForegroundColor Gray
    exit 1
}

# 步骤3: 解压FFmpeg
Write-Host "`n[步骤 3/5] 解压FFmpeg..." -ForegroundColor Yellow

if (Test-Path $INSTALL_DIR) {
    Write-Host "  目录已存在，正在清理..." -ForegroundColor Gray
    Remove-Item -Path $INSTALL_DIR -Recurse -Force
}

New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null

try {
    Expand-Archive -Path $ffmpegZip -DestinationPath $INSTALL_DIR -Force
    
    # 查找解压后的目录
    $extractedDir = Get-ChildItem -Path $INSTALL_DIR -Directory | Select-Object -First 1
    
    if ($extractedDir) {
        # 将内容移到根目录
        Get-ChildItem -Path $extractedDir.FullName | Move-Item -Destination $INSTALL_DIR -Force
        Remove-Item -Path $extractedDir.FullName -Recurse -Force
    }
    
    Write-Host "✓ FFmpeg解压完成" -ForegroundColor Green
    Write-Host "  安装位置: $INSTALL_DIR" -ForegroundColor Gray
} catch {
    Write-Host "✗ FFmpeg解压失败: $_" -ForegroundColor Red
    exit 1
}

# 步骤4: 创建pkg-config文件
Write-Host "`n[步骤 4/5] 创建pkg-config配置..." -ForegroundColor Yellow

$pkgConfigDir = "$INSTALL_DIR\lib\pkgconfig"
New-Item -ItemType Directory -Path $pkgConfigDir -Force | Out-Null

# 创建libavutil.pc
$avutilPc = @"
prefix=$INSTALL_DIR
exec_prefix=`${prefix}
libdir=`${prefix}/lib
includedir=`${prefix}/include

Name: libavutil
Description: FFmpeg utility library
Version: 57.28.100
Requires:
Conflicts:
Libs: -L`${libdir} -lavutil
Cflags: -I`${includedir}
"@

# 创建libavcodec.pc
$avcodecPc = @"
prefix=$INSTALL_DIR
exec_prefix=`${prefix}
libdir=`${prefix}/lib
includedir=`${prefix}/include

Name: libavcodec
Description: FFmpeg codec library
Version: 59.37.100
Requires: libavutil
Conflicts:
Libs: -L`${libdir} -lavcodec
Cflags: -I`${includedir}
"@

# 创建libavformat.pc
$avformatPc = @"
prefix=$INSTALL_DIR
exec_prefix=`${prefix}
libdir=`${prefix}/lib
includedir=`${prefix}/include

Name: libavformat
Description: FFmpeg container format library
Version: 59.27.100
Requires: libavcodec libavutil
Conflicts:
Libs: -L`${libdir} -lavformat
Cflags: -I`${includedir}
"@

# 创建libswscale.pc
$swscalePc = @"
prefix=$INSTALL_DIR
exec_prefix=`${prefix}
libdir=`${prefix}/lib
includedir=`${prefix}/include

Name: libswscale
Description: FFmpeg image scaling library
Version: 6.7.100
Requires: libavutil
Conflicts:
Libs: -L`${libdir} -lswscale
Cflags: -I`${includedir}
"@

$avutilPc | Out-File -FilePath "$pkgConfigDir\libavutil.pc" -Encoding ASCII
$avcodecPc | Out-File -FilePath "$pkgConfigDir\libavcodec.pc" -Encoding ASCII
$avformatPc | Out-File -FilePath "$pkgConfigDir\libavformat.pc" -Encoding ASCII
$swscalePc | Out-File -FilePath "$pkgConfigDir\libswscale.pc" -Encoding ASCII

Write-Host "✓ pkg-config文件创建完成" -ForegroundColor Green

# 步骤5: 设置环境变量
Write-Host "`n[步骤 5/5] 设置环境变量..." -ForegroundColor Yellow

$envTarget = if ($isAdmin) { "Machine" } else { "User" }

# 设置FFMPEG_DIR
[Environment]::SetEnvironmentVariable("FFMPEG_DIR", $INSTALL_DIR, $envTarget)
Write-Host "✓ FFMPEG_DIR = $INSTALL_DIR" -ForegroundColor Green

# 设置PKG_CONFIG_PATH
[Environment]::SetEnvironmentVariable("PKG_CONFIG_PATH", $pkgConfigDir, $envTarget)
Write-Host "✓ PKG_CONFIG_PATH = $pkgConfigDir" -ForegroundColor Green

# 添加到PATH
$currentPath = [Environment]::GetEnvironmentVariable("PATH", $envTarget)
$ffmpegBin = "$INSTALL_DIR\bin"

if ($currentPath -notlike "*$ffmpegBin*") {
    $newPath = "$currentPath;$ffmpegBin"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, $envTarget)
    Write-Host "✓ 已添加到PATH: $ffmpegBin" -ForegroundColor Green
} else {
    Write-Host "✓ PATH已包含FFmpeg" -ForegroundColor Green
}

# 设置当前会话的环境变量
$env:FFMPEG_DIR = $INSTALL_DIR
$env:PKG_CONFIG_PATH = $pkgConfigDir
$env:PATH = "$env:PATH;$ffmpegBin"

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "配置完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan

Write-Host "`n环境变量已设置:" -ForegroundColor White
Write-Host "  FFMPEG_DIR = $INSTALL_DIR" -ForegroundColor Gray
Write-Host "  PKG_CONFIG_PATH = $pkgConfigDir" -ForegroundColor Gray
Write-Host "  PATH += $ffmpegBin" -ForegroundColor Gray

Write-Host "`n验证安装:" -ForegroundColor White
Write-Host "  运行以下命令验证:" -ForegroundColor Gray
Write-Host "  ffmpeg -version" -ForegroundColor Cyan

# 验证FFmpeg
Write-Host "`n正在验证FFmpeg安装..." -ForegroundColor Yellow
try {
    $ffmpegVersion = & "$ffmpegBin\ffmpeg.exe" -version 2>&1 | Select-Object -First 1
    Write-Host "✓ $ffmpegVersion" -ForegroundColor Green
} catch {
    Write-Host "⚠️  无法验证FFmpeg: $_" -ForegroundColor Yellow
}

Write-Host "`n下一步:" -ForegroundColor White
Write-Host "  1. 关闭并重新打开PowerShell窗口（使环境变量生效）" -ForegroundColor Gray
Write-Host "  2. 运行: cd device-simulator" -ForegroundColor Cyan
Write-Host "  3. 运行: cargo clean" -ForegroundColor Cyan
Write-Host "  4. 运行: cargo build" -ForegroundColor Cyan

Write-Host "`n如果仍然遇到pkg-config错误:" -ForegroundColor Yellow
Write-Host "  请安装pkg-config: choco install pkgconfiglite" -ForegroundColor Gray
Write-Host "  或手动下载: https://sourceforge.net/projects/pkgconfiglite/" -ForegroundColor Gray

# 清理临时文件
if (Test-Path $ffmpegZip) {
    Remove-Item -Path $ffmpegZip -Force
}

Write-Host "`n按任意键退出..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
