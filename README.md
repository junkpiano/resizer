# Resizer

A CLI tool to compress images to fit under a target size in KB by adjusting quality and downscaling if needed.

## Installation

Clone the repository and build with Cargo:

```bash
git clone <repository-url>
cd resizer
cargo build --release
```

The binary will be available at `target/release/resizer`.

## Usage

```bash
resizer [OPTIONS] <input> <output>
```

### Options

- `--target-kb <TARGET_KB>`: Target size in KB (upper bound)
- `--format <FORMAT>`: Output format: jpeg, webp, or png [default: webp]
- `--max-width <MAX_WIDTH>`: Optional max width
- `--max-height <MAX_HEIGHT>`: Optional max height
- `--min-quality <MIN_QUALITY>`: Min quality (1..=100) [default: 30]
- `--max-quality <MAX_QUALITY>`: Max quality (1..=100) [default: 95]
- `--max-downscale-rounds <MAX_DOWNSCALE_ROUNDS>`: How many downscale rounds to attempt [default: 10]
- `--help`: Print help

### Examples

Compress an image to under 100 KB as WebP:

```bash
resizer input.jpg output.webp --target-kb 100
```

Compress with max dimensions and PNG format:

```bash
resizer input.png output.png --target-kb 50 --format png --max-width 800 --max-height 600
```

### Behavior

- Uses binary search on quality to find the highest quality that fits the target size.
- If min quality still exceeds the target, downscales the image by 10% and retries.
- JPEG and WebP convert to RGB (transparent pixels become black).
- PNG preserves alpha channels if present.
- For PNG, quality controls compression level (higher = smaller but slower).

## License

MIT License - see LICENSE file.