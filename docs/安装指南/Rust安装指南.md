# 🦀 Rust 安装指南

## 📍 安装程序位置

已下载到：`C:\Users\chengang\AppData\Local\Temp\rustup-init.exe`

---

## 🚀 安装步骤

### 方式一：双击运行（最简单）

1. **打开文件资源管理器**
2. **复制并粘贴以下路径到地址栏**：
   ```
   C:\Users\chengang\AppData\Local\Temp
   ```
3. **找到并双击** `rustup-init.exe`
4. **在弹出的命令行窗口中**：
   - 输入 `1` 然后按 `Enter`（选择默认安装）
   - 等待安装完成（约5分钟）
5. **安装完成后**：
   - 关闭所有 PowerShell/CMD 窗口
   - 重新打开 PowerShell
   - 运行 `rustc --version` 验证安装

---

### 方式二：命令行运行

打开 PowerShell，运行：

```powershell
# 运行安装程序
& "C:\Users\chengang\AppData\Local\Temp\rustup-init.exe"

# 按照提示操作：
# 1. 输入 1 然后按 Enter
# 2. 等待安装完成
# 3. 重启 PowerShell
```

---

## ✅ 验证安装

安装完成并重启 PowerShell 后，运行：

```powershell
# 检查 Rust 版本
rustc --version

# 检查 Cargo 版本
cargo --version

# 应该看到类似输出：
# rustc 1.75.0 (82e1608df 2023-12-21)
# cargo 1.75.0 (1d8b05cdd 2023-11-20)
```

---

## 🎯 安装完成后的下一步

安装成功后，告诉我，我会帮你：

1. ✅ 编译 Rust 项目（5-10分钟）
2. ✅ 安装前端依赖（2-3分钟）
3. ✅ 准备测试视频
4. ✅ 启动所有服务

---

## ❓ 常见问题

### Q: 安装后找不到 rustc 命令？

**A**: 需要重启 PowerShell 或添加环境变量：

```powershell
# 手动添加到当前会话
$env:Path += ";$env:USERPROFILE\.cargo\bin"

# 验证
rustc --version
```

### Q: 安装很慢？

**A**: Rust 安装需要下载约 200MB 的文件，请耐心等待。

### Q: 想更换安装位置？

**A**: 在安装时选择 `2) Customize installation`，然后设置自定义路径。

---

## 🔗 相关资源

- Rust 官网：https://www.rust-lang.org/
- Rustup 文档：https://rust-lang.github.io/rustup/
- Rust 中文社区：https://rust.cc/

---

**准备好了吗？安装完成后告诉我，我们继续！** 🚀
