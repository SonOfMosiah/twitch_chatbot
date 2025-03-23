use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
// Just import the UnboundedReceiver which is what we need
use tokio::sync::mpsc::UnboundedReceiver;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::TwitchIRCClient;
use twitch_irc::transport::tcp::{TCPTransport, NoTLS};
use twitch_irc::ClientConfig;
use twitch_irc::message::ServerMessage;
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
    pub async fn new(config: &Config, oauth_manager: Arc<Mutex<OAuthManager>>) -> Result<(UnboundedReceiver<ServerMessage>, Self)> {
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

        let (incoming_messages, inner) = TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);

        Ok((incoming_messages, TwitchClient { 
            inner,
            oauth_manager,
        }))
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
    pub fn new_with_static_auth(username: &str, token: &str) -> (UnboundedReceiver<ServerMessage>, Self) {
        let client_config = ClientConfig::new_simple(
            StaticLoginCredentials::new(
                username.to_string(),
                Some(token.to_string()),
            )
        );

        let (incoming_messages, inner) = TwitchIRCClient::<TCPTransport<NoTLS>, StaticLoginCredentials>::new(client_config);
        
        // Create a dummy OAuth manager
        let oauth_manager = Arc::new(Mutex::new(
            OAuthManager::new(
                "dummy".to_string(),
                vec!["chat:read".to_string(), "chat:edit".to_string()]
            )
        ));

        (incoming_messages, TwitchClient { 
            inner,
            oauth_manager,
        })
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
                warn!("Join failed due to authentication issue, refreshing token: {}", e);
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
    pub async fn send_message(&mut self, channel: &str, message: &str, username: &str) -> Result<()> {
        // The Twitch IRC library wants lowercase channel name without # prefix
        let channel_name = if channel.starts_with('#') {
            channel.trim_start_matches('#').to_lowercase()
        } else {
            channel.to_lowercase()
        };
        
        info!("Sending message to {}: {}", channel_name, message);
        
        match self.inner.say(channel_name.clone(), message.to_string()).await {
            Ok(_) => {
                info!("Successfully sent message to {}", channel_name);
                Ok(())
            },
            Err(e) => {
                warn!("Failed to send message: {}", e);
                if e.to_string().contains("authentication") {
                    warn!("Message send failed due to authentication issue, refreshing token");
                    self.recreate_client(username).await?;
                    match self.inner.say(channel_name.clone(), message.to_string()).await {
                        Ok(_) => {
                            info!("Successfully sent message after token refresh");
                            Ok(())
                        },
                        Err(retry_e) => {
                            warn!("Still failed to send message after token refresh: {}", retry_e);
                            Err(anyhow::anyhow!("Failed to send message after token refresh: {}", retry_e))
                        }
                    }
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