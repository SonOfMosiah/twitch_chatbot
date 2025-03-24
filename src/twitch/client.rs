use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
// Just import the UnboundedReceiver which is what we need
use crate::config::Config;
use crate::twitch::helix::HelixChatClient;
use crate::twitch::oauth::OAuthManager;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info, warn};
use twitch_irc::ClientConfig;
use twitch_irc::TwitchIRCClient;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::transport::tcp::{NoTLS, TCPTransport};

// Now using the types from twitch_api crate

/// Represents a connection to Twitch chat
#[derive(Clone)]
pub struct TwitchClient {
    /// IRC client for traditional chat operations
    inner: TwitchIRCClient<TCPTransport<NoTLS>, StaticLoginCredentials>,
    /// OAuth manager for authentication
    oauth_manager: Arc<Mutex<OAuthManager>>,
    /// Helix API client for modern chat operations
    helix: Arc<Mutex<HelixChatClient>>,
}

impl TwitchClient {
    /// Create a new Twitch client with the given configuration
    ///
    /// # Arguments
    /// * `config` - The configuration for connecting to Twitch
    /// * `oauth_manager` - The OAuth manager for authentication
    ///
    /// # Returns
    /// A new TwitchClient instance with IRC and Helix API capabilities
    pub async fn new(
        config: &Config,
        oauth_manager: Arc<Mutex<OAuthManager>>,
    ) -> Result<(UnboundedReceiver<ServerMessage>, Self)> {
        // Get the current access token
        let token = {
            let mut manager = oauth_manager.lock().await;
            manager.get_access_token().await?
        };

        // Create the IRC client with static credentials
        let client_config = ClientConfig::new_simple(StaticLoginCredentials::new(
            config.bot_username.clone(),
            Some(token),
        ));

        let (incoming_messages, inner) =
            TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);

        // Create Helix API client
        let helix = HelixChatClient::new(oauth_manager.clone()).await?;

        Ok((
            incoming_messages,
            TwitchClient {
                inner,
                oauth_manager: oauth_manager.clone(),
                helix: Arc::new(Mutex::new(helix)),
            },
        ))
    }

    /// Recreate the client with a fresh token
    async fn recreate_client(&mut self, username: &str) -> Result<()> {
        info!("Refreshing OAuth token and recreating IRC client");

        // Get a fresh token
        let token = {
            let mut manager = self.oauth_manager.lock().await;
            manager.get_access_token().await?
        };

        // Create a new IRC client with the fresh token
        let client_config = ClientConfig::new_simple(StaticLoginCredentials::new(
            username.to_string(),
            Some(token),
        ));

        let (_incoming_messages, inner) =
            TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);

        // Replace the inner client
        self.inner = inner;

        // The Helix client doesn't need to be recreated as it will automatically
        // get fresh tokens via the shared OAuth manager

        Ok(())
    }

    /// Create a new Twitch client with static credentials (used for testing)
    ///
    /// # Arguments
    /// * `username` - The bot's username
    /// * `token` - The OAuth token
    ///
    /// # Returns
    /// A new TwitchClient instance
    #[allow(dead_code)]
    pub fn new_with_static_auth(
        username: &str,
        token: &str,
    ) -> (UnboundedReceiver<ServerMessage>, Self) {
        let client_config = ClientConfig::new_simple(StaticLoginCredentials::new(
            username.to_string(),
            Some(token.to_string()),
        ));

        let (incoming_messages, inner) =
            TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);

        // Create a dummy OAuth manager
        let oauth_manager = Arc::new(Mutex::new(OAuthManager::new(
            "dummy".to_string(),
            vec!["chat:read".to_string(), "chat:edit".to_string()],
        )));

        // For testing, we'll use a placeholder for the Helix client
        // In real code, this would be constructed properly, but for tests
        // we keep it simple
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dummy_helix = rt.block_on(async {
            match HelixChatClient::new(oauth_manager.clone()).await {
                Ok(client) => client,
                Err(_) => panic!("Failed to create dummy Helix client for testing"),
            }
        });

        (
            incoming_messages,
            TwitchClient {
                inner,
                oauth_manager: oauth_manager.clone(),
                helix: Arc::new(Mutex::new(dummy_helix)),
            },
        )
    }

    /// Join a Twitch channel
    ///
    /// # Arguments
    /// * `channel` - The channel name to join
    /// * `username` - The bot's username (needed for token refresh)
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn join_channel(&mut self, channel: &str, username: &str) -> Result<()> {
        // Log that we're trying to join
        info!("Attempting to join channel: {}", channel);

        // The Twitch IRC library wants lowercase channel name without # prefix
        let channel_name = if channel.starts_with('#') {
            channel.trim_start_matches('#').to_lowercase()
        } else {
            channel.to_lowercase()
        };

        info!("Formatted channel name for joining: {}", channel_name);

        let join_result = self.inner.join(channel_name.clone());

        // Check if join failed due to auth issues
        if let Err(e) = &join_result {
            warn!("Join failed with error: {}", e);
            if e.to_string().contains("authentication") {
                warn!(
                    "Join failed due to authentication issue, refreshing token: {}",
                    e
                );
                self.recreate_client(username).await?;
                let retry_result = self.inner.join(channel_name.clone());
                if let Err(retry_err) = retry_result {
                    warn!("Join retry failed after token refresh: {}", retry_err);
                } else {
                    info!("Join retry succeeded after token refresh");
                }
            }
        } else {
            info!("Successfully joined channel: {}", channel_name);
        }

        Ok(())
    }

    /// Send a message to a channel
    ///
    /// # Arguments
    /// * `channel` - The channel to send the message to
    /// * `message` - The message to send
    /// * `username` - The bot's username (needed for token refresh)
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn send_message(
        &mut self,
        channel: &str,
        message: &str,
        username: &str,
    ) -> Result<()> {
        // The Twitch IRC library wants lowercase channel name without # prefix
        let channel_name = if channel.starts_with('#') {
            channel.trim_start_matches('#').to_lowercase()
        } else {
            channel.to_lowercase()
        };

        info!("Sending message to {}: {}", channel_name, message);

        // First try to send via IRC for backward compatibility
        match self
            .inner
            .say(channel_name.clone(), message.to_string())
            .await
        {
            Ok(_) => {
                info!("Successfully sent message to {} via IRC", channel_name);
                return Ok(());
            }
            Err(e) => {
                // If it's an auth error, try refreshing the token and retry via IRC
                if e.to_string().contains("authentication") {
                    warn!("Message send failed due to authentication issue, refreshing token");
                    if let Err(e) = self.recreate_client(username).await {
                        warn!("Failed to refresh token: {}", e);
                        // Continue to API fallback
                    } else {
                        // Retry via IRC
                        match self
                            .inner
                            .say(channel_name.clone(), message.to_string())
                            .await
                        {
                            Ok(_) => {
                                info!("Successfully sent message after token refresh");
                                return Ok(());
                            }
                            Err(retry_e) => {
                                warn!(
                                    "Still failed to send message after token refresh: {}",
                                    retry_e
                                );
                                // Continue to API fallback
                            }
                        }
                    }
                } else {
                    warn!("Failed to send message via IRC: {}", e);
                    // Continue to API fallback
                }
            }
        }

        // Fallback to using the Helix API
        info!(
            "Falling back to Helix API for sending message to {}",
            channel_name
        );

        // Try to send via the Twitch Helix API
        let mut helix = self.helix.lock().await;
        match helix.send_chat_message(&channel_name, message, None).await {
            Ok(_) => {
                info!(
                    "Successfully sent message via Helix API to {}",
                    channel_name
                );
                Ok(())
            }
            Err(api_e) => {
                error!("Failed to send message via Helix API: {}", api_e);
                Err(anyhow!("Failed to send message: {}", api_e))
            }
        }
    }

    /// Send a reply to a specific message in a channel using Twitch API
    ///
    /// # Arguments
    /// * `channel` - The channel to send the message to
    /// * `message` - The message to send
    /// * `reply_to` - The message ID to reply to
    /// * `username` - The bot's username (needed for token refresh) - not used anymore
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn send_reply(
        &mut self,
        channel: &str,
        message: &str,
        reply_to: &str,
        _username: &str,
    ) -> Result<()> {
        // Ensure channel name is correctly formatted (without # prefix)
        let channel_name = if channel.starts_with('#') {
            channel.trim_start_matches('#').to_lowercase()
        } else {
            channel.to_lowercase()
        };

        info!(
            "Sending Helix API reply to message ID {} in {}: {}",
            reply_to, channel_name, message
        );

        // We'll use the Helix API client to send a reply
        let mut helix = self.helix.lock().await;
        match helix.send_reply(&channel_name, message, reply_to).await {
            Ok(_) => {
                info!("Successfully sent reply via Helix API");
                Ok(())
            }
            Err(api_e) => {
                error!("Failed to send reply via Helix API: {}", api_e);
                Err(anyhow!("Failed to send reply: {}", api_e))
            }
        }
    }

    /// Get the OAuth manager used by this client
    ///
    /// # Returns
    /// The OAuth manager
    #[allow(dead_code)]
    pub fn get_oauth_manager(&self) -> Arc<Mutex<OAuthManager>> {
        self.oauth_manager.clone()
    }

    /// Get the Helix API client for direct access to Twitch API functions
    ///
    /// # Returns
    /// The Helix client
    #[allow(dead_code)]
    pub fn get_helix_client(&self) -> Arc<Mutex<HelixChatClient>> {
        self.helix.clone()
    }
}

#[cfg(test)]
mod tests {
    // Note: Testing actual Twitch connection would require mocking the Twitch API
    // These tests will be added later
}
