#!/bin/bash
# Stop All Services Script

echo "========================================"
echo "Stopping All Services"
echo "========================================"
echo ""

STOPPED=false

# Stop from process ID file
if [ -f ".process-ids.json" ]; then
    echo "Stopping services from process ID file..."
    
    PLATFORM_PID=$(jq -r '.Platform // empty' .process-ids.json 2>/dev/null)
    DEVICE_PID=$(jq -r '.Device // empty' .process-ids.json 2>/dev/null)
    FRONTEND_PID=$(jq -r '.Frontend // empty' .process-ids.json 2>/dev/null)
    
    if [ ! -z "$PLATFORM_PID" ] && kill -0 "$PLATFORM_PID" 2>/dev/null; then
        echo "  Stopping Platform Server (PID: $PLATFORM_PID)..."
        kill -9 "$PLATFORM_PID" 2>/dev/null
        echo "  + Platform Server stopped"
        STOPPED=true
    fi
    
    if [ ! -z "$DEVICE_PID" ] && kill -0 "$DEVICE_PID" 2>/dev/null; then
        echo "  Stopping Device Simulator (PID: $DEVICE_PID)..."
        kill -9 "$DEVICE_PID" 2>/dev/null
        echo "  + Device Simulator stopped"
        STOPPED=true
    fi
    
    if [ ! -z "$FRONTEND_PID" ] && kill -0 "$FRONTEND_PID" 2>/dev/null; then
        echo "  Stopping Frontend (PID: $FRONTEND_PID)..."
        kill -9 "$FRONTEND_PID" 2>/dev/null
        echo "  + Frontend stopped"
        STOPPED=true
    fi
    
    rm -f .process-ids.json
    echo ""
fi

# Stop by process name
echo "Stopping services by process name..."

PLATFORM_PIDS=$(pgrep -f "platform-server" 2>/dev/null)
if [ ! -z "$PLATFORM_PIDS" ]; then
    for PID in $PLATFORM_PIDS; do
        echo "  Stopping Platform Server (PID: $PID)..."
        kill -9 "$PID" 2>/dev/null
        STOPPED=true
    done
    echo "  + Platform Server stopped"
fi

DEVICE_PIDS=$(pgrep -f "device-simulator" 2>/dev/null)
if [ ! -z "$DEVICE_PIDS" ]; then
    for PID in $DEVICE_PIDS; do
        echo "  Stopping Device Simulator (PID: $PID)..."
        kill -9 "$PID" 2>/dev/null
        STOPPED=true
    done
    echo "  + Device Simulator stopped"
fi

NODE_PIDS=$(pgrep -f "npm run dev" 2>/dev/null)
if [ ! -z "$NODE_PIDS" ]; then
    for PID in $NODE_PIDS; do
        echo "  Stopping Frontend (PID: $PID)..."
        kill -9 "$PID" 2>/dev/null
        STOPPED=true
    done
    echo "  + Frontend (Node.js) stopped"
fi

echo ""

# Clean up device processes record
if [ -f ".device-processes.json" ]; then
    echo "Cleaning device processes record..."
    rm -f .device-processes.json
    echo "  + Device processes record cleaned"
    echo ""
fi

if [ "$STOPPED" = true ]; then
    echo "========================================"
    echo "All services stopped!"
    echo "========================================"
else
    echo "========================================"
    echo "No running services found"
    echo "========================================"
fi

echo ""
