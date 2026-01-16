use anyhow::{Context, Result, bail};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use crate::cli::{Args, OutFormat};

pub fn encode(img: &DynamicImage, fmt: OutFormat, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buf);
        match fmt {
            OutFormat::Jpeg => {
                // JPEG doesn't support alpha
                let rgb = img.to_rgb8();
                let (w, h) = rgb.dimensions();
                let enc = JpegEncoder::new_with_quality(&mut cursor, quality);
                enc.write_image(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
                    .context("JPEG encode failed")?;
            }
            OutFormat::Webp => {
                // WebP preserves alpha if present, otherwise converts to RGB
                // webp quality is 0-100 (higher = better quality, larger file)
                if img.color().has_alpha() {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let encoder = webp::Encoder::from_rgba(&rgba, w, h);
                    let encoded = encoder.encode(quality as f32);
                    cursor.write_all(&encoded).context("WebP encode failed")?;
                } else {
                    let rgb = img.to_rgb8();
                    let (w, h) = rgb.dimensions();
                    let encoder = webp::Encoder::from_rgb(&rgb, w, h);
                    let encoded = encoder.encode(quality as f32);
                    cursor.write_all(&encoded).context("WebP encode failed")?;
                }
            }
            OutFormat::Png => {
                let level = quality.min(9);
                if img.color().has_alpha() {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let enc = PngEncoder::new_with_quality(
                        &mut cursor,
                        CompressionType::Level(level),
                        PngFilterType::Adaptive,
                    );
                    enc.write_image(rgba.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                        .context("PNG encode failed")?;
                } else {
                    let rgb = img.to_rgb8();
                    let (w, h) = rgb.dimensions();
                    let enc = PngEncoder::new_with_quality(
                        &mut cursor,
                        CompressionType::Level(level),
                        PngFilterType::Adaptive,
                    );
                    enc.write_image(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
                        .context("PNG encode failed")?;
                }
            }
        }
    }
    Ok(buf)
}

pub fn fit_quality(
    img: &DynamicImage,
    fmt: OutFormat,
    target_bytes: u64,
    qmin: u8,
    qmax: u8,
    round: u8,
) -> Result<(Vec<u8>, u8)> {
    if qmin > qmax || qmin == 0 || qmax > 100 {
        bail!("quality range must be within 1..=100 and min <= max");
    }

    let (w, h) = img.dimensions();
    eprintln!(
        "  [Round {}] Testing dimensions {}x{}, target: {:.1}KB",
        round,
        w,
        h,
        target_bytes as f64 / 1024.0
    );

    let mut lo = qmin as i32;
    let mut hi = qmax as i32;

    let mut best: Option<(Vec<u8>, u8)> = None;
    let mut iteration = 0;

    while lo <= hi {
        iteration += 1;
        let mid = ((lo + hi) / 2) as u8;
        let data = encode(img, fmt, mid)?;
        let size = data.len() as u64;

        eprint!(
            "    Iter {}: q={} -> {:.1}KB",
            iteration,
            mid,
            size as f64 / 1024.0
        );

        if size <= target_bytes {
            best = Some((data, mid));
            eprintln!(" ✓ (fits, trying higher quality)");
            lo = mid as i32 + 1; // try higher quality
        } else {
            eprintln!(" ✗ (too large, reducing quality)");
            hi = mid as i32 - 1; // need smaller
        }
    }

    // If nothing fits, return min quality result (caller may downscale)
    if let Some(ok) = best {
        Ok(ok)
    } else {
        eprintln!("    No quality fits, encoding at min quality for downscaling");
        let data = encode(img, fmt, qmin)?;
        Ok((data, qmin))
    }
}

pub fn apply_max_dimensions(
    mut img: DynamicImage,
    max_w: Option<u32>,
    max_h: Option<u32>,
) -> DynamicImage {
    if max_w.is_none() && max_h.is_none() {
        return img;
    }
    let (w, h) = img.dimensions();

    let scale_w = max_w.map(|mw| mw as f32 / w as f32).unwrap_or(1.0);
    let scale_h = max_h.map(|mh| mh as f32 / h as f32).unwrap_or(1.0);
    let scale = scale_w.min(scale_h).min(1.0);

    if scale < 1.0 {
        let new_w = (w as f32 * scale).max(1.0).round() as u32;
        let new_h = (h as f32 * scale).max(1.0).round() as u32;
        img = img.resize(new_w, new_h, FilterType::Lanczos3);
    }
    img
}

pub fn downscale_10_percent(img: &DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    let new_w = ((w as f32) * 0.9).floor().max(1.0) as u32;
    let new_h = ((h as f32) * 0.9).floor().max(1.0) as u32;
    img.resize(new_w, new_h, FilterType::Lanczos3)
}

pub fn load_and_prepare_image(args: &Args) -> Result<DynamicImage> {
    let img = image::open(&args.input)
        .with_context(|| format!("failed to open input image: {:?}", args.input))?;

    // Apply dimension constraints first
    let img = apply_max_dimensions(img, args.max_width, args.max_height);

    Ok(img)
}

pub fn pre_downscale_large_images(img: &mut DynamicImage, target_bytes: u64) {
    // Pre-downscale very large images to speed up quality search
    // Rough heuristic: WebP uses ~0.3-1 bytes per pixel depending on quality
    // Use 2 bytes/pixel as safe upper bound for high quality
    let (w, h) = img.dimensions();
    let current_pixels = (w as u64) * (h as u64);
    let max_reasonable_pixels = target_bytes / 2; // 2 bytes per pixel upper bound
    if current_pixels > max_reasonable_pixels * 4 {
        // Image is way too large, pre-downscale to ~2x the estimated max
        let scale = ((max_reasonable_pixels * 2) as f64 / current_pixels as f64).sqrt();
        let new_w = ((w as f64 * scale).max(1.0).round() as u32).max(1);
        let new_h = ((h as f64 * scale).max(1.0).round() as u32).max(1);
        eprintln!(
            "Pre-downscaling from {}x{} to {}x{} (image too large for target)",
            w, h, new_w, new_h
        );
        *img = img.resize(new_w, new_h, FilterType::Lanczos3);
    }
}

pub fn write_success_output(
    input: &PathBuf,
    output: &PathBuf,
    data: &[u8],
    dimensions: (u32, u32),
    format: OutFormat,
    compression_info: &str,
) {
    eprintln!(
        "✓ SUCCESS: {:?} -> {:?}  {:.1}KB  size={}x{}  {}  format={:?}",
        input,
        output,
        (data.len() as f64) / 1024.0,
        dimensions.0,
        dimensions.1,
        compression_info,
        format
    );
}

pub fn write_warning_output(
    output_kb: f64,
    target_kb: u64,
    dimensions: (u32, u32),
    format: OutFormat,
    compression_info: &str,
) {
    eprintln!(
        "⚠ WARNING: Could not reach target. Output={:.1}KB (target={:.1}KB) size={}x{} {} format={:?}",
        output_kb, target_kb as f64, dimensions.0, dimensions.1, compression_info, format
    );
}

pub fn process_png_compression(img: DynamicImage, args: &Args, target_bytes: u64) -> Result<()> {
    let mut current_img = img;
    for round in 0..=args.max_downscale_rounds {
        let data = encode(&current_img, OutFormat::Png, args.png_compression_level)?;
        let size = data.len() as u64;

        if round == 0 {
            eprintln!(
                "  [PNG] Encoding at compression level {}, initial size {:.1}KB",
                args.png_compression_level,
                size as f64 / 1024.0
            );
        } else {
            eprintln!(
                "  [PNG Round {}] Downscaled, size {:.1}KB",
                round,
                size as f64 / 1024.0
            );
        }

        if size <= target_bytes {
            fs::write(&args.output, &data)
                .with_context(|| format!("failed to write output: {:?}", args.output))?;

            let dimensions = current_img.dimensions();
            let compression_info = format!("compression_level={}", args.png_compression_level);
            write_success_output(
                &args.input,
                &args.output,
                &data,
                dimensions,
                args.format,
                &compression_info,
            );
            return Ok(());
        }

        if round == args.max_downscale_rounds {
            fs::write(&args.output, &data)
                .with_context(|| format!("failed to write output: {:?}", args.output))?;

            let dimensions = current_img.dimensions();
            let compression_info = format!("compression_level={}", args.png_compression_level);
            write_warning_output(
                (data.len() as f64) / 1024.0,
                args.target_kb,
                dimensions,
                args.format,
                &compression_info,
            );
            return Ok(());
        }

        current_img = downscale_10_percent(&current_img);
    }
    Ok(())
}

pub fn process_lossy_compression(
    mut img: DynamicImage,
    args: &Args,
    target_bytes: u64,
) -> Result<()> {
    pre_downscale_large_images(&mut img, target_bytes);

    let mut last_data = Vec::new();
    let mut last_q = args.min_quality;

    for round in 0..=args.max_downscale_rounds {
        let (data, q) = fit_quality(
            &img,
            args.format,
            target_bytes,
            args.min_quality,
            args.max_quality,
            round,
        )?;

        last_data = data;
        last_q = q;

        if (last_data.len() as u64) <= target_bytes {
            fs::write(&args.output, &last_data)
                .with_context(|| format!("failed to write output: {:?}", args.output))?;

            let dimensions = img.dimensions();
            let compression_info = format!("quality={}", last_q);
            write_success_output(
                &args.input,
                &args.output,
                &last_data,
                dimensions,
                args.format,
                &compression_info,
            );
            return Ok(());
        }

        if round == args.max_downscale_rounds {
            break;
        }
        eprintln!("  → Downscaling by 10% and retrying...");
        img = downscale_10_percent(&img);
    }

    // Write best-effort output
    fs::write(&args.output, &last_data)
        .with_context(|| format!("failed to write output: {:?}", args.output))?;

    let dimensions = img.dimensions();
    let compression_info = format!("quality={}", last_q);
    write_warning_output(
        (last_data.len() as f64) / 1024.0,
        args.target_kb,
        dimensions,
        args.format,
        &compression_info,
    );

    Ok(())
}
