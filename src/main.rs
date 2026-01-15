use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageEncoder};
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, ValueEnum)]
enum OutFormat {
    Jpeg,
    Webp,
    Png,
}

#[derive(Parser, Debug)]
#[command(
    name = "resizer",
    about = "Compress an image to be <= target size (KB)"
)]
struct Args {
    /// Input image path
    input: PathBuf,
    /// Output image path
    output: PathBuf,

    /// Target size in KB (upper bound)
    #[arg(long)]
    target_kb: u64,

    /// Output format: jpeg, webp, or png
    #[arg(long, value_enum, default_value_t = OutFormat::Webp)]
    format: OutFormat,

    /// Optional max width
    #[arg(long)]
    max_width: Option<u32>,
    /// Optional max height
    #[arg(long)]
    max_height: Option<u32>,

    /// Min quality (1..=100). If still too big, the tool will downscale.
    #[arg(long, default_value_t = 30)]
    min_quality: u8,

    /// Max quality (1..=100)
    #[arg(long, default_value_t = 95)]
    max_quality: u8,

    /// How many downscale rounds to attempt if min_quality is still too large
    #[arg(long, default_value_t = 10)]
    max_downscale_rounds: u8,

    /// PNG compression level (0-9, higher = slower but smaller)
    #[arg(long, default_value_t = 6)]
    png_compression_level: u8,
}

fn encode(img: &DynamicImage, fmt: OutFormat, quality: u8) -> Result<Vec<u8>> {
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

fn fit_quality(
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

fn apply_max_dimensions(
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

fn downscale_10_percent(img: &DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    let new_w = ((w as f32) * 0.9).floor().max(1.0) as u32;
    let new_h = ((h as f32) * 0.9).floor().max(1.0) as u32;
    img.resize(new_w, new_h, FilterType::Lanczos3)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let target_bytes = args.target_kb * 1024;

    let mut img = image::open(&args.input)
        .with_context(|| format!("failed to open input image: {:?}", args.input))?;

    let (orig_w, orig_h) = img.dimensions();
    eprintln!(
        "Starting compression: {}x{} -> target {:.1}KB ({:?} format)",
        orig_w,
        orig_h,
        target_bytes as f64 / 1024.0,
        args.format
    );

    // JPEG converts to RGB (dropping alpha, transparent pixels become black).
    // WebP preserves alpha if present, otherwise converts to RGB.
    // PNG preserves alpha if present.
    img = apply_max_dimensions(img, args.max_width, args.max_height);

    if args.format == OutFormat::Png {
        // PNG: lossless, no quality search, just try compression level and downscale if needed
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
                let (w, h) = current_img.dimensions();
                eprintln!(
                    "✓ SUCCESS: {:?} -> {:?}  {:.1}KB  size={}x{}  compression_level={}  format={:?}",
                    args.input,
                    args.output,
                    (data.len() as f64) / 1024.0,
                    w,
                    h,
                    args.png_compression_level,
                    args.format
                );
                return Ok(());
            }
            if round == args.max_downscale_rounds {
                fs::write(&args.output, &data)
                    .with_context(|| format!("failed to write output: {:?}", args.output))?;
                let (w, h) = current_img.dimensions();
                eprintln!(
                    "⚠ WARNING: Could not reach target. Output={:.1}KB (target={:.1}KB) size={}x{} compression_level={} format={:?}",
                    (data.len() as f64) / 1024.0,
                    args.target_kb as f64,
                    w,
                    h,
                    args.png_compression_level,
                    args.format
                );
                return Ok(());
            }
            current_img = downscale_10_percent(&current_img);
        }
    } else {
        // JPEG/WebP: lossy, use quality search
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
            img = img.resize(new_w, new_h, FilterType::Lanczos3);
        }

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

                let (w, h) = img.dimensions();
                eprintln!();
                eprintln!(
                    "✓ SUCCESS: {:?} -> {:?}  {:.1}KB  size={}x{}  quality={}  format={:?}",
                    args.input,
                    args.output,
                    (last_data.len() as f64) / 1024.0,
                    w,
                    h,
                    last_q,
                    args.format
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

        let (w, h) = img.dimensions();
        eprintln!();
        eprintln!(
            "⚠ WARNING: Could not reach target. Output={:.1}KB (target={:.1}KB) size={}x{} quality={} format={:?}",
            (last_data.len() as f64) / 1024.0,
            args.target_kb as f64,
            w,
            h,
            last_q,
            args.format
        );
    }
    Ok(())
}
