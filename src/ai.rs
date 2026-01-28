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

const PROMPT_TEMPLATE: &str = r#"You are an AI Asset Scout. Your task is to take a user's short input and expand it into optimized search queries for image search engines.

Rules:
1. Identify the object (e.g., Braun Player, BMW Logo)
2. Identify the category (Logo, Product, Architecture, Icon, etc.)
3. Generate 3 search queries that include technical modifiers like 'high-res', 'transparent PNG', 'SVG vector', 'white background', or 'studio lighting'
4. If the input is a brand, prioritize official logos and current brand identity

Respond with ONLY a JSON array of 3 search strings, no other text.
Example: ["BMW official logo SVG transparent", "BMW 2024 roundel logo high-res png", "BMW brand guidelines logo"]

User input: "#;

pub async fn expand_prompt(prompt: &str, config: &Config) -> Result<Vec<String>> {
    let api_key = config
        .keys
        .gemini
        .as_ref()
        .context("Gemini API key not set. Run: fetchr config set-key gemini <KEY>")?;

    let client = reqwest::Client::new();

    let request = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: format!("{}{}", PROMPT_TEMPLATE, prompt),
            }],
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

    // Parse the JSON array from the response
    let queries: Vec<String> = serde_json::from_str(content)
        .with_context(|| format!("Failed to parse AI response as JSON array: {}", content))?;

    Ok(queries)
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
