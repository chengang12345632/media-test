#!/bin/bash
# Start Device Simulator with Random Device Info

DEVICE_ID=""
SERVER_ADDR="127.0.0.1:8443"
BUILD_MODE="debug"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --device-id)
            DEVICE_ID="$2"
            shift 2
            ;;
        --server-addr)
            SERVER_ADDR="$2"
            shift 2
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        *)
            shift
            ;;
    esac
done

echo "========================================"
echo "Starting Device Simulator"
echo "========================================"
echo ""

# Generate random device info
generate_random_device() {
    TYPES=("Camera" "Sensor" "Monitor" "Recorder" "Gateway")
    LOCATIONS=("Office" "Warehouse" "Lobby" "Parking" "Lab" "Factory" "Store")
    
    TYPE=${TYPES[$RANDOM % ${#TYPES[@]}]}
    LOCATION=${LOCATIONS[$RANDOM % ${#LOCATIONS[@]}]}
    NUMBER=$((100 + RANDOM % 900))
    
    echo "device_${TYPE}_${LOCATION}_${NUMBER}" | tr '[:upper:]' '[:lower:]'
}

# Determine device ID
if [ -z "$DEVICE_ID" ]; then
    DEVICE_ID=$(generate_random_device)
    
    echo "Generated random device info:"
    echo "  Device ID:   $DEVICE_ID"
    echo ""
else
    echo "Using specified device ID: $DEVICE_ID"
    echo ""
fi

# Determine executable path
EXE_PATH="target/$BUILD_MODE/device-simulator"

echo "Checking executable..."
if [ ! -f "$EXE_PATH" ]; then
    echo "  X device-simulator not found ($BUILD_MODE)"
    echo ""
    echo "Please build the project first:"
    if [ "$BUILD_MODE" = "release" ]; then
        echo "  cd device-simulator"
        echo "  cargo build --release"
    else
        echo "  cd device-simulator"
        echo "  cargo build"
    fi
    echo ""
    exit 1
fi
echo "  + Executable ready"
echo ""

# Check existing devices
EXISTING_COUNT=$(pgrep -f "device-simulator" | wc -l)
if [ $EXISTING_COUNT -gt 0 ]; then
    echo "Checking existing device processes..."
    echo "  Found $EXISTING_COUNT device simulator(s) running"
    echo ""
    read -p "  Continue to start new device? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "  Cancelled"
        exit 0
    fi
    echo ""
fi

# Start device simulator
echo "Starting device simulator..."
echo "  Device ID:     $DEVICE_ID"
echo "  Server:        $SERVER_ADDR"
echo "  Build Mode:    $BUILD_MODE"
echo ""

cd device-simulator
RUST_LOG=info ../$EXE_PATH --device-id "$DEVICE_ID" --server-addr "$SERVER_ADDR" > /dev/null 2>&1 &
DEVICE_PID=$!
cd ..

echo "========================================"
echo "Device simulator started!"
echo "========================================"
echo ""
echo "Device Information:"
echo "  Device ID:     $DEVICE_ID"
echo "  Server:        $SERVER_ADDR"
echo "  Process ID:    $DEVICE_PID"
echo "  Build Mode:    $BUILD_MODE"
echo ""
echo "Stop device:"
echo "  kill $DEVICE_PID"
echo ""
echo "View device list:"
echo "  curl http://localhost:8080/api/v1/devices"
echo ""

# Save device info
DEVICES_FILE=".device-processes.json"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

if [ -f "$DEVICES_FILE" ]; then
    # Clean up stopped processes and add new one
    jq --arg did "$DEVICE_ID" --arg pid "$DEVICE_PID" --arg addr "$SERVER_ADDR" --arg mode "$BUILD_MODE" --arg time "$TIMESTAMP" \
        '. + [{DeviceId: $did, ProcessId: ($pid|tonumber), ServerAddr: $addr, BuildMode: $mode, StartTime: $time}]' \
        "$DEVICES_FILE" > "${DEVICES_FILE}.tmp" 2>/dev/null && mv "${DEVICES_FILE}.tmp" "$DEVICES_FILE"
else
    cat > "$DEVICES_FILE" << EOF
[
  {
    "DeviceId": "$DEVICE_ID",
    "ProcessId": $DEVICE_PID,
    "ServerAddr": "$SERVER_ADDR",
    "BuildMode": "$BUILD_MODE",
    "StartTime": "$TIMESTAMP"
  }
]
EOF
fi

echo "Device info saved to: $DEVICES_FILE"
echo ""
