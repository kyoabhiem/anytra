mod domain;
mod usecases;
mod infrastructure;
mod interface;

use clap::Parser;
use infrastructure::config::Config;
use infrastructure::logger::init_tracing;
use interface::mcp::server::run_stdio_server;
use std::time::Duration;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "anytra", about = "MCP server that enhances prompts")]
struct Cli {
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Optional: graceful shutdown timeout in seconds
    #[arg(long, default_value_t = 5)]
    shutdown_timeout: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load configuration from environment
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize logging with config
    init_tracing(&config.logging.level);

    info!("starting anytra");

    // Create provider with configuration
    let openrouter_config = config.openrouter.clone();
    let provider = match infrastructure::providers::openrouter::OpenRouterClient::new(openrouter_config) {
        Ok(c) => Box::new(c) as Box<dyn domain::llm::LLMProvider + Send + Sync>,
        Err(e) => {
            eprintln!("Failed to create OpenRouter client: {}", e);
            std::process::exit(1);
        }
    };

    let usecase = usecases::enhance_prompt::EnhancePrompt::new(provider, config);

    let timeout = Duration::from_secs(cli.shutdown_timeout);
    run_stdio_server(usecase, timeout).await
}
