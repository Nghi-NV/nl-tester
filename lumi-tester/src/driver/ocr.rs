//! Cross-Platform OCR Engine
//!
//! Uses native OCR APIs for best performance without external dependencies:
//! - macOS: Vision Framework (via Swift helper)
//! - Windows: Windows.Media.Ocr API (via PowerShell)
//! - Linux/Fallback: Tesseract CLI

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

/// Result of OCR text detection
#[derive(Debug, Clone)]
pub struct OcrMatch {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub confidence: f32,
}

/// OCR Engine - auto-detects platform and uses native API
#[derive(Clone, Default)]
pub struct OcrEngine {
    backend: OcrBackend,
}

#[derive(Clone, Default, Debug)]
enum OcrBackend {
    #[default]
    MacOSVision,
    WindowsOcr,
    Tesseract,
}

impl OcrEngine {
    pub async fn new() -> Result<Self> {
        let backend = Self::detect_backend();
        println!("      🔍 OCR backend: {:?}", backend);
        Ok(Self { backend })
    }

    fn detect_backend() -> OcrBackend {
        #[cfg(target_os = "macos")]
        {
            OcrBackend::MacOSVision
        }
        #[cfg(target_os = "windows")]
        {
            OcrBackend::WindowsOcr
        }
        #[cfg(target_os = "linux")]
        {
            OcrBackend::Tesseract
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            OcrBackend::Tesseract
        }
    }

    pub fn is_regex_pattern(text: &str) -> bool {
        text.contains('*')
            || text.contains('+')
            || text.contains('?')
            || text.contains('[')
            || text.contains('(')
            || text.contains('|')
            || text.contains('^')
            || text.contains('$')
            || text.contains("\\d")
            || text.contains("\\w")
            || text.contains("\\s")
            || text.contains("\\b")
    }

    pub fn find_text(
        &self,
        image_data: &[u8],
        search_text: &str,
        is_regex: bool,
    ) -> Result<Vec<OcrMatch>> {
        let start = Instant::now();

        let temp_path = std::env::temp_dir().join(format!("ocr_{}.png", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, image_data).context("Failed to write temp image")?;

        let all_lines = match self.backend {
            OcrBackend::MacOSVision => self.run_macos_vision(&temp_path)?,
            OcrBackend::WindowsOcr => self.run_windows_ocr(&temp_path)?,
            OcrBackend::Tesseract => self.run_tesseract(&temp_path)?,
        };

        let _ = std::fs::remove_file(&temp_path);

        // Debug output
        if !all_lines.is_empty() {
            let texts: Vec<&str> = all_lines.iter().map(|l| l.text.as_str()).collect();
            println!("      📝 Lines: {:?}", texts);
        }

        // Filter matches
        let regex = if is_regex {
            // Add case-insensitive flag if not already present
            let pattern = if search_text.starts_with("(?i)") {
                search_text.to_string()
            } else {
                format!("(?i){}", search_text)
            };
            Some(Regex::new(&pattern).context("Invalid regex pattern")?)
        } else {
            None
        };
        let search_lower = search_text.to_lowercase();

        let matches: Vec<OcrMatch> = all_lines
            .into_iter()
            .filter(|line| {
                if let Some(ref re) = regex {
                    re.is_match(&line.text)
                } else {
                    line.text.to_lowercase().contains(&search_lower)
                }
            })
            .collect();

        println!(
            "      ⚡ OCR completed in {}ms ({} matches)",
            start.elapsed().as_millis(),
            matches.len()
        );

        Ok(matches)
    }

    pub fn find_text_at_index(
        &self,
        image_data: &[u8],
        search_text: &str,
        is_regex: bool,
        index: usize,
    ) -> Result<Option<OcrMatch>> {
        let matches = self.find_text(image_data, search_text, is_regex)?;
        Ok(matches.into_iter().nth(index))
    }

    // ==================== macOS Vision Framework ====================
    fn run_macos_vision(&self, image_path: &std::path::Path) -> Result<Vec<OcrMatch>> {
        // Find the compiled OCR helper binary
        let helper_path = Self::find_ocr_helper()?;

        let output = Command::new(&helper_path)
            .arg(image_path)
            .output()
            .context("Failed to run macOS Vision OCR")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Fallback to Tesseract if Vision fails
            if stderr.contains("Error") {
                eprintln!(
                    "      ⚠️ Vision failed, falling back to Tesseract: {}",
                    stderr.trim()
                );
                return self.run_tesseract(image_path);
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 5 {
                let x: i32 = parts[1].parse().unwrap_or(0);
                let y: i32 = parts[2].parse().unwrap_or(0);
                let width: u32 = parts[3].parse().unwrap_or(0);
                let height: u32 = parts[4].parse().unwrap_or(0);

                matches.push(OcrMatch {
                    text: parts[0].to_string(),
                    x: x + (width as i32 / 2),
                    y: y + (height as i32 / 2),
                    width,
                    height,
                    confidence: 1.0,
                });
            }
        }

        // Sort by position (top to bottom, left to right)
        matches.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
        Ok(matches)
    }

    fn find_ocr_helper() -> Result<std::path::PathBuf> {
        // Try multiple locations for the compiled binary
        let candidates = [
            Some(std::path::PathBuf::from("resources/ocr_helper")),
            std::env::current_exe()
                .ok()
                .map(|p| p.with_file_name("ocr_helper")),
            dirs::home_dir().map(|h| h.join(".lumi-tester/ocr_helper")),
        ];

        for candidate in candidates.iter().flatten() {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        anyhow::bail!(
            "OCR helper not found. Please compile: swiftc -O -o resources/ocr_helper resources/ocr_helper.swift"
        )
    }

    // ==================== Windows OCR API ====================
    fn run_windows_ocr(&self, image_path: &std::path::Path) -> Result<Vec<OcrMatch>> {
        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Storage.StorageFile,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]

$imagePath = '{}'

# Load image
$file = [Windows.Storage.StorageFile]::GetFileFromPathAsync($imagePath).GetAwaiter().GetResult()
$stream = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read).GetAwaiter().GetResult()
$decoder = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream).GetAwaiter().GetResult()
$bitmap = $decoder.GetSoftwareBitmapAsync().GetAwaiter().GetResult()

# Run OCR
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
$result = $engine.RecognizeAsync($bitmap).GetAwaiter().GetResult()

# Output as TSV: text, x, y, width, height
foreach ($line in $result.Lines) {{
    $words = $line.Words | ForEach-Object {{ $_.Text }} | Join-String -Separator ' '
    $rect = $line.Words[0].BoundingRect
    Write-Output "$words`t$([int]$rect.X)`t$([int]$rect.Y)`t$([int]$rect.Width)`t$([int]$rect.Height)"
}}
"#,
            image_path.to_string_lossy().replace("\\", "\\\\")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .context("Failed to run Windows OCR")?;

        if !output.status.success() {
            // Fallback to Tesseract if Windows OCR fails
            return self.run_tesseract(image_path);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 5 {
                matches.push(OcrMatch {
                    text: parts[0].to_string(),
                    x: parts[1].parse().unwrap_or(0) + parts[3].parse::<i32>().unwrap_or(0) / 2,
                    y: parts[2].parse().unwrap_or(0) + parts[4].parse::<i32>().unwrap_or(0) / 2,
                    width: parts[3].parse().unwrap_or(0),
                    height: parts[4].parse().unwrap_or(0),
                    confidence: 1.0,
                });
            }
        }

        Ok(matches)
    }

    // ==================== Tesseract CLI (Fallback) ====================
    fn run_tesseract(&self, image_path: &std::path::Path) -> Result<Vec<OcrMatch>> {
        let output = Command::new("tesseract")
            .arg(image_path)
            .arg("stdout")
            .arg("-l")
            .arg("eng+vie")
            .arg("--psm")
            .arg("3")
            .arg("tsv")
            .output()
            .context(
                "Tesseract not found. Please install: brew install tesseract tesseract-lang",
            )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Tesseract failed: {}", stderr);
        }

        let tsv = String::from_utf8_lossy(&output.stdout);
        self.parse_tesseract_tsv(&tsv)
    }

    fn parse_tesseract_tsv(&self, tsv: &str) -> Result<Vec<OcrMatch>> {
        let lines: Vec<&str> = tsv.lines().collect();
        if lines.len() <= 1 {
            return Ok(Vec::new());
        }

        // Group words by line
        let mut line_groups: HashMap<(i32, i32, i32), Vec<WordInfo>> = HashMap::new();

        for line in lines.iter().skip(1) {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 12 {
                continue;
            }

            let text = cols[11].trim();
            if text.is_empty() {
                continue;
            }

            let key = (
                cols[2].parse().unwrap_or(0),
                cols[3].parse().unwrap_or(0),
                cols[4].parse().unwrap_or(0),
            );

            line_groups.entry(key).or_default().push(WordInfo {
                text: text.to_string(),
                left: cols[6].parse().unwrap_or(0),
                top: cols[7].parse().unwrap_or(0),
                width: cols[8].parse().unwrap_or(0),
                height: cols[9].parse().unwrap_or(0),
                conf: cols[10].parse().unwrap_or(0.0),
            });
        }

        let mut matches: Vec<OcrMatch> = line_groups
            .into_iter()
            .map(|(_, words)| {
                let text = words
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let min_left = words.iter().map(|w| w.left).min().unwrap_or(0);
                let min_top = words.iter().map(|w| w.top).min().unwrap_or(0);
                let max_right = words
                    .iter()
                    .map(|w| w.left + w.width as i32)
                    .max()
                    .unwrap_or(0);
                let max_bottom = words
                    .iter()
                    .map(|w| w.top + w.height as i32)
                    .max()
                    .unwrap_or(0);

                OcrMatch {
                    text,
                    x: (min_left + max_right) / 2,
                    y: (min_top + max_bottom) / 2,
                    width: (max_right - min_left) as u32,
                    height: (max_bottom - min_top) as u32,
                    confidence: words.iter().map(|w| w.conf).sum::<f32>()
                        / words.len() as f32
                        / 100.0,
                }
            })
            .collect();

        matches.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
        Ok(matches)
    }
}

#[derive(Debug)]
struct WordInfo {
    text: String,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    conf: f32,
}
