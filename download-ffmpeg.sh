#!/bin/bash

echo "FFmpeg Download Script"
echo "======================"

FFMPEG_DIR="$(dirname "$0")/ffmpeg"
mkdir -p "$FFMPEG_DIR"

if [ -f "$FFMPEG_DIR/ffmpeg" ]; then
    echo "ffmpeg already exists!"
    exit 0
fi

echo ""
echo "Please install ffmpeg:"
echo ""
echo "Ubuntu/Debian:  sudo apt install ffmpeg"
echo "macOS:          brew install ffmpeg"
echo "Arch:           sudo pacman -S ffmpeg"
echo ""
echo "Or download manually from: https://ffmpeg.org/download.html"
echo ""
echo "After installation, the binary should be at: $FFMPEG_DIR/ffmpeg"
echo "(Create a symlink with: sudo ln -s $(which ffmpeg) $FFMPEG_DIR/ffmpeg)"