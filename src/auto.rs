// Auto mode: autonomous operation with smart retries
// This module will be implemented in the next phase

use anyhow::Result;
use crate::ai::ExpandedQuery;
use crate::config::Config;
use crate::search::ImageResult;

/// Reasons why a search result might be considered a failure
#[derive(Debug, Clone)]
pub enum FailureReason {
    NoResults,
    AllUrlsUnavailable,
    ImageTooSmall { width: u32, height: u32 },
}

/// Log entry for auto-mode decisions
#[derive(Debug, Clone)]
pub struct AutoDecision {
    pub query: String,
    pub action: String,
    pub reason: String,
}

/// Session log for transparency
#[derive(Debug, Default)]
pub struct AutoSession {
    pub decisions: Vec<AutoDecision>,
}

impl AutoSession {
    pub fn new() -> Self {
        Self { decisions: Vec::new() }
    }

    pub fn log(&mut self, query: &str, action: &str, reason: &str) {
        self.decisions.push(AutoDecision {
            query: query.to_string(),
            action: action.to_string(),
            reason: reason.to_string(),
        });
    }

    pub fn print_summary(&self) {
        if self.decisions.is_empty() {
            return;
        }
        println!("\n  \x1b[1mAuto-mode decisions:\x1b[0m");
        for decision in &self.decisions {
            println!(
                "    \x1b[90m[{}]\x1b[0m {} - {}",
                decision.query, decision.action, decision.reason
            );
        }
    }
}

/// Evaluate if an image result meets quality thresholds
pub fn evaluate_result(result: &ImageResult, _query: &str) -> Option<FailureReason> {
    // Check for minimum dimensions (icons should be at least 32x32, others 100x100)
    if result.width > 0 && result.height > 0 && (result.width < 32 || result.height < 32) {
        return Some(FailureReason::ImageTooSmall {
            width: result.width,
            height: result.height,
        });
    }
    None
}

/// Generate a reformulated query after a failure
pub async fn reformulate_query(
    original: &str,
    previous: &ExpandedQuery,
    failure: &FailureReason,
    attempt: u32,
    config: &Config,
) -> Result<ExpandedQuery> {
    // Build a hint based on the failure reason
    let hint = match failure {
        FailureReason::NoResults => "try alternative keywords or broader terms".to_string(),
        FailureReason::AllUrlsUnavailable => "try different image sources".to_string(),
        FailureReason::ImageTooSmall { width, height } => {
            format!("look for higher resolution images (was {}x{})", width, height)
        }
    };

    // Create a reformulation prompt
    let reformulation_prompt = format!(
        "{} (attempt {}: previous query '{}' failed - {})",
        original, attempt, previous.query, hint
    );

    // Use the AI to generate a new query
    crate::ai::expand_prompt(&reformulation_prompt, config).await
}

/// Find an image with automatic retry on failure
pub async fn find_with_retry(
    query: &str,
    config: &Config,
    max_retries: u32,
    session: &mut AutoSession,
    verbose: bool,
) -> Result<Option<(ImageResult, ExpandedQuery)>> {
    let mut last_expanded: Option<ExpandedQuery> = None;
    let mut last_failure: Option<FailureReason> = None;

    for attempt in 1..=max_retries {
        if verbose {
            session.log(query, &format!("attempt {}", attempt), "starting search");
        }

        // Expand or reformulate the query
        let expanded = if attempt == 1 {
            crate::ai::expand_prompt(query, config).await?
        } else if let (Some(prev), Some(failure)) = (&last_expanded, &last_failure) {
            reformulate_query(query, prev, failure, attempt, config).await?
        } else {
            crate::ai::expand_prompt(query, config).await?
        };

        // Search for images
        let results = crate::search::search_images(&expanded, query, 5, config).await?;

        if results.is_empty() {
            last_failure = Some(FailureReason::NoResults);
            last_expanded = Some(expanded);
            if verbose {
                session.log(query, "no results", "will retry with reformulated query");
            }
            continue;
        }

        // Try each result, tracking why we reject them
        let mut had_quality_failure = false;
        let mut quality_failure: Option<FailureReason> = None;

        for result in results {
            // Check quality
            if let Some(failure) = evaluate_result(&result, query) {
                if verbose {
                    session.log(
                        query,
                        "rejected",
                        &format!("image too small: {}x{}", result.width, result.height),
                    );
                }
                had_quality_failure = true;
                quality_failure = Some(failure);
                continue;
            }

            // Check URL availability
            if check_url_available(&result.download_url).await {
                if verbose {
                    session.log(query, "found", &format!("selected: {}", result.title));
                }
                return Ok(Some((result, expanded)));
            } else if verbose {
                session.log(query, "url unavailable", &result.download_url);
            }
        }

        // Use quality failure if that was the issue, otherwise URLs were the problem
        last_failure = if had_quality_failure {
            quality_failure
        } else {
            Some(FailureReason::AllUrlsUnavailable)
        };
        last_expanded = Some(expanded);

        if verbose {
            session.log(query, "all urls failed", "will retry with reformulated query");
        }
    }

    if verbose {
        session.log(
            query,
            "gave up",
            &format!("after {} attempts", max_retries),
        );
    }

    Ok(None)
}

/// Quick HEAD request to check if a URL is accessible
async fn check_url_available(url: &str) -> bool {
    let client = reqwest::Client::new();
    match client.head(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
