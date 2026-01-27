# Sizify

![Crates.io Version](https://img.shields.io/crates/v/sizify?style=flat&link=https%3A%2F%2Fcrates.io%2Fcrates%2Fsizify)


A CLI tool to compress images to fit under a target size in KB by adjusting quality and downscaling if needed.

## Installation

```bash
cargo install sizify
```

## Usage

```bash
sizify [OPTIONS] <input> <output>
```

### Options

- `--target-kb <TARGET_KB>`: Target size in KB (upper bound)
- `--format <FORMAT>`: Output format: jpeg, webp, or png [default: webp]
- `--max-width <MAX_WIDTH>`: Optional max width
- `--max-height <MAX_HEIGHT>`: Optional max height
- `--min-quality <MIN_QUALITY>`: Min quality (1..=100) [default: 30]
- `--max-quality <MAX_QUALITY>`: Max quality (1..=100) [default: 95]
- `--max-downscale-rounds <MAX_DOWNSCALE_ROUNDS>`: How many downscale rounds to attempt [default: 10]
- `--png-compression-level <PNG_COMPRESSION_LEVEL>`: PNG compression level (0-9, higher = slower but smaller) [default: 6]
- `--help`: Print help

### Examples

Compress an image to under 100 KB as WebP:

```bash
sizify input.jpg output.webp --target-kb 100
```

Compress with max dimensions and PNG format:

```bash
sizify input.png output.png --target-kb 50 --format png --max-width 800 --max-height 600 --png-compression-level 9
```

### Behavior

- For JPEG and WebP: Uses binary search on quality to find the highest quality that fits the target size. If min quality still exceeds the target, downscales the image by 10% and retries. Pre-downscaling is applied for very large images.
- For PNG: No quality search; encodes at the specified compression level. If too large, downscales by 10% and retries.
- JPEG converts to RGB (transparent pixels become black).
- WebP preserves alpha if present.
- PNG preserves alpha if present.

## License

MIT License - see LICENSE file.