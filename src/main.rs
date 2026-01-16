mod cli;
mod processor;

use crate::cli::{Args, OutFormat};
use crate::processor::{
    load_and_prepare_image, process_lossy_compression, process_png_compression,
};
use anyhow::Result;
use clap::Parser;
use image::GenericImageView;

fn main() -> Result<()> {
    let args = Args::parse();
    let target_bytes = args.target_kb * 1024;

    let img = load_and_prepare_image(&args)?;

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
    match args.format {
        OutFormat::Png => process_png_compression(img, &args, target_bytes),
        _ => process_lossy_compression(img, &args, target_bytes),
    }
}
