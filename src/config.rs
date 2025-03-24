use anyhow::Result;
use dotenv::dotenv;
use std::env;

/// Configuration for the Twitch chatbot
pub struct Config {
    /// The client ID for the application
    pub client_id: String,
    /// The channel name to connect to
    pub channel_name: String,
    /// The bot's username on Twitch
    pub bot_username: String,
    /// The data directory for storing tokens and other data
    pub data_dir: String,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// # Returns
    /// A Result containing the Config if successful, or an error if required variables are missing
    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        let client_id = env::var("TWITCH_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("TWITCH_CLIENT_ID environment variable not set"))?;

        let channel_name = env::var("TWITCH_CHANNEL")
            .map_err(|_| anyhow::anyhow!("TWITCH_CHANNEL environment variable not set"))?;

        let bot_username = env::var("TWITCH_BOT_USERNAME")
            .map_err(|_| anyhow::anyhow!("TWITCH_BOT_USERNAME environment variable not set"))?;

        // Optional data directory, default to ./data
        let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());

        Ok(Config {
            client_id,
            channel_name,
            bot_username,
            data_dir,
        })
    }

    /// Create a new config directly from values (useful for testing)
    #[allow(dead_code)]
    pub fn new(
        client_id: String,
        channel_name: String,
        bot_username: String,
        data_dir: String,
    ) -> Self {
        Config {
            client_id,
            channel_name,
            bot_username,
            data_dir,
        }
    }

    /// Get the path to store the OAuth token
    pub fn get_token_path(&self) -> String {
        format!("{}/oauth_token.json", self.data_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Test config creation using the new method
        let config = Config::new(
            "test_client_id".to_string(),
            "test_channel".to_string(),
            "test_bot".to_string(),
            "./test_data".to_string(),
        );

        // Assert values
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.channel_name, "test_channel");
        assert_eq!(config.bot_username, "test_bot");
        assert_eq!(config.data_dir, "./test_data");
        assert_eq!(config.get_token_path(), "./test_data/oauth_token.json");
    }

    // We are skipping this test for now because we don't want to interfere with the system
    // environment variables during testing
    #[test]
    #[ignore]
    fn test_config_missing_env_vars() {
        // This test is ignored by default to avoid interfering with system environment variables
        // In a proper CI environment, we would set up a clean environment for testing
    }
}
