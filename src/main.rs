mod cli;
mod commands;
mod config;
#[cfg(test)]
mod test_helpers;
mod twitch;
mod users;

use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{Level, debug, error, info};
use tracing_subscriber::FmtSubscriber;
use twitch_irc::message::ServerMessage;

use cli::{Cli, Commands};
use commands::{
    CommandHandler, CommandRegistry, EightBallCommand, HelpCommand, PingCommand, UptimeCommand,
};
use config::Config;
use twitch::{OAuthManager, TwitchClient};
use users::{UserManager, WelcomeService};

/// The main entry point for the application
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.debug { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    match &cli.command {
        Some(Commands::Start { channel }) => {
            start_bot(cli.debug, cli.prefix.clone(), channel.clone()).await?;
        }
        Some(Commands::GenEnv { path }) => {
            generate_env_file(path)?;
        }
        Some(Commands::Auth { force }) => {
            authenticate(*force).await?;
        }
        None => {
            // Default to start command if no subcommand is specified
            start_bot(cli.debug, cli.prefix.clone(), None).await?;
        }
    }

    Ok(())
}

/// Authenticate with Twitch
///
/// # Arguments
/// * `force` - Force re-authentication even if tokens exist
///
/// # Returns
/// A Result indicating success or failure
async fn authenticate(force: bool) -> Result<()> {
    // Load configuration
    info!("Loading configuration");
    let config = Config::from_env()?;

    // Make sure data directory exists
    let data_dir = std::path::Path::new(&config.data_dir);
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
    }

    // Set up OAuth manager
    let oauth_scopes = vec![
        "chat:read".to_string(),
        "chat:edit".to_string(),
        "user:read:email".to_string(), // Needed to get the bot's user ID
        "user:write:chat".to_string(), // Needed for sending replies via Helix API
    ];

    let oauth_manager = Arc::new(Mutex::new(OAuthManager::new(
        config.client_id.clone(),
        oauth_scopes,
    )));

    // Try to load existing token if not forcing re-auth
    let token_path = config.get_token_path();
    let token_file = std::path::Path::new(&token_path);

    if !force && token_file.exists() {
        info!("Loading OAuth token from {}", token_path);
        if let Err(e) = oauth_manager.lock().await.load_token(&token_path) {
            error!("Failed to load token: {}", e);
            println!("Failed to load existing token, will re-authenticate.");
        } else {
            println!("Existing token loaded. Use --force to re-authenticate.");
            return Ok(());
        }
    }

    // Start authentication process
    info!("Starting authentication process");
    oauth_manager.lock().await.authenticate().await?;

    // Save the token for future use
    info!("Saving OAuth token to {}", token_path);
    oauth_manager.lock().await.save_token(&token_path)?;

    println!("Authentication successful! Token saved to {}", token_path);

    Ok(())
}

/// Start the bot with the given configuration
async fn start_bot(_debug: bool, prefix: String, channel_override: Option<String>) -> Result<()> {
    // Load configuration
    info!("Loading configuration");
    let mut config = Config::from_env()?;

    // Override channel if specified
    if let Some(channel) = channel_override {
        config.channel_name = channel;
    }

    info!("Starting SOM Chatbot");
    info!("Connecting to channel: {}", config.channel_name);

    // Make sure data directory exists
    let data_dir = std::path::Path::new(&config.data_dir);
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
    }

    // Set up OAuth manager
    let oauth_scopes = vec![
        "chat:read".to_string(),
        "chat:edit".to_string(),
        "user:read:email".to_string(), // Needed to get the bot's user ID
        "user:write:chat".to_string(), // Needed for sending replies via Helix API
    ];

    let oauth_manager = Arc::new(Mutex::new(OAuthManager::new(
        config.client_id.clone(),
        oauth_scopes,
    )));

    // Try to load existing token
    let token_path = config.get_token_path();
    let token_file = std::path::Path::new(&token_path);
    if token_file.exists() {
        info!("Loading OAuth token from {}", token_path);
        if let Err(e) = oauth_manager.lock().await.load_token(&token_path) {
            error!("Failed to load token: {}", e);
            // Continue to re-authenticate
        }
    }

    // Authenticate if needed
    if !oauth_manager.lock().await.is_authenticated() {
        info!("OAuth token not found or invalid, starting authentication");
        oauth_manager.lock().await.authenticate().await?;

        // Save the token for future use
        info!("Saving OAuth token to {}", token_path);
        oauth_manager.lock().await.save_token(&token_path)?;
    }

    // Create Twitch client with OAuth
    let (incoming_messages, mut client) = TwitchClient::new(&config, oauth_manager.clone()).await?;

    // Set up user manager
    let users_file_path = format!("{}/known_users.txt", config.data_dir);
    let user_manager = Arc::new(UserManager::new(&users_file_path));

    // Load known users
    info!("Loading known users from {}", users_file_path);
    user_manager.load().await?;

    // Join channel
    client
        .join_channel(&config.channel_name, &config.bot_username)
        .await?;
    info!("Joined channel: {}", config.channel_name);

    // Create welcome service with random messages
    let welcome_service = Arc::new(WelcomeService::new(
        Arc::new(client.clone()),
        user_manager.clone(),
        None, // Use default random messages
    ));

    // Set up command registry
    let registry = CommandRegistry::new();
    let registry_arc = Arc::new(RwLock::new(registry));

    // Set up command descriptions for help command
    let descriptions = vec![
        ("ping".to_string(), "Responds with Pong!".to_string()),
        (
            "uptime".to_string(),
            "Shows how long the bot has been running".to_string(),
        ),
        (
            "help".to_string(),
            "Shows help information for available commands".to_string(),
        ),
        (
            "8ball".to_string(),
            "Ask the Magic 8-Ball a yes/no question. Usage: !8ball <question>".to_string(),
        ),
    ];

    // Create and register commands
    {
        let mut registry = registry_arc.write().await;
        registry.register("ping", Arc::new(PingCommand));
        registry.register("uptime", Arc::new(UptimeCommand::new()));
        registry.register("8ball", Arc::new(EightBallCommand::new()));
        registry.register(
            "help",
            Arc::new(HelpCommand::new(prefix.clone(), descriptions)),
        );

        info!(
            "Registered commands: ping, uptime, 8ball, help with prefix: '{}'",
            prefix
        );
    }

    // Create command handler
    let command_handler = Arc::new(CommandHandler::new(
        Arc::new(client.clone()),
        registry_arc.clone(),
        prefix,
        config.bot_username.clone(), // Pass bot username for responding
    ));

    // Set up message handling
    info!("Setting up message handling");

    // Clone services for the async block
    let welcome_service_clone = welcome_service.clone();
    let command_handler_clone = command_handler.clone();
    let user_manager_clone = user_manager.clone();
    let channel_name = config.channel_name.clone();

    // Spawn a task to process incoming messages
    tokio::spawn(async move {
        let mut incoming_messages = incoming_messages;

        info!("Waiting for messages...");

        // Add a test log every 10 seconds to confirm the task is still running
        let message_task = Arc::new(Mutex::new(0));
        let message_task_clone = message_task.clone();

        // Spawn a task to periodically log that we're still waiting for messages
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                let mut counter = message_task_clone.lock().await;
                *counter += 1;
                info!("Still waiting for messages... (heartbeat: {})", *counter);
            }
        });

        while let Some(msg) = incoming_messages.recv().await {
            info!("Received a message from Twitch: {:?}", msg);

            // Log every message we receive
            match &msg {
                ServerMessage::Privmsg(privmsg) => {
                    info!("[CHAT] {}: {}", privmsg.sender.name, privmsg.message_text);

                    // Process for welcome service
                    if let Err(e) = welcome_service_clone.process_message(privmsg.clone()).await {
                        error!("Error processing welcome: {}", e);
                    }

                    // Process for command handling
                    if let Err(e) = command_handler_clone.handle_message(privmsg.clone()).await {
                        error!("Error handling command: {}", e);
                    }
                }
                ServerMessage::Join(join) => {
                    info!("[JOIN] {} joined the channel", join.user_login);
                }
                ServerMessage::Part(part) => {
                    info!("[PART] {} left the channel", part.user_login);
                }
                ServerMessage::Notice(notice) => {
                    info!("[NOTICE] Channel {}: {}", channel_name, notice.message_text);
                }
                _ => {
                    debug!("Received other message type: {:?}", msg);
                }
            }
        }

        // Save known users when shutting down
        if let Err(e) = user_manager_clone.save().await {
            error!("Error saving known users: {}", e);
        }
    });

    // Send a message to the channel to indicate the bot is running
    client
        .send_message(
            &config.channel_name,
            "SOM Chatbot is now online!",
            &config.bot_username,
        )
        .await?;
    info!("Sent greeting message to channel: {}", config.channel_name);

    // Keep the application running
    info!("Bot is now running. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");

    // Save known users before exiting
    info!("Saving known users...");
    user_manager.save().await?;

    Ok(())
}

/// Generate a sample .env file
fn generate_env_file(path: &str) -> Result<()> {
    info!("Generating sample .env file at {}", path);

    let contents = r#"# Your Twitch client ID (get one from Twitch Developer Dashboard)
TWITCH_CLIENT_ID=your_client_id_here
# The channel to join
TWITCH_CHANNEL=channel_name
# The bot's username
TWITCH_BOT_USERNAME=your_bot_username
# Optional: Data directory for storing tokens and user data
# DATA_DIR=./data
"#;

    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())?;

    info!("Sample .env file generated successfully!");

    Ok(())
}
