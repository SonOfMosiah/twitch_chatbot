mod handler;
mod basic;
mod eight_ball;

use twitch_irc::message::PrivmsgMessage;
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

pub use handler::CommandHandler;
pub use basic::{PingCommand, HelpCommand, UptimeCommand};
pub use eight_ball::EightBallCommand;

/// Trait for defining chat commands
pub trait Command: Send + Sync {
    /// Execute the command based on a chat message
    ///
    /// # Arguments
    /// * `msg` - The chat message that triggered the command
    /// * `args` - The arguments provided to the command
    ///
    /// # Returns
    /// A string response to send to the chat, or None if no response is needed
    fn execute(&self, msg: &PrivmsgMessage, args: Vec<&str>) -> Result<Option<String>>;

    /// Get the help text for this command
    fn help(&self) -> &str;
}

/// A registry of available commands
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    /// Create a new empty command registry
    pub fn new() -> Self {
        CommandRegistry {
            commands: HashMap::new(),
        }
    }

    /// Register a command with the given name
    ///
    /// # Arguments
    /// * `name` - The name of the command (without prefix)
    /// * `command` - The command implementation
    pub fn register<S: Into<String>>(&mut self, name: S, command: Arc<dyn Command>) {
        self.commands.insert(name.into(), command);
    }

    /// Check if a command exists in the registry
    ///
    /// # Arguments
    /// * `name` - The name of the command to check
    ///
    /// # Returns
    /// true if the command exists, false otherwise
    pub fn has_command<S: AsRef<str>>(&self, name: S) -> bool {
        self.commands.contains_key(name.as_ref())
    }

    /// Get a command from the registry
    ///
    /// # Arguments
    /// * `name` - The name of the command to get
    ///
    /// # Returns
    /// Some(command) if the command exists, None otherwise
    pub fn get_command<S: AsRef<str>>(&self, name: S) -> Option<Arc<dyn Command>> {
        self.commands.get(name.as_ref()).cloned()
    }
    
    /// Get all command names in the registry
    ///
    /// # Returns
    /// A vector of command names
    pub fn get_command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestCommand;

    impl Command for TestCommand {
        fn execute(&self, _msg: &PrivmsgMessage, args: Vec<&str>) -> Result<Option<String>> {
            Ok(Some(format!("Test command executed with {} args", args.len())))
        }

        fn help(&self) -> &str {
            "A test command"
        }
    }

    #[test]
    fn test_command_registry() {
        let mut registry = CommandRegistry::new();
        let cmd = Arc::new(TestCommand);
        
        // Register command
        registry.register("test", cmd);
        
        // Check if command exists
        assert!(registry.has_command("test"));
        assert!(!registry.has_command("unknown"));
        
        // Get command
        assert!(registry.get_command("test").is_some());
        assert!(registry.get_command("unknown").is_none());
    }
}