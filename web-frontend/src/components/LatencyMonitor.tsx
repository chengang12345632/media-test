// 延迟监控显示组件
import React, { useEffect, useState, useCallback } from 'react';
import './LatencyMonitor.css';

interface LatencyStatistics {
  session_id: string;
  total_segments: number;
  total_bytes: number;
  average_latency_ms: number;
  current_latency_ms: number;
  min_latency_ms: number;
  max_latency_ms: number;
  p50_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  throughput_mbps: number;
  packet_loss_rate: number;
}

interface LatencyAlert {
  type: 'LatencyAlert';
  session_id: string;
  alert: {
    TransmissionLatency?: { segment_id: string; latency_ms: number; threshold_ms: number };
    ProcessingLatency?: { segment_id: string; latency_ms: number; threshold_ms: number };
    DistributionLatency?: { segment_id: string; latency_ms: number; threshold_ms: number };
    EndToEndLatency?: { segment_id: string; latency_ms: number; threshold_ms: number };
  };
  timestamp: number;
}

interface StatisticsUpdate {
  type: 'StatisticsUpdate';
  session_id: string;
  statistics: LatencyStatistics;
  timestamp: number;
}

type AlertMessage = LatencyAlert | StatisticsUpdate;

interface LatencyMonitorProps {
  sessionId?: string;
  apiBaseUrl?: string;
}

export const LatencyMonitor: React.FC<LatencyMonitorProps> = ({
  sessionId,
  apiBaseUrl = 'http://localhost:8443',
}) => {
  const [statistics, setStatistics] = useState<LatencyStatistics | null>(null);
  const [alerts, setAlerts] = useState<string[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 格式化延迟值
  const formatLatency = (ms: number): string => {
    if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
    if (ms < 1000) return `${ms.toFixed(1)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  // 格式化吞吐量
  const formatThroughput = (mbps: number): string => {
    if (mbps < 1) return `${(mbps * 1000).toFixed(0)} Kbps`;
    return `${mbps.toFixed(2)} Mbps`;
  };

  // 格式化丢包率
  const formatPacketLoss = (rate: number): string => {
    return `${(rate * 100).toFixed(2)}%`;
  };

  // 获取延迟等级（用于颜色显示）
  const getLatencyLevel = (latency: number): 'excellent' | 'good' | 'fair' | 'poor' => {
    if (latency < 50) return 'excellent';
    if (latency < 100) return 'good';
    if (latency < 200) return 'fair';
    return 'poor';
  };

  // 添加告警
  const addAlert = useCallback((message: string) => {
    setAlerts((prev) => {
      const newAlerts = [message, ...prev].slice(0, 10); // 只保留最近10条
      return newAlerts;
    });
  }, []);

  // 订阅延迟告警
  useEffect(() => {
    const alertUrl = sessionId
      ? `${apiBaseUrl}/api/v1/latency/sessions/${sessionId}/alerts`
      : `${apiBaseUrl}/api/v1/latency/alerts`;

    console.log('Connecting to latency alerts:', alertUrl);
    const eventSource = new EventSource(alertUrl);

    eventSource.onopen = () => {
      console.log('Connected to latency alerts');
      setConnected(true);
      setError(null);
    };

    eventSource.onmessage = (event) => {
      try {
        const message: AlertMessage = JSON.parse(event.data);
        console.log('Received alert message:', message);

        if (message.type === 'StatisticsUpdate') {
          setStatistics(message.statistics);
        } else if (message.type === 'LatencyAlert') {
          const alert = message.alert;
          let alertText = '';

          if ('TransmissionLatency' in alert) {
            const a = alert.TransmissionLatency!;
            alertText = `传输延迟告警: ${a.latency_ms}ms (阈值: ${a.threshold_ms}ms)`;
          } else if ('ProcessingLatency' in alert) {
            const a = alert.ProcessingLatency!;
            alertText = `处理延迟告警: ${a.latency_ms}ms (阈值: ${a.threshold_ms}ms)`;
          } else if ('DistributionLatency' in alert) {
            const a = alert.DistributionLatency!;
            alertText = `分发延迟告警: ${a.latency_ms}ms (阈值: ${a.threshold_ms}ms)`;
          } else if ('EndToEndLatency' in alert) {
            const a = alert.EndToEndLatency!;
            alertText = `端到端延迟告警: ${a.latency_ms}ms (阈值: ${a.threshold_ms}ms)`;
          }

          if (alertText) {
            addAlert(alertText);
          }
        }
      } catch (err) {
        console.error('Failed to parse alert message:', err);
      }
    };

    eventSource.onerror = (err) => {
      console.error('SSE connection error:', err);
      setConnected(false);
      setError('连接失败，正在重试...');
    };

    return () => {
      console.log('Closing latency alerts connection');
      eventSource.close();
      setConnected(false);
    };
  }, [sessionId, apiBaseUrl, addAlert]);

  if (!statistics && !error) {
    return (
      <div className="latency-monitor loading">
        <div className="loading-spinner"></div>
        <p>等待延迟数据...</p>
      </div>
    );
  }

  return (
    <div className="latency-monitor">
      <div className="monitor-header">
        <h3>📊 延迟监控</h3>
        <div className={`connection-status ${connected ? 'connected' : 'disconnected'}`}>
          {connected ? '● 已连接' : '○ 未连接'}
        </div>
      </div>

      {error && (
        <div className="error-message">
          ⚠️ {error}
        </div>
      )}

      {statistics && (
        <>
          {/* 主要延迟指标 */}
          <div className="metrics-grid">
            <div className={`metric-card ${getLatencyLevel(statistics.average_latency_ms)}`}>
              <div className="metric-label">平均延迟</div>
              <div className="metric-value">{formatLatency(statistics.average_latency_ms)}</div>
            </div>

            <div className={`metric-card ${getLatencyLevel(statistics.current_latency_ms)}`}>
              <div className="metric-label">当前延迟</div>
              <div className="metric-value">{formatLatency(statistics.current_latency_ms)}</div>
            </div>

            <div className="metric-card">
              <div className="metric-label">吞吐量</div>
              <div className="metric-value">{formatThroughput(statistics.throughput_mbps)}</div>
            </div>

            <div className="metric-card">
              <div className="metric-label">丢包率</div>
              <div className="metric-value">{formatPacketLoss(statistics.packet_loss_rate)}</div>
            </div>
          </div>

          {/* 详细统计 */}
          <div className="details-section">
            <h4>详细统计</h4>
            <div className="details-grid">
              <div className="detail-item">
                <span className="detail-label">最小延迟:</span>
                <span className="detail-value">{formatLatency(statistics.min_latency_ms)}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">最大延迟:</span>
                <span className="detail-value">{formatLatency(statistics.max_latency_ms)}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">P50延迟:</span>
                <span className="detail-value">{formatLatency(statistics.p50_latency_ms)}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">P95延迟:</span>
                <span className="detail-value">{formatLatency(statistics.p95_latency_ms)}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">P99延迟:</span>
                <span className="detail-value">{formatLatency(statistics.p99_latency_ms)}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">总分片数:</span>
                <span className="detail-value">{statistics.total_segments.toLocaleString()}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">总字节数:</span>
                <span className="detail-value">
                  {(statistics.total_bytes / 1024 / 1024).toFixed(2)} MB
                </span>
              </div>
            </div>
          </div>

          {/* 告警列表 */}
          {alerts.length > 0 && (
            <div className="alerts-section">
              <h4>⚠️ 延迟告警</h4>
              <div className="alerts-list">
                {alerts.map((alert, index) => (
                  <div key={index} className="alert-item">
                    {alert}
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default LatencyMonitor;
