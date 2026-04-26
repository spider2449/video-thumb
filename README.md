# Video Thumb Viewer

A Rust-based video thumbnail viewer with SQLite storage, tagging, and rating features.

## Features

- Recursive folder scanning for video files
- SQLite database storage for file metadata
- Thumbnail generation with embedded ffmpeg
- Tab-based UI for multiple watched paths
- 5-star rating system
- Custom tags with color coding
- Search and filter by name, rating, tags
- Sort by name, date, or rating
- Right-click context menu for all operations
- Custom screenshot capture at specified timestamps
- Configurable external video player

## Prerequisites

- Rust (latest stable)
- ffmpeg (for thumbnail generation)

## Installation

### 1. Install Rust

```bash
# Windows (using rustup)
winget install Rustlang.Rustup

# Or download from https://rustup.rs
```

### 2. Download ffmpeg

Download ffmpeg and place the executable in the `ffmpeg/` directory:

- Windows: Place `ffmpeg.exe` in `ffmpeg/`
- Linux/Mac: Place `ffmpeg` in `ffmpeg/`

Download from: https://ffmpeg.org/download.html

Or use these quick links:
- Windows: https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip
- Linux: `sudo apt install ffmpeg` or from package manager
- Mac: `brew install ffmpeg`

### 3. Build the project

```bash
cd video-thumb-viewer
cargo build --release
```

The executable will be at `target/release/video-thumb-viewer.exe`

## Usage

1. Run the application
2. Click "+ 新增路徑" to add a folder to watch
3. Click "🔄 更新資料庫" to scan and generate thumbnails
4. Use tabs to switch between watched folders
5. Right-click on thumbnails for options (rating, tags, etc.)
6. Double-click to play video

## Keyboard Shortcuts

- `Escape`: Close context menu

## Data Storage

- Database: `%LOCALAPPDATA%/video-thumb-viewer/data.db`
- Thumbnails: `%LOCALAPPDATA%/video-thumb-viewer/thumbs/`
- Settings: Stored in SQLite database

## Supported Video Formats

mp4, mkv, avi, mov, wmv, flv, webm, m4v, mpg, mpeg, 3gp, ogv, ts, m2ts, vob, rm, rmvb

## Screenshots Directory

Thumbnails are automatically generated and stored in the `thumbs/` directory using MD5 hash of the file path as the filename.

## License

MIT