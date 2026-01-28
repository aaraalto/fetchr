# Fetchr

**AI-powered image asset fetcher** — Retrieve multiple assets at once using AI-expanded search queries.

## Setup

Fetchr requires two API keys:
- **Gemini API Key** — [Get one here](https://aistudio.google.com/)
- **Serper API Key** — [Get one here](https://serper.dev/)

```bash
# Build from source
cargo build --release

# Configure API keys
af config set-key gemini YOUR_GEMINI_API_KEY
af config set-key serper YOUR_SERPER_API_KEY
```

## Usage

```bash
# Interactive mode
./run fetchr

# Fetch assets directly
fetchr "Tesla logo" "Apple logo" "Nike swoosh"

# Skip confirmation and set output directory
fetchr "sunset wallpaper" "ocean waves" -y -o ./assets
```

## License

MIT License — Created by Aaron Aalto
