use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, PartialEq, ValueEnum)]
pub enum OutFormat {
    Jpeg,
    Webp,
    Png,
}

#[derive(Parser, Debug)]
#[command(
    name = "resizer",
    about = "Compress an image to be <= target size (KB)"
)]
pub struct Args {
    /// Input image path
    pub input: std::path::PathBuf,
    /// Output image path
    pub output: std::path::PathBuf,

    /// Target size in KB (upper bound)
    #[arg(long)]
    pub target_kb: u64,

    /// Output format: jpeg, webp, or png
    #[arg(long, value_enum, default_value_t = OutFormat::Webp)]
    pub format: OutFormat,

    /// Optional max width
    #[arg(long)]
    pub max_width: Option<u32>,
    /// Optional max height
    #[arg(long)]
    pub max_height: Option<u32>,

    /// Min quality (1..=100). If still too big, the tool will downscale.
    #[arg(long, default_value_t = 30)]
    pub min_quality: u8,

    /// Max quality (1..=100)
    #[arg(long, default_value_t = 95)]
    pub max_quality: u8,

    /// How many downscale rounds to attempt if min_quality is still too large
    #[arg(long, default_value_t = 10)]
    pub max_downscale_rounds: u8,

    /// PNG compression level (0-9, higher = slower but smaller)
    #[arg(long, default_value_t = 6)]
    pub png_compression_level: u8,
}
