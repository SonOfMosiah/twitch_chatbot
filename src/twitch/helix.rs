//! Twitch Helix API client functionality
//!
//! This module provides a client for interacting with the Twitch Helix API,
//! focusing on chat message operations like sending messages and replies.

use anyhow::{Result, anyhow};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::twitch::oauth::OAuthManager;

/// Response from Twitch API when sending a message
#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    data: Vec<MessageData>,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    message_id: String,
    is_sent: bool,
    drop_reason: Option<DropReason>,
}

#[derive(Debug, Deserialize)]
struct DropReason {
    code: String,
    message: String,
}

/// Request body for the send message API
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    broadcaster_id: String,
    sender_id: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_parent_message_id: Option<String>,
}

/// Twitch User data response
#[derive(Debug, Deserialize)]
struct UserResponse {
    data: Vec<UserData>,
}

#[derive(Debug, Deserialize)]
struct UserData {
    id: String,
    #[allow(dead_code)]
    login: String,
    #[allow(dead_code)]
    display_name: String,
}

/// Helix API-enabled Twitch client for chat operations
pub struct HelixChatClient {
    /// HTTP client for API calls
    http_client: HttpClient,
    /// OAuth token manager for authentication
    oauth_manager: Arc<Mutex<OAuthManager>>,
    /// Bot's Twitch user ID
    bot_user_id: Option<String>,
    /// Channel cache to avoid repeated API lookups
    channel_cache: std::collections::HashMap<String, String>,
}

impl HelixChatClient {
    /// Create a new Helix API client for chat operations
    ///
    /// # Arguments
    /// * `oauth_manager` - Manager for OAuth tokens
    ///
    /// # Returns
    /// A new HelixChatClient instance
    pub async fn new(oauth_manager: Arc<Mutex<OAuthManager>>) -> Result<Self> {
        // Create HTTP client with reasonable timeout
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            http_client,
            oauth_manager,
            bot_user_id: None,
            channel_cache: std::collections::HashMap::new(),
        })
    }

    /// Get the bot's user ID (cached or from API)
    async fn get_bot_user_id(&mut self) -> Result<String> {
        // Return cached value if available
        if let Some(id) = &self.bot_user_id {
            return Ok(id.clone());
        }

        // Get a fresh token
        let token = {
            let mut manager = self.oauth_manager.lock().await;
            manager.get_access_token().await?
        };

        // Get client ID for API request
        let client_id = {
            let manager = self.oauth_manager.lock().await;
            manager.get_client_id().to_string()
        };

        // Make the API call to get the bot's user ID
        let response = self
            .http_client
            .get("https://api.twitch.tv/helix/users")
            .header("Authorization", format!("Bearer {}", token))
            .header("Client-Id", client_id)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to get user ID: {}", error_text));
        }

        // Parse the response
        let users: UserResponse = response.json().await?;

        if users.data.is_empty() {
            return Err(anyhow!("No user data returned"));
        }

        // Cache and return the user ID
        let user_id = users.data[0].id.clone();
        self.bot_user_id = Some(user_id.clone());

        Ok(user_id)
    }

    /// Get a broadcaster's user ID from their username
    async fn get_broadcaster_id(&mut self, username: &str) -> Result<String> {
        // Check cache first
        if let Some(id) = self.channel_cache.get(username) {
            return Ok(id.clone());
        }

        // Get a fresh token
        let token = {
            let mut manager = self.oauth_manager.lock().await;
            manager.get_access_token().await?
        };

        // Get client ID for API request
        let client_id = {
            let manager = self.oauth_manager.lock().await;
            manager.get_client_id().to_string()
        };

        // Make the API call to get the broadcaster's user ID
        let response = self
            .http_client
            .get("https://api.twitch.tv/helix/users")
            .header("Authorization", format!("Bearer {}", token))
            .header("Client-Id", client_id)
            .query(&[("login", username)])
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to get broadcaster ID: {}", error_text));
        }

        // Parse the response
        let users: UserResponse = response.json().await?;

        if users.data.is_empty() {
            return Err(anyhow!("No user data found for {}", username));
        }

        let user_id = users.data[0].id.clone();

        // Cache the result
        self.channel_cache
            .insert(username.to_string(), user_id.clone());

        Ok(user_id)
    }

    /// Send a chat message via Helix API
    ///
    /// # Arguments
    /// * `channel` - Channel name (without # prefix)
    /// * `message` - Message text to send
    /// * `reply_to` - Optional message ID to reply to
    ///
    /// # Returns
    /// Result containing the sent message ID
    pub async fn send_chat_message(
        &mut self,
        channel: &str,
        message: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        // Get required IDs
        let bot_user_id = self.get_bot_user_id().await?;
        let broadcaster_id = self.get_broadcaster_id(channel).await?;

        // Get a fresh token
        let token = {
            let mut manager = self.oauth_manager.lock().await;
            manager.get_access_token().await?
        };

        // Get client ID for API request
        let client_id = {
            let manager = self.oauth_manager.lock().await;
            manager.get_client_id().to_string()
        };

        // Prepare the request body
        let request_body = SendMessageRequest {
            broadcaster_id,
            sender_id: bot_user_id,
            message: message.to_string(),
            reply_parent_message_id: reply_to.map(|s| s.to_string()),
        };

        // Make the API call
        info!("Sending message to {}: {}", channel, message);
        let response = self
            .http_client
            .post("https://api.twitch.tv/helix/chat/messages")
            .header("Authorization", format!("Bearer {}", token))
            .header("Client-Id", client_id)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("API error: {}", error_text);
            return Err(anyhow!("Failed to send message: {}", error_text));
        }

        // Parse the response
        let send_response: SendMessageResponse = response.json().await?;

        if send_response.data.is_empty() {
            return Err(anyhow!("No data returned from send message API"));
        }

        let message_data = &send_response.data[0];

        if !message_data.is_sent {
            if let Some(reason) = &message_data.drop_reason {
                return Err(anyhow!(
                    "Message not sent: {} - {}",
                    reason.code,
                    reason.message
                ));
            } else {
                return Err(anyhow!("Message not sent for unknown reason"));
            }
        }

        info!("Successfully sent message, ID: {}", message_data.message_id);
        Ok(message_data.message_id.clone())
    }

    /// Send a reply to a specific message
    ///
    /// # Arguments
    /// * `channel` - Channel name (without # prefix)
    /// * `message` - Message text to send
    /// * `reply_to` - Message ID to reply to
    ///
    /// # Returns
    /// Result containing the sent message ID
    pub async fn send_reply(
        &mut self,
        channel: &str,
        message: &str,
        reply_to: &str,
    ) -> Result<String> {
        self.send_chat_message(channel, message, Some(reply_to))
            .await
    }
}
