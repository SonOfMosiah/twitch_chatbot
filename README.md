# SOM Chatbot

A Twitch chatbot with OAuth integration, welcome messages for first-time chatters, and expandable command system.

## Features

- Connect to Twitch chat using secure OAuth authentication
- Device Code Flow for easy authentication without exposing tokens
- Automatic token refresh when needed
- First-time chatter detection and welcome messages
- Expandable command system with modular design
- CLI interface with command-line options
- Persistence for known users
- Test suite with proper mocking

## Built-in Commands

- `!ping` - Responds with "Pong!"
- `!uptime` - Shows how long the bot has been running
- `!help` - Shows help information for available commands
- `!8ball [question]` - Ask the magic 8-ball a question and get a random response

## Requirements

- Rust (https://www.rust-lang.org/tools/install)
- A Twitch account for the bot
- Twitch Developer Application credentials
  - Register an application at: https://dev.twitch.tv/console/apps
  - Set the OAuth Redirect URL to: http://localhost
  - Note your Client ID (the bot handles authentication via Device Code Flow)

## Installation

1. Clone the repository
2. Build the project:
   ```
   cargo build --release
   ```
3. Create a `.env` file with your Twitch credentials (see `.env.example`)

## Usage

### Generate a sample .env file

```
cargo run -- gen-env
```

Edit the generated `.env` file with your Twitch Developer credentials:

```
TWITCH_CLIENT_ID=your_client_id_here
TWITCH_CHANNEL=channel_to_connect_to
TWITCH_BOT_USERNAME=your_bot_account_name
DATA_DIR=./data
```

### Authenticate

You can authenticate separately before starting the bot:

```
cargo run -- auth
```

If you've authenticated previously and need to update with new scopes:

```
cargo run -- auth --force
```

### Start the bot

```
cargo run -- start
```

The first time you run the bot, it will prompt you with a Twitch authorization URL and a code. Visit the URL on your browser, enter the code, and authorize the application. The bot will automatically store and refresh the tokens as needed.

> **Note about OAuth Scopes**: The bot requires several OAuth scopes, including `user:write:chat` for replying to messages. If you previously authorized the bot without this scope, you'll need to re-authenticate using `cargo run -- auth --force` to get a new token with all required scopes.

With debug output:

```
cargo run -- -d start
```

Connect to a specific channel (overrides config):

```
cargo run -- start -c channel_name
```

### Command-line Options

```
Usage: som_chatbot [OPTIONS] [COMMAND]

Commands:
  start    Start the bot
  gen-env  Generate a sample .env file
  help     Print this message or the help of the given subcommand(s)

Options:
  -c, --channel <CHANNEL>  The channel to connect to (overrides config)
  -d, --debug              Enable debug output
  -p, --prefix <PREFIX>    The command prefix for the bot [default: !]
  -h, --help               Print help
  -V, --version            Print version
```

## Development

### Running Tests

```
cargo test
```

### Code Style and Linting

This project uses rustfmt for code formatting and clippy for linting. Several helpful aliases are defined in `.cargo/config.toml`:

```bash
# Format all code according to the rustfmt.toml configuration
cargo fmt
# or with custom alias:
cargo format-all

# Run clippy with strict linting rules
cargo lint

# Run clippy with fix suggestions applied automatically
cargo fix

# Check all targets and features with pedantic warnings
cargo check-all
```

You can also run the standard commands:

```bash
# Check code formatting
cargo fmt --check

# Run clippy with default settings
cargo clippy
```

### Adding New Commands

To add a new command, create a new struct that implements the `Command` trait, and register it in the command registry in `main.rs`:

1. Create a new file in the `src/commands/` directory (see `eight_ball.rs` as an example)
2. Implement the `Command` trait for your new command
3. Add the command to the registry in `main.rs`:

```rust
// Register commands
let mut registry = CommandRegistry::new();
registry.register("ping", Box::new(PingCommand::new()));
registry.register("help", Box::new(HelpCommand::new()));
registry.register("8ball", Box::new(EightBallCommand::new()));
registry.register("your_command", Box::new(YourCommand::new()));  // Add your command here
```

### Working with OAuth

The bot uses the Device Code Flow for authentication, which is handled automatically. If you need to use the OAuth token in your commands, you can access it through the `TwitchClient`:

```rust
let oauth_manager = client.get_oauth_manager();
let token = oauth_manager.lock().await.get_access_token().await?;
```

### First-time Chatter Detection

The `WelcomeService` detects and welcomes first-time chatters. You can customize welcome messages or add AI-generated personalized welcomes by implementing the `get_ai_welcome_message` method.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

## Project Structure

- `src/`
  - `main.rs` - Entry point and application setup
  - `cli.rs` - Command-line interface with CLAP
  - `config.rs` - Configuration management
  - `commands/` - Chat command system
    - `mod.rs` - Command registry and trait definitions
    - `basic.rs` - Basic commands (ping, help, uptime)
    - `eight_ball.rs` - Magic 8-ball command
    - `handler.rs` - Command handler
  - `twitch/` - Twitch API integration
    - `mod.rs` - Twitch module exports
    - `client.rs` - Twitch chat client
    - `oauth.rs` - OAuth authentication flow
  - `users/` - User management
    - `mod.rs` - User tracking system
    - `welcome.rs` - First-time chatter welcome system