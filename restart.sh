#!/bin/bash

echo "🔨 Building corna..."

# Build the project and capture output
if cargo build --quiet 2> build_errors.txt; then
    echo "✅ Build successful!"

    # Kill any existing corna process
    if pgrep -f "corna" > /dev/null; then
        echo "🔥 Killing existing corna process..."
        pkill -f corna
        sleep 0.5  # Give it a moment to die
    fi

    # Start new process in background
    echo "🚀 Starting corna in background..."
    ./target/debug/corna > corna.log 2>&1 &
    NEW_PID=$!

    echo "✨ corna started with PID $NEW_PID"
    echo "📋 Logs: tail -f corna.log"

else
    echo "❌ Build failed!"
    echo "📝 Build errors:"
    cat build_errors.txt
    exit 1
fi