#!/bin/bash
# Setup script permissions for Linux/macOS

echo "Setting execute permissions for shell scripts..."

chmod +x start-debug.sh
chmod +x start-release.sh
chmod +x start-device.sh
chmod +x stop-all.sh

echo "Done! All shell scripts are now executable."
echo ""
echo "You can now run:"
echo "  ./start-debug.sh    - Start all services (Debug mode)"
echo "  ./start-release.sh  - Start all services (Release mode)"
echo "  ./start-device.sh   - Start a device simulator"
echo "  ./stop-all.sh       - Stop all services"
echo ""
