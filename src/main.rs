mod domain;
mod usecases;
mod infrastructure;
mod interface;

use clap::Parser;
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
    init_tracing(&cli.log_level);

    info!("starting anytra");

    let provider = match infrastructure::providers::openrouter::OpenRouterClient::from_env() {
        Ok(c) => Box::new(c) as Box<dyn domain::llm::LLMProvider + Send + Sync>,
        Err(e) => {
            return Err(anyhow::anyhow!("OpenRouter not configured: {}", e));
        }
    };

    let usecase = usecases::enhance_prompt::EnhancePrompt::new(provider);

    let timeout = Duration::from_secs(cli.shutdown_timeout);
    run_stdio_server(usecase, timeout).await
}
