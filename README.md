# SOM Chatbot

A simple Twitch chatbot that runs locally on your machine.

## Features

- Connect to Twitch chat
- Execute custom commands
- Simple command system with modular design
- CLI interface with command-line options

## Built-in Commands

- `!ping` - Responds with "Pong!"
- `!uptime` - Shows how long the bot has been running
- `!help` - Shows help information for available commands

## Requirements

- Rust (https://www.rust-lang.org/tools/install)
- A Twitch account for the bot
- OAuth token for the bot account

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

### Start the bot

```
cargo run -- start
```

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
  -c, --config <FILE>  Sets a custom config file path
  -d, --debug          Enable debug mode
  -p, --prefix <PREFIX>  The command prefix for the bot [default: !]
  -h, --help           Print help
  -V, --version        Print version
```

## Development

### Running Tests

```
cargo test
```

### Adding New Commands

To add a new command, create a new struct that implements the `Command` trait, and register it in the `main.rs` file.

## License

MIT

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.