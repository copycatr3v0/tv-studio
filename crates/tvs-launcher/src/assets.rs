//! Wallpaper intake. Every image is decoded and re-encoded on the desktop: that drops metadata and any
//! payload appended to the file, caps the size the TV has to decode (RAM matters on 1 GB TVs) and
//! guarantees the launcher receives a plain JPEG.

use image::{imageops::FilterType, GenericImageView, ImageFormat};
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::Cursor;

pub const MAX_W: u32 = 1920;
pub const MAX_H: u32 = 1080;
pub const MAX_INPUT_BYTES: usize = 30 * 1024 * 1024;
pub const MAX_WALLPAPERS: usize = 30;
/// Largest source image side accepted before resizing (8K).
pub const MAX_SOURCE_SIDE: u32 = 8192;

static NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9_-]{1,60}\.jpg$").unwrap());

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AssetError {
    #[error("{0}: file is larger than 30 MB")]
    TooLarge(String),
    #[error("{0}: not a readable JPEG, PNG or WebP image")]
    Unreadable(String),
    #[error("{0}: file names may only use letters, digits, - and _")]
    BadName(String),
    #[error("A theme can carry at most 30 wallpapers")]
    TooMany,
}

/// A safe on-TV file name derived from the user's file name: `Sunset Beach!.png` → `sunset-beach.jpg`.
pub fn safe_name(original: &str, index: usize) -> String {
    let stem = original.rsplit_once('.').map(|(s, _)| s).unwrap_or(original);
    let mut out: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    while out.contains("--") { out = out.replace("--", "-"); }
    let out = out.trim_matches('-');
    let base = if out.is_empty() { format!("wallpaper-{index:02}") } else { out.chars().take(50).collect() };
    format!("{base}.jpg")
}

pub fn valid_name(name: &str) -> bool { NAME.is_match(name) }

/// Decode, fit within 1920×1080 (never upscale), and encode as JPEG quality 88.
pub fn normalize(label: &str, bytes: &[u8]) -> Result<Vec<u8>, AssetError> {
    if bytes.len() > MAX_INPUT_BYTES { return Err(AssetError::TooLarge(label.into())); }
    let fmt = image::guess_format(bytes).map_err(|_| AssetError::Unreadable(label.into()))?;
    if !matches!(fmt, ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP) { return Err(AssetError::Unreadable(label.into())); }
    // Bounded decode: a small file can claim enormous dimensions (decompression bomb).
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), fmt);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().map_err(|_| AssetError::Unreadable(label.into()))?;
    let (w, h) = img.dimensions();
    let img = if w > MAX_W || h > MAX_H { img.resize(MAX_W, MAX_H, FilterType::Lanczos3) } else { img };
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(Cursor::new(&mut out), 88);
    enc.encode_image(&rgb).map_err(|_| AssetError::Unreadable(label.into()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x % 255) as u8, (y % 255) as u8, 80]));
        let mut b = Vec::new();
        image::DynamicImage::ImageRgb8(img).write_to(&mut Cursor::new(&mut b), ImageFormat::Png).unwrap();
        b
    }

    #[test]
    fn downsizes_and_reencodes() {
        let out = normalize("big.png", &png(3840, 2160)).unwrap();
        assert_eq!(image::guess_format(&out).unwrap(), ImageFormat::Jpeg);
        assert_eq!(image::load_from_memory(&out).unwrap().dimensions(), (1920, 1080));
    }

    #[test]
    fn never_upscales_and_strips_trailing_payload() {
        let mut b = png(800, 450);
        b.extend_from_slice(b"<?php evil(); ?>");
        let out = normalize("small.png", &b).unwrap();
        assert_eq!(image::load_from_memory(&out).unwrap().dimensions(), (800, 450));
        assert!(!out.windows(5).any(|w| w == b"<?php"));
    }

    #[test]
    fn refuses_images_claiming_huge_dimensions() {
        // A valid PNG header announcing 60000×60000 pixels, without the pixel data.
        let mut png = Vec::new();
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut png).write_image(&[0u8; 3], 1, 1, image::ExtendedColorType::Rgb8).unwrap();
        // IHDR width/height live at bytes 16..24; patch them and fix the CRC.
        png[16..20].copy_from_slice(&60_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&60_000u32.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(matches!(normalize("bomb.png", &png), Err(AssetError::Unreadable(_))));
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data { c ^= b as u32; for _ in 0..8 { c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }; } }
        !c
    }

    #[test]
    fn rejects_non_images() {
        assert_eq!(normalize("x.jpg", b"not an image"), Err(AssetError::Unreadable("x.jpg".into())));
    }

    #[test]
    fn names() {
        assert_eq!(safe_name("Sunset Beach!.PNG", 0), "sunset-beach.jpg");
        assert_eq!(safe_name("..\\..\\evil.jpg", 3), "evil.jpg");
        assert_eq!(safe_name("???.png", 4), "wallpaper-04.jpg");
        assert!(valid_name("sunset-beach.jpg"));
        assert!(!valid_name("../x.jpg") && !valid_name("a b.jpg") && !valid_name("x.png"));
    }
}
