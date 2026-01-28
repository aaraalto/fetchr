mod ai;
mod config;
mod download;
mod search;

use std::io::{self, Write};
use std::time::Duration;

use anyhow::Result;
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

#[derive(Parser)]
#[command(name = "fetchr")]
#[command(about = "AI-powered image asset fetcher - retrieve multiple assets at once")]
#[command(version = VERSION)]
struct Cli {
    /// Asset descriptions (one image will be fetched per query)
    queries: Vec<String>,

    /// Output directory
    #[arg(short, long, default_value = "downloads")]
    output: String,

    /// Skip confirmation and download immediately
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
            if cli.queries.is_empty() {
                // Interactive mode
                print_banner();
                interactive_mode(&cli.output).await?;
            } else {
                print_banner();
                cmd_find(&cli.queries, &cli.output, cli.yes).await?;
            }
        }
    }

    Ok(())
}

async fn interactive_mode(output: &str) -> Result<()> {
    println!("  \x1b[1mEnter assets to fetch (comma-separated):\x1b[0m");
    print!("  \x1b[36m>\x1b[0m ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let queries: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if queries.is_empty() {
        println!("\n  No queries entered. Exiting.");
        return Ok(());
    }

    println!();
    cmd_find(&queries, output, false).await
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

async fn cmd_find(queries: &[String], output: &str, yes: bool) -> Result<()> {
    let cfg = config::load()?;

    println!(
        "  \x1b[1mSearching for {} asset{}\x1b[0m\n",
        queries.len(),
        if queries.len() == 1 { "" } else { "s" }
    );

    let mut all_results = Vec::new();

    for (i, query) in queries.iter().enumerate() {
        // Step 1: AI expansion for this query
        let spinner = create_spinner(&format!(
            "[{}/{}] Expanding \"{}\"...",
            i + 1,
            queries.len(),
            truncate_title(query, 30)
        ));
        let search_queries = ai::expand_prompt(query, &cfg).await?;
        spinner.finish_with_message(format!(
            "\x1b[32m✓\x1b[0m [{}/{}] Generated {} search queries for \"{}\"",
            i + 1,
            queries.len(),
            search_queries.len(),
            truncate_title(query, 30)
        ));

        // Step 2: Search and get the best image (limit=1)
        let spinner = create_spinner(&format!(
            "[{}/{}] Finding best match...",
            i + 1,
            queries.len()
        ));
        let results = search::search_images(&search_queries, 1, &cfg).await?;

        if let Some(result) = results.into_iter().next() {
            spinner.finish_with_message(format!(
                "\x1b[32m✓\x1b[0m [{}/{}] Found: {}",
                i + 1,
                queries.len(),
                truncate_title(&result.title, 45)
            ));
            all_results.push((query.clone(), result));
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

    for (i, (query, result)) in all_results.iter().enumerate() {
        println!(
            "  \x1b[36m{:>2}.\x1b[0m \x1b[1m{}\x1b[0m",
            i + 1,
            truncate_title(query, 50)
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

    // Step 5: Download
    println!();
    let images: Vec<_> = all_results.iter().map(|(_, r)| r.clone()).collect();
    download::download_images(&images, output).await?;
    println!("\n  \x1b[32m✓\x1b[0m Done! {} image{} saved to \x1b[1m{}\x1b[0m",
        images.len(),
        if images.len() == 1 { "" } else { "s" },
        output
    );

    Ok(())
}
