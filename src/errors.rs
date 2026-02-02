use reqwest::StatusCode;

/// Format an API error into a user-friendly message
pub fn format_api_error(service: &str, status: StatusCode, body: &str) -> String {
    let (icon, explanation, hint) = match status {
        // Authentication errors
        StatusCode::UNAUTHORIZED => (
            "🔑",
            "Invalid API key",
            format!("Run: fetchr config set-key {} <YOUR_KEY>", service.to_lowercase()),
        ),
        StatusCode::FORBIDDEN => (
            "🚫",
            "Access denied",
            "Your API key may lack permissions or be revoked. Check your API dashboard.".to_string(),
        ),

        // Rate limiting
        StatusCode::TOO_MANY_REQUESTS => (
            "⏳",
            "Rate limit exceeded",
            "Too many requests. Wait a moment and try again.".to_string(),
        ),

        // Server errors
        StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT => (
            "🔧",
            "Service temporarily unavailable",
            format!("{} is experiencing issues. Try again in a few minutes.", service),
        ),
        StatusCode::INTERNAL_SERVER_ERROR => (
            "💥",
            "Server error",
            format!("{} encountered an internal error. This is not your fault.", service),
        ),

        // Client errors
        StatusCode::BAD_REQUEST => (
            "❌",
            "Invalid request",
            "The search query may contain invalid characters.".to_string(),
        ),
        StatusCode::NOT_FOUND => (
            "🔍",
            "Not found",
            "The API endpoint may have changed. Check for updates.".to_string(),
        ),

        // Payment/quota
        StatusCode::PAYMENT_REQUIRED => (
            "💳",
            "Payment required",
            "Your API quota may be exhausted. Check your billing.".to_string(),
        ),

        // Default
        _ => (
            "⚠️",
            "Request failed",
            format!("HTTP {} - check your internet connection", status.as_u16()),
        ),
    };

    // Build the message
    let mut msg = format!("{} {} error: {}", icon, service, explanation);

    // Add hint
    msg.push_str(&format!("\n   Hint: {}", hint));

    // Add technical details in debug mode (if body is useful)
    if !body.is_empty() && body.len() < 200 {
        // Try to extract a useful message from JSON error responses
        if let Some(extracted) = extract_error_message(body) {
            msg.push_str(&format!("\n   Detail: {}", extracted));
        }
    }

    msg
}

/// Try to extract a meaningful error message from JSON response body
fn extract_error_message(body: &str) -> Option<String> {
    // Try common JSON error formats
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        // OpenAI/Gemini style: {"error": {"message": "..."}}
        if let Some(msg) = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
            return Some(msg.to_string());
        }
        // Simple style: {"message": "..."}
        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
            return Some(msg.to_string());
        }
        // Serper style: {"error": "..."}
        if let Some(msg) = json.get("error").and_then(|m| m.as_str()) {
            return Some(msg.to_string());
        }
    }
    None
}

/// Format a network/connection error
pub fn format_network_error(service: &str, error: &reqwest::Error) -> String {
    let (icon, explanation, hint) = if error.is_timeout() {
        (
            "⏱️",
            "Connection timed out",
            format!("{} took too long to respond. Check your internet or try again.", service),
        )
    } else if error.is_connect() {
        (
            "🌐",
            "Connection failed",
            "Check your internet connection.".to_string(),
        )
    } else if error.is_decode() {
        (
            "📦",
            "Invalid response",
            format!("{} returned unexpected data. The API may have changed.", service),
        )
    } else {
        (
            "❌",
            "Network error",
            "An unexpected network error occurred.".to_string(),
        )
    };

    format!("{} {}: {}\n   Hint: {}", icon, service, explanation, hint)
}
