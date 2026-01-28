use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;

const MAX_RETRIES: u32 = 3;

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}

/// Structured response from Gemini with query and Serper filters
#[derive(Debug, Clone, Deserialize)]
pub struct ExpandedQuery {
    pub query: String,
    #[serde(default)]
    pub img_size: Option<String>,
    #[serde(default)]
    pub img_type: Option<String>,
}

const PROMPT_TEMPLATE: &str = r#"You are an AI Asset Scout. Your task is to take a user's short input and create ONE highly optimized search query with appropriate image filters.

Analyze the input and determine:
1. What the user wants (logo, product photo, icon, artwork, etc.)
2. The best single search query that will find a high-quality, relevant image
3. The appropriate Serper image filters

Available filters:
- img_size: "large" (high-res photos/products), "medium" (general use), "icon" (small icons/favicons)
- img_type: "photo" (real photographs), "clipart" (logos, icons, vector-style), "lineart" (simple drawings), "face" (portraits)

Guidelines:
- For LOGOS/BRANDS: Use img_type "clipart", include "official", "transparent", "vector" or "SVG" in query
- For PRODUCTS: Use img_type "photo", img_size "large", include "studio", "product shot", "white background"
- For ICONS: Use img_size "icon" or "medium", img_type "clipart"
- For PHOTOS/SCENES: Use img_type "photo", img_size "large"

Respond with ONLY a JSON object (no markdown, no extra text):
{"query": "your optimized search query", "img_size": "large|medium|icon|null", "img_type": "photo|clipart|lineart|face|null"}

Example for "BMW logo":
{"query": "BMW official logo transparent SVG vector", "img_size": "large", "img_type": "clipart"}

Example for "iPhone 15":
{"query": "iPhone 15 Pro product photo studio white background", "img_size": "large", "img_type": "photo"}
"#;

const PROMPT_SUFFIX: &str = "User input: ";

pub async fn expand_prompt(prompt: &str, config: &Config) -> Result<ExpandedQuery> {
    // Try to get learning context from feedback history
    let learning_context = crate::feedback::get_learning_context(3)
        .unwrap_or(None)
        .unwrap_or_default();

    expand_prompt_with_context(prompt, config, &learning_context).await
}

pub async fn expand_prompt_with_context(
    prompt: &str,
    config: &Config,
    learning_context: &str,
) -> Result<ExpandedQuery> {
    let api_key = config
        .keys
        .gemini
        .as_ref()
        .context("Gemini API key not set. Run: fetchr config set-key gemini <KEY>")?;

    let client = reqwest::Client::new();

    // Build the full prompt with optional learning context
    let full_prompt = format!(
        "{}{}{}{}",
        PROMPT_TEMPLATE,
        learning_context,
        PROMPT_SUFFIX,
        prompt
    );

    let request = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part { text: full_prompt }],
        }],
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let gemini_response = retry_request(MAX_RETRIES, || async {
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to call Gemini API")?;

        let status = response.status();
        if is_rate_limit_status(status) {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("rate_limit: Gemini API error ({}): {}", status, body);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({}): {}", status, body);
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini response")?;

        Ok(gemini_response)
    })
    .await?;

    let content = gemini_response
        .candidates
        .first()
        .context("No response from Gemini")?
        .content
        .parts
        .first()
        .context("No content in Gemini response")?
        .text
        .clone();

    // Clean up the response (remove markdown code blocks if present)
    let content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse the JSON object from the response
    let expanded: ExpandedQuery = serde_json::from_str(content)
        .with_context(|| format!("Failed to parse AI response as JSON: {}", content))?;

    Ok(expanded)
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
