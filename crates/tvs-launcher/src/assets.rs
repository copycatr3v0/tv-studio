//! Wallpaper intake. Every still image (JPEG, PNG, WebP, GIF, BMP, TIFF) is decoded and re-encoded on the desktop
//! as JPEG: that drops metadata and any payload appended to the file and caps the size the TV has to decode (RAM
//! matters on 1 GB TVs). An animated GIF or WebP is a live wallpaper: it is checked (size, dimensions, that it
//! decodes) and sent as it is, because re-encoding would lose the animation. Other formats (AVIF, HEIC) are
//! converted to JPEG by the desktop UI before they get here.

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

static NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9_-]{1,60}\.(jpg|gif|webp)$").unwrap());
/// An animated wallpaper is sent unchanged, so it gets a tighter cap than a still that is re-encoded.
pub const MAX_ANIMATED_BYTES: usize = 15 * 1024 * 1024;
/// Largest side of an animated wallpaper (the launcher scales it to 1080p while playing).
pub const MAX_ANIMATED_SIDE: u32 = 4096;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AssetError {
    #[error("{0}: file is larger than 30 MB")]
    TooLarge(String),
    #[error("{0}: not a readable JPEG, PNG, WebP, GIF, BMP or TIFF image")]
    Unreadable(String),
    #[error("{0}: file names may only use letters, digits, - and _")]
    BadName(String),
    #[error("{0}: an animated wallpaper can be at most 15 MB and 4096 pixels on a side")]
    AnimatedTooLarge(String),
    #[error("A theme can carry at most 30 wallpapers")]
    TooMany,
}

/// A safe on-TV file name derived from the user's file name: `Sunset Beach!.png` → `sunset-beach.jpg`
/// (`ext` is what `normalize` produced: jpg, or gif/webp for an animation).
pub fn safe_name(original: &str, index: usize, ext: &str) -> String {
    let stem = original.rsplit_once('.').map(|(s, _)| s).unwrap_or(original);
    let mut out: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    while out.contains("--") { out = out.replace("--", "-"); }
    let out = out.trim_matches('-');
    let base = if out.is_empty() { format!("wallpaper-{index:02}") } else { out.chars().take(50).collect() };
    format!("{base}.{ext}")
}

pub fn valid_name(name: &str) -> bool { NAME.is_match(name) }

/// Whether a GIF or WebP holds more than one frame.
fn animated(fmt: ImageFormat, bytes: &[u8]) -> bool {
    match fmt {
        // WebP: the extended header's animation flag (VP8X chunk, flags byte bit 1).
        ImageFormat::WebP => bytes.len() > 21 && &bytes[12..16] == b"VP8X" && bytes[20] & 0x02 != 0,
        ImageFormat::Gif => {
            use image::AnimationDecoder;
            image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map(|d| d.into_frames().take(2).count() > 1).unwrap_or(false)
        }
        _ => false,
    }
}

/// A still becomes a JPEG fitted within 1920×1080 (never upscaled, quality 88); an animated GIF/WebP is checked and
/// kept as it is. Returns the bytes and the extension to send them under.
pub fn normalize(label: &str, bytes: &[u8]) -> Result<(Vec<u8>, &'static str), AssetError> {
    if bytes.len() > MAX_INPUT_BYTES { return Err(AssetError::TooLarge(label.into())); }
    let fmt = image::guess_format(bytes).map_err(|_| AssetError::Unreadable(label.into()))?;
    if !matches!(fmt, ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP | ImageFormat::Gif | ImageFormat::Bmp | ImageFormat::Tiff) {
        return Err(AssetError::Unreadable(label.into()));
    }
    // Bounded decode: a small file can claim enormous dimensions (decompression bomb).
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), fmt);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().map_err(|_| AssetError::Unreadable(label.into()))?;
    let (w, h) = img.dimensions();
    if animated(fmt, bytes) {
        if bytes.len() > MAX_ANIMATED_BYTES || w > MAX_ANIMATED_SIDE || h > MAX_ANIMATED_SIDE { return Err(AssetError::AnimatedTooLarge(label.into())); }
        return Ok((bytes.to_vec(), if fmt == ImageFormat::Gif { "gif" } else { "webp" }));
    }
    let img = if w > MAX_W || h > MAX_H { img.resize(MAX_W, MAX_H, FilterType::Lanczos3) } else { img };
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(Cursor::new(&mut out), 88);
    enc.encode_image(&rgb).map_err(|_| AssetError::Unreadable(label.into()))?;
    Ok((out, "jpg"))
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
        let (out, ext) = normalize("big.png", &png(3840, 2160)).unwrap();
        assert_eq!(ext, "jpg");
        assert_eq!(image::guess_format(&out).unwrap(), ImageFormat::Jpeg);
        assert_eq!(image::load_from_memory(&out).unwrap().dimensions(), (1920, 1080));
    }

    #[test]
    fn never_upscales_and_strips_trailing_payload() {
        let mut b = png(800, 450);
        b.extend_from_slice(b"<?php evil(); ?>");
        let (out, _) = normalize("small.png", &b).unwrap();
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
        assert_eq!(safe_name("Sunset Beach!.PNG", 0, "jpg"), "sunset-beach.jpg");
        assert_eq!(safe_name("..\\..\\evil.jpg", 3, "jpg"), "evil.jpg");
        assert_eq!(safe_name("???.png", 4, "jpg"), "wallpaper-04.jpg");
        assert_eq!(safe_name("Rain Loop.gif", 0, "gif"), "rain-loop.gif");
        assert!(valid_name("sunset-beach.jpg") && valid_name("rain.gif") && valid_name("loop.webp"));
        assert!(!valid_name("../x.jpg") && !valid_name("a b.jpg") && !valid_name("x.png"));
    }

    fn gif(frames: usize) -> Vec<u8> {
        use image::codecs::gif::{GifEncoder, Repeat};
        let mut b = Vec::new();
        {
            let mut e = GifEncoder::new(&mut b);
            e.set_repeat(Repeat::Infinite).unwrap();
            for i in 0..frames {
                let f = image::RgbaImage::from_pixel(64, 36, image::Rgba([(i * 80) as u8, 40, 200, 255]));
                e.encode_frame(image::Frame::new(f)).unwrap();
            }
        }
        b
    }

    #[test]
    fn animated_gif_is_kept_and_a_still_gif_becomes_jpeg() {
        let anim = gif(3);
        let (out, ext) = normalize("loop.gif", &anim).unwrap();
        assert_eq!(ext, "gif");
        assert_eq!(out, anim, "sent unchanged so it keeps moving");
        let (out, ext) = normalize("still.gif", &gif(1)).unwrap();
        assert_eq!(ext, "jpg");
        assert_eq!(image::guess_format(&out).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn bmp_and_tiff_become_jpeg() {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(40, 30, image::Rgb([9, 9, 9])));
        for f in [ImageFormat::Bmp, ImageFormat::Tiff] {
            let mut b = Vec::new();
            img.write_to(&mut Cursor::new(&mut b), f).unwrap();
            assert_eq!(normalize("x", &b).unwrap().1, "jpg");
        }
    }

    #[test]
    fn animated_webp_flag_is_read_from_the_header() {
        let mut b = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\x02\0\0\0".to_vec();
        b.resize(40, 0);
        assert!(animated(ImageFormat::WebP, &b));
        b[20] = 0;
        assert!(!animated(ImageFormat::WebP, &b));
    }
}
