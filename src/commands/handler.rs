use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use twitch_irc::message::PrivmsgMessage;
use tracing::{info, debug, error};

use crate::commands::CommandRegistry;
use crate::twitch::TwitchClient;

/// Handler for processing incoming chat messages and executing commands
pub struct CommandHandler {
    client: Arc<TwitchClient>,
    registry: Arc<RwLock<CommandRegistry>>,
    prefix: String,
    bot_username: String,
}

impl CommandHandler {
    /// Create a new command handler
    ///
    /// # Arguments
    /// * `client` - The Twitch client for sending messages
    /// * `registry` - The registry of available commands
    /// * `prefix` - The command prefix (e.g., "!")
    ///
    /// # Returns
    /// A new CommandHandler instance
    pub fn new(
        client: Arc<TwitchClient>,
        registry: Arc<RwLock<CommandRegistry>>,
        prefix: String,
        bot_username: String,
    ) -> Self {
        CommandHandler {
            client,
            registry,
            prefix,
            bot_username,
        }
    }

    /// Process an incoming chat message
    ///
    /// # Arguments
    /// * `msg` - The chat message to process
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn handle_message(&self, msg: PrivmsgMessage) -> Result<()> {
        let content = msg.message_text.trim();
        
        debug!("Processing message for commands: '{}'", content);
        debug!("Command prefix: '{}'", self.prefix);
        
        // Check if the message is a command (starts with the prefix)
        if !content.starts_with(&self.prefix) {
            debug!("Message does not start with prefix, ignoring");
            return Ok(());
        }
        
        // Parse the command name and arguments
        let without_prefix = &content[self.prefix.len()..];
        let parts: Vec<&str> = without_prefix.split_whitespace().collect();
        
        if parts.is_empty() {
            debug!("Empty command after prefix, ignoring");
            return Ok(());
        }
        
        let command_name = parts[0].to_lowercase();
        let args = if parts.len() > 1 { parts[1..].to_vec() } else { Vec::new() };
        
        debug!("Command name: '{}', args: {:?}", command_name, args);
        
        // Get the command from the registry
        let registry = self.registry.read().await;
        
        // Log available commands for debugging
        let available_commands = registry.get_command_names();
        debug!("Available commands: {:?}", available_commands);
        
        if let Some(command) = registry.get_command(&command_name) {
            info!("Found command '{}', executing", command_name);
            match command.execute(&msg, args) {
                Ok(Some(response)) => {
                    // Send the response to the chat as a reply to the original command
                    info!("Command '{}' returning response: '{}'", command_name, response);
                    let mut client = self.client.as_ref().clone();
                    
                    // For 8ball command, use reply to reference the original question
                    if command_name == "8ball" {
                        client.send_reply(&msg.channel_login, &response, &msg.message_id, &self.bot_username).await?;
                    } else {
                        // For other commands, use regular message
                        client.send_message(&msg.channel_login, &response, &self.bot_username).await?;
                    }
                }
                Ok(None) => {
                    // No response needed
                    debug!("Command '{}' executed with no response", command_name);
                }
                Err(e) => {
                    // Command execution failed
                    error!("Command '{}' execution failed: {}", command_name, e);
                }
            }
        } else {
            debug!("Command '{}' not found in registry", command_name);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Note: Testing CommandHandler would require mocking TwitchClient
    // These tests will be added later
}