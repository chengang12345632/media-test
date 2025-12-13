# 🚀 快速开始

## 一键启动

```powershell
# 首次运行需要先编译（约5-10分钟）
powershell -ExecutionPolicy Bypass -File .\quick-test-setup.ps1

# 启动所有服务
powershell -ExecutionPolicy Bypass -File .\start-all-simple.ps1
```

启动所有服务：平台服务器、设备模拟器、前端界面

**访问**: http://localhost:5173

---

## 一键测试

```powershell
powershell -ExecutionPolicy Bypass -File .\run-full-test.ps1
```

自动完成：配置环境、编译、启动服务、运行测试、生成报告

**预计时间**: 15-20分钟

---

## 详细文档

### 快速开始
- 📖 [快速开始指南](docs/快速开始/快速开始指南.md) - 15分钟完整教程

### 测试指南
- 📖 [测试执行指南](docs/测试指南/测试执行指南.md) - 详细测试步骤

### 故障排查
- 📖 [编译问题修复](docs/故障排查/编译问题修复指南.md) - 编译错误解决
- 📖 [常见问题解决](docs/故障排查/常见问题解决.md) - 常见问题FAQ

### 系统文档
- 📖 [系统架构](docs/系统文档/系统架构设计文档.md) - 完整架构设计
- 📖 [API文档](docs/系统文档/API接口文档.md) - API接口规范
- 📖 [开发手册](docs/系统文档/开发手册.md) - 开发指南

### 技术实现
- 📖 [直通播放实现](docs/技术实现/直通播放实现方案.md) - 直通播放技术方案

---

## 快速命令

```powershell
# 编译项目（首次运行）
powershell -ExecutionPolicy Bypass -File .\quick-test-setup.ps1

# 启动所有服务
powershell -ExecutionPolicy Bypass -File .\start-all-simple.ps1

# 查看服务状态
Invoke-RestMethod -Uri "http://localhost:8080/api/v1/health"

# 停止所有服务
$jobIds = Get-Content ".job-ids.json" | ConvertFrom-Json
Stop-Job -Id $jobIds.Platform,$jobIds.Device,$jobIds.Frontend
Remove-Job -Id $jobIds.Platform,$jobIds.Device,$jobIds.Frontend
```

---

**更多信息请查看 [docs](docs/) 目录**
