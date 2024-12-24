use tokio::sync::OnceCell;

// cache the provider too
static TOKEN_STORE: OnceCell<CachedGoogleCloudToken> = OnceCell::const_new();

pub async fn get_token_cache() -> Result<String, gcp_auth::Error> {
    if let Some(cached_token) = TOKEN_STORE.get() {
        let now = chrono::Utc::now();
        if cached_token.expiry > now + chrono::Duration::minutes(5) {
            return Ok(cached_token.token.clone());
        }
    }
    
    // Token doesn't exist or is expiring soon, fetch a new one
    let token_provider = gcp_auth::provider().await?;
    let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
    let token = token_provider.token(scopes).await?;
    
    // Cache the new token
    let new_token = CachedGoogleCloudToken {
        token: token.as_str().to_string(),
        expiry: chrono::Utc::now() + chrono::Duration::seconds(3600 - 300),
    };
    
    // Initialize or update the token
    if TOKEN_STORE.get().is_none() {
        _ = TOKEN_STORE.set(new_token.clone());
    } else {
        // Use async block to return a future
        TOKEN_STORE.get_or_init(|| async { new_token.clone() }).await;
    }
    
    Ok(token.as_str().to_string())
}

#[derive(Debug, Clone)]
struct CachedGoogleCloudToken {
    token: String,
    expiry: chrono::DateTime<chrono::Utc>,
}