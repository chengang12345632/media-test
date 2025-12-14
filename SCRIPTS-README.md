# 启动脚本使用指南

## 快速开始

### Windows 系统

```powershell
# 1. Debug 模式（开发推荐）
.\start-debug.ps1

# 2. Release 模式（性能测试）
.\start-release.ps1

# 3. 单独启动设备（随机设备信息）
.\start-device.ps1

# 4. 停止所有服务
.\stop-all.ps1
```

### Linux/macOS 系统

```bash
# 首次使用：设置执行权限
chmod +x *.sh

# 1. Debug 模式（开发推荐）
./start-debug.sh

# 2. Release 模式（性能测试）
./start-release.sh

# 3. 单独启动设备（随机设备信息）
./start-device.sh

# 4. 停止所有服务
./stop-all.sh
```

## 主要特性

✅ **跨平台支持** - Windows (PowerShell) 和 Linux/macOS (Bash)  
✅ **自动进程管理** - 启动前自动检查并停止现有进程  
✅ **随机设备生成** - 自动生成设备ID（如 `device_camera_office_234`）  
✅ **两种构建模式** - Debug（快速）和 Release（优化）  
✅ **跳过编译选项** - 快速重启无需重新编译  

## 使用示例

### Windows (PowerShell)

```powershell
# 日常开发
.\start-debug.ps1

# 跳过编译快速重启
.\start-debug.ps1 -SkipBuild

# 性能测试
.\start-release.ps1

# 启动随机设备
.\start-device.ps1

# 启动指定设备
.\start-device.ps1 -DeviceId "my_camera_001"

# Release 模式设备
.\start-device.ps1 -Release

# 停止所有
.\stop-all.ps1
```

### Linux/macOS (Bash)

```bash
# 日常开发
./start-debug.sh

# 跳过编译快速重启
./start-debug.sh --skip-build

# 性能测试
./start-release.sh

# 启动随机设备
./start-device.sh

# 启动指定设备
./start-device.sh --device-id "my_camera_001"

# 指定服务器地址
./start-device.sh --server-addr "192.168.1.100:8443"

# Release 模式设备
./start-device.sh --release

# 停止所有
./stop-all.sh
```

## 服务地址

- Platform Server: http://localhost:8080
- Frontend: http://localhost:5173
- Device Simulator: WebSocket 连接

## 💡 使用场景

### 场景 1: 日常开发

**Windows:**
```powershell
.\start-debug.ps1
# 等待 10-20 秒后访问 http://localhost:5173
.\stop-all.ps1
```

**Linux/macOS:**
```bash
./start-debug.sh
# 等待 10-20 秒后访问 http://localhost:5173
./stop-all.sh
```

### 场景 2: 测试多设备

**Windows:**
```powershell
.\start-debug.ps1
.\start-device.ps1  # 设备 1
.\start-device.ps1  # 设备 2
.\start-device.ps1  # 设备 3
Invoke-RestMethod -Uri http://localhost:8080/api/v1/devices
.\stop-all.ps1
```

**Linux/macOS:**
```bash
./start-debug.sh
./start-device.sh  # 设备 1
./start-device.sh  # 设备 2
./start-device.sh  # 设备 3
curl http://localhost:8080/api/v1/devices | jq
./stop-all.sh
```

### 场景 3: 性能测试

**Windows:**
```powershell
.\start-release.ps1
.\test-live-streaming.ps1
.\stop-all.ps1
```

**Linux/macOS:**
```bash
./start-release.sh
# 运行性能测试
./stop-all.sh
```

### 场景 4: 快速重启

**Windows:**
```powershell
.\stop-all.ps1
.\start-debug.ps1 -SkipBuild
```

**Linux/macOS:**
```bash
./stop-all.sh
./start-debug.sh --skip-build
```

## 🔍 故障排查

### 问题 1: 端口被占用

**Windows:**
```powershell
.\stop-all.ps1
netstat -ano | findstr "8080"
netstat -ano | findstr "5173"
netstat -ano | findstr "8443"
taskkill /F /PID <PID>
```

**Linux/macOS:**
```bash
./stop-all.sh
lsof -i :8080
lsof -i :5173
lsof -i :8443
kill -9 <PID>
```

### 问题 2: 编译失败

**Windows:**
```powershell
cargo clean
.\start-debug.ps1
```

**Linux/macOS:**
```bash
cargo clean
./start-debug.sh
```

### 问题 3: 前端依赖问题

**Windows:**
```powershell
cd web-frontend
Remove-Item -Recurse -Force node_modules
Remove-Item package-lock.json
npm install
cd ..
.\start-debug.ps1
```

**Linux/macOS:**
```bash
cd web-frontend
rm -rf node_modules package-lock.json
npm install
cd ..
./start-debug.sh
```

### 问题 4: 进程未正确停止

**Windows:**
```powershell
Get-Process platform-server -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process device-simulator -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process node -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item .process-ids.json -ErrorAction SilentlyContinue
Remove-Item .device-processes.json -ErrorAction SilentlyContinue
```

**Linux/macOS:**
```bash
pkill -9 platform-server
pkill -9 device-simulator
pkill -9 node
rm -f .process-ids.json .device-processes.json
```

### 问题 5: Shell 脚本权限问题 (Linux/macOS)

```bash
# 设置执行权限
chmod +x *.sh

# 或单独设置
chmod +x start-debug.sh start-release.sh start-device.sh stop-all.sh
```

### 问题 6: jq 命令未找到 (Linux/macOS)

Shell 脚本使用 `jq` 来解析 JSON 文件。如果未安装：

```bash
# Ubuntu/Debian
sudo apt-get install jq

# macOS
brew install jq

# CentOS/RHEL
sudo yum install jq
```

如果无法安装 `jq`，可以手动编辑 `.process-ids.json` 文件。

## 📝 进程管理文件

脚本会创建以下文件来跟踪进程：

- `.process-ids.json` - 主服务进程ID
- `.device-processes.json` - 设备模拟器进程ID

这些文件会在停止服务时自动清理。

## 🎯 最佳实践

1. **开发时使用 Debug 模式** - 编译快，便于调试
2. **测试时使用 Release 模式** - 性能更好，更接近生产环境
3. **定期清理进程** - 使用 `stop-all` 脚本确保资源释放
4. **多设备测试** - 使用 `start-device` 脚本模拟多个设备
5. **保持脚本更新** - 使用新的整合脚本替代旧脚本

## 🔗 相关文档

- [START-HERE.md](START-HERE.md) - 项目入门指南
- [README.md](README.md) - 项目总览
- [SCRIPTS-GUIDE.md](SCRIPTS-GUIDE.md) - 详细脚本指南

## ⚙️ 高级选项

### 自定义环境变量

**Windows:**
```powershell
$env:RUST_LOG = "debug"
.\start-debug.ps1 -SkipBuild
```

**Linux/macOS:**
```bash
export RUST_LOG=debug
./start-debug.sh --skip-build
```

### 查看进程状态

**Windows:**
```powershell
Get-Process platform-server, device-simulator, node -ErrorAction SilentlyContinue
Get-Content .process-ids.json | ConvertFrom-Json
Get-Content .device-processes.json | ConvertFrom-Json
```

**Linux/macOS:**
```bash
ps aux | grep -E "platform-server|device-simulator|node"
cat .process-ids.json | jq
cat .device-processes.json | jq
```

## 📞 获取帮助

如果遇到问题：

1. 查看本文档的故障排查部分
2. 检查服务日志（在各个终端窗口中）
3. 使用 `stop-all` 脚本清理所有进程后重试
4. 查看项目 README 和相关文档

---

**提示**: 建议将旧的启动脚本移到 `scripts/legacy/` 目录，保持项目根目录整洁。
