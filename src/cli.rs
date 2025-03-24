use clap::{Parser, Subcommand};

/// A Twitch chatbot that runs locally
#[derive(Parser, Debug)]
#[command(name = "som_chatbot")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A Twitch chatbot that runs locally", long_about = None)]
pub struct Cli {
    /// Sets a custom config file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,

    /// Enable debug mode
    #[arg(short, long)]
    pub debug: bool,

    /// The command prefix for the bot
    #[arg(short, long, default_value = "!")]
    pub prefix: String,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the bot
    Start {
        /// Channel to join (overrides config file)
        #[arg(short, long)]
        channel: Option<String>,
    },

    /// Generate a sample .env file
    GenEnv {
        /// Path to output the sample .env file
        #[arg(default_value = ".env.example")]
        path: String,
    },

    /// Authenticate with Twitch (get new tokens)
    Auth {
        /// Force re-authentication even if tokens exist
        #[arg(short, long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
