use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::ExpandedQuery;
use crate::config::Config;

const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone)]
pub struct ImageResult {
    pub id: String,
    pub title: String,
    pub download_url: String,
    pub width: u32,
    pub height: u32,
    pub source_query: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerperRequest {
    q: String,
    num: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    img_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    img_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    images: Option<Vec<SerperImage>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerperImage {
    title: String,
    image_url: String,
    image_width: Option<u32>,
    image_height: Option<u32>,
}

pub async fn search_images(
    expanded: &ExpandedQuery,
    original_query: &str,
    limit: usize,
    config: &Config,
) -> Result<Vec<ImageResult>> {
    let api_key = config
        .keys
        .serper
        .as_ref()
        .context("Serper API key not set. Run: fetchr config set-key serper <KEY>")?;

    let client = reqwest::Client::new();

    let request = SerperRequest {
        q: expanded.query.clone(),
        num: limit.min(10),
        img_size: expanded.img_size.clone(),
        img_type: expanded.img_type.clone(),
    };

    let search_response = retry_request(MAX_RETRIES, || async {
        let response = client
            .post("https://google.serper.dev/images")
            .header("X-API-KEY", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .with_context(|| format!("Failed to search Serper for: {}", expanded.query))?;

        let status = response.status();
        if is_rate_limit_status(status) {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("rate_limit: Serper API error ({}): {}", status, body);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Serper API error ({}): {}", status, body);
        }

        let search_response: SerperResponse = response
            .json()
            .await
            .context("Failed to parse Serper response")?;

        Ok(search_response)
    })
    .await?;

    let mut results: Vec<ImageResult> = Vec::new();

    if let Some(images) = search_response.images {
        for image in images.into_iter().take(limit) {
            results.push(ImageResult {
                id: format!("{:x}", simple_hash(&image.image_url)),
                title: image.title,
                download_url: image.image_url,
                width: image.image_width.unwrap_or(0),
                height: image.image_height.unwrap_or(0),
                source_query: original_query.to_string(),
            });
        }
    }

    Ok(results)
}

fn simple_hash(input: &str) -> u64 {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

fn is_rate_limit_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
}

fn is_rate_limit_error(e: &anyhow::Error) -> bool {
    e.to_string().contains("rate_limit:")
}

async fn retry_request<F, Fut, T>(max_retries: u32, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = Duration::from_secs(1);
    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if is_rate_limit_error(&e) && attempt < max_retries => {
                eprintln!("Rate limited, retrying in {}s...", delay.as_secs());
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => {
                // Strip the "rate_limit:" prefix for final error
                let msg = e.to_string();
                if let Some(stripped) = msg.strip_prefix("rate_limit: ") {
                    anyhow::bail!("{}", stripped);
                }
                return Err(e);
            }
        }
    }
    unreachable!()
}
