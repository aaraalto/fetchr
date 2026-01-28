mod ai;
mod auto;
mod config;
mod download;
mod feedback;
mod search;

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};

const VERSION: &str = "1.0";
const AUTHOR: &str = "Aaron Aalto";

const BANNER: &str = r#"
    ███████╗███████╗████████╗ ██████╗██╗  ██╗██████╗
    ██╔════╝██╔════╝╚══██╔══╝██╔════╝██║  ██║██╔══██╗
    █████╗  █████╗     ██║   ██║     ███████║██████╔╝
    ██╔══╝  ██╔══╝     ██║   ██║     ██╔══██║██╔══██╗
    ██║     ███████╗   ██║   ╚██████╗██║  ██║██║  ██║
    ╚═╝     ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝
"#;

fn print_banner() {
    println!("\x1b[36m{}\x1b[0m", BANNER);
    println!(
        "    \x1b[90mv{} · Created by {}\x1b[0m",
        VERSION, AUTHOR
    );
    println!("    \x1b[90mRetrieve multiple assets at once\x1b[0m\n");
}

fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() >= 2) // Minimum 2 chars to prevent accidental searches
        .collect()
}

fn parse_queries_from_file(path: &PathBuf) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read queries from {}", path.display()))?;
    Ok(parse_comma_separated(&content))
}

#[derive(Parser)]
#[command(name = "fetchr")]
#[command(about = "AI-powered image asset fetcher - retrieve multiple assets at once")]
#[command(version = VERSION)]
struct Cli {
    /// Asset descriptions - comma-separated (no quotes needed)
    /// Example: fetchr Tesla logo, Apple icon, Nike swoosh
    #[arg(trailing_var_arg = true)]
    queries: Vec<String>,

    /// Read queries from a text file (comma-separated)
    #[arg(short = 'f', long = "file")]
    file: Option<PathBuf>,

    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    yes: bool,

    /// Prompt for ratings after download
    #[arg(long)]
    rate: bool,

    /// Autonomous mode: skip confirmations, auto-retry on failure
    #[arg(long)]
    auto: bool,

    /// Maximum query reformulations in auto mode (default: 3)
    #[arg(long, default_value = "3")]
    max_retries: u32,

    /// Verbose logging (show AI decisions and retry attempts)
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage feedback history
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
}

#[derive(Subcommand)]
enum HistoryAction {
    /// Show feedback statistics
    Stats,
    /// Clear all feedback history
    Clear,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set an API key
    SetKey {
        /// Provider name (gemini, serper)
        provider: String,
        /// API key value
        key: String,
    },
    /// Show current configuration
    Show,
}

/// Options for the find command
#[derive(Clone)]
struct FindOptions {
    yes: bool,
    rate: bool,
    auto_mode: bool,
    max_retries: u32,
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Config { action }) => match action {
            ConfigAction::SetKey { provider, key } => {
                config::set_key(&provider, &key)?;
                println!("Saved {} key", provider);
            }
            ConfigAction::Show => {
                config::show()?;
            }
        },
        Some(Commands::History { action }) => match action {
            HistoryAction::Stats => {
                let (up, down, skip) = feedback::get_stats()?;
                println!("Feedback history:");
                println!("  Thumbs up:   {}", up);
                println!("  Thumbs down: {}", down);
                println!("  Skipped:     {}", skip);
                println!("  Total:       {}", up + down + skip);
            }
            HistoryAction::Clear => {
                feedback::clear_history()?;
                println!("Feedback history cleared.");
            }
        },
        None => {
            print_banner();

            let opts = FindOptions {
                yes: cli.yes || cli.auto,  // auto mode implies yes
                rate: cli.rate,
                auto_mode: cli.auto,
                max_retries: cli.max_retries,
                verbose: cli.verbose,
            };

            // Collect queries from file, CLI args, or interactive mode
            let queries = if let Some(file_path) = &cli.file {
                parse_queries_from_file(file_path)?
            } else if !cli.queries.is_empty() {
                // Join all args and split by comma (no quotes needed)
                parse_comma_separated(&cli.queries.join(" "))
            } else {
                Vec::new()
            };

            if queries.is_empty() {
                // Interactive mode
                interactive_mode().await?;
            } else {
                cmd_find(&queries, &opts).await?;
            }
        }
    }

    Ok(())
}

async fn interactive_mode() -> Result<()> {
    println!("  \x1b[1mEnter assets to fetch (comma-separated):\x1b[0m");
    print!("  \x1b[36m>\x1b[0m ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let queries = parse_comma_separated(&input);

    if queries.is_empty() {
        println!("\n  No valid queries entered (min 2 characters each). Exiting.");
        return Ok(());
    }

    let opts = FindOptions {
        yes: false,
        rate: false,
        auto_mode: false,
        max_retries: 3,
        verbose: false,
    };

    println!();
    cmd_find(&queries, &opts).await
}

fn create_spinner(msg: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(msg.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}

fn format_dimensions(width: u32, height: u32) -> String {
    if width > 0 && height > 0 {
        format!("{}x{}", width, height)
    } else {
        "unknown".to_string()
    }
}

fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        format!("{}...", &title[..max_len - 3])
    }
}

/// Stores info needed for feedback after download
struct DownloadedImageInfo {
    result: search::ImageResult,
    expanded_query: String,
    filters: feedback::SearchFilters,
}

async fn cmd_find(queries: &[String], opts: &FindOptions) -> Result<()> {
    let cfg = config::load()?;
    let output_dir = download::get_download_dir()?;

    // Show queries and confirm before searching (API calls cost money)
    println!(
        "  \x1b[1mReady to search for {} asset{}:\x1b[0m\n",
        queries.len(),
        if queries.len() == 1 { "" } else { "s" }
    );

    for (i, query) in queries.iter().enumerate() {
        println!("  \x1b[36m{:>2}.\x1b[0m {}", i + 1, query);
    }
    println!();

    if !opts.yes {
        print!("  Proceed with search? \x1b[90m[Y/n]\x1b[0m ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if !input.is_empty() && input != "y" && input != "yes" {
            println!("\n  Cancelled.");
            return Ok(());
        }
        println!();
    }

    let mut all_results: Vec<DownloadedImageInfo> = Vec::new();
    let mut auto_session = auto::AutoSession::new();

    for (i, query) in queries.iter().enumerate() {
        if opts.auto_mode {
            // Auto mode: use retry logic with reformulation
            let spinner = create_spinner(&format!(
                "[{}/{}] Auto-searching \"{}\"...",
                i + 1,
                queries.len(),
                truncate_title(query, 30)
            ));

            match auto::find_with_retry(
                query,
                &cfg,
                opts.max_retries,
                &mut auto_session,
                opts.verbose,
            )
            .await
            {
                Ok(Some((result, expanded))) => {
                    spinner.finish_with_message(format!(
                        "\x1b[32m✓\x1b[0m [{}/{}] Found: {}",
                        i + 1,
                        queries.len(),
                        truncate_title(&result.title, 45)
                    ));
                    all_results.push(DownloadedImageInfo {
                        result,
                        expanded_query: expanded.query.clone(),
                        filters: feedback::SearchFilters {
                            img_size: expanded.img_size.clone(),
                            img_type: expanded.img_type.clone(),
                        },
                    });
                }
                Ok(None) => {
                    spinner.finish_with_message(format!(
                        "\x1b[33m!\x1b[0m [{}/{}] No results for \"{}\" (after {} retries)",
                        i + 1,
                        queries.len(),
                        truncate_title(query, 30),
                        opts.max_retries
                    ));
                }
                Err(e) => {
                    spinner.finish_with_message(format!(
                        "\x1b[31m✗\x1b[0m [{}/{}] Error for \"{}\": {}",
                        i + 1,
                        queries.len(),
                        truncate_title(query, 30),
                        e
                    ));
                }
            }
        } else {
            // Normal mode: single attempt
            // Step 1: AI expansion for this query
            let spinner = create_spinner(&format!(
                "[{}/{}] Optimizing \"{}\"...",
                i + 1,
                queries.len(),
                truncate_title(query, 30)
            ));
            let expanded = ai::expand_prompt(query, &cfg).await?;
            let filter_info = match (&expanded.img_size, &expanded.img_type) {
                (Some(s), Some(t)) => format!(" [{}:{}]", s, t),
                (Some(s), None) => format!(" [{}]", s),
                (None, Some(t)) => format!(" [{}]", t),
                (None, None) => String::new(),
            };
            spinner.finish_with_message(format!(
                "\x1b[32m✓\x1b[0m [{}/{}] Query: \"{}\"{}",
                i + 1,
                queries.len(),
                truncate_title(&expanded.query, 40),
                filter_info
            ));

            // Step 2: Search and get the best image (fetch top 3 for fallback)
            let spinner = create_spinner(&format!(
                "[{}/{}] Finding best match...",
                i + 1,
                queries.len()
            ));
            let results = search::search_images(&expanded, query, 3, &cfg).await?;

            // Try to find a valid image (HEAD check for availability)
            let mut found_result = None;
            for result in results {
                if check_url_available(&result.download_url).await {
                    found_result = Some(result);
                    break;
                }
            }

            if let Some(result) = found_result {
                spinner.finish_with_message(format!(
                    "\x1b[32m✓\x1b[0m [{}/{}] Found: {}",
                    i + 1,
                    queries.len(),
                    truncate_title(&result.title, 45)
                ));
                all_results.push(DownloadedImageInfo {
                    result,
                    expanded_query: expanded.query.clone(),
                    filters: feedback::SearchFilters {
                        img_size: expanded.img_size.clone(),
                        img_type: expanded.img_type.clone(),
                    },
                });
            } else {
                spinner.finish_with_message(format!(
                    "\x1b[33m!\x1b[0m [{}/{}] No results for \"{}\"",
                    i + 1,
                    queries.len(),
                    truncate_title(query, 30)
                ));
            }
        }
    }

    // Show auto-mode decision log if verbose
    if opts.auto_mode && opts.verbose {
        auto_session.print_summary();
    }

    // Step 3: Display results summary
    if all_results.is_empty() {
        println!("\n  No images found.");
        return Ok(());
    }

    println!("\n  \x1b[1mFound {} image{}:\x1b[0m\n", all_results.len(), if all_results.len() == 1 { "" } else { "s" });

    for (i, info) in all_results.iter().enumerate() {
        println!(
            "  \x1b[36m{:>2}.\x1b[0m \x1b[1m{}\x1b[0m",
            i + 1,
            truncate_title(&info.result.source_query, 50)
        );
        println!(
            "      {} · \x1b[4m{}\x1b[0m",
            format_dimensions(info.result.width, info.result.height),
            info.result.download_url
        );
        println!();
    }

    // Step 4: Confirm download
    let should_download = if opts.yes {
        true
    } else {
        print!("  Download all? \x1b[90m[Y/n]\x1b[0m ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        input.is_empty() || input == "y" || input == "yes"
    };

    if !should_download {
        println!("\n  Cancelled.");
        return Ok(());
    }

    // Step 5: Download to system Downloads/fetchr folder
    println!();
    let image_results: Vec<_> = all_results.iter().map(|info| info.result.clone()).collect();
    download::download_images(&image_results, &output_dir).await?;
    println!("\n  \x1b[32m✓\x1b[0m Done! {} image{} saved to \x1b[1m{}\x1b[0m",
        all_results.len(),
        if all_results.len() == 1 { "" } else { "s" },
        output_dir.display()
    );

    // Step 6: Prompt for ratings if enabled
    if opts.rate && !all_results.is_empty() {
        prompt_for_ratings(&all_results).await?;
    }

    Ok(())
}

/// Prompt user to rate downloaded images
async fn prompt_for_ratings(results: &[DownloadedImageInfo]) -> Result<()> {
    println!("\n  \x1b[1mRate these results to help improve future searches:\x1b[0m");
    println!("  \x1b[90m(1 = thumbs up, 2 = thumbs down, Enter = skip)\x1b[0m\n");

    for info in results {
        print!(
            "  {} \x1b[90m[1/2/Enter]\x1b[0m ",
            truncate_title(&info.result.source_query, 40)
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        let rating = match input {
            "1" => feedback::Rating::ThumbsUp,
            "2" => feedback::Rating::ThumbsDown,
            _ => feedback::Rating::Skip,
        };

        let entry = feedback::FeedbackEntry {
            timestamp: Utc::now(),
            original_query: info.result.source_query.clone(),
            expanded_query: info.expanded_query.clone(),
            filters: info.filters.clone(),
            image_url: info.result.download_url.clone(),
            image_title: info.result.title.clone(),
            rating,
        };

        feedback::append_entry(entry)?;

        let rating_str = match rating {
            feedback::Rating::ThumbsUp => "\x1b[32m+\x1b[0m",
            feedback::Rating::ThumbsDown => "\x1b[31m-\x1b[0m",
            feedback::Rating::Skip => "\x1b[90m~\x1b[0m",
        };
        println!("    {}", rating_str);
    }

    println!("\n  \x1b[90mFeedback saved. Run 'fetchr history stats' to view.\x1b[0m");
    Ok(())
}

/// Quick HEAD request to check if a URL is accessible
async fn check_url_available(url: &str) -> bool {
    let client = reqwest::Client::new();
    match client.head(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
