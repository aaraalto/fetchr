# Fetchr Architecture — MVP

## 1. Goal
Build a CLI tool that uses AI to expand a natural language prompt into optimized search queries, fetches images from a single API source, and downloads them locally.

**Ship CLI first. Desktop comes later.**

## 2. Project Structure

```
fetchr/
├── Cargo.toml          # Single crate for MVP (no workspace)
├── src/
│   ├── main.rs         # CLI entry point (clap)
│   ├── ai.rs           # AI prompt expansion
│   ├── search.rs       # Image search (Unsplash API)
│   ├── download.rs     # Async image downloads
│   └── config.rs       # Config file handling
└── docs/
```

**Why no workspace?** A workspace adds complexity. Start with one crate. Split later if needed.

## 3. Core Flow

```
User Prompt → AI Expansion → Search API → Download Images
     ↓              ↓             ↓            ↓
"Braun clock"  [3 queries]   [results]    ./downloads/
```

1. **AI Expansion**: Send prompt to OpenAI/Anthropic. Get back 3 optimized search strings.
2. **Search**: Query Unsplash API with each string. Collect results.
3. **Download**: Async download top N images to local folder.

## 4. MVP Tech Stack

| Component | Choice | Reason |
|-----------|--------|--------|
| Async runtime | `tokio` | Industry standard |
| HTTP client | `reqwest` | Simple, async |
| CLI parser | `clap` (derive) | Clean, minimal boilerplate |
| AI client | `reqwest` + raw API | Avoid heavy SDK dependencies |
| Serialization | `serde` + `serde_json` | Required for API responses |
| Progress bars | `indicatif` | Good UX during downloads |
| Config | `dirs` + `serde` | Simple TOML file in ~/.config/fetchr/ |

## 5. API Source: Unsplash

For MVP, use **Unsplash** only:
- Free tier: 50 requests/hour (enough for testing)
- High-quality images
- Simple API: `GET /search/photos?query=...`
- Returns direct download URLs

**Do NOT add more sources until Unsplash works end-to-end.**

## 6. Configuration

Store config in `~/.config/fetchr/config.toml`:

```toml
[keys]
openai = "sk-..."
unsplash = "..."

[defaults]
limit = 5
output_dir = "./downloads"
```

**No keychain for MVP.** A plaintext TOML file is fine for a personal dev tool. Add secure storage later if distributing publicly.

## 7. Error Handling

Keep it simple:
- Use `anyhow` for error propagation
- Print actionable messages to stderr
- Exit with non-zero code on failure

## 8. What's Deferred (Post-MVP)

| Feature | Why Deferred |
|---------|--------------|
| Desktop app (Tauri) | Massive scope; ship CLI first |
| Plugin/trait system | No need until 2+ sources exist |
| SQLite caching | Optimize later if API limits hurt |
| Keychain storage | Only needed for public distribution |
| Multiple API sources | Get one working first |
| Interactive TUI picker | Start with simple numbered list |

## 9. MVP Definition of Done

The MVP is complete when:
- [ ] `af find "Braun clock"` expands prompt via AI
- [ ] Searches Unsplash with expanded queries
- [ ] Downloads top 5 images to `./downloads/`
- [ ] Shows progress bars during download
- [ ] `af config set-key openai <KEY>` saves to config file

That's it. Ship this, then iterate.
