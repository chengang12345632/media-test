# Web Frontend 功能合并需求文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | Web Frontend 2 功能合并到 Web Frontend |
| 创建日期 | 2025-12-14 |
| 状态 | 草稿 |
| 优先级 | 中 |

## 简介

本需求文档定义了将 `web-frontend2` 目录中的新功能和优化合并到主 `web-frontend` 目录的需求。通过分析两个目录的差异，识别新增功能并制定合并策略。

## 术语表

- **WebTransport**: 基于 QUIC 的新一代 Web 传输协议，提供低延迟双向通信
- **WebCodecs**: 浏览器原生视频编解码 API，支持硬件加速
- **Config Module**: 前端配置管理模块，集中管理 API 地址和功能开关
- **Source Directory**: 源目录 (web-frontend2)
- **Target Directory**: 目标目录 (web-frontend)

## 差异分析

### 新增文件

1. **src/config.ts** - 配置管理模块
   - 集中管理 HTTP API URL
   - WebTransport 功能开关和 URL
   - 证书哈希配置
   - 环境配置注入支持

2. **src/components/WebTransportPlayer.tsx** - WebTransport 播放器组件
   - 使用 WebTransport 协议接收视频流
   - 使用 WebCodecs API 进行硬件加速解码
   - 支持超低延迟播放 (<50ms)
   - 实时 FPS 和延迟监控
   - 播放控制（暂停/恢复/倍速）

3. **src/services/webtransport.ts** - WebTransport 客户端服务
   - WebTransport 连接管理
   - 视频流接收和解析
   - 控制命令发送（暂停/恢复/定位/倍速）
   - 二进制协议处理（45字节固定头部）
   - 缓冲区管理和精确读取

### 修改文件

1. **src/App.tsx**
   - 在直通播放模式下使用 WebTransportPlayer 替代 VideoPlayer
   - 添加 certHash 属性传递

## 需求列表

### 需求 1: 配置管理模块集成

**用户故事**: 作为开发者，我希望有统一的配置管理模块，以便集中管理 API 地址和功能开关。

#### 验收标准

1. WHEN 前端应用启动 THEN System SHALL加载配置文件并打印配置信息到控制台
2. THE System SHALL支持从环境变量注入配置（通过 Vite）
3. THE System SHALL提供默认配置作为开发环境回退
4. THE System SHALL导出便捷访问的配置常量（HTTP_API_URL, WEBTRANSPORT_ENABLED, WEBTRANSPORT_URL, CERT_HASH）
5. WHEN 配置加载失败 THEN System SHALL使用默认配置并记录警告

### 需求 2: WebTransport 播放器组件集成

**用户故事**: 作为用户，我希望在直通播放时使用 WebTransport 协议，以便获得更低的延迟和更好的性能。

#### 验收标准

1. THE System SHALL在直通播放模式下使用 WebTransportPlayer 组件
2. WHEN 浏览器不支持 WebTransport THEN System SHALL显示错误提示并建议使用 Chrome/Edge
3. WHEN 浏览器不支持 WebCodecs THEN System SHALL显示错误提示并建议使用 Chrome/Edge
4. WHEN WebTransport 连接建立 THEN System SHALL显示连接状态和视频画面
5. THE System SHALL实时显示 FPS、延迟、接收数据量等统计信息
6. THE System SHALL支持播放控制（暂停/恢复/倍速）
7. WHEN 组件卸载 THEN System SHALL正确清理 WebTransport 连接和 VideoDecoder

### 需求 3: WebTransport 客户端服务集成

**用户故事**: 作为开发者，我希望有完整的 WebTransport 客户端服务，以便处理视频流接收和播放控制。

#### 验收标准

1. THE System SHALL实现 WebTransport 连接管理
2. THE System SHALL支持证书哈希验证（用于开发环境）
3. WHEN 接收视频流 THEN System SHALL正确解析45字节固定头部
4. WHEN 接收视频流 THEN System SHALL精确读取指定字节数的视频数据
5. THE System SHALL实现缓冲区管理，处理分片数据
6. THE System SHALL支持发送控制命令（暂停/恢复/定位/倍速/停止/获取状态）
7. WHEN 连接断开 THEN System SHALL触发错误回调并清理资源
8. THE System SHALL计算并报告端到端延迟

### 需求 4: App 组件更新

**用户故事**: 作为用户，我希望系统能够根据播放模式自动选择最佳播放器，以便获得最优的播放体验。

#### 验收标准

1. WHEN 用户启动直通播放 THEN System SHALL使用 WebTransportPlayer 组件
2. WHEN 用户启动录像回放 THEN System SHALL使用 VideoPlayer 组件
3. THE System SHALL正确传递 sessionId 和 certHash 属性到 WebTransportPlayer
4. WHEN 用户返回上一页 THEN System SHALL正确清理播放器资源

### 需求 5: 向后兼容性

**用户故事**: 作为开发者，我希望合并后的代码保持向后兼容，以便不影响现有功能。

#### 验收标准

1. THE System SHALL保持现有 VideoPlayer 组件的功能不变
2. THE System SHALL保持现有录像回放功能不变
3. THE System SHALL保持现有设备列表和录像列表功能不变
4. WHEN WebTransport 功能被禁用 THEN System SHALL回退到原有的播放方式
5. THE System SHALL不破坏现有的 API 调用和数据流

## 非功能性需求

### 性能需求

- WebTransport 播放延迟 < 50ms
- 配置加载时间 < 100ms
- 组件初始化时间 < 200ms

### 兼容性需求

- 支持 Chrome 97+ (WebTransport)
- 支持 Chrome 94+ (WebCodecs)
- 支持 Edge 97+ (WebTransport)
- 支持 Edge 94+ (WebCodecs)
- 对不支持的浏览器提供友好的错误提示

### 可维护性需求

- 代码复用率 > 80%
- 配置集中管理
- 清晰的组件职责划分
- 完整的错误处理和日志记录

## 约束条件

1. 必须保持现有功能的完整性
2. 必须提供浏览器兼容性检查
3. 必须正确处理资源清理
4. 不能引入新的依赖包（除非必要）
5. 必须保持代码风格一致

## 依赖关系

1. 依赖现有的 web-frontend 项目结构
2. 依赖 Vite 构建工具
3. 依赖 React 18+
4. 依赖 TypeScript 5+
5. 需要浏览器支持 WebTransport 和 WebCodecs API

## 合并策略

### 阶段 1: 新增文件复制

1. 复制 `src/config.ts` 到目标目录
2. 复制 `src/components/WebTransportPlayer.tsx` 到目标目录
3. 复制 `src/services/webtransport.ts` 到目标目录

### 阶段 2: 修改现有文件

1. 更新 `src/App.tsx`，添加 WebTransportPlayer 导入和使用
2. 确保在直通播放模式下使用 WebTransportPlayer

### 阶段 3: 测试验证

1. 测试配置模块加载
2. 测试 WebTransport 播放器功能
3. 测试向后兼容性
4. 测试错误处理

## 验收标准总结

系统将被认为满足需求，当且仅当：

1. ✅ 配置管理模块正常工作
2. ✅ WebTransport 播放器在支持的浏览器中正常工作
3. ✅ 不支持的浏览器显示友好的错误提示
4. ✅ 现有功能保持完整性
5. ✅ 所有播放控制功能正常工作
6. ✅ 资源清理正确执行
7. ✅ 延迟和性能指标符合预期

## 附录

### 参考文档

- WebTransport API 规范: https://w3c.github.io/webtransport/
- WebCodecs API 规范: https://w3c.github.io/webcodecs/
- 系统架构设计文档 v1.1

### 变更历史

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2025-12-14 | 初始版本 | 系统架构团队 |
