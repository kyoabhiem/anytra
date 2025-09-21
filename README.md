# Anytra

A Rust-based MCP (Model Context Protocol) server that enhances your prompts using AI to make them clearer and more effective.

## What it does

This tool helps you improve your prompts before sending them to AI models. It can:

- Make prompts clearer and more specific
- Add context and structure
- Adjust tone and style
- Support different languages
- Provide quality assurance

## Quick Start

### Requirements
- Rust 1.75 or newer
- OpenRouter API key

### Installation & Setup

1. **Clone and build:**
   ```bash
   git clone <repository-url>
   cd anytra
   cargo build --release
   ```

2. **Get an OpenRouter API key:**
   - Sign up at [openrouter.ai](https://openrouter.ai)
   - Get your API key from the dashboard

3. **Set up environment:**
   ```bash
   export OPENROUTER_API_KEY=your_api_key_here
   # Optional: set model preference
   export OPENROUTER_MODEL=openrouter/auto
   ```

4. **Run the server:**
   ```bash
   cargo run -- --log-level info
   ```

## Usage

### Basic Enhancement
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"enhance_prompt","arguments":{"prompt":"write code for fibonacci"}}}' | cargo run --quiet --
```

### Advanced Enhancement
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"enhance_prompt","arguments":{"prompt":"explain quantum physics","goal":"make it simple","style":"concise","tone":"friendly","audience":"students"}}}' | cargo run --quiet --
```

## Integration

### With Claude Desktop
Add this to your Claude Desktop config:

```json
{
  "mcpServers": {
    "prompt-enhancer": {
      "command": "cargo",
      "args": ["run", "--quiet", "--release"],
      "env": {
        "OPENROUTER_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

### With Windsurf
Update your MCP config file:

```json
{
  "mcpServers": {
    "prompt-enhancer": {
      "command": "/path/to/mcp-prompt-server/target/release/mcp-prompt-server",
      "args": ["--log-level", "info"],
      "env": {
        "OPENROUTER_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

## Features

- **AI-Powered Enhancement**: Uses OpenRouter API for intelligent prompt improvement
- **Quality Validation**: Ensures enhanced prompts meet quality standards
- **Fallback Support**: Works even when AI services are unavailable
- **Flexible Options**: Customize enhancement with goals, styles, tones, and more
- **Multi-language Support**: Enhance prompts in different languages

## Command Line Options

- `--log-level <level>`: Set logging level (error, warn, info, debug, trace)
- `--shutdown-timeout <secs>`: Graceful shutdown timeout in seconds (default: 5)

## Environment Variables

- `OPENROUTER_API_KEY`: Required for AI enhancement
- `OPENROUTER_MODEL`: Optional model selection (default: openrouter/auto)
- `OPENROUTER_REFERER`: Optional, recommended for routing
- `OPENROUTER_TITLE`: Optional, recommended for routing

## Testing

Run tests:
```bash
cargo test
```

Test the server:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | cargo run --quiet --
```

## Project Structure

```
anytra/
├── src/
│   ├── domain/           # Core business logic
│   │   ├── models.rs     # Data structures
│   │   ├── llm.rs        # AI provider interface
│   │   ├── validation.rs # Quality checks
│   │   └── fewshot.rs    # Example prompts
│   ├── usecases/         # Application logic
│   │   └── enhance_prompt.rs
│   ├── infrastructure/   # External services
│   │   └── providers/
│   │       └── openrouter.rs
│   └── interface/        # MCP server
│       └── mcp/
│           └── server.rs
├── Cargo.toml
└── README.md
```
