#![allow(dead_code)]
/// Test helpers for unit tests
use crate::config::Config;
use crate::twitch::{OAuthManager, TwitchClient};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create a test TwitchClient that doesn't actually connect to Twitch
pub fn create_test_client() -> TwitchClient {
    // Discard the message receiver as it's not needed for tests
    let (_, client) = TwitchClient::new_with_static_auth("test_bot", "test_token");
    client
}

/// Create a test config for unit tests
pub fn create_test_config() -> Config {
    Config::new(
        "test_client_id".to_string(),
        "test_channel".to_string(),
        "test_bot".to_string(),
        "./test_data".to_string(),
    )
}

/// Create a test OAuth manager
pub fn create_test_oauth_manager() -> Arc<Mutex<OAuthManager>> {
    Arc::new(Mutex::new(OAuthManager::new(
        "test_client_id".to_string(),
        vec!["chat:read".to_string(), "chat:edit".to_string()],
    )))
}
