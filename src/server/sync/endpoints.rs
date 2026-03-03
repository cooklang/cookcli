/// API endpoint for authentication and user management
#[cfg(debug_assertions)]
pub const API: &str = "http://localhost:3000/api";
#[cfg(not(debug_assertions))]
pub const API: &str = "https://cook.md/api";

/// Sync server endpoint for recipe synchronization
#[cfg(debug_assertions)]
pub const SYNC: &str = "http://localhost:8000";
#[cfg(not(debug_assertions))]
pub const SYNC: &str = "https://cook.md/api";

/// Get the API endpoint, with env var override for development
pub fn api_endpoint() -> String {
    if let Ok(ep) = std::env::var("COOK_API_ENDPOINT") {
        return ep.trim_end_matches('/').to_string();
    }
    if let Ok(base) = std::env::var("COOK_ENDPOINT") {
        return format!("{}/api", base.trim_end_matches('/'));
    }
    API.to_string()
}

/// Get the sync endpoint, with env var override for development
pub fn sync_endpoint() -> String {
    if let Ok(ep) = std::env::var("COOK_SYNC_ENDPOINT") {
        return ep.trim_end_matches('/').to_string();
    }
    if let Ok(base) = std::env::var("COOK_ENDPOINT") {
        let base = base.trim_end_matches('/');
        if base.contains("localhost") || base.contains("127.0.0.1") {
            return "http://127.0.0.1:8000".to_string();
        }
        return format!("{base}/api");
    }
    SYNC.to_string()
}

/// Get base URL (strip /api suffix) for auth redirects
pub fn base_url() -> String {
    let api = api_endpoint();
    api.strip_suffix("/api").unwrap_or(&api).to_string()
}
