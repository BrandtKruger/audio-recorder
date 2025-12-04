# Audio Recorder & Transcriber

A Rust application for transcribing audio files (especially meeting recordings) to text files for meeting minutes.

[![GitHub](https://img.shields.io/github/license/BrandtKruger/audio-recorder)](https://github.com/BrandtKruger/audio-recorder)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

## Features

- 🎙️ Transcribes audio files to text
- 🎤 **Live recording from microphone with real-time transcription**
- 👥 **Speaker diarization** - Identify and label different speakers (optional)
- 📝 Saves transcriptions with timestamps
- 🎵 Supports multiple audio formats (WAV, MP3, M4A, FLAC, etc.)
- 🌍 Auto-detects language or specify manually
- ⚡ Uses OpenAI Whisper for accurate transcription

## Prerequisites

1. **Install Rust**: If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/)
   ```bash
   # This will install rustc, cargo, and other tools
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install CMake** (required for building whisper-rs):
   ```bash
   # macOS
   brew install cmake
   
   # Windows
   # Download and install from https://cmake.org/download/
   # Or use Chocolatey: choco install cmake
   # Or use winget: winget install Kitware.CMake
   
   # Linux (Ubuntu/Debian)
   sudo apt-get install cmake
   
   # Or download from https://cmake.org/download/
   ```

3. **Download a Whisper model**: You need to download a Whisper model file. Recommended models:
   - `ggml-base.en.bin` - Base English model (good balance of speed/accuracy)
   - `ggml-small.en.bin` - Small English model (faster, less accurate)
   - `ggml-medium.en.bin` - Medium English model (slower, more accurate)

   Download from: https://huggingface.co/ggerganov/whisper.cpp/tree/main

4. **Create a models directory and download the model**:
   ```bash
   # macOS/Linux (using curl)
   mkdir -p models
   cd models
   curl -L -o ggml-base.en.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```
   
   ```bash
   # Linux (using wget)
   mkdir -p models
   cd models
   wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```
   
   ```powershell
   # Windows (using PowerShell)
   New-Item -ItemType Directory -Force -Path models
   cd models
   Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin" -OutFile "ggml-base.en.bin"
   ```
   
   **Note:** Model files are large (~140MB) and are not included in this repository. You must download them separately.

## Installation

### Building from Source

**You can build on any platform for that same platform:**

```bash
# Clone the repository
git clone https://github.com/BrandtKruger/audio-recorder.git
cd audio-recorder

# Build the release version
cargo build --release
```

The executable will be in:
- **macOS/Linux**: `target/release/audio-recorder`
- **Windows**: `target/release/audio-recorder.exe`

### Cross-Compilation (Optional)

You can cross-compile for Windows from macOS or Linux, but it requires additional setup:
- Install a Windows target: `rustup target add x86_64-pc-windows-msvc`
- Install a cross-compilation toolchain (mingw-w64, etc.)
- Configure linker settings

**For most users, building directly on Windows is simpler and recommended.**

### Platform-Specific Notes

- **macOS**: Uses Metal for GPU acceleration (faster transcription)
- **Windows**: Uses CPU (works perfectly, may be slightly slower)
- **Linux**: Uses CPU (works perfectly)

## Usage

### Live Recording from Microphone

Record and transcribe in real-time from your microphone:

```bash
# Start live recording (press Enter to stop)
cargo run --release -- --live

# Or specify output file
cargo run --release -- --live --output live_meeting.txt

# Adjust chunk size (default: 5 seconds)
cargo run --release -- --live --chunk-seconds 10
```

The app will:
- Record from your default microphone
- Transcribe audio in chunks (default: 5 seconds)
- Display transcriptions in real-time
- Save everything to a text file
- Process remaining audio when you stop

### Transcribe Audio File

Transcribe an audio file (output will be `input_filename.txt`):

```bash
cargo run --release -- --input meeting.wav
```

### Specify Output File

```bash
cargo run --release -- --input meeting.wav --output minutes.txt
```

### Specify Model Path

```bash
cargo run --release -- --input meeting.wav --model ./models/ggml-small.en.bin
```

### Specify Language

```bash
# Auto-detect (default)
cargo run --release -- --input meeting.wav

# Specify language
cargo run --release -- --input meeting.wav --language en
cargo run --release -- --input meeting.wav --language es  # Spanish
cargo run --release -- --input meeting.wav --language fr  # French
```

### Speaker Diarization (Identify Different Speakers)

Enable speaker identification to label who said what:

```bash
# Transcribe with speaker identification
cargo run --release -- --input meeting.wav --speaker-diarization

# Full example with all options
cargo run --release -- \
  --input meeting.wav \
  --output minutes.txt \
  --speaker-diarization \
  --language en
```

**Setting up Speaker Diarization:**

Speaker diarization requires a model file to be downloaded manually:

1. **Visit the model page**: https://huggingface.co/pyannote/segmentation-3.0
2. **Accept the terms**: You may need to create a free Hugging Face account and accept the model's terms of use
3. **Download the model**: Look for the ONNX model file (usually named something like `segmentation.onnx` or similar)
4. **Place in models directory**: Save it as `models/segmentation.onnx`

   ```bash
   mkdir -p models
   # After downloading, move/copy the file to:
   # models/segmentation.onnx
   ```

**Note:** The model file is typically 50-100MB. If the model file is not found, the app will show a helpful error message with download instructions.

### Full Examples

**Live Recording:**
```bash
cargo run --release -- \
  --live \
  --output meeting_minutes.txt \
  --model ./models/ggml-base.en.bin \
  --language en \
  --chunk-seconds 5
```

**File Transcription:**
```bash
cargo run --release -- \
  --input /path/to/meeting_recording.mp3 \
  --output meeting_minutes.txt \
  --model ./models/ggml-base.en.bin \
  --language en
```

## Supported Audio Formats

The app supports any format that `symphonia` can decode, including:
- WAV
- MP3
- M4A
- FLAC
- OGG
- And more...

## Output Format

The transcription file includes:
- Header with source file information
- Timestamped segments in format: `[MM:SS - MM:SS] transcribed text`

Example (without speaker diarization):
```
Meeting Minutes - Transcription
Source: meeting.wav

[00:00 - 00:05] Welcome everyone to today's meeting.
[00:05 - 00:12] Let's start by reviewing the agenda.
[00:12 - 00:25] First item on the agenda is the quarterly review.
...
```

Example (with speaker diarization):
```
Meeting Minutes - Transcription
Source: meeting.wav

[00:00 - 00:05] Speaker 1: Welcome everyone to today's meeting.
[00:05 - 00:12] Speaker 2: Thanks for having me.
[00:12 - 00:25] Speaker 1: Let's start by reviewing the agenda.
[00:25 - 00:35] Speaker 2: I have a question about item three.
...
```

## Notes

- The app automatically resamples audio to 16kHz (Whisper's expected sample rate)
- Transcription time depends on audio length and model size
- Larger models are more accurate but slower
- The `base.en` model is recommended for English meetings
- **Live recording**: Transcriptions appear in real-time as you speak. The app processes audio in chunks for better responsiveness
- **Chunk size**: Smaller chunks (3-5 seconds) provide faster feedback, larger chunks (10+ seconds) may be more accurate
- **Speaker diarization**: Uses pyannote-rs for speaker identification. On first use, models will be downloaded from Hugging Face (requires internet). The current implementation uses gap-based speaker assignment - full embedding-based clustering is in development for improved accuracy

## Windows-Specific Instructions

### Requirements for Windows

1. **Install Rust**:
   - Download and run the installer from [rustup.rs](https://rustup.rs/)
   - Or use: `winget install Rustlang.Rustup` or `choco install rust`

2. **Install CMake**:
   - Download from [cmake.org](https://cmake.org/download/)
   - Or use: `winget install Kitware.CMake` or `choco install cmake`
   - Make sure CMake is in your PATH

3. **Install Visual Studio Build Tools** (required for compiling Rust on Windows):
   - Download "Build Tools for Visual Studio" from [Microsoft](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
   - Install "Desktop development with C++" workload
   - Or install the full Visual Studio with C++ support

4. **Download Whisper Model**:
   ```powershell
   # In PowerShell
   New-Item -ItemType Directory -Force -Path models
   cd models
   Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin" -OutFile "ggml-base.en.bin"
   ```

5. **Build and Run**:
   ```powershell
   # Build the project (must be done on Windows)
   cargo build --release
   
   # Run live transcription
   .\target\release\audio-recorder.exe --live
   
   # Or transcribe a file
   .\target\release\audio-recorder.exe --input meeting.wav --output minutes.txt
   ```

   **Note:** While cross-compilation from other platforms is possible, building directly on Windows is recommended for simplicity.

### Windows Notes

- The executable will be `audio-recorder.exe` in `target\release\`
- On Windows, transcription uses CPU (no GPU acceleration like macOS Metal)
- Make sure your microphone permissions are enabled in Windows Settings
- If you get "No input device available", check Windows Sound Settings

## Repository

This project is hosted on GitHub: [https://github.com/BrandtKruger/audio-recorder](https://github.com/BrandtKruger/audio-recorder)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

