#!/bin/bash
# Simple HTTP server for testing the WASM emulator

echo "🎮 Starting RGB Emulator server..."
echo "📡 Server will be available at: http://localhost:8000"
echo "🛑 Press Ctrl+C to stop the server"
echo ""

# Check if Python 3 is available
if command -v python3 &> /dev/null; then
    python3 -m http.server 8000
# Fall back to Python 2
elif command -v python &> /dev/null; then
    python -m SimpleHTTPServer 8000
else
    echo "❌ Error: Python is not installed"
    echo "Please install Python or use another HTTP server"
    exit 1
fi
