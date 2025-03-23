use anyhow::Result;
use std::sync::Arc;
use std::any::Any;
use twitch_irc::message::PrivmsgMessage;
use tracing::{info, debug};
use rand::prelude::IndexedRandom;
use rand::rng;

use crate::twitch::TwitchClient;
use crate::users::UserManager;

/// Mock TwitchClient for testing
#[derive(Clone)]
pub struct MockTwitchClient {}

// Implement the necessary methods for MockTwitchClient
impl MockTwitchClient {
    // Mock the send_message method for testing
    pub async fn send_message(&mut self, _channel: &str, _message: &str, _username: &str) -> Result<()> {
        // Just return success without actually sending anything
        Ok(())
    }
}

/// Service to welcome new chatters in the channel
pub struct WelcomeService {
    /// The client for sending messages (can be TwitchClient or MockTwitchClient for testing)
    client: Arc<dyn Any + Send + Sync>,
    /// The user manager for tracking users
    user_manager: Arc<UserManager>,
    /// Whether the welcome feature is enabled
    enabled: bool,
    /// Welcome message templates (use {username} as placeholder)
    welcome_messages: Vec<String>,
    /// Whether to use AI for generating welcome messages
    use_ai: bool,
}

impl WelcomeService {
    /// Create a new welcome service
    ///
    /// # Arguments
    /// * `client` - The Twitch client for sending messages
    /// * `user_manager` - The user manager for tracking users
    /// * `custom_messages` - Optional list of custom welcome message templates (use {username} as placeholder)
    ///
    /// # Returns
    /// A new WelcomeService instance
    pub fn new(
        client: Arc<dyn Any + Send + Sync>,
        user_manager: Arc<UserManager>,
        custom_messages: Option<Vec<String>>,
    ) -> Self {
        // Default welcome messages if none provided
        let default_messages = vec![
            "Welcome to the channel, {username}! Thanks for dropping by!".to_string(),
            "Hey {username}! Great to see you here for the first time!".to_string(),
            "Welcome aboard, {username}! Hope you enjoy the stream!".to_string(),
            "A wild {username} appears! Welcome to the stream!".to_string(),
            "Welcome, {username}! Make yourself at home!".to_string(),
            "Thanks for joining us, {username}! Glad to have you here!".to_string(),
            "First time here, {username}? Welcome to the community!".to_string(),
            "{username} has entered the chat! Welcome!".to_string(),
            "Welcome to the stream, {username}! Don't forget to follow if you enjoy the content!".to_string(),
            "Hello, {username}! Welcome to the channel!".to_string(),
        ];
        
        WelcomeService {
            client,
            user_manager,
            enabled: true,
            welcome_messages: custom_messages.unwrap_or(default_messages),
            use_ai: false,
        }
    }

    /// Enable or disable the welcome service
    ///
    /// # Arguments
    /// * `enabled` - Whether the service should be enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Add a new welcome message template
    ///
    /// # Arguments
    /// * `message` - The welcome message template to add
    pub fn add_welcome_message(&mut self, message: String) {
        self.welcome_messages.push(message);
    }
    
    /// Set all welcome message templates
    ///
    /// # Arguments
    /// * `messages` - The new list of welcome message templates
    pub fn set_welcome_messages(&mut self, messages: Vec<String>) {
        self.welcome_messages = messages;
    }
    
    /// Toggle AI-generated welcome messages
    ///
    /// # Arguments
    /// * `use_ai` - Whether to use AI for generating welcome messages
    pub fn set_use_ai(&mut self, use_ai: bool) {
        self.use_ai = use_ai;
    }
    
    /// Get a random welcome message
    ///
    /// # Arguments
    /// * `username` - The username to insert into the message
    ///
    /// # Returns
    /// A personalized welcome message
    fn get_random_welcome_message(&self, username: &str) -> String {
        let mut rng = rng();
        
        // Get a random message template
        let template = if let Some(message) = self.welcome_messages.choose(&mut rng) {
            message
        } else {
            // Fallback if the messages list is somehow empty
            "Welcome, {username}!"
        };
        
        // Replace the placeholder with the actual username
        template.replace("{username}", username)
    }
    
    /// Get an AI-generated welcome message (placeholder for future implementation)
    ///
    /// # Arguments
    /// * `username` - The username to welcome
    ///
    /// # Returns
    /// A personalized welcome message
    async fn get_ai_welcome_message(&self, username: &str) -> Result<String> {
        // This is a placeholder for future AI implementation
        // For now, it just returns a static message with the username
        Ok(format!("AI welcomes you to the channel, {}! This message would normally be AI-generated.", username))
    }

    /// Process a chat message to detect and welcome first-time chatters
    ///
    /// # Arguments
    /// * `msg` - The chat message to process
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn process_message(&self, msg: &PrivmsgMessage) -> Result<()> {
        // Skip if the service is disabled
        if !self.enabled {
            return Ok(());
        }

        let user_id = &msg.sender.id;
        let username = &msg.sender.name;
        let channel = &msg.channel_login;

        // Check if this is a first-time chatter
        if self.user_manager.is_first_time_chatter(user_id) {
            info!("First-time chatter detected: {} ({})", username, user_id);
            
            // Get the welcome message (either AI-generated or random)
            let welcome_message = if self.use_ai {
                self.get_ai_welcome_message(username).await?
            } else {
                self.get_random_welcome_message(username)
            };
            
            // Send the welcome message
            debug!("Sending welcome message to: {}", username);
            
            // For actual TwitchClient: send the message 
            if let Some(_twitch_client) = self.client.downcast_ref::<TwitchClient>() {
                // Clone the client to make it mutable
                let client_arc = self.client.clone();
                let twitch_client = client_arc.downcast_ref::<TwitchClient>().unwrap();
                let mut client_mut = twitch_client.clone();
                // Use channel name for the bot username parameter - the actual bot username will be used
                client_mut.send_message(channel, &welcome_message, channel).await?;
            } 
            // For MockTwitchClient: handle in the mock implementation
            else if let Some(mock_client) = self.client.downcast_ref::<MockTwitchClient>() {
                let mut mock_client = mock_client.clone();
                mock_client.send_message(channel, &welcome_message, channel).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use twitch_irc::message::{IRCMessage, IRCTags, IRCPrefix, TwitchUserBasics};
    use chrono::Utc;
    use crate::config::Config;
    use std::sync::Arc;
    use tempfile::tempdir;

    // Helper to create a test message
    fn create_test_message(user_id: &str, username: &str) -> PrivmsgMessage {
        let irc_message = IRCMessage {
            tags: IRCTags::new(),
            prefix: Some(IRCPrefix::HostOnly {
                host: format!("{}!{}@{}.tmi.twitch.tv", username, username, username),
            }),
            command: "PRIVMSG".to_string(),
            params: vec!["#test_channel".to_string(), "Hello!".to_string()],
        };
        
        PrivmsgMessage {
            channel_login: "test_channel".to_string(),
            message_text: "Hello!".to_string(),
            sender: TwitchUserBasics {
                id: user_id.to_string(),
                login: username.to_lowercase(),
                name: username.to_string(),
            },
            source: irc_message,
            channel_id: "456".to_string(),
            message_id: "abc".to_string(),
            server_timestamp: Utc::now(),
            name_color: None,
            badges: Vec::new(),
            badge_info: Vec::new(),
            emotes: Vec::new(),
            bits: None,
            is_action: false,
        }
    }

    // Fixed async test with proper mocking
    #[tokio::test]
    async fn test_welcome_service_random() -> Result<()> {
        // Create a temporary directory for user data
        let temp_dir = tempdir()?;
        let users_path = temp_dir.path().join("users.txt");
        
        // Create mock client for testing
        let client = Arc::new(MockTwitchClient {});
        
        // Create user manager
        let user_manager = Arc::new(UserManager::new(users_path.to_str().unwrap()));
        
        // Create welcome service with custom messages
        let custom_messages = vec![
            "Welcome, {username}!".to_string(),
            "Hello, {username}!".to_string(),
        ];
        
        let welcome_service = WelcomeService::new(
            client.clone(),
            user_manager.clone(),
            Some(custom_messages)
        );
        
        // Process first message from user1 (should be welcomed)
        let msg1 = create_test_message("user1", "User1");
        welcome_service.process_message(&msg1).await?;
        
        // Process second message from user1 (should not be welcomed again)
        let msg2 = create_test_message("user1", "User1");
        welcome_service.process_message(&msg2).await?;
        
        // Process message from user2 (should be welcomed)
        let msg3 = create_test_message("user2", "User2");
        welcome_service.process_message(&msg3).await?;
        
        // Verify users are saved
        assert!(!user_manager.is_first_time_chatter("user1"));
        assert!(!user_manager.is_first_time_chatter("user2"));
        assert!(user_manager.is_first_time_chatter("user3"));
        
        Ok(())
    }
    
    // Fixed async test with proper mocking
    #[tokio::test]
    async fn test_welcome_service_ai() -> Result<()> {
        // Create a temporary directory for user data
        let temp_dir = tempdir()?;
        let users_path = temp_dir.path().join("users.txt");
        
        // Create mock client for testing
        let client = Arc::new(MockTwitchClient {});
        
        // Create user manager
        let user_manager = Arc::new(UserManager::new(users_path.to_str().unwrap()));
        
        // Create welcome service with AI enabled
        let mut welcome_service = WelcomeService::new(
            client.clone(),
            user_manager.clone(),
            None // Use default messages
        );
        
        // Enable AI
        welcome_service.set_use_ai(true);
        
        // Process message from user3 (should be welcomed with AI message)
        let msg = create_test_message("user3", "User3");
        welcome_service.process_message(&msg).await?;
        
        // Verify user is saved
        assert!(!user_manager.is_first_time_chatter("user3"));
        
        Ok(())
    }
    
    // MockTwitchClient is now defined outside this module
    
    #[test]
    fn test_random_welcome_message() {
        // No need for config or client connection
        let user_manager = Arc::new(UserManager::new("test.txt"));
        
        // Create service with just two messages for testing
        let messages = vec![
            "Welcome, {username}!".to_string(),
            "Hello, {username}!".to_string(),
        ];
        
        // Create a welcome service with direct access to the method
        let service = WelcomeService {
            client: Arc::new(MockTwitchClient {}),
            user_manager,
            enabled: true,
            welcome_messages: messages,
            use_ai: false,
        };
        
        // Get a random message
        let message = service.get_random_welcome_message("TestUser");
        
        // Check that it contains the username and is one of our templates
        assert!(
            message == "Welcome, TestUser!" || 
            message == "Hello, TestUser!"
        );
    }
}