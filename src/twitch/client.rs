use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::TwitchIRCClient;
use twitch_irc::transport::tcp::{TCPTransport, NoTLS};
use twitch_irc::ClientConfig;
use crate::config::Config;
use crate::twitch::oauth::OAuthManager;
use tracing::{info, warn};

/// Represents a connection to Twitch chat
#[derive(Clone)]
pub struct TwitchClient {
    inner: TwitchIRCClient<TCPTransport<NoTLS>, StaticLoginCredentials>,
    oauth_manager: Arc<Mutex<OAuthManager>>,
}

impl TwitchClient {
    /// Create a new Twitch client with the given configuration
    ///
    /// # Arguments
    /// * `config` - The configuration for connecting to Twitch
    /// * `oauth_manager` - The OAuth manager for authentication
    ///
    /// # Returns
    /// A new TwitchClient instance
    pub async fn new(config: &Config, oauth_manager: Arc<Mutex<OAuthManager>>) -> Result<Self> {
        // Get the current access token
        let token = {
            let mut manager = oauth_manager.lock().await;
            manager.get_access_token().await?
        };
        
        // Create the client with static credentials
        let client_config = ClientConfig::new_simple(
            StaticLoginCredentials::new(
                config.bot_username.clone(),
                Some(token),
            )
        );

        let (_incoming_messages, inner) = TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);

        Ok(TwitchClient { 
            inner,
            oauth_manager,
        })
    }
    
    /// Recreate the client with a fresh token
    async fn recreate_client(&mut self, username: &str) -> Result<()> {
        info!("Refreshing OAuth token and recreating client");
        
        // Get a fresh token
        let token = {
            let mut manager = self.oauth_manager.lock().await;
            manager.get_access_token().await?
        };
        
        // Create a new client with the fresh token
        let client_config = ClientConfig::new_simple(
            StaticLoginCredentials::new(
                username.to_string(),
                Some(token),
            )
        );

        let (_incoming_messages, inner) = TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);
        
        // Replace the inner client
        self.inner = inner;
        
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
    pub fn new_with_static_auth(username: &str, token: &str) -> Self {
        let client_config = ClientConfig::new_simple(
            StaticLoginCredentials::new(
                username.to_string(),
                Some(token.to_string()),
            )
        );

        let (_incoming_messages, inner) = TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);
        
        // Create a dummy OAuth manager
        let oauth_manager = Arc::new(Mutex::new(
            OAuthManager::new(
                "dummy".to_string(),
                vec!["chat:read".to_string(), "chat:edit".to_string()]
            )
        ));

        TwitchClient { 
            inner,
            oauth_manager,
        }
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
        let join_result = self.inner.join(channel.to_string());
        
        // Check if join failed due to auth issues
        if let Err(e) = &join_result {
            if e.to_string().contains("authentication") {
                warn!("Join failed due to authentication issue, refreshing token: {}", e);
                self.recreate_client(username).await?;
                let _ = self.inner.join(channel.to_string());
            }
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
    pub async fn send_message(&mut self, channel: &str, message: &str, username: &str) -> Result<()> {
        match self.inner.say(channel.to_string(), message.to_string()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.to_string().contains("authentication") {
                    warn!("Message send failed due to authentication issue, refreshing token: {}", e);
                    self.recreate_client(username).await?;
                    self.inner.say(channel.to_string(), message.to_string()).await?;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Failed to send message: {}", e))
                }
            }
        }
    }
    
    /// Get the OAuth manager used by this client
    ///
    /// # Returns
    /// The OAuth manager
    pub fn get_oauth_manager(&self) -> Arc<Mutex<OAuthManager>> {
        self.oauth_manager.clone()
    }
}

#[cfg(test)]
mod tests {
    // Note: Testing actual Twitch connection would require mocking the Twitch API
    // These tests will be added later
}