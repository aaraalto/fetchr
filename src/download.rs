use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use tokio::fs;

use crate::search::ImageResult;

pub async fn download_images(images: &[ImageResult], output_dir: &str) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir)
        .await
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

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
        let output_dir = output_dir.to_string();

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
    output_dir: &str,
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
    let filename = format!("{}.{}", image.id, ext);
    let filepath = Path::new(output_dir).join(&filename);

    fs::write(&filepath, &bytes)
        .await
        .with_context(|| format!("Failed to save image to {:?}", filepath))?;

    pb.set_position(100);

    Ok(())
}
