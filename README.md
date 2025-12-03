# Audio Recorder & Transcriber

A Rust application for transcribing audio files (especially meeting recordings) to text files for meeting minutes.

[![GitHub](https://img.shields.io/github/license/BrandtKruger/audio-recorder)](https://github.com/BrandtKruger/audio-recorder)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

## Features

- 🎙️ Transcribes audio files to text
- 🎤 **Live recording from microphone with real-time transcription**
- 📝 Saves transcriptions with timestamps
- 🎵 Supports multiple audio formats (WAV, MP3, M4A, FLAC, etc.)
- 🌍 Auto-detects language or specify manually
- ⚡ Uses OpenAI Whisper for accurate transcription

## Prerequisites

1. **Install CMake** (required for building whisper-rs):
   ```bash
   # macOS
   brew install cmake
   
   # Linux (Ubuntu/Debian)
   sudo apt-get install cmake
   
   # Or download from https://cmake.org/download/
   ```

2. **Download a Whisper model**: You need to download a Whisper model file. Recommended models:
   - `ggml-base.en.bin` - Base English model (good balance of speed/accuracy)
   - `ggml-small.en.bin` - Small English model (faster, less accurate)
   - `ggml-medium.en.bin` - Medium English model (slower, more accurate)

   Download from: https://huggingface.co/ggerganov/whisper.cpp/tree/main

3. **Create a models directory and download the model**:
   ```bash
   # Example: Download base English model (using curl, available on macOS)
   mkdir -p models
   cd models
   curl -L -o ggml-base.en.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```
   
   Or if you have `wget` installed (Linux):
   ```bash
   mkdir -p models
   cd models
   wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```
   
   **Note:** Model files are large (~140MB) and are not included in this repository. You must download them separately.

## Installation

```bash
cargo build --release
```

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

Example:
```
Meeting Minutes - Transcription
Source: meeting.wav

[00:00 - 00:05] Welcome everyone to today's meeting.
[00:05 - 00:12] Let's start by reviewing the agenda.
[00:12 - 00:25] First item on the agenda is the quarterly review.
...
```

## Notes

- The app automatically resamples audio to 16kHz (Whisper's expected sample rate)
- Transcription time depends on audio length and model size
- Larger models are more accurate but slower
- The `base.en` model is recommended for English meetings
- **Live recording**: Transcriptions appear in real-time as you speak. The app processes audio in chunks for better responsiveness
- **Chunk size**: Smaller chunks (3-5 seconds) provide faster feedback, larger chunks (10+ seconds) may be more accurate

## Repository

This project is hosted on GitHub: [https://github.com/BrandtKruger/audio-recorder](https://github.com/BrandtKruger/audio-recorder)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

