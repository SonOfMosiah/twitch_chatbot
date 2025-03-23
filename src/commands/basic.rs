use anyhow::Result;
use twitch_irc::message::PrivmsgMessage;
use crate::commands::Command;

/// A simple ping command that responds with "Pong!"
pub struct PingCommand;

impl Command for PingCommand {
    fn execute(&self, msg: &PrivmsgMessage, _args: Vec<&str>) -> Result<Option<String>> {
        // Echo the message and sender name to confirm we're receiving commands
        Ok(Some(format!("Pong! Received from {} who said: {}", msg.sender.name, msg.message_text)))
    }

    fn help(&self) -> &str {
        "Responds with Pong!"
    }
}

/// A command that displays help information for all commands
pub struct HelpCommand {
    prefix: String,
    descriptions: Vec<(String, String)>,
}

impl HelpCommand {
    /// Create a new help command
    ///
    /// # Arguments
    /// * `prefix` - The command prefix (e.g., "!")
    /// * `descriptions` - A list of (command_name, help_text) pairs
    ///
    /// # Returns
    /// A new HelpCommand instance
    pub fn new(prefix: String, descriptions: Vec<(String, String)>) -> Self {
        HelpCommand {
            prefix,
            descriptions,
        }
    }
}

impl Command for HelpCommand {
    fn execute(&self, _msg: &PrivmsgMessage, args: Vec<&str>) -> Result<Option<String>> {
        if args.is_empty() {
            // Show a list of all commands
            let commands: Vec<String> = self.descriptions
                .iter()
                .map(|(name, _)| format!("{}{}", self.prefix, name))
                .collect();
            
            Ok(Some(format!("Available commands: {}", commands.join(", "))))
        } else {
            // Show help for a specific command
            let command_name = args[0].to_lowercase();
            
            if let Some((_, help)) = self.descriptions
                .iter()
                .find(|(name, _)| name.to_lowercase() == command_name) {
                Ok(Some(help.clone()))
            } else {
                Ok(Some(format!("Unknown command: {}{}", self.prefix, command_name)))
            }
        }
    }

    fn help(&self) -> &str {
        "Shows help information for available commands"
    }
}

/// A command that shows information about uptime
pub struct UptimeCommand {
    started_at: std::time::Instant,
}

impl UptimeCommand {
    /// Create a new uptime command
    ///
    /// # Returns
    /// A new UptimeCommand instance
    pub fn new() -> Self {
        UptimeCommand {
            started_at: std::time::Instant::now(),
        }
    }
}

impl Command for UptimeCommand {
    fn execute(&self, _msg: &PrivmsgMessage, _args: Vec<&str>) -> Result<Option<String>> {
        let elapsed = self.started_at.elapsed();
        
        let hours = elapsed.as_secs() / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;
        let seconds = elapsed.as_secs() % 60;
        
        Ok(Some(format!("Bot has been running for {}h {}m {}s", hours, minutes, seconds)))
    }

    fn help(&self) -> &str {
        "Shows how long the bot has been running"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use twitch_irc::message::{IRCMessage, IRCTags, IRCPrefix, TwitchUserBasics, Badge, Emote};
    use chrono::Utc;
    
    fn create_dummy_privmsg() -> PrivmsgMessage {
        let irc_message = IRCMessage {
            tags: IRCTags::new(),
            prefix: Some(IRCPrefix::HostOnly {
                host: "test_user!test_user@test_user.tmi.twitch.tv".to_string(),
            }),
            command: "PRIVMSG".to_string(),
            params: vec!["#test_channel".to_string(), "!test".to_string()],
        };
        
        PrivmsgMessage {
            channel_login: "test_channel".to_string(),
            message_text: "!test".to_string(),
            sender: TwitchUserBasics {
                id: "123".to_string(),
                login: "test_user".to_string(),
                name: "Test_User".to_string(),
            },
            source: irc_message,
            channel_id: "456".to_string(),
            message_id: "abc".to_string(),
            server_timestamp: Utc::now(),
            name_color: None,
            badges: Vec::<Badge>::new(),
            badge_info: Vec::new(),
            emotes: Vec::<Emote>::new(),
            bits: None,
            is_action: false,
        }
    }
    
    #[test]
    fn test_ping_command() {
        let command = PingCommand;
        
        // Create a dummy message
        let msg = create_dummy_privmsg();
        
        // Execute the command
        let result = command.execute(&msg, Vec::new()).unwrap();
        
        // Assert the result contains "Pong!"
        assert!(result.unwrap().contains("Pong!"));
    }
    
    #[test]
    fn test_help_command() {
        let descriptions = vec![
            ("ping".to_string(), "Responds with Pong!".to_string()),
            ("help".to_string(), "Shows help information".to_string()),
        ];
        
        let command = HelpCommand::new("!".to_string(), descriptions);
        
        // Create a dummy message
        let msg = create_dummy_privmsg();
        
        // Execute the command with no args (list all commands)
        let result = command.execute(&msg, Vec::new()).unwrap();
        assert_eq!(result, Some("Available commands: !ping, !help".to_string()));
        
        // Execute the command with a specific command
        let result = command.execute(&msg, vec!["ping"]).unwrap();
        assert_eq!(result, Some("Responds with Pong!".to_string()));
    }
}