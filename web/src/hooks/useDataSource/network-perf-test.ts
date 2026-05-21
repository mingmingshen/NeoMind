/**
 * 网络请求性能测试
 * 添加到 useDataSource.ts 中测试实际网络耗时
 */

export async function testNetworkPerformance() {
  const api = (await import('@/lib/api')).api

  console.log('🔍 [网络性能测试] 开始测试...')

  // 测试1: 单个查询的网络时间
  const deviceIds = ['TH_b597b3db', 'TH_8c71c65e', 'TH_8f072f7d']
  const metrics = ['battery', 'temperature', 'humidity', 'devName']

  for (const deviceId of deviceIds) {
    for (const metric of metrics) {
      const start = performance.now()
      const response = await api.getDeviceTelemetry(
        deviceId,
        metric,
        Math.floor(Date.now() / 1000) - 3600, // 1小时前
        Math.floor(Date.now() / 1000),
        50
      )
      const end = performance.now()

      console.log(`📊 [网络] ${deviceId}.${metric}: ${(end - start).toFixed(1)}ms`)
    }
  }

  console.log('✅ [网络性能测试] 完成')
}
