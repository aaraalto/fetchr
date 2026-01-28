1. Overview
   The CLI tool (binary name: af or asset-fetch) is designed for high-speed asset retrieval. It should prioritize Human-First Design, meaning it provides visual feedback (spinners/progress bars) and handles natural language prompts.

2. Command Hierarchy
   af find <PROMPT> (Primary Command)

The core "AI-powered" search.

Positional Argument: <PROMPT> — The natural language description (e.g., "Braun SK4 Record Player").

Flags:

-n, --limit <INT>: Max number of images to return (Default: 5).

-o, --output <PATH>: Destination directory (Default: ./downloads/{prompt}).

-f, --format <EXT>: Preferred format: png, svg, jpg, any (Default: any).

--ai-only: Only prints the AI-expanded search terms without downloading.

-y, --yes: Skip confirmation and download all top matches immediately.

af config

Manage credentials and preferences.

af config set-key <PROVIDER> <KEY>: Store API keys (OpenAI, Google, etc.) in the macOS Keychain.

af config theme <auto|dark|light>: Set the terminal UI aesthetic.

af list

View history of downloaded assets.

af list --open: Open the folder of the last search in macOS Finder.

3. User Experience (UX) Flow
   Interactive Mode (Default)

If the --yes flag is omitted, the tool should follow this flow:

Thinking: A spinner appears: 🤖 AI is refining your search...

Selection: The terminal displays a numbered list or a TUI (Terminal User Interface) grid of found images with their dimensions and source.

Action: User types 1, 3, 4 to select specific images or all to grab everything.

Download: Progress bars appear for each concurrent download.

Scripted Mode (Non-Interactive)

Designed for automation (e.g., CI/CD or shell scripts).

Bash
af find "BMW Logo" --limit 1 --format svg --yes --output ./brand-assets
Behavior: No spinners. The tool outputs JSON metadata to stdout and error logs to stderr.

4. Error Handling
   The CLI must provide Actionable Errors:

Bad API Key: "Error: Google Search API key is invalid. Run 'af config set-key' to update."

No Results: "AI expanded your search to 'Braun SK4 high-res', but no direct matches were found. Try a broader prompt."

5. Technical Implementation Details (Rust)
   Parser: Use clap with the derive feature for argument parsing.

UI Elements:

indicatif: For the download progress bars.

ratatui: (Optional) If we want a full-screen interactive image picker.

inquire: For simple multi-select lists.

Performance: Use tokio::spawn to handle concurrent downloads so 10 images don't take 10x longer than 1.

Example usage:

Bash

# Search for a specific brand asset

af find "BMW Logo 2024" -f svg -n 1

# Search for a complex object with a custom output folder

af find "1960s Braun record player" --output ~/Desktop/Ref_Images --limit 10
