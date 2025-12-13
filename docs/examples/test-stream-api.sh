#!/bin/bash

# 统一低延迟视频流传输系统 - API测试脚本
# 
# 使用方法:
#   ./test-stream-api.sh [mode] [source_id]
#
# 示例:
#   ./test-stream-api.sh live device_001
#   ./test-stream-api.sh playback rec_001

set -e

# 配置
BASE_URL="https://localhost:8443/api/v1"
MODE="${1:-playback}"
SOURCE_ID="${2:-rec_001}"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."
    
    if ! command -v curl &> /dev/null; then
        log_error "curl 未安装"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        log_warn "jq 未安装，JSON输出将不会格式化"
    fi
    
    log_success "依赖检查完成"
}

# 启动流会话
start_stream() {
    log_info "启动流会话 (模式: $MODE, 源: $SOURCE_ID)..."
    
    if [ "$MODE" = "live" ]; then
        REQUEST_BODY=$(cat <<EOF
{
  "mode": "live",
  "source": {
    "device_id": "$SOURCE_ID"
  },
  "config": {
    "client_id": "test_client_001",
    "low_latency_mode": true,
    "target_latency_ms": 50
  }
}
EOF
)
    else
        REQUEST_BODY=$(cat <<EOF
{
  "mode": "playback",
  "source": {
    "file_id": "$SOURCE_ID",
    "start_position": 0.0,
    "playback_rate": 1.0
  },
  "config": {
    "client_id": "test_client_001",
    "low_latency_mode": true,
    "target_latency_ms": 100
  }
}
EOF
)
    fi
    
    RESPONSE=$(curl -s -k -X POST "$BASE_URL/stream/start" \
        -H "Content-Type: application/json" \
        -d "$REQUEST_BODY")
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    SESSION_ID=$(echo "$RESPONSE" | grep -o '"session_id":"[^"]*"' | cut -d'"' -f4)
    
    if [ -z "$SESSION_ID" ]; then
        log_error "启动流会话失败"
        exit 1
    fi
    
    log_success "流会话已创建: $SESSION_ID"
    echo "$SESSION_ID"
}

# 查询流状态
get_status() {
    local session_id=$1
    log_info "查询流状态..."
    
    RESPONSE=$(curl -s -k "$BASE_URL/stream/$session_id/status")
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "状态查询完成"
}

# 暂停流
pause_stream() {
    local session_id=$1
    log_info "暂停流..."
    
    RESPONSE=$(curl -s -k -X POST "$BASE_URL/stream/$session_id/control" \
        -H "Content-Type: application/json" \
        -d '{"command": "pause"}')
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "流已暂停"
}

# 恢复流
resume_stream() {
    local session_id=$1
    log_info "恢复流..."
    
    RESPONSE=$(curl -s -k -X POST "$BASE_URL/stream/$session_id/control" \
        -H "Content-Type: application/json" \
        -d '{"command": "resume"}')
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "流已恢复"
}

# 定位（仅回放）
seek_stream() {
    local session_id=$1
    local position=$2
    log_info "定位到 ${position}s..."
    
    RESPONSE=$(curl -s -k -X POST "$BASE_URL/stream/$session_id/control" \
        -H "Content-Type: application/json" \
        -d "{\"command\": \"seek\", \"position\": $position}")
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "定位完成"
}

# 设置倍速（仅回放）
set_rate() {
    local session_id=$1
    local rate=$2
    log_info "设置播放速率为 ${rate}x..."
    
    RESPONSE=$(curl -s -k -X POST "$BASE_URL/stream/$session_id/control" \
        -H "Content-Type: application/json" \
        -d "{\"command\": \"set_rate\", \"rate\": $rate}")
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "播放速率已设置"
}

# 停止流
stop_stream() {
    local session_id=$1
    log_info "停止流..."
    
    RESPONSE=$(curl -s -k -X DELETE "$BASE_URL/stream/$session_id")
    
    if command -v jq &> /dev/null; then
        echo "$RESPONSE" | jq .
    else
        echo "$RESPONSE"
    fi
    
    log_success "流已停止"
}

# 主测试流程
main() {
    echo "========================================="
    echo "统一低延迟视频流传输系统 - API测试"
    echo "========================================="
    echo ""
    
    check_dependencies
    echo ""
    
    # 1. 启动流
    SESSION_ID=$(start_stream)
    echo ""
    
    # 2. 查询状态
    sleep 2
    get_status "$SESSION_ID"
    echo ""
    
    # 3. 暂停
    sleep 2
    pause_stream "$SESSION_ID"
    echo ""
    
    # 4. 恢复
    sleep 2
    resume_stream "$SESSION_ID"
    echo ""
    
    # 5. 回放模式特有功能
    if [ "$MODE" = "playback" ]; then
        # 定位
        sleep 2
        seek_stream "$SESSION_ID" 30.0
        echo ""
        
        # 倍速
        sleep 2
        set_rate "$SESSION_ID" 2.0
        echo ""
    fi
    
    # 6. 查询最终状态
    sleep 2
    get_status "$SESSION_ID"
    echo ""
    
    # 7. 停止流
    sleep 2
    stop_stream "$SESSION_ID"
    echo ""
    
    echo "========================================="
    log_success "测试完成！"
    echo "========================================="
}

# 运行主流程
main
