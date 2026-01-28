use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::search::ImageResult;

/// Get the default download directory (system Downloads/fetchr)
pub fn get_download_dir() -> Result<PathBuf> {
    let downloads = dirs::download_dir()
        .context("Could not find system Downloads directory")?;
    Ok(downloads.join("fetchr"))
}

/// Sanitize a string to be safe for use as a filename
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn download_images(images: &[ImageResult], output_dir: &Path) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir)
        .await
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    let multi_progress = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:30.cyan/blue}] {msg}")
        .unwrap()
        .progress_chars("#>-");

    let client = reqwest::Client::new();

    // Download all images concurrently
    let mut handles = Vec::new();

    for image in images {
        let pb = multi_progress.add(ProgressBar::new(100));
        pb.set_style(style.clone());
        pb.set_message(format!("{}", &image.id[..8.min(image.id.len())]));

        let client = client.clone();
        let image = image.clone();
        let output_dir = output_dir.to_path_buf();

        let handle = tokio::spawn(async move {
            let result = download_single(&client, &image, &output_dir, &pb).await;
            pb.finish_with_message(format!(
                "{} {}",
                &image.id[..8.min(image.id.len())],
                if result.is_ok() { "done" } else { "failed" }
            ));
            result
        });

        handles.push(handle);
    }

    // Wait for all downloads
    let mut errors = Vec::new();
    for handle in handles {
        if let Err(e) = handle.await? {
            errors.push(e);
        }
    }

    if !errors.is_empty() {
        eprintln!("\nSome downloads failed:");
        for e in &errors {
            eprintln!("  - {}", e);
        }
    }

    Ok(())
}

async fn download_single(
    client: &reqwest::Client,
    image: &ImageResult,
    output_dir: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    let response = client
        .get(&image.download_url)
        .send()
        .await
        .context("Failed to start download")?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed with status: {}", response.status());
    }

    let bytes = response.bytes().await.context("Failed to read image data")?;

    pb.set_position(50);

    // Get extension from URL or default to jpg
    let ext = image
        .download_url
        .rsplit('.')
        .next()
        .filter(|e| ["jpg", "jpeg", "png", "gif", "webp", "svg"].contains(&e.to_lowercase().as_str()))
        .unwrap_or("jpg");

    // Use sanitized source query as filename
    let base_name = sanitize_filename(&image.source_query);
    let filename = format!("{}.{}", base_name, ext);
    let filepath = output_dir.join(&filename);

    fs::write(&filepath, &bytes)
        .await
        .with_context(|| format!("Failed to save image to {:?}", filepath))?;

    pb.set_position(100);

    Ok(())
}
