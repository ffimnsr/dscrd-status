use anyhow::{bail, Result};
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod fingerprint;
mod gateway;
mod heartbeat;
mod presence;
mod scraper;

/// Discord presence keeper — maintains a user's online status via the Gateway.
///
/// ⚠️  For educational/research purposes only.
///     Using this tool is against Discord's Terms of Service.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Discord user token (or set DISCORD_TOKEN env var)
    #[arg(short, long)]
    token: Option<String>,

    /// Status to maintain: online, idle, dnd
    #[arg(short, long, default_value = "online")]
    status: String,

    /// Enable verbose (debug) logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (non-fatal if missing).
    let _ = dotenv::dotenv();

    let args = Args::parse();

    // Configure logging level.
    let filter = if args.verbose {
        "dscrd_status=debug,info"
    } else {
        "dscrd_status=info,warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();

    // Resolve the Discord token.
    let token = match args.token.or_else(|| std::env::var("DISCORD_TOKEN").ok()) {
        Some(t) if !t.is_empty() => t,
        _ => bail!(
            "Discord token is required. Pass --token <TOKEN> or set DISCORD_TOKEN env var."
        ),
    };

    // Validate status argument.
    let status = args.status.to_lowercase();
    if !["online", "idle", "dnd", "invisible"].contains(&status.as_str()) {
        bail!(
            "Invalid status '{}'. Must be one of: online, idle, dnd, invisible",
            status
        );
    }

    info!("dscrd-status starting — target status: {}", status);
    info!("⚠️  This is for educational/research purposes only (against Discord ToS)");

    // Build HTTP client with a temporary UA for scraping.
    let http_client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/122.0.0.0 Safari/537.36",
        )
        .build()?;

    // Scrape the latest Discord build number.
    info!("Scraping Discord build number...");
    let discord_info = scraper::fetch_discord_info(&http_client).await;
    info!("Using build number: {}", discord_info.build_number);

    // Select a random browser fingerprint for this session.
    let fingerprint = fingerprint::select_fingerprint(discord_info.build_number);
    info!(
        "Selected fingerprint: {} on {} (build {})",
        fingerprint.browser_version, fingerprint.os, discord_info.build_number
    );

    // Handle SIGINT / SIGTERM for graceful shutdown.
    let shutdown = tokio::signal::ctrl_c();

    tokio::select! {
        result = gateway::run_gateway(token, status, fingerprint) => {
            if let Err(e) = result {
                error!("Gateway exited with error: {}", e);
                std::process::exit(1);
            }
        }
        _ = shutdown => {
            info!("Shutting down gracefully (SIGINT/SIGTERM received)");
        }
    }

    Ok(())
}
