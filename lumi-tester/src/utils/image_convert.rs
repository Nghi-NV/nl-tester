//! Shared image re-encoding helpers for lighter-weight on-disk storage.
//!
//! `--snapshot` (failure screenshot capture) can accumulate hundreds of
//! full-resolution device screenshots across a long-running suite. Each
//! `take_screenshot` call writes a raw PNG; this module re-encodes that PNG
//! as lossless WebP right after capture and removes the original, so the
//! evidence directory stays smaller without losing any pixels (this is
//! debugging evidence, not a display thumbnail, so lossy compression is not
//! used here - see `report::summary_html::embed_image_thumb` for the
//! separate lossy-safe *thumbnail* re-encode used only for HTML embedding).

use anyhow::{Context, Result};
use image::RgbImage;
use std::path::{Path, PathBuf};

/// Encodes an in-memory RGB image buffer as lossless WebP bytes.
fn encode_rgb_lossless(rgb: &RgbImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
    encoder
        .encode(rgb, rgb.width(), rgb.height(), image::ColorType::Rgb8)
        .context("failed to encode image as WebP")?;
    Ok(buf)
}

/// Re-encodes the PNG (or any `image`-decodable format) at `path` as
/// lossless WebP, writes it next to the original with a `.webp` extension,
/// deletes the original file, and returns the new path.
///
/// On any failure (decode error, encode error, write error) the original
/// file is left untouched and the error is returned - callers should fall
/// back to keeping the original screenshot rather than losing evidence.
pub fn convert_to_webp_in_place(path: &Path) -> Result<PathBuf> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let img = image::load_from_memory(&bytes)
        .with_context(|| format!("failed to decode image at {}", path.display()))?;
    let buf = encode_rgb_lossless(&img.to_rgb8())
        .with_context(|| format!("failed to encode {} as WebP", path.display()))?;

    let webp_path = path.with_extension("webp");
    std::fs::write(&webp_path, &buf)
        .with_context(|| format!("failed to write {}", webp_path.display()))?;

    // Only remove the source once the WebP write succeeded - never leave the
    // caller with neither file.
    let _ = std::fs::remove_file(path);

    Ok(webp_path)
}

/// Saves an in-memory RGB image buffer directly as lossless WebP at `path`
/// (no intermediate file, no source to delete) - for callers that already
/// hold decoded pixels in memory (e.g. camera evidence frames) and would
/// otherwise write a PNG straight to disk via `ImageBuffer::save`.
pub fn save_rgb_as_webp_lossless(img: &RgbImage, path: &Path) -> Result<()> {
    let buf = encode_rgb_lossless(img)
        .with_context(|| format!("failed to encode {} as WebP", path.display()))?;
    std::fs::write(path, &buf).with_context(|| format!("failed to write {}", path.display()))
}

/// Guesses an image's MIME type from its file extension, for embedding raw
/// bytes in a data URI without re-encoding. Defaults to PNG for anything
/// unrecognized (the pre-existing behavior before WebP evidence existed).
pub fn guess_image_mime(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn write_test_png(path: &Path) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(64, 32, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255, 255, 255])
            } else {
                Rgb([10, 20, 30])
            }
        });
        img.save(path).expect("failed to write test PNG");
    }

    #[test]
    fn converts_png_to_webp_and_removes_original() {
        let dir = std::env::temp_dir().join(format!("lumi_webp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let png_path = dir.join("shot.png");
        write_test_png(&png_path);

        let webp_path = convert_to_webp_in_place(&png_path).expect("conversion should succeed");

        assert_eq!(webp_path.extension().unwrap(), "webp");
        assert!(webp_path.exists(), "webp file should exist");
        assert!(!png_path.exists(), "original PNG should be removed");

        // Lossless round-trip: decoding the WebP back must reproduce the
        // exact pixels written to the source PNG, not just "close enough" -
        // this is failure evidence, so no compression artifacts are allowed.
        let decoded = image::open(&webp_path).unwrap().to_rgb8();
        let original: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(64, 32, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255, 255, 255])
            } else {
                Rgb([10, 20, 30])
            }
        });
        assert_eq!(decoded.as_raw(), original.as_raw());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conversion_failure_leaves_original_untouched() {
        let dir = std::env::temp_dir().join(format!("lumi_webp_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let bogus_path = dir.join("not_an_image.png");
        std::fs::write(&bogus_path, b"not a real image").unwrap();

        let result = convert_to_webp_in_place(&bogus_path);

        assert!(result.is_err());
        assert!(bogus_path.exists(), "original file must survive a failed conversion");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guesses_mime_by_extension() {
        assert_eq!(guess_image_mime("evidence/fail_1.webp"), "image/webp");
        assert_eq!(guess_image_mime("evidence/fail_1.WEBP"), "image/webp");
        assert_eq!(guess_image_mime("evidence/fail_1.jpg"), "image/jpeg");
        assert_eq!(guess_image_mime("evidence/fail_1.jpeg"), "image/jpeg");
        assert_eq!(guess_image_mime("evidence/fail_1.png"), "image/png");
        assert_eq!(guess_image_mime("evidence/fail_1"), "image/png");
    }
}
