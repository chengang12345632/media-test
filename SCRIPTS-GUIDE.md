# 启动脚本使用指南

本项目提供了一套整合的 PowerShell 启动脚本，用于快速启动和管理服务。

## 📋 脚本列表

### 主要启动脚本

| 脚本 | 说明 | 构建模式 |
|------|------|----------|
| `start-debug.ps1` | 启动所有服务（Debug 模式） | Debug |
| `start-release.ps1` | 启动所有服务（Release 模式） | Release |
| `start-device.ps1` | 单独启动设备模拟器 | Debug/Release |
| `stop-all.ps1` | 停止所有运行中的服务 | - |

### 旧脚本（已整合）

以下脚本已被新脚本整合，建议使用新脚本：

- ~~`start-all-simple.ps1`~~ → 使用 `start-debug.ps1`
- ~~`start-services.ps1`~~ → 使用 `start-debug.ps1`
- ~~`quick-test-setup.ps1`~~ → 使用 `start-debug.ps1`（自动编译）
- ~~`rebuild-and-restart.ps1`~~ → 使用 `start-debug.ps1` 或 `start-release.ps1`
- ~~`restart-after-fix.ps1`~~ → 使用 `start-debug.ps1` 或 `start-release.ps1`

## 🚀 快速开始

### 1. Debug 模式启动（开发推荐）

```powershell
# 首次启动（包含编译）
.\start-debug.ps1

# 跳过编译直接启动
.\start-debug.ps1 -SkipBuild
```

**特点：**
- 编译速度快
- 包含调试信息
- 适合开发和调试

### 2. Release 模式启动（性能测试）

```powershell
# 首次启动（包含编译）
.\start-release.ps1

# 跳过编译直接启动
.\start-release.ps1 -SkipBuild
```

**特点：**
- 性能优化
- 编译时间较长
- 适合性能测试和生产环境

### 3. 单独启动设备

```powershell
# 随机生成设备信息（推荐）
.\start-device.ps1

# 指定设备ID
.\start-device.ps1 -DeviceId "my_device_001"

# 指定服务器地址
.\start-device.ps1 -ServerAddr "192.168.1.100:8443"

# Release 模式
.\start-device.ps1 -Release

# 组合使用
.\start-device.ps1 -DeviceId "camera_lobby_001" -ServerAddr "127.0.0.1:8443" -Release
```

**随机设备信息示例：**
- `device_camera_office_234`
- `device_sensor_warehouse_567`
- `device_monitor_parking_891`

### 4. 停止所有服务

```powershell
.\stop-all.ps1
```

## 🔧 功能特性

### 自动进程管理

所有启动脚本都会：
1. **检查现有进程** - 自动检测是否有服务正在运行
2. **停止旧进程** - 启动前自动停止现有进程
3. **保存进程ID** - 将进程ID保存到 `.process-ids.json`
4. **清理资源** - 确保端口和资源被正确释放

### 编译管理

- **自动编译** - 默认会编译所有组件
- **跳过编译** - 使用 `-SkipBuild` 参数跳过编译步骤
- **依赖检查** - 自动检查和安装前端依赖

### 随机设备生成

`start-device.ps1` 会随机生成设备信息：
- **设备类型**: Camera, Sensor, Monitor, Recorder, Gateway
- **位置**: Office, Warehouse, Lobby, Parking, Lab, Factory, Store
- **编号**: 100-999 随机数字

## 📊 服务信息

启动后的服务地址：

| 服务 | 地址 | 说明 |
|------|------|------|
| Platform Server | http://localhost:8080 | 后端 API 服务 |
| Frontend | http://localhost:5173 | Web 前端界面 |
| Device Simulator | - | 设备模拟器（WebSocket） |

## 💡 使用场景

### 场景 1: 日常开发

```powershell
# 启动所有服务（Debug 模式）
.\start-debug.ps1

# 等待 10-20 秒后访问
# http://localhost:5173

# 完成后停止
.\stop-all.ps1
```

### 场景 2: 测试多设备

```powershell
# 启动主服务
.\start-debug.ps1

# 启动多个设备
.\start-device.ps1  # 设备 1
.\start-device.ps1  # 设备 2
.\start-device.ps1  # 设备 3

# 查看所有设备
Invoke-RestMethod -Uri http://localhost:8080/api/v1/devices

# 停止所有
.\stop-all.ps1
```

### 场景 3: 性能测试

```powershell
# 使用 Release 模式
.\start-release.ps1

# 运行性能测试
.\test-live-streaming.ps1

# 停止服务
.\stop-all.ps1
```

### 场景 4: 快速重启

```powershell
# 修改代码后快速重启（跳过编译）
.\stop-all.ps1
.\start-debug.ps1 -SkipBuild
```

## 🔍 故障排查

### 问题 1: 端口被占用

```powershell
# 停止所有服务
.\stop-all.ps1

# 检查端口占用
netstat -ano | findstr "8080"
netstat -ano | findstr "5173"
netstat -ano | findstr "8443"

# 强制结束进程（替换 PID）
taskkill /F /PID <PID>
```

### 问题 2: 编译失败

```powershell
# 清理构建缓存
cargo clean

# 重新编译
.\start-debug.ps1
```

### 问题 3: 前端依赖问题

```powershell
cd web-frontend
Remove-Item -Recurse -Force node_modules
Remove-Item package-lock.json
npm install
cd ..
.\start-debug.ps1
```

### 问题 4: 进程未正确停止

```powershell
# 手动停止所有相关进程
Get-Process platform-server -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process device-simulator -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process node -ErrorAction SilentlyContinue | Stop-Process -Force

# 删除进程ID文件
Remove-Item .process-ids.json -ErrorAction SilentlyContinue
Remove-Item .device-processes.json -ErrorAction SilentlyContinue
```

## 📝 进程管理文件

脚本会创建以下文件来跟踪进程：

- `.process-ids.json` - 主服务进程ID
- `.device-processes.json` - 设备模拟器进程ID

这些文件会在停止服务时自动清理。

## 🎯 最佳实践

1. **开发时使用 Debug 模式** - 编译快，便于调试
2. **测试时使用 Release 模式** - 性能更好，更接近生产环境
3. **定期清理进程** - 使用 `stop-all.ps1` 确保资源释放
4. **多设备测试** - 使用 `start-device.ps1` 模拟多个设备
5. **保持脚本更新** - 使用新的整合脚本替代旧脚本

## 🔗 相关文档

- [START-HERE.md](START-HERE.md) - 项目入门指南
- [README.md](README.md) - 项目总览
- [test-live-streaming.ps1](test-live-streaming.ps1) - 自动化测试脚本

## ⚙️ 高级选项

### 自定义环境变量

```powershell
# 修改日志级别
$env:RUST_LOG = "debug"
.\start-debug.ps1 -SkipBuild

# 修改服务端口（需要修改代码配置）
$env:PLATFORM_PORT = "8081"
.\start-debug.ps1
```

### 查看进程状态

```powershell
# 查看所有相关进程
Get-Process platform-server, device-simulator, node -ErrorAction SilentlyContinue

# 查看进程详细信息
Get-Content .process-ids.json | ConvertFrom-Json

# 查看设备进程
Get-Content .device-processes.json | ConvertFrom-Json
```

## 📞 获取帮助

如果遇到问题：

1. 查看本文档的故障排查部分
2. 检查服务日志（在各个 PowerShell 窗口中）
3. 使用 `stop-all.ps1` 清理所有进程后重试
4. 查看项目 README 和相关文档

---

**提示**: 建议将旧的启动脚本移到 `scripts/legacy/` 目录，保持项目根目录整洁。
