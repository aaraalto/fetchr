use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Rating for a downloaded image
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rating {
    ThumbsUp,
    ThumbsDown,
    Skip,
}

/// Filters used during search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub img_size: Option<String>,
    pub img_type: Option<String>,
}

/// A single feedback entry for a downloaded image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub timestamp: DateTime<Utc>,
    pub original_query: String,
    pub expanded_query: String,
    pub filters: SearchFilters,
    pub image_url: String,
    pub image_title: String,
    pub rating: Rating,
}

/// Container for all feedback history
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FeedbackHistory {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<FeedbackEntry>,
}

impl FeedbackHistory {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}

/// Get the path to the history JSON file
fn history_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join("fetchr");
    Ok(config_dir.join("history.json"))
}

/// Load feedback history from disk
pub fn load_history() -> Result<FeedbackHistory> {
    let path = history_path()?;

    if path.exists() {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read history from {:?}", path))?;
        serde_json::from_str(&content).with_context(|| "Failed to parse history file")
    } else {
        Ok(FeedbackHistory::new())
    }
}

/// Save feedback history to disk
pub fn save_history(history: &FeedbackHistory) -> Result<()> {
    let path = history_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    let content = serde_json::to_string_pretty(history)
        .context("Failed to serialize history")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write history to {:?}", path))?;

    Ok(())
}

/// Append a single feedback entry to history
pub fn append_entry(entry: FeedbackEntry) -> Result<()> {
    let mut history = load_history()?;
    history.entries.push(entry);
    save_history(&history)?;
    Ok(())
}

/// Generate learning context for the AI prompt based on past feedback
/// Returns a formatted string with good and bad examples
pub fn get_learning_context(limit: usize) -> Result<Option<String>> {
    let history = load_history()?;

    if history.entries.is_empty() {
        return Ok(None);
    }

    // Collect good and bad examples (most recent first)
    let good_examples: Vec<&FeedbackEntry> = history
        .entries
        .iter()
        .rev()
        .filter(|e| e.rating == Rating::ThumbsUp)
        .take(limit)
        .collect();

    let bad_examples: Vec<&FeedbackEntry> = history
        .entries
        .iter()
        .rev()
        .filter(|e| e.rating == Rating::ThumbsDown)
        .take(limit)
        .collect();

    if good_examples.is_empty() && bad_examples.is_empty() {
        return Ok(None);
    }

    let mut context = String::from("\nBased on past feedback from the user:\n");

    if !good_examples.is_empty() {
        context.push_str("Good results (user liked these):\n");
        for entry in good_examples.iter().take(3) {
            context.push_str(&format!(
                "- \"{}\" -> \"{}\" [filters: size={}, type={}]\n",
                entry.original_query,
                entry.expanded_query,
                entry.filters.img_size.as_deref().unwrap_or("none"),
                entry.filters.img_type.as_deref().unwrap_or("none")
            ));
        }
    }

    if !bad_examples.is_empty() {
        context.push_str("Bad results (user disliked these - avoid similar patterns):\n");
        for entry in bad_examples.iter().take(3) {
            context.push_str(&format!(
                "- \"{}\" -> \"{}\" [filters: size={}, type={}]\n",
                entry.original_query,
                entry.expanded_query,
                entry.filters.img_size.as_deref().unwrap_or("none"),
                entry.filters.img_type.as_deref().unwrap_or("none")
            ));
        }
    }

    Ok(Some(context))
}

/// Get statistics about feedback history
pub fn get_stats() -> Result<(usize, usize, usize)> {
    let history = load_history()?;

    let thumbs_up = history.entries.iter().filter(|e| e.rating == Rating::ThumbsUp).count();
    let thumbs_down = history.entries.iter().filter(|e| e.rating == Rating::ThumbsDown).count();
    let skipped = history.entries.iter().filter(|e| e.rating == Rating::Skip).count();

    Ok((thumbs_up, thumbs_down, skipped))
}

/// Clear all feedback history
pub fn clear_history() -> Result<()> {
    let path = history_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete history file {:?}", path))?;
    }
    Ok(())
}
