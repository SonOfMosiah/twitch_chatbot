use anyhow::Result;
use rand::prelude::{SliceRandom, IndexedRandom};
use rand::{rng};
use twitch_irc::message::PrivmsgMessage;

use crate::commands::Command;

/// Possible response types for the 8-ball
enum ResponseType {
    Affirmative,
    Negative,
    Neutral,
    Uncertain,
}

/// A command that simulates a Magic 8-Ball
pub struct EightBallCommand {
    // All possible responses organized by type
    responses: Vec<(ResponseType, Vec<&'static str>)>,
}

impl Default for EightBallCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl EightBallCommand {
    /// Create a new 8-ball command with default responses
    pub fn new() -> Self {
        let responses = vec![
            (
                ResponseType::Affirmative,
                vec![
                    "It is certain.",
                    "It is decidedly so.",
                    "Without a doubt.",
                    "Yes definitely.",
                    "You may rely on it.",
                    "As I see it, yes.",
                    "Most likely.",
                    "Outlook good.",
                    "Yes.",
                    "Signs point to yes.",
                ],
            ),
            (
                ResponseType::Negative,
                vec![
                    "Don't count on it.",
                    "My reply is no.",
                    "My sources say no.",
                    "Outlook not so good.",
                    "Very doubtful.",
                ],
            ),
            (
                ResponseType::Neutral,
                vec![
                    "Reply hazy, try again.",
                    "Ask again later.",
                    "Better not tell you now.",
                    "Cannot predict now.",
                    "Concentrate and ask again.",
                ],
            ),
            (
                ResponseType::Uncertain,
                vec![
                    "Maybe...",
                    "I'm not sure about that.",
                    "The answer is unclear.",
                    "Could go either way.",
                    "The future is uncertain on this.",
                ],
            ),
        ];

        EightBallCommand { responses }
    }

    /// Get a random response from the 8-ball
    ///
    /// # Arguments
    /// * `question` - The question being asked (for future AI integration)
    ///
    /// # Returns
    /// A string response to the question
    fn get_random_response(&self, _question: &str) -> String {
        let mut rng = rng();
        
        // First, choose a random response type
        if let Some((_, responses)) = self.responses.choose(&mut rng) {
            // Then choose a random response of that type
            if let Some(response) = responses.choose(&mut rng) {
                return response.to_string();
            }
        }
        
        // Fallback in case something goes wrong
        "The magic 8-ball is cloudy right now.".to_string()
    }
    
    /// In the future, this could be replaced with an AI-based response selector
    fn get_response(&self, question: &str) -> String {
        // Currently just returns a random response
        // This method exists to make it easier to add AI capabilities later
        self.get_random_response(question)
    }
}

impl Command for EightBallCommand {
    fn execute(&self, msg: &PrivmsgMessage, args: Vec<&str>) -> Result<Option<String>> {
        // If there are no arguments, prompt for a question
        if args.is_empty() {
            return Ok(Some("Ask me a question and I shall reveal your fate!".to_string()));
        }
        
        // Join all arguments to form the question
        let question = args.join(" ");
        
        // Get a response from the 8-ball
        let response = self.get_response(&question);
        
        // Format the response
        let username = &msg.sender.name;
        // todo: might want to change the format here to just return the response
        Ok(Some(format!("@{} asked: {} 🎱 {}", username, question, response)))
    }

    fn help(&self) -> &str {
        "Ask the Magic 8-Ball a yes/no question. Usage: !8ball <question>"
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
            params: vec!["#test_channel".to_string(), "!8ball Will I win?".to_string()],
        };
        
        PrivmsgMessage {
            channel_login: "test_channel".to_string(),
            message_text: "!8ball Will I win?".to_string(),
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
    fn test_eight_ball_command_no_args() {
        let command = EightBallCommand::new();
        let msg = create_dummy_privmsg();
        
        // Execute the command with no arguments
        let result = command.execute(&msg, Vec::new()).unwrap();
        assert_eq!(result, Some("Ask me a question and I shall reveal your fate!".to_string()));
    }
    
    #[test]
    fn test_eight_ball_command_with_question() {
        let command = EightBallCommand::new();
        let msg = create_dummy_privmsg();
        
        // Execute the command with a question
        let result = command.execute(&msg, vec!["Will", "I", "win?"]).unwrap();
        
        // We can't check the exact response since it's random, but we can check the format
        let result = result.unwrap();
        assert!(result.starts_with("@Test_User asked: Will I win? 🎱 "));
        assert!(result.len() > 30); // Make sure there's a substantial response
    }
}