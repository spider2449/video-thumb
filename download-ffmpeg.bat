@echo off
echo Downloading ffmpeg...

set "FFMPEG_DIR=%~dp0ffmpeg"
mkdir "%FFMPEG_DIR%" 2>nul

if exist "%FFMPEG_DIR%\ffmpeg.exe" (
    echo ffmpeg.exe already exists!
    pause
    exit /b
)

echo.
echo Please download ffmpeg from: https://www.gyan.dev/ffmpeg/builds/
echo.
echo 1. Download "ffmpeg-release-essentials.zip"
echo 2. Extract the zip file
echo 3. Copy ffmpeg.exe to: %FFMPEG_DIR%
echo.
echo Or run this command with winget (if available):
echo   winget install ffmpeg
echo.
pause