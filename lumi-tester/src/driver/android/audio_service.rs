//! Audio capture and analysis service
//!
//! Connects to nl-android AudioServer (port 8890) to capture and analyze audio.
//! Used for testing audio ducking behavior in navigation apps.

use anyhow::{anyhow, Result};
use std::io::Read;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Audio configuration matching nl-android AudioConfig
const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;
const BITS_PER_SAMPLE: u16 = 16;
const AUDIO_PORT: u16 = 8890;

/// Audio capture service
pub struct AudioService;

/// Represents an audio ducking event (volume drop then recovery)
#[derive(Debug, Clone)]
pub struct DuckingEvent {
    /// Time from start when ducking began
    pub start_time: Duration,
    /// Time from start when volume recovered
    pub end_time: Duration,
    /// Volume level before ducking (0.0 - 1.0)
    pub volume_before: f64,
    /// Minimum volume during ducking (0.0 - 1.0)
    pub volume_during: f64,
    /// Percentage drop (0-100)
    pub drop_percent: f64,
}

/// Audio analysis results
#[derive(Debug)]
pub struct AudioAnalysis {
    /// Total capture duration
    pub duration: Duration,
    /// Average volume level (0.0 - 1.0)
    pub average_volume: f64,
    /// Peak volume level
    pub peak_volume: f64,
    /// Detected ducking events
    pub ducking_events: Vec<DuckingEvent>,
    /// Volume samples over time (for visualization)
    pub volume_timeline: Vec<(Duration, f64)>,
}

/// Active audio capture session
pub struct AudioCapture {
    stream: Arc<Mutex<Option<TcpStream>>>,
    samples: Arc<Mutex<Vec<i16>>>,
    start_time: Instant,
    is_running: Arc<Mutex<bool>>,
}

impl AudioService {
    /// Start capturing audio from nl-android AudioServer
    pub async fn start_capture(serial: Option<&str>) -> Result<AudioCapture> {
        // Setup port forward first
        Self::setup_port_forward(serial).await?;

        // Connect to audio server
        let stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", AUDIO_PORT).parse().unwrap(),
            Duration::from_secs(5),
        )
        .map_err(|e| anyhow!("Failed to connect to AudioServer: {}", e))?;

        stream.set_read_timeout(Some(Duration::from_millis(100)))?;

        let capture = AudioCapture {
            stream: Arc::new(Mutex::new(Some(stream))),
            samples: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
            is_running: Arc::new(Mutex::new(true)),
        };

        // Start capture thread
        let stream_clone = capture.stream.clone();
        let samples_clone = capture.samples.clone();
        let is_running = capture.is_running.clone();

        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];

            loop {
                // Check if still running
                if !*is_running.lock().unwrap() {
                    break;
                }

                // Read from stream
                let stream_guard = stream_clone.lock().unwrap();
                if let Some(ref stream) = *stream_guard {
                    let mut stream_ref = stream;
                    match stream_ref.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            drop(stream_guard);
                            // Convert bytes to i16 samples
                            let mut samples_guard = samples_clone.lock().unwrap();
                            for chunk in buffer[..n].chunks_exact(2) {
                                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                                samples_guard.push(sample);
                            }
                        }
                        Ok(_) => {
                            drop(stream_guard);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            drop(stream_guard);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => {
                            drop(stream_guard);
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        });

        eprintln!("  🎵 Audio capture started");
        Ok(capture)
    }

    /// Setup port forward for audio
    async fn setup_port_forward(serial: Option<&str>) -> Result<()> {
        let serial_args: Vec<String> = match serial {
            Some(s) => vec!["-s".to_string(), s.to_string()],
            None => vec![],
        };

        let mut args = serial_args;
        args.extend([
            "forward".to_string(),
            format!("tcp:{}", AUDIO_PORT),
            format!("tcp:{}", AUDIO_PORT),
        ]);

        let _ = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await?;

        Ok(())
    }

    /// Calculate RMS volume from samples (0.0 - 1.0)
    pub fn calculate_rms(samples: &[i16]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();

        let rms = (sum_squares / samples.len() as f64).sqrt();

        // Normalize to 0.0 - 1.0 range
        (rms / i16::MAX as f64).min(1.0)
    }

    /// Detect audio ducking events
    /// Looks for patterns: volume drop > threshold, followed by recovery
    pub fn detect_ducking(volumes: &[(Duration, f64)], drop_threshold: f64) -> Vec<DuckingEvent> {
        let mut events = Vec::new();

        if volumes.len() < 10 {
            return events;
        }

        // Calculate average volume (for reference)
        let avg_vol: f64 = volumes.iter().map(|(_, v)| v).sum::<f64>() / volumes.len() as f64;

        let mut i = 0;
        while i < volumes.len() - 5 {
            let (time, vol) = volumes[i];

            // Look for significant drop
            if vol >= avg_vol * 0.5 {
                // Find if volume drops significantly in next few samples
                let mut found_drop = false;
                let mut min_vol = vol;
                let mut drop_idx = i;

                for j in (i + 1)..std::cmp::min(i + 20, volumes.len()) {
                    let (_, v) = volumes[j];
                    if v < min_vol {
                        min_vol = v;
                        drop_idx = j;
                    }

                    let drop_pct = ((vol - min_vol) / vol) * 100.0;
                    if drop_pct >= drop_threshold {
                        found_drop = true;
                        break;
                    }
                }

                if found_drop {
                    // Find recovery point
                    let mut recovery_idx = drop_idx;
                    for j in drop_idx..std::cmp::min(drop_idx + 50, volumes.len()) {
                        let (_, v) = volumes[j];
                        if v >= vol * 0.8 {
                            recovery_idx = j;
                            break;
                        }
                    }

                    let drop_pct = ((vol - min_vol) / vol.max(0.001)) * 100.0;

                    events.push(DuckingEvent {
                        start_time: volumes[drop_idx].0,
                        end_time: volumes[recovery_idx].0,
                        volume_before: vol,
                        volume_during: min_vol,
                        drop_percent: drop_pct,
                    });

                    i = recovery_idx;
                }
            }

            i += 1;
        }

        events
    }
}

impl AudioCapture {
    /// Stop capture and analyze the audio
    pub fn stop_and_analyze(self) -> AudioAnalysis {
        // Stop capture
        *self.is_running.lock().unwrap() = false;

        // Close stream
        if let Some(stream) = self.stream.lock().unwrap().take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }

        let duration = self.start_time.elapsed();
        let samples = self.samples.lock().unwrap().clone();

        eprintln!(
            "  🎵 Audio capture stopped. {} samples captured",
            samples.len()
        );

        // Calculate volume timeline (every 100ms)
        let samples_per_window = (SAMPLE_RATE as usize * CHANNELS as usize) / 10; // 100ms window
        let mut volume_timeline = Vec::new();
        let mut window_idx = 0;

        while window_idx * samples_per_window < samples.len() {
            let start = window_idx * samples_per_window;
            let end = std::cmp::min(start + samples_per_window, samples.len());
            let window = &samples[start..end];

            let rms = AudioService::calculate_rms(window);
            let time = Duration::from_millis((window_idx * 100) as u64);
            volume_timeline.push((time, rms));

            window_idx += 1;
        }

        // Calculate statistics
        let average_volume = if !volume_timeline.is_empty() {
            volume_timeline.iter().map(|(_, v)| v).sum::<f64>() / volume_timeline.len() as f64
        } else {
            0.0
        };

        let peak_volume = volume_timeline
            .iter()
            .map(|(_, v)| *v)
            .fold(0.0_f64, f64::max);

        // Detect ducking events
        let ducking_events = AudioService::detect_ducking(&volume_timeline, 30.0);

        AudioAnalysis {
            duration,
            average_volume,
            peak_volume,
            ducking_events,
            volume_timeline,
        }
    }
}

impl AudioAnalysis {
    /// Print analysis summary
    pub fn print_summary(&self) {
        eprintln!("\n  📊 Audio Analysis Results:");
        eprintln!("     Duration: {:.1}s", self.duration.as_secs_f64());
        eprintln!("     Average Volume: {:.1}%", self.average_volume * 100.0);
        eprintln!("     Peak Volume: {:.1}%", self.peak_volume * 100.0);
        eprintln!("     Ducking Events: {}", self.ducking_events.len());

        for (i, event) in self.ducking_events.iter().enumerate() {
            eprintln!(
                "       [{i}] {:.1}s - {:.1}s: {:.0}% drop",
                event.start_time.as_secs_f64(),
                event.end_time.as_secs_f64(),
                event.drop_percent
            );
        }
    }

    /// Check if audio ducking was detected
    pub fn has_ducking(&self, min_events: usize, min_drop_percent: f64) -> bool {
        let valid_events: Vec<_> = self
            .ducking_events
            .iter()
            .filter(|e| e.drop_percent >= min_drop_percent)
            .collect();

        valid_events.len() >= min_events
    }
}
