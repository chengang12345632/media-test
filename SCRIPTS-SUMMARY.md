# 启动脚本整合总结

## 📦 已创建的文件

### Windows 脚本 (PowerShell)

| 文件 | 说明 | 用途 |
|------|------|------|
| `start-debug.ps1` | Debug 模式启动 | 开发环境，快速编译 |
| `start-release.ps1` | Release 模式启动 | 生产环境，性能优化 |
| `start-device.ps1` | 单独启动设备 | 启动设备模拟器（随机生成设备信息） |
| `stop-all.ps1` | 停止所有服务 | 清理所有运行中的进程 |

### Linux/macOS 脚本 (Bash)

| 文件 | 说明 | 用途 |
|------|------|------|
| `start-debug.sh` | Debug 模式启动 | 开发环境，快速编译 |
| `start-release.sh` | Release 模式启动 | 生产环境，性能优化 |
| `start-device.sh` | 单独启动设备 | 启动设备模拟器（随机生成设备信息） |
| `stop-all.sh` | 停止所有服务 | 清理所有运行中的进程 |
| `setup-permissions.sh` | 设置脚本权限 | 一键设置所有 shell 脚本的执行权限 |

### 文档文件

| 文件 | 说明 |
|------|------|
| `SCRIPTS-README.md` | 快速使用指南 |
| `SCRIPTS-GUIDE.md` | 详细使用文档 |
| `SCRIPTS-SUMMARY.md` | 本文件，整合总结 |

### 更新的文件

| 文件 | 更新内容 |
|------|----------|
| `README.md` | 添加了跨平台启动脚本说明 |

## ✨ 主要功能

### 1. 跨平台支持
- ✅ Windows (PowerShell .ps1)
- ✅ Linux (Bash .sh)
- ✅ macOS (Bash .sh)

### 2. 自动进程管理
- ✅ 启动前自动检查现有进程
- ✅ 自动停止冲突进程
- ✅ 保存进程ID到 JSON 文件
- ✅ 清理资源和端口

### 3. 随机设备生成
- ✅ 自动生成设备类型（Camera, Sensor, Monitor, Recorder, Gateway）
- ✅ 自动生成位置（Office, Warehouse, Lobby, Parking, Lab, Factory, Store）
- ✅ 自动生成编号（100-999）
- ✅ 示例：`device_camera_office_234`

### 4. 两种构建模式
- ✅ **Debug**: 快速编译，包含调试信息，适合开发
- ✅ **Release**: 性能优化，编译较慢，适合测试和生产

### 5. 灵活选项
- ✅ 跳过编译选项（`-SkipBuild` / `--skip-build`）
- ✅ 自定义设备ID
- ✅ 自定义服务器地址
- ✅ 选择构建模式

## 🚀 快速开始

### Windows

```powershell
# 启动所有服务（Debug 模式）
.\start-debug.ps1

# 访问前端
# http://localhost:5173

# 停止所有服务
.\stop-all.ps1
```

### Linux/macOS

```bash
# 首次使用：设置权限
chmod +x *.sh
# 或使用
./setup-permissions.sh

# 启动所有服务（Debug 模式）
./start-debug.sh

# 访问前端
# http://localhost:5173

# 停止所有服务
./stop-all.sh
```

## 📋 命令对照表

| 功能 | Windows | Linux/macOS |
|------|---------|-------------|
| Debug 启动 | `.\start-debug.ps1` | `./start-debug.sh` |
| Release 启动 | `.\start-release.ps1` | `./start-release.sh` |
| 跳过编译 | `.\start-debug.ps1 -SkipBuild` | `./start-debug.sh --skip-build` |
| 启动设备 | `.\start-device.ps1` | `./start-device.sh` |
| 指定设备ID | `.\start-device.ps1 -DeviceId "xxx"` | `./start-device.sh --device-id "xxx"` |
| Release 设备 | `.\start-device.ps1 -Release` | `./start-device.sh --release` |
| 停止所有 | `.\stop-all.ps1` | `./stop-all.sh` |

## 🔄 与旧脚本的对比

### 旧脚本（已整合）

- ❌ `start-all-simple.ps1` - 功能分散
- ❌ `start-services.ps1` - 无进程管理
- ❌ `quick-test-setup.ps1` - 仅编译
- ❌ `rebuild-and-restart.ps1` - 功能重复
- ❌ `restart-after-fix.ps1` - 功能重复

### 新脚本（推荐使用）

- ✅ `start-debug.ps1` / `start-debug.sh` - 统一 Debug 启动
- ✅ `start-release.ps1` / `start-release.sh` - 统一 Release 启动
- ✅ `start-device.ps1` / `start-device.sh` - 设备管理
- ✅ `stop-all.ps1` / `stop-all.sh` - 统一停止

### 改进点

1. **跨平台支持** - 同时支持 Windows 和 Linux/macOS
2. **自动进程管理** - 启动前自动清理
3. **随机设备生成** - 方便多设备测试
4. **统一接口** - 减少脚本数量，简化使用
5. **更好的错误处理** - 详细的错误提示
6. **进程跟踪** - JSON 文件记录进程信息

## 📝 生成的文件

脚本运行时会生成以下文件：

- `.process-ids.json` - 主服务进程ID
- `.device-processes.json` - 设备进程ID列表

这些文件用于进程管理，会在停止服务时自动清理。

## 🎯 使用建议

1. **日常开发**: 使用 `start-debug` 脚本
2. **性能测试**: 使用 `start-release` 脚本
3. **多设备测试**: 使用 `start-device` 脚本多次启动
4. **快速重启**: 使用 `-SkipBuild` / `--skip-build` 参数
5. **清理环境**: 定期使用 `stop-all` 脚本

## 🔗 相关文档

- [SCRIPTS-README.md](SCRIPTS-README.md) - 快速使用指南
- [SCRIPTS-GUIDE.md](SCRIPTS-GUIDE.md) - 详细使用文档
- [README.md](README.md) - 项目总览

## 📞 问题反馈

如果遇到问题：

1. 查看 [SCRIPTS-README.md](SCRIPTS-README.md) 的故障排查部分
2. 查看 [SCRIPTS-GUIDE.md](SCRIPTS-GUIDE.md) 的详细说明
3. 提交 GitHub Issue

---

**更新日期**: 2024-12-14  
**版本**: 1.0.0  
**状态**: ✅ 已完成
