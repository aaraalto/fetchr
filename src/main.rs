mod ai;
mod config;
mod download;
mod search;

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
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
        None => {
            print_banner();

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
                cmd_find(&queries, cli.yes).await?;
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

    println!();
    cmd_find(&queries, false).await
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

async fn cmd_find(queries: &[String], yes: bool) -> Result<()> {
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

    if !yes {
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

    let mut all_results = Vec::new();

    for (i, query) in queries.iter().enumerate() {
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
            all_results.push(result);
        } else {
            spinner.finish_with_message(format!(
                "\x1b[33m!\x1b[0m [{}/{}] No results for \"{}\"",
                i + 1,
                queries.len(),
                truncate_title(query, 30)
            ));
        }
    }

    // Step 3: Display results summary
    if all_results.is_empty() {
        println!("\n  No images found.");
        return Ok(());
    }

    println!("\n  \x1b[1mFound {} image{}:\x1b[0m\n", all_results.len(), if all_results.len() == 1 { "" } else { "s" });

    for (i, result) in all_results.iter().enumerate() {
        println!(
            "  \x1b[36m{:>2}.\x1b[0m \x1b[1m{}\x1b[0m",
            i + 1,
            truncate_title(&result.source_query, 50)
        );
        println!(
            "      {} · \x1b[4m{}\x1b[0m",
            format_dimensions(result.width, result.height),
            result.download_url
        );
        println!();
    }

    // Step 4: Confirm download
    let should_download = if yes {
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
    download::download_images(&all_results, &output_dir).await?;
    println!("\n  \x1b[32m✓\x1b[0m Done! {} image{} saved to \x1b[1m{}\x1b[0m",
        all_results.len(),
        if all_results.len() == 1 { "" } else { "s" },
        output_dir.display()
    );

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
