#!/bin/bash
# Debug Mode Startup Script

echo "========================================"
echo "Starting All Services (Debug Mode)"
echo "========================================"
echo ""

# Stop existing processes
echo "[1/4] Checking and stopping existing processes..."
STOPPED=false

if [ -f ".process-ids.json" ]; then
    PLATFORM_PID=$(jq -r '.Platform // empty' .process-ids.json 2>/dev/null)
    DEVICE_PID=$(jq -r '.Device // empty' .process-ids.json 2>/dev/null)
    FRONTEND_PID=$(jq -r '.Frontend // empty' .process-ids.json 2>/dev/null)
    
    if [ ! -z "$PLATFORM_PID" ] && kill -0 "$PLATFORM_PID" 2>/dev/null; then
        echo "  Stopping Platform Server (PID: $PLATFORM_PID)..."
        kill -9 "$PLATFORM_PID" 2>/dev/null
        STOPPED=true
    fi
    
    if [ ! -z "$DEVICE_PID" ] && kill -0 "$DEVICE_PID" 2>/dev/null; then
        echo "  Stopping Device Simulator (PID: $DEVICE_PID)..."
        kill -9 "$DEVICE_PID" 2>/dev/null
        STOPPED=true
    fi
    
    if [ ! -z "$FRONTEND_PID" ] && kill -0 "$FRONTEND_PID" 2>/dev/null; then
        echo "  Stopping Frontend (PID: $FRONTEND_PID)..."
        kill -9 "$FRONTEND_PID" 2>/dev/null
        STOPPED=true
    fi
fi

# Stop by process name
for PROC in platform-server device-simulator node; do
    PIDS=$(pgrep -f "$PROC" 2>/dev/null)
    if [ ! -z "$PIDS" ]; then
        for PID in $PIDS; do
            echo "  Stopping $PROC (PID: $PID)..."
            kill -9 "$PID" 2>/dev/null
            STOPPED=true
        done
    fi
done

if [ "$STOPPED" = true ]; then
    echo "  Waiting for processes to exit..."
    sleep 3
    echo "  Existing processes stopped"
else
    echo "  No running processes found"
fi
echo ""

# Build projects
if [ "$1" != "--skip-build" ]; then
    echo "[2/4] Building projects (Debug mode)..."
    
    echo "  Building platform-server..."
    cd platform-server
    cargo build > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        echo "  X platform-server build failed"
        exit 1
    fi
    echo "  + platform-server build successful"
    cd ..
    
    echo "  Building device-simulator..."
    cd device-simulator
    cargo build > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        echo "  X device-simulator build failed"
        exit 1
    fi
    echo "  + device-simulator build successful"
    cd ..
    
    echo "  Checking frontend dependencies..."
    cd web-frontend
    if [ ! -d "node_modules" ]; then
        echo "  Installing frontend dependencies..."
        npm install > /dev/null 2>&1
        if [ $? -ne 0 ]; then
            echo "  X frontend dependencies installation failed"
            exit 1
        fi
    fi
    echo "  + frontend dependencies ready"
    cd ..
    echo ""
else
    echo "[2/4] Skipping build step"
    echo ""
fi

# Check executables
echo "[3/4] Checking executables..."
PLATFORM_EXE="target/debug/platform-server"
DEVICE_EXE="target/debug/device-simulator"

if [ ! -f "$PLATFORM_EXE" ]; then
    echo "  X platform-server not found"
    echo "  Please run: ./start-debug.sh"
    exit 1
fi

if [ ! -f "$DEVICE_EXE" ]; then
    echo "  X device-simulator not found"
    echo "  Please run: ./start-debug.sh"
    exit 1
fi

echo "  + All executables ready"
echo ""

# Start services
echo "[4/4] Starting services..."

echo "  Starting Platform Server..."
cd "$(dirname "$0")"
RUST_LOG=info ./target/debug/platform-server > /dev/null 2>&1 &
PLATFORM_PID=$!
echo "  + Platform Server started (PID: $PLATFORM_PID)"
sleep 3

echo "  Starting Device Simulator..."
cd device-simulator
RUST_LOG=info ../target/debug/device-simulator --device-id device_001 --server-addr 127.0.0.1:8443 > /dev/null 2>&1 &
DEVICE_PID=$!
echo "  + Device Simulator started (PID: $DEVICE_PID)"
cd ..
sleep 3

echo "  Starting Frontend..."
cd web-frontend
npm run dev > /dev/null 2>&1 &
FRONTEND_PID=$!
echo "  + Frontend started (PID: $FRONTEND_PID)"
cd ..

# Save process IDs
cat > .process-ids.json << EOF
{
  "Platform": $PLATFORM_PID,
  "Device": $DEVICE_PID,
  "Frontend": $FRONTEND_PID,
  "Mode": "debug",
  "StartTime": "$(date '+%Y-%m-%d %H:%M:%S')"
}
EOF

echo ""
echo "========================================"
echo "All services started!"
echo "========================================"
echo ""
echo "Service Information:"
echo "  Platform Server: http://localhost:8080"
echo "  Frontend:        http://localhost:5173"
echo "  Device ID:       device_001"
echo "  Build Mode:      Debug"
echo ""
echo "Process IDs:"
echo "  Platform: $PLATFORM_PID"
echo "  Device:   $DEVICE_PID"
echo "  Frontend: $FRONTEND_PID"
echo ""
echo "Stop all services:"
echo "  kill $PLATFORM_PID $DEVICE_PID $FRONTEND_PID"
echo ""
echo "Wait 10-20 seconds, then visit: http://localhost:5173"
echo ""
