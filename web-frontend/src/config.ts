/**
 * 前端配置
 * 
 * 从 vite.config.ts 注入的配置
 */

interface AppConfig {
  httpApiUrl: string
  webtransportEnabled: boolean
  webtransportUrl: string
}

// 声明全局变量（由 vite 注入）
declare const __APP_CONFIG__: AppConfig

// 导出配置
export const config: AppConfig = typeof __APP_CONFIG__ !== 'undefined' 
  ? __APP_CONFIG__ 
  : {
      // 默认配置（开发环境回退）
      httpApiUrl: 'http://localhost:8080',
      webtransportEnabled: true,
      webtransportUrl: 'https://localhost:8081',
    }

// 便捷访问
export const HTTP_API_URL = config.httpApiUrl
export const WEBTRANSPORT_ENABLED = config.webtransportEnabled
export const WEBTRANSPORT_URL = config.webtransportUrl

// 证书哈希（用于开发环境绕过证书验证）
// 运行 .\common\certificate\get-cert-hash.ps1 获取最新哈希
export const CERT_HASH = 'JTy7QpWiqIEvrbCUSc0JfngUh7NTcFFc1sMA0ojcRmY='

// 打印配置信息
console.log('🔧 Frontend Configuration:')
console.log('   HTTP API:', HTTP_API_URL)
console.log('   WebTransport:', WEBTRANSPORT_ENABLED ? WEBTRANSPORT_URL : 'disabled')
console.log('   Cert Hash:', CERT_HASH)
