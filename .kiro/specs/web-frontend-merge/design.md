# Web Frontend 功能合并设计文档

## 文档信息

| 项目 | 内容 |
|------|------|
| 功能名称 | Web Frontend 2 功能合并到 Web Frontend |
| 创建日期 | 2025-12-14 |
| 状态 | 草稿 |
| 版本 | v1.0 |

## 概述

本设计文档描述了如何将 `web-frontend2` 中的新功能（WebTransport 播放器、配置管理模块、WebTransport 客户端服务）合并到主 `web-frontend` 目录中。合并策略采用增量式方法，确保向后兼容性，同时引入超低延迟的 WebTransport 播放能力。

### 设计目标

1. **功能增强**: 为直通播放模式添加 WebTransport + WebCodecs 支持，实现 <50ms 超低延迟
2. **向后兼容**: 保持现有 VideoPlayer 和 WebCodecsPlayer 的功能完整性
3. **配置集中**: 引入统一的配置管理模块，便于环境配置和功能开关
4. **渐进增强**: 提供浏览器兼容性检查，对不支持的浏览器给出友好提示
5. **代码复用**: 最大化利用现有组件，避免重复代码

### 核心优势

- **超低延迟**: WebTransport (QUIC) + WebCodecs 实现端到端延迟 <50ms
- **硬件加速**: 利用 WebCodecs API 进行 GPU 加速视频解码
- **实时监控**: 提供 FPS、延迟、数据量等实时统计信息
- **播放控制**: 支持暂停/恢复/倍速播放等控制功能

## 架构

### 整体架构

```
web-frontend/
├── src/
│   ├── config.ts                          [新增] 配置管理模块
│   ├── App.tsx                            [修改] 添加 WebTransportPlayer 支持
│   ├── components/
│   │   ├── VideoPlayer.tsx                [保持] 现有播放器（MP4直接播放）
│   │   ├── WebCodecsPlayer.tsx            [保持] H.264 SSE播放器
│   │   ├── WebTransportPlayer.tsx         [新增] WebTransport播放器
│   │   ├── DeviceList.tsx                 [保持] 设备列表
│   │   └── RecordingList.tsx              [保持] 录像列表
│   └── services/
│       ├── api.ts                         [保持] HTTP API客户端
│       └── webtransport.ts                [新增] WebTransport客户端
```

### 播放器选择逻辑

```
用户操作
    │
    ├─ 直通播放 (Live Mode)
    │   └─> WebTransportPlayer (新)
    │       ├─ 浏览器支持检查
    │       ├─ WebTransport 连接
    │       └─ WebCodecs 硬件解码
    │
    └─ 录像回放 (Playback Mode)
        ├─ MP4 文件
        │   └─> VideoPlayer (现有)
        │       └─ 直接 HTTP 流式传输
        │
        └─ H.264 文件
            └─> WebCodecsPlayer (现有)
                └─ SSE + WebCodecs 解码
```

### 数据流

```
直通播放模式 (WebTransport):
Device → Platform Server → WebTransport → WebTransportPlayer → WebCodecs → Canvas

录像回放模式 (MP4):
Storage → Platform Server → HTTP Stream → VideoPlayer → <video> element

录像回放模式 (H.264):
Storage → Platform Server → SSE → WebCodecsPlayer → WebCodecs → Canvas
```

## 组件和接口

### 1. 配置管理模块 (config.ts)

**职责**: 集中管理前端配置，支持环境变量注入

**接口**:
```typescript
interface AppConfig {
  httpApiUrl: string
  webtransportEnabled: boolean
  webtransportUrl: string
}

// 导出配置常量
export const HTTP_API_URL: string
export const WEBTRANSPORT_ENABLED: boolean
export const WEBTRANSPORT_URL: string
export const CERT_HASH: string
```

**配置来源**:
1. Vite 环境变量注入 (`__APP_CONFIG__`)
2. 默认配置（开发环境回退）

**配置项**:
- `httpApiUrl`: HTTP API 基础地址（默认: `http://localhost:8080`）
- `webtransportEnabled`: WebTransport 功能开关（默认: `true`）
- `webtransportUrl`: WebTransport 服务器地址（默认: `https://localhost:8081`）
- `CERT_HASH`: 开发环境证书哈希（用于绕过证书验证）

### 2. WebTransport 客户端服务 (webtransport.ts)

**职责**: 管理 WebTransport 连接，接收视频流，发送控制命令

**核心类**: `WebTransportClient`

**接口**:
```typescript
class WebTransportClient {
  // 连接管理
  async connect(sessionId: string, serverUrl: string, certHash?: string): Promise<void>
  async close(): Promise<void>
  
  // 回调设置
  setSegmentCallback(callback: SegmentCallback): void
  setErrorCallback(callback: ErrorCallback): void
  
  // 播放控制
  async pause(): Promise<void>
  async resume(): Promise<void>
  async seek(position: number): Promise<void>
  async setRate(rate: number): Promise<void>
  async getStatus(): Promise<ControlResponse>
  
  // 静态方法
  static isSupported(): boolean
}

// 数据类型
interface SegmentMetadata {
  segment_id: string
  timestamp: number
  duration: number
  is_keyframe: boolean
  data_length: number
  send_time_ms: number
  latency_ms?: number
}

interface ControlCommand {
  type: 'Pause' | 'Resume' | 'Seek' | 'SetRate' | 'Stop' | 'GetStatus'
  position?: number
  rate?: number
}

interface ControlResponse {
  success: boolean
  message: string
  current_state: string
  current_position?: number
  playback_rate?: number
}
```

**协议格式**:

视频流协议（45字节固定头部 + 可变长度数据）:
```
1. segment_id     (16字节) - UUID
2. timestamp      (8字节)  - f64, 小端序
3. duration       (8字节)  - f64, 小端序
4. is_keyframe    (1字节)  - bool
5. data_length    (4字节)  - u32, 小端序
6. send_time_ms   (8字节)  - u64, 小端序
7. video_data     (N字节)  - H.264 NAL units
```

**缓冲区管理**:
- 维护内部读取缓冲区 (`readBuffer`)
- 实现 `readExactly()` 方法，处理分片数据
- 支持跨 chunk 边界的精确字节读取

### 3. WebTransportPlayer 组件

**职责**: 使用 WebTransport 接收视频流，使用 WebCodecs 解码并渲染

**Props**:
```typescript
interface WebTransportPlayerProps {
  sessionId: string
  serverUrl?: string
}
```

**状态管理**:
```typescript
// 连接状态
status: string
error: string | null
isPlaying: boolean

// 统计信息
segmentCount: number
fps: number
bytesReceived: number
bufferSize: number
videoDuration: number
latency: number
avgLatency: number
```

**生命周期**:
1. **初始化阶段**:
   - 检查浏览器支持（WebTransport + WebCodecs）
   - 初始化 VideoDecoder
   - 创建 WebTransportClient
   - 建立 WebTransport 连接

2. **播放阶段**:
   - 接收视频分片（通过回调）
   - 创建 EncodedVideoChunk
   - 解码并渲染到 Canvas
   - 更新统计信息

3. **清理阶段**:
   - 关闭 WebTransport 连接
   - 关闭 VideoDecoder
   - 释放资源

**VideoDecoder 配置**:
```typescript
{
  codec: 'avc1.42E01E',  // H.264 Baseline Profile Level 3.0
  optimizeForLatency: true,
  hardwareAcceleration: 'prefer-hardware'
}
```

### 4. App 组件更新

**修改内容**:
1. 导入 WebTransportPlayer 和 config
2. 在直通播放模式下使用 WebTransportPlayer
3. 传递 sessionId 和 certHash 属性

**播放器选择逻辑**:
```typescript
// 直通播放模式
if (state.view === 'live' && state.sessionId) {
  return <WebTransportPlayer 
    sessionId={state.sessionId}
    serverUrl={WEBTRANSPORT_URL}
  />
}

// 录像回放模式
if (state.view === 'player' && state.sessionId) {
  return <VideoPlayer 
    sessionId={state.sessionId}
    fileId={state.selectedFileId}
  />
}
```

## 数据模型

### 配置数据模型

```typescript
interface AppConfig {
  httpApiUrl: string          // HTTP API 基础地址
  webtransportEnabled: boolean // WebTransport 功能开关
  webtransportUrl: string     // WebTransport 服务器地址
}
```

### 视频分片元数据

```typescript
interface SegmentMetadata {
  segment_id: string      // 分片唯一标识（UUID）
  timestamp: number       // 时间戳（秒）
  duration: number        // 持续时间（秒）
  is_keyframe: boolean    // 是否为关键帧
  data_length: number     // 数据长度（字节）
  send_time_ms: number    // 服务器发送时间（毫秒）
  latency_ms?: number     // 端到端延迟（毫秒）
}
```

### 播放器状态

```typescript
interface PlayerState {
  status: string          // 状态描述
  error: string | null    // 错误信息
  isPlaying: boolean      // 是否正在播放
  
  // 统计信息
  segmentCount: number    // 接收分片数
  fps: number             // 实时帧率
  bytesReceived: number   // 接收字节数
  bufferSize: number      // 解码缓冲区大小
  videoDuration: number   // 视频时长
  latency: number         // 当前延迟
  avgLatency: number      // 平均延迟
}
```

### 控制命令

```typescript
interface ControlCommand {
  type: 'Pause' | 'Resume' | 'Seek' | 'SetRate' | 'Stop' | 'GetStatus'
  position?: number       // 定位位置（秒）
  rate?: number           // 播放速率
}

interface ControlResponse {
  success: boolean
  message: string
  current_state: string
  current_position?: number
  playback_rate?: number
}
```


## 正确性属性

*属性是一个特征或行为，应该在系统的所有有效执行中保持为真——本质上是关于系统应该做什么的形式化陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。*

基于需求文档中的验收标准，我们识别出以下可测试的正确性属性：

### 属性 1: 环境变量配置注入

*对于任何*有效的环境变量配置，配置模块应该正确读取并导出这些值，而不是使用默认值。

**验证: 需求 1.2**

### 属性 2: 统计信息更新

*对于任何*接收到的视频分片，播放器应该更新统计信息（segmentCount、bytesReceived、latency 等），并且这些值应该单调递增或保持合理范围。

**验证: 需求 2.5**

### 属性 3: 播放控制响应

*对于任何*播放控制命令（暂停/恢复/倍速），WebTransportClient 应该正确编码并发送命令，并且播放器状态应该相应更新。

**验证: 需求 2.6, 3.6**

### 属性 4: 协议头部解析

*对于任何*符合协议格式的45字节头部数据，WebTransportClient 应该正确解析出所有字段（segment_id、timestamp、duration、is_keyframe、data_length、send_time_ms），并且解析结果应该与原始数据一致。

**验证: 需求 3.3**

### 属性 5: 精确字节读取

*对于任何*指定的字节数 N（在合理范围内），readExactly 方法应该返回恰好 N 个字节的数据，或者在流结束时返回 null。

**验证: 需求 3.4**

### 属性 6: 缓冲区拼接一致性

*对于任何*跨多个 chunk 的数据读取，缓冲区管理应该正确拼接数据，使得最终读取的数据与原始发送的数据完全一致。

**验证: 需求 3.5**

### 属性 7: 延迟计算正确性

*对于任何*接收到的视频分片，计算的延迟（receive_time_ms - send_time_ms）应该是非负数，并且在合理范围内（例如 0-5000ms）。

**验证: 需求 3.8**

### 属性 8: VideoPlayer 功能保持

*对于任何*现有的 VideoPlayer 使用场景（MP4 文件回放），合并后的系统应该产生与合并前相同的行为和结果。

**验证: 需求 5.1**

### 属性 9: 录像回放功能保持

*对于任何*现有的录像回放流程（选择设备 → 选择录像 → 播放），合并后的系统应该产生与合并前相同的行为和结果。

**验证: 需求 5.2**

### 属性 10: 设备和录像列表功能保持

*对于任何*设备列表和录像列表的操作（加载、显示、选择），合并后的系统应该产生与合并前相同的行为和结果。

**验证: 需求 5.3**

## 错误处理

### 1. 浏览器兼容性错误

**场景**: 浏览器不支持 WebTransport 或 WebCodecs

**处理策略**:
- 在 WebTransportPlayer 初始化时检查 `WebTransportClient.isSupported()` 和 `'VideoDecoder' in window`
- 显示友好的错误消息，建议用户使用 Chrome 97+ 或 Edge 97+
- 不尝试初始化播放器，避免运行时错误

**用户体验**:
```
❌ 浏览器不支持 WebTransport API (需要 Chrome 97+ 或 Edge 97+)
```

### 2. WebTransport 连接错误

**场景**: 无法建立 WebTransport 连接（网络问题、服务器不可用、证书问题）

**处理策略**:
- 在 `connect()` 方法中捕获异常
- 调用 `errorCallback` 通知播放器组件
- 更新播放器状态为"连接失败"
- 记录详细错误日志到控制台

**用户体验**:
```
❌ 连接失败: [错误详情]
```

### 3. 视频解码错误

**场景**: VideoDecoder 无法解码视频数据（格式不匹配、数据损坏）

**处理策略**:
- 在 VideoDecoder 的 `error` 回调中捕获错误
- 更新播放器状态显示解码错误
- 记录错误信息，包括 chunk 类型和时间戳
- 继续尝试解码后续帧（不中断整个播放）

**用户体验**:
```
❌ 解码错误: [错误详情]
```

### 4. 协议解析错误

**场景**: 接收到的数据不符合协议格式（头部长度不足、data_length 异常）

**处理策略**:
- 在 `handleVideoStream()` 中进行合理性检查
- 检查 data_length 是否超过限制（10MB）
- 如果解析失败，记录错误并终止流处理
- 调用 `errorCallback` 通知播放器

**错误检查**:
```typescript
if (data_length > 10 * 1024 * 1024) {
  console.error(`Invalid data length: ${data_length}`)
  break
}
```

### 5. 资源清理错误

**场景**: 组件卸载时清理资源失败

**处理策略**:
- 在 `cleanup()` 方法中使用 try-catch 包裹所有清理操作
- 即使某个资源清理失败，也继续清理其他资源
- 记录清理错误到控制台（警告级别）
- 确保不抛出异常到 React 组件

**实现**:
```typescript
const cleanup = () => {
  if (clientRef.current) {
    try {
      clientRef.current.close()
    } catch (e) {
      console.warn('Failed to close client:', e)
    }
    clientRef.current = null
  }
  
  if (decoderRef.current) {
    try {
      decoderRef.current.close()
    } catch (e) {
      console.warn('Failed to close decoder:', e)
    }
    decoderRef.current = null
  }
}
```

### 6. 配置加载错误

**场景**: 配置模块无法加载配置（环境变量格式错误）

**处理策略**:
- 使用默认配置作为回退
- 记录警告到控制台
- 确保应用仍然可以启动

**实现**:
```typescript
export const config: AppConfig = typeof __APP_CONFIG__ !== 'undefined' 
  ? __APP_CONFIG__ 
  : {
      // 默认配置（开发环境回退）
      httpApiUrl: 'http://localhost:8080',
      webtransportEnabled: true,
      webtransportUrl: 'https://localhost:8081',
    }
```

### 7. 控制命令错误

**场景**: 发送控制命令失败（连接已断开、服务器不响应）

**处理策略**:
- 在控制方法（pause、resume、seek 等）中捕获异常
- 记录错误到控制台
- 不更新播放器状态（保持当前状态）
- 可选：显示临时错误提示

**实现**:
```typescript
const handlePause = async () => {
  if (clientRef.current) {
    try {
      await clientRef.current.pause()
      setIsPlaying(false)
    } catch (err) {
      console.error('Pause failed:', err)
      // 不更新状态，保持当前播放状态
    }
  }
}
```

## 测试策略

### 单元测试

单元测试用于验证具体的例子和边缘情况：

**配置模块测试** (`config.test.ts`):
- 测试默认配置加载（边缘情况）
- 测试配置常量导出（例子）
- 测试配置打印到控制台（例子）

**WebTransportClient 测试** (`webtransport.test.ts`):
- 测试浏览器支持检查（例子）
- 测试证书哈希转换（例子）
- 测试 UUID 字节转字符串（例子）
- 测试连接断开时的错误回调（边缘情况）

**WebTransportPlayer 测试** (`WebTransportPlayer.test.tsx`):
- 测试浏览器不支持时的错误显示（边缘情况）
- 测试连接建立后的状态显示（例子）
- 测试组件卸载时的资源清理（例子）

**App 组件测试** (`App.test.tsx`):
- 测试直通播放模式使用 WebTransportPlayer（例子）
- 测试录像回放模式使用 VideoPlayer（例子）
- 测试属性传递（例子）
- 测试返回时的资源清理（例子）
- 测试 WebTransport 禁用时的回退（例子）

**向后兼容性测试**:
- 测试现有 VideoPlayer 功能（回归测试）
- 测试现有 WebCodecsPlayer 功能（回归测试）
- 测试设备列表和录像列表功能（回归测试）

### 属性测试

属性测试用于验证通用属性在所有输入下都成立：

**测试框架**: 使用 `fast-check` 库进行属性测试（JavaScript/TypeScript 的属性测试库）

**配置**: 每个属性测试运行至少 100 次迭代

**标注格式**: 每个属性测试必须使用注释标注对应的设计文档属性
```typescript
// Feature: web-frontend-merge, Property 1: 环境变量配置注入
```

**属性测试用例**:

1. **属性 1: 环境变量配置注入** (`config.property.test.ts`)
   - 生成随机的配置对象
   - 模拟环境变量注入
   - 验证配置模块导出的值与注入的值一致

2. **属性 2: 统计信息更新** (`WebTransportPlayer.property.test.tsx`)
   - 生成随机的视频分片序列
   - 模拟分片接收
   - 验证统计信息单调递增且在合理范围内

3. **属性 3: 播放控制响应** (`webtransport.property.test.ts`)
   - 生成随机的控制命令序列
   - 验证每个命令都被正确编码和发送
   - 验证播放器状态正确更新

4. **属性 4: 协议头部解析** (`webtransport.property.test.ts`)
   - 生成随机的分片元数据
   - 编码为45字节头部
   - 解析并验证结果与原始数据一致

5. **属性 5: 精确字节读取** (`webtransport.property.test.ts`)
   - 生成随机长度的数据流
   - 生成随机的读取长度序列
   - 验证 readExactly 返回正确数量的字节

6. **属性 6: 缓冲区拼接一致性** (`webtransport.property.test.ts`)
   - 生成随机的数据块序列（模拟分片传输）
   - 使用 readExactly 读取跨块数据
   - 验证读取结果与原始数据一致

7. **属性 7: 延迟计算正确性** (`webtransport.property.test.ts`)
   - 生成随机的发送时间戳
   - 模拟接收并计算延迟
   - 验证延迟为非负数且在合理范围内

8. **属性 8-10: 向后兼容性** (`compatibility.property.test.tsx`)
   - 生成随机的用户操作序列
   - 在合并前后的代码上执行相同操作
   - 验证结果一致（使用快照测试或行为比较）

### 集成测试

集成测试验证组件之间的交互：

**端到端流程测试**:
1. 启动应用 → 选择设备 → 启动直通播放 → 验证 WebTransportPlayer 渲染
2. 启动应用 → 选择设备 → 选择录像 → 验证 VideoPlayer 渲染
3. 直通播放 → 接收视频流 → 验证统计信息更新
4. 直通播放 → 发送控制命令 → 验证播放状态变化
5. 播放中 → 返回上一页 → 验证资源清理

**浏览器兼容性测试**:
- 在支持的浏览器中测试（Chrome 97+, Edge 97+）
- 在不支持的浏览器中测试错误提示

### 测试覆盖率目标

- 单元测试覆盖率: > 80%
- 属性测试覆盖核心逻辑: 100%
- 集成测试覆盖主要用户流程: 100%

### 测试执行

```bash
# 运行所有测试
npm test

# 运行单元测试
npm test -- --testPathPattern=".test.ts"

# 运行属性测试
npm test -- --testPathPattern=".property.test.ts"

# 运行集成测试
npm test -- --testPathPattern=".integration.test.ts"

# 生成覆盖率报告
npm test -- --coverage
```

## 性能考虑

### 1. 延迟优化

**目标**: 端到端延迟 < 50ms

**优化策略**:
- 使用 WebTransport (QUIC) 减少连接建立时间
- 使用 WebCodecs 硬件加速解码
- 配置 VideoDecoder 为 `optimizeForLatency: true`
- 最小化缓冲区大小

**监控指标**:
- 发送时间戳（服务器）
- 接收时间戳（客户端）
- 解码队列大小
- 渲染延迟

### 2. 内存管理

**策略**:
- 及时关闭 VideoFrame（调用 `frame.close()`）
- 限制读取缓冲区大小（防止内存泄漏）
- 组件卸载时清理所有资源
- 限制延迟样本数量（最近30个）

**内存限制**:
- 单次读取缓冲区: < 100MB
- 单个分片数据: < 10MB
- 延迟样本数组: 30个

### 3. 渲染性能

**优化**:
- 使用 Canvas 渲染（比 DOM 更高效）
- 只在帧尺寸变化时调整 Canvas 大小
- 使用 requestAnimationFrame 进行平滑渲染（由 VideoDecoder 自动处理）

**FPS 监控**:
- 每秒更新一次 FPS 统计
- 避免频繁的状态更新

### 4. 网络性能

**优化**:
- 使用 WebTransport 的多路复用能力
- 避免不必要的控制命令发送
- 复用 WebTransport 连接

## 部署考虑

### 1. 构建配置

**Vite 配置更新** (`vite.config.ts`):
```typescript
export default defineConfig({
  define: {
    __APP_CONFIG__: JSON.stringify({
      httpApiUrl: process.env.VITE_HTTP_API_URL || 'http://localhost:8080',
      webtransportEnabled: process.env.VITE_WEBTRANSPORT_ENABLED !== 'false',
      webtransportUrl: process.env.VITE_WEBTRANSPORT_URL || 'https://localhost:8081',
    })
  }
})
```

**环境变量**:
- `VITE_HTTP_API_URL`: HTTP API 地址
- `VITE_WEBTRANSPORT_ENABLED`: WebTransport 功能开关
- `VITE_WEBTRANSPORT_URL`: WebTransport 服务器地址

### 2. 证书配置

**开发环境**:
- 使用自签名证书
- 配置 CERT_HASH 用于证书哈希验证
- 或者使用 Chrome Developer Mode（推荐）

**生产环境**:
- 使用有效的 TLS 证书
- 不需要证书哈希验证
- 确保证书包含 WebTransport 所需的扩展

### 3. 浏览器要求

**最低版本**:
- Chrome 97+ (WebTransport)
- Chrome 94+ (WebCodecs)
- Edge 97+ (WebTransport)
- Edge 94+ (WebCodecs)

**功能检测**:
- 在应用启动时检查浏览器支持
- 显示友好的升级提示

### 4. 回退策略

**WebTransport 不可用时**:
- 通过配置开关禁用 WebTransport
- 回退到现有的 VideoPlayer（SSE + WebCodecs）
- 保持基本播放功能可用

## 安全考虑

### 1. 证书验证

**开发环境**:
- 使用证书哈希绕过验证（仅用于开发）
- 明确标注 CERT_HASH 仅用于开发环境

**生产环境**:
- 使用标准的 TLS 证书验证
- 不使用证书哈希

### 2. 输入验证

**协议解析**:
- 验证 data_length 在合理范围内（< 10MB）
- 验证时间戳和延迟在合理范围内
- 防止缓冲区溢出

**控制命令**:
- 验证 position 和 rate 参数
- 防止注入攻击

### 3. 资源限制

**内存限制**:
- 限制读取缓冲区大小
- 限制单个分片大小
- 防止内存耗尽攻击

**连接限制**:
- 每个会话只建立一个 WebTransport 连接
- 及时关闭不使用的连接

## 迁移路径

### 阶段 1: 文件复制（低风险）

1. 复制 `src/config.ts`
2. 复制 `src/components/WebTransportPlayer.tsx`
3. 复制 `src/services/webtransport.ts`
4. 验证文件编译无错误

**验证**:
- 运行 `npm run build`
- 确保没有编译错误

### 阶段 2: App 组件集成（中风险）

1. 更新 `src/App.tsx`
2. 导入 WebTransportPlayer 和 config
3. 添加直通播放模式的条件渲染
4. 验证现有功能不受影响

**验证**:
- 测试录像回放功能
- 测试设备列表和录像列表
- 测试直通播放（使用 WebTransportPlayer）

### 阶段 3: 测试和优化（低风险）

1. 编写单元测试
2. 编写属性测试
3. 编写集成测试
4. 性能测试和优化

**验证**:
- 运行所有测试
- 测试覆盖率 > 80%
- 延迟 < 50ms

### 回滚计划

如果出现问题，可以快速回滚：

1. **阶段 1 回滚**: 删除新增的3个文件
2. **阶段 2 回滚**: 恢复 App.tsx 到原始版本
3. **阶段 3 回滚**: 删除测试文件（不影响功能）

## 依赖关系

### 新增依赖

**开发依赖**:
- `fast-check`: 属性测试库
- `@testing-library/react`: React 组件测试
- `@testing-library/jest-dom`: Jest DOM 匹配器

**安装命令**:
```bash
npm install --save-dev fast-check @testing-library/react @testing-library/jest-dom
```

### 现有依赖

- React 18+
- TypeScript 5+
- Vite 4+

### 浏览器 API 依赖

- WebTransport API (Chrome 97+, Edge 97+)
- WebCodecs API (Chrome 94+, Edge 94+)
- Canvas API (所有现代浏览器)

## 文档更新

### 需要更新的文档

1. **README.md**: 添加 WebTransport 功能说明
2. **API 文档**: 添加 WebTransport 端点说明
3. **部署指南**: 添加环境变量配置说明
4. **浏览器兼容性**: 更新浏览器要求

### 新增文档

1. **WebTransport 使用指南**: 如何配置和使用 WebTransport 播放器
2. **故障排查指南**: 常见问题和解决方案
3. **性能优化指南**: 如何优化延迟和性能

## 未来改进

### 短期改进

1. **控制命令编码**: 实现完整的 bincode 编码/解码（目前使用 JSON）
2. **错误恢复**: 实现自动重连机制
3. **性能监控**: 添加更详细的性能指标

### 长期改进

1. **自适应码率**: 根据网络状况调整视频质量
2. **多流支持**: 同时播放多个视频流
3. **录制功能**: 支持录制 WebTransport 流
4. **回放控制**: 支持更精确的定位和倍速播放

## 总结

本设计文档描述了将 web-frontend2 的新功能合并到 web-frontend 的完整方案。通过增量式合并、严格的测试策略和向后兼容性保证，我们可以安全地引入 WebTransport 超低延迟播放功能，同时保持现有功能的稳定性。

**关键成功因素**:
1. 分阶段实施，降低风险
2. 完整的测试覆盖（单元测试 + 属性测试 + 集成测试）
3. 浏览器兼容性检查和友好的错误提示
4. 及时的资源清理和错误处理
5. 详细的文档和故障排查指南
