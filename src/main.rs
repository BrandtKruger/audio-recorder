use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use symphonia::core::audio::{AudioBuffer, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use whisper_rs::{FullParams, WhisperContext, WhisperContextParameters};

#[derive(Parser, Debug)]
#[command(name = "audio-recorder")]
#[command(about = "Transcribe audio files or live microphone input to text for meeting minutes", long_about = None)]
struct Args {
    /// Path to the audio file to transcribe (omit for live recording)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Record from microphone instead of transcribing a file
    #[arg(short, long)]
    live: bool,

    /// Path to the output text file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Path to the Whisper model file (default: ./models/ggml-base.en.bin)
    #[arg(short, long, default_value = "./models/ggml-base.en.bin")]
    model: PathBuf,

    /// Language code (e.g., "en", "es", "fr"). Default: auto-detect
    #[arg(short, long)]
    language: Option<String>,

    /// Chunk size in seconds for live transcription (default: 5)
    #[arg(short = 'c', long, default_value = "5")]
    chunk_seconds: u64,
}

fn resolve_model_path(path: &PathBuf) -> Result<PathBuf> {
    // If path is absolute and exists, use it
    if path.is_absolute() && path.exists() {
        return Ok(path.clone());
    }

    // If path is relative and exists in current directory, use it
    if path.exists() {
        return Ok(path.canonicalize()?);
    }

    // Try relative to current directory
    let current_dir = std::env::current_dir()?;
    let relative_path = current_dir.join(path);
    if relative_path.exists() {
        return Ok(relative_path.canonicalize()?);
    }

    // Try relative to project root (look for Cargo.toml)
    let mut search_dir = current_dir.clone();
    loop {
        let cargo_toml = search_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let project_path = search_dir.join(path);
            if project_path.exists() {
                return Ok(project_path.canonicalize()?);
            }
            break;
        }
        match search_dir.parent() {
            Some(parent) => search_dir = parent.to_path_buf(),
            None => break,
        }
    }

    // If still not found, return original path with helpful error
    anyhow::bail!(
        "Model file not found: {}\n\
         Searched in:\n\
         - {}\n\
         - {}\n\
         - Project root (where Cargo.toml is located)\n\
         Please ensure the model file exists or provide an absolute path.\n\
         Current directory: {}",
        path.display(),
        path.display(),
        relative_path.display(),
        current_dir.display()
    )
}

fn load_audio_file(path: &PathBuf) -> Result<Vec<f32>> {
    println!("Loading audio file: {}", path.display());

    // Open the media source
    let src = File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Create a probe hint using the file extension
    let mut hint = Hint::new();
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            hint.with_extension(ext_str);
        }
    }

    // Use the default probe to identify the format
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .with_context(|| "Failed to probe audio format")?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .with_context(|| "No supported audio tracks found")?;

    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .with_context(|| "Failed to create decoder")?;

    let sample_rate = track.codec_params.sample_rate
        .with_context(|| "Sample rate not specified")?;
    println!("Sample rate: {} Hz", sample_rate);

    // Decode all samples
    let mut samples = Vec::new();
    let mut frame_count = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Convert decoded buffer to f32
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                let mut audio_buf_f32: AudioBuffer<f32> = AudioBuffer::new(duration, spec);

                decoded.convert(&mut audio_buf_f32);

                // Convert to mono f32 samples
                let channels = audio_buf_f32.spec().channels.count();
                let planes = audio_buf_f32.planes();
                let plane_slices = planes.planes();
                let buf_frames = audio_buf_f32.frames();
                
                for i in 0..buf_frames {
                    let mut sum = 0.0;
                    for ch in 0..channels {
                        sum += plane_slices[ch][i];
                    }
                    samples.push(sum / channels as f32);
                }

                frame_count += 1;
                if frame_count % 100 == 0 {
                    print!("\rDecoded {} frames...", frame_count);
                    std::io::stdout().flush().unwrap();
                }
            }
            Err(e) => {
                eprintln!("\nDecode error: {:?}", e);
                break;
            }
        }
    }

    println!("\rDecoded {} frames, {} samples", frame_count, samples.len());

    // Resample to 16kHz if needed (Whisper expects 16kHz)
    if sample_rate != 16000 {
        println!("Resampling from {} Hz to 16000 Hz...", sample_rate);
        samples = resample(&samples, sample_rate, 16000);
    }

    Ok(samples)
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut resampled = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;

        if src_idx + 1 < samples.len() {
            // Linear interpolation
            let sample = samples[src_idx] as f64 * (1.0 - frac) + samples[src_idx + 1] as f64 * frac;
            resampled.push(sample as f32);
        } else if src_idx < samples.len() {
            resampled.push(samples[src_idx]);
        }
    }

    resampled
}

fn transcribe_audio(model_path: &PathBuf, audio_samples: &[f32], language: Option<String>) -> Result<String> {
    let resolved_path = resolve_model_path(model_path)?;
    println!("Loading Whisper model: {}", resolved_path.display());
    let ctx_params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(
        resolved_path.to_str().unwrap(),
        ctx_params
    )
    .with_context(|| format!("Failed to load Whisper model from {}", resolved_path.display()))?;

    println!("Initializing transcription...");
    let mut state = ctx.create_state()
        .context("Failed to create Whisper state")?;

    let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    
    // Set language if provided
    if let Some(ref lang) = language {
        params.set_language(Some(lang.as_str()));
    } else {
        params.set_language(None); // Auto-detect
    }

    params.set_translate(false);
    params.set_print_progress(true);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(false);
    params.set_single_segment(false);

    println!("Transcribing audio (this may take a while)...");
    state.full(params, audio_samples)
        .context("Transcription failed")?;

    // Extract the transcription
    let num_segments = state.full_n_segments()
        .context("Failed to get number of segments")?;
    
    let mut transcript = String::new();
    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)
            .context("Failed to get segment text")?;
        let start_timestamp = state.full_get_segment_t0(i)
            .context("Failed to get segment start time")?;
        let end_timestamp = state.full_get_segment_t1(i)
            .context("Failed to get segment end time")?;

        let start_sec = start_timestamp / 100;
        let end_sec = end_timestamp / 100;
        let start_min = start_sec / 60;
        let start_sec = start_sec % 60;
        let end_min = end_sec / 60;
        let end_sec = end_sec % 60;

        transcript.push_str(&format!(
            "[{:02}:{:02} - {:02}:{:02}] {}\n",
            start_min, start_sec, end_min, end_sec, segment.trim()
        ));
    }

    Ok(transcript)
}

fn record_and_transcribe_live(
    model_path: &PathBuf,
    output_path: &PathBuf,
    language: Option<String>,
    chunk_seconds: u64,
) -> Result<()> {
    println!("=== Live Recording & Transcription ===");
    
    // Resolve and load Whisper model
    let resolved_path = resolve_model_path(model_path)?;
    println!("Loading Whisper model: {}", resolved_path.display());
    let ctx_params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(
        resolved_path.to_str().unwrap(),
        ctx_params
    )
    .with_context(|| format!("Failed to load Whisper model from {}", resolved_path.display()))?;
    
    // Create a second context for final processing (WhisperContext can't be cloned)
    let ctx_params_final = WhisperContextParameters::default();
    let ctx_final = WhisperContext::new_with_params(
        resolved_path.to_str().unwrap(),
        ctx_params_final
    )
    .with_context(|| format!("Failed to load Whisper model from {}", resolved_path.display()))?;

    // Setup audio input
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .context("No input device available")?;

    println!("Recording from: {}", input_device.name()?);

    // Get supported config
    let mut supported_configs = input_device.supported_input_configs()?;
    let config = supported_configs
        .next()
        .context("No supported config")?
        .with_max_sample_rate()
        .config();

    println!("Using config: {:?}", config);
    println!("Sample rate: {} Hz", config.sample_rate.0);

    // Prepare output file
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
    let file_clone = Arc::new(Mutex::new(file));
    
    {
        let mut file = file_clone.lock().unwrap();
        writeln!(file, "Meeting Minutes - Live Transcription")
            .context("Failed to write to output file")?;
        writeln!(file, "Started: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
            .context("Failed to write to output file")?;
    }

    // Audio buffer for collecting samples
    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let audio_buffer_clone = audio_buffer.clone();
    let recording = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let recording_clone = recording.clone();

    // Calculate chunk size in samples (16kHz)
    let chunk_size_samples = (chunk_seconds * 16000) as usize;
    let sample_rate = config.sample_rate.0;

    println!("\nRecording... Press Enter to stop.\n");
    println!("Transcribing in {} second chunks...\n", chunk_seconds);

    // Build input stream
    let channels = config.channels as usize;
    let stream = input_device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if recording_clone.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(mut buffer) = audio_buffer_clone.lock() {
                    // Convert to mono if stereo, and resample if needed
                    for chunk in data.chunks(channels) {
                        let mut sum = 0.0;
                        for &sample in chunk {
                            sum += sample;
                        }
                        let mono_sample = sum / channels as f32;
                        buffer.push(mono_sample);
                    }
                }
            }
        },
        move |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;

    // Start a thread for periodic transcription
    let ctx_clone = Arc::new(ctx);
    let language_clone = language.clone();
    let file_clone_transcription = file_clone.clone();
    let recording_transcription = recording.clone();
    let audio_buffer_transcription = audio_buffer.clone();
    
    let transcription_handle = std::thread::spawn(move || {
        let mut last_processed = 0;
        let mut segment_counter = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(chunk_seconds));
            
            if !recording_transcription.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let samples_to_process = {
                let buffer = audio_buffer_transcription.lock().unwrap();
                if buffer.len() - last_processed < chunk_size_samples {
                    continue;
                }
                buffer[last_processed..].to_vec()
            };

            if samples_to_process.is_empty() {
                continue;
            }

            // Resample to 16kHz if needed
            let samples_16k = if sample_rate != 16000 {
                resample(&samples_to_process, sample_rate, 16000)
            } else {
                samples_to_process
            };

            // Transcribe chunk
            let mut state = match ctx_clone.create_state() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create Whisper state: {}", e);
                    continue;
                }
            };

            let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
            
            if let Some(ref lang) = language_clone {
                params.set_language(Some(lang.as_str()));
            } else {
                params.set_language(None);
            }

            params.set_translate(false);
            params.set_print_progress(false);
            params.set_print_special(false);
            params.set_print_realtime(false);
            params.set_suppress_blank(true);
            params.set_suppress_non_speech_tokens(false);
            params.set_single_segment(false);

            if let Err(e) = state.full(params, &samples_16k) {
                eprintln!("Transcription error: {}", e);
                continue;
            }

            // Extract and print transcription
            let num_segments = match state.full_n_segments() {
                Ok(n) => n,
                Err(_) => continue,
            };

            for i in 0..num_segments {
                let segment = match state.full_get_segment_text(i) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                
                let start_timestamp = match state.full_get_segment_t0(i) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                
                let end_timestamp = match state.full_get_segment_t1(i) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let start_sec_total = start_timestamp / 100;
                let end_sec_total = end_timestamp / 100;
                let start_min_total = start_sec_total / 60;
                let start_sec_remainder = start_sec_total % 60;
                let end_min_total = end_sec_total / 60;
                let end_sec_remainder = end_sec_total % 60;

                let total_start_sec = (segment_counter * chunk_seconds) as i64 + start_sec_total as i64;
                let total_end_sec = (segment_counter * chunk_seconds) as i64 + end_sec_total as i64;
                let final_start_min = total_start_sec / 60;
                let final_start_sec = total_start_sec % 60;
                let final_end_min = total_end_sec / 60;
                let final_end_sec = total_end_sec % 60;

                let transcript_line = format!(
                    "[{:02}:{:02} - {:02}:{:02}] {}\n",
                    final_start_min, final_start_sec, final_end_min, final_end_sec,
                    segment.trim()
                );

                print!("{}", transcript_line);
                io::stdout().flush().unwrap();

                if let Ok(mut file) = file_clone_transcription.lock() {
                    let _ = file.write_all(transcript_line.as_bytes());
                }
            }

            segment_counter += 1;
            last_processed += samples_16k.len();
        }
    });

    // Wait for user to press Enter
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Stop recording
    recording.store(false, std::sync::atomic::Ordering::Relaxed);
    drop(stream);

    // Process remaining audio
    println!("\nProcessing remaining audio...");
    let remaining_samples = {
        let buffer = audio_buffer.lock().unwrap();
        buffer.clone()
    };

    if !remaining_samples.is_empty() {
        let samples_16k = if sample_rate != 16000 {
            resample(&remaining_samples, sample_rate, 16000)
        } else {
            remaining_samples
        };

        let mut state = ctx_final.create_state()
            .context("Failed to create Whisper state")?;

        let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        
        if let Some(ref lang) = language {
            params.set_language(Some(lang.as_str()));
        } else {
            params.set_language(None);
        }

        params.set_translate(false);
        params.set_print_progress(false);
        params.set_suppress_blank(true);

        state.full(params, &samples_16k)
            .context("Final transcription failed")?;

        let num_segments = state.full_n_segments()
            .context("Failed to get number of segments")?;

        {
            let mut file = file_clone.lock().unwrap();
            for i in 0..num_segments {
                let segment = state.full_get_segment_text(i)?;
                let start_timestamp = state.full_get_segment_t0(i)?;
                let end_timestamp = state.full_get_segment_t1(i)?;

                let start_sec = start_timestamp / 100;
                let end_sec = end_timestamp / 100;
                let start_min = start_sec / 60;
                let start_sec = start_sec % 60;
                let end_min = end_sec / 60;
                let end_sec = end_sec % 60;

                let transcript_line = format!(
                    "[{:02}:{:02} - {:02}:{:02}] {}\n",
                    start_min, start_sec, end_min, end_sec, segment.trim()
                );

                print!("{}", transcript_line);
                writeln!(file, "{}", transcript_line)?;
            }
        }
    }

    transcription_handle.join().unwrap();

    {
        let mut file = file_clone.lock().unwrap();
        writeln!(file, "\nEnded: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    }

    println!("\n✓ Recording stopped!");
    println!("✓ Transcription saved to: {}", output_path.display());

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Check if we're doing live recording or file transcription
    if args.live || args.input.is_none() {
        // Live recording mode
        let output_path = args.output.unwrap_or_else(|| {
            PathBuf::from(format!("live_transcription_{}.txt", 
                chrono::Local::now().format("%Y%m%d_%H%M%S")))
        });

        record_and_transcribe_live(
            &args.model,
            &output_path,
            args.language,
            args.chunk_seconds,
        )
    } else {
        // File transcription mode
        let input_path = args.input.unwrap();
        let output_path = args.output.unwrap_or_else(|| {
            let mut output = input_path.clone();
            output.set_extension("txt");
            output
        });

        println!("=== Audio Transcription Tool ===");
        println!("Input: {}", input_path.display());
        println!("Output: {}", output_path.display());
        println!();

        // Load and decode audio file
        let audio_samples = load_audio_file(&input_path)?;

        if audio_samples.is_empty() {
            anyhow::bail!("No audio samples found in file");
        }

        println!("Audio loaded: {} samples ({} seconds)", 
                 audio_samples.len(), 
                 audio_samples.len() as f32 / 16000.0);

        // Transcribe using Whisper
        let transcript = transcribe_audio(&args.model, &audio_samples, args.language)?;

        // Save transcription to file
        println!("\nSaving transcription to: {}", output_path.display());
        let mut file = File::create(&output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        
        writeln!(file, "Meeting Minutes - Transcription")
            .context("Failed to write to output file")?;
        writeln!(file, "Source: {}\n", input_path.display())
            .context("Failed to write to output file")?;
        writeln!(file, "{}", transcript)
            .context("Failed to write to output file")?;

        println!("✓ Transcription complete!");
        println!("✓ Saved to: {}", output_path.display());

        Ok(())
    }
}
