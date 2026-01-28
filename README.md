# Fetchr

```
    ███████╗███████╗████████╗ ██████╗██╗  ██╗██████╗
    ██╔════╝██╔════╝╚══██╔══╝██╔════╝██║  ██║██╔══██╗
    █████╗  █████╗     ██║   ██║     ███████║██████╔╝
    ██╔══╝  ██╔══╝     ██║   ██║     ██╔══██║██╔══██╗
    ██║     ███████╗   ██║   ╚██████╗██║  ██║██║  ██║
    ╚═╝     ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝
```

**AI-powered image asset fetcher** — Retrieve multiple assets at once.

Fetchr uses AI to expand your search queries and finds the best matching image for each asset you need. Perfect for designers, developers, and content creators who need to quickly gather multiple images.

## Features

- **AI-Powered Search** — Uses Gemini AI to expand your queries into optimized search terms
- **Multi-Asset Fetching** — Fetch multiple images in a single command
- **Smart Selection** — Automatically selects the best matching image for each query
- **Rate Limit Handling** — Built-in retry logic with exponential backoff
- **Clean CLI** — Beautiful terminal output with spinners and progress indicators

## Installation

### From Source

```bash
git clone https://github.com/yourusername/fetchr.git
cd fetchr
cargo build --release
```

The binary will be available at `./target/release/af`.

### Add to PATH (Optional)

```bash
# Copy to local bin
cp ./target/release/af ~/.local/bin/

# Or add to your shell profile
export PATH="$PATH:/path/to/fetchr/target/release"
```

## Setup

Fetchr requires two API keys:

1. **Gemini API Key** — For AI-powered query expansion ([Get one here](https://aistudio.google.com/))
2. **Serper API Key** — For image search ([Get one here](https://serper.dev/))

### Configure via CLI

```bash
af config set-key gemini YOUR_GEMINI_API_KEY
af config set-key serper YOUR_SERPER_API_KEY
```

### Or via Environment Variables

```bash
export GEMINI_API_KEY=your_gemini_key
export SERPER_API_KEY=your_serper_key
```

## Common Commands

### Interactive Mode

```bash
# Start interactive mode
run fetchr

# You'll see the banner and a prompt:
# > Enter assets to fetch (comma-separated):
# > Tesla logo, Apple logo, Nike swoosh
```

### Direct Commands

```bash
# Fetch a single asset
fetchr "BMW logo transparent"

# Fetch multiple assets at once
fetchr "Apple logo" "Google logo" "Microsoft logo"

# Skip confirmation prompt
fetchr "sunset wallpaper" "ocean waves" -y

# Custom output directory
fetchr "cat photo" "dog photo" -o ./assets

# Combine options
fetchr "icon set" "ui buttons" -o ./design-assets -y
```

### Configuration

```bash
# Set API keys
fetchr config set-key gemini YOUR_KEY
fetchr config set-key serper YOUR_KEY

# View current configuration
fetchr config show
```

### Help

```bash
# General help
fetchr --help

# Config help
fetchr config --help
```

## Example Workflow

```bash
$ fetchr "Tesla logo" "SpaceX logo" "Neuralink logo"

    ███████╗███████╗████████╗ ██████╗██╗  ██╗██████╗
    ██╔════╝██╔════╝╚══██╔══╝██╔════╝██║  ██║██╔══██╗
    █████╗  █████╗     ██║   ██║     ███████║██████╔╝
    ██╔══╝  ██╔══╝     ██║   ██║     ██╔══██║██╔══██╗
    ██║     ███████╗   ██║   ╚██████╗██║  ██║██║  ██║
    ╚═╝     ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝

    v1.0 · Created by Aaron Aalto
    Retrieve multiple assets at once

  Searching for 3 assets

✓ [1/3] Generated 3 search queries for "Tesla logo"
✓ [1/3] Found: Tesla Logo PNG Transparent
✓ [2/3] Generated 3 search queries for "SpaceX logo"
✓ [2/3] Found: SpaceX Logo Vector
✓ [3/3] Generated 3 search queries for "Neuralink logo"
✓ [3/3] Found: Neuralink Official Logo

  Found 3 images:

   1. Tesla logo
      2000x2000 · https://example.com/tesla-logo.png

   2. SpaceX logo
      1500x500 · https://example.com/spacex-logo.png

   3. Neuralink logo
      1200x1200 · https://example.com/neuralink-logo.png

  Download all? [Y/n] y

  ✓ Done! 3 images saved to downloads
```

## Project Structure

```
fetchr/
├── src/
│   ├── main.rs      # CLI entry point and command handling
│   ├── ai.rs        # Gemini AI integration for query expansion
│   ├── search.rs    # Serper image search integration
│   ├── download.rs  # Image downloading logic
│   └── config.rs    # Configuration management
├── Cargo.toml
└── README.md
```

## License

MIT License — see [LICENSE](LICENSE) for details.

## Author

**Aaron Aalto**

---

*Built with Rust*
