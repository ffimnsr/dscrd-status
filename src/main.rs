use anyhow::{bail, Result};
use clap::Parser;
use schedule::{Schedule, WindowState};
use tokio::time::Instant;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod fingerprint;
mod gateway;
mod heartbeat;
mod presence;
mod schedule;
mod scraper;

/// Discord presence keeper — maintains a user's online status via the Gateway.
///
/// ⚠️  For educational/research purposes only.
///     Using this tool is against Discord's Terms of Service.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Discord user token
    #[arg(short, long, env = "DISCORD_TOKEN")]
    token: Option<String>,

    /// Status to maintain: online, idle, dnd, invisible
    #[arg(short, long, default_value = "online", env = "DISCORD_STATUS")]
    status: String,

    /// Enable verbose (debug) logging
    #[arg(short, long, env = "DISCORD_VERBOSE")]
    verbose: bool,

    /// Local start time for the active window in 24-hour format, for example 09:00
    #[arg(long, env = "DISCORD_ACTIVE_START")]
    active_start: Option<String>,

    /// Local end time for the active window in 24-hour format, for example 17:30
    #[arg(long, env = "DISCORD_ACTIVE_END")]
    active_end: Option<String>,

    /// IANA timezone used for the active window, for example America/Chicago
    #[arg(long, env = "DISCORD_TIMEZONE")]
    timezone: Option<String>,
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

    // Resolve the Discord token (clap reads DISCORD_TOKEN env var automatically).
    let token = match args.token {
        Some(t) if !t.is_empty() => t,
        _ => bail!(
            "Discord token is required. Pass --token <TOKEN> or set DISCORD_TOKEN in .env / env."
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

    let schedule = Schedule::from_env(
        args.active_start.as_deref(),
        args.active_end.as_deref(),
        args.timezone.as_deref(),
    )?;

    info!("dscrd-status starting — target status: {}", status);
    info!("⚠️  This is for educational/research purposes only (against Discord ToS)");
    log_schedule(&schedule);

    loop {
        match schedule.state_at(chrono::Utc::now())? {
            WindowState::Always => {
                run_gateway_session(&token, &status).await?;
                break;
            }
            WindowState::Inactive {
                next_start,
                timezone,
            } => {
                let next_local = next_start.with_timezone(&timezone);
                info!(
                    "Outside active window. Sleeping until {} ({})",
                    next_local.format("%Y-%m-%d %H:%M:%S"),
                    timezone
                );

                tokio::select! {
                    result = shutdown_signal() => {
                        result?;
                        info!("Shutting down gracefully (SIGINT/SIGTERM received)");
                        break;
                    }
                    _ = tokio::time::sleep_until(instant_from_utc(next_start)) => {}
                }
            }
            WindowState::Active {
                window_end,
                timezone,
            } => {
                let end_local = window_end.with_timezone(&timezone);
                info!(
                    "Inside active window. Session will pause at {} ({})",
                    end_local.format("%Y-%m-%d %H:%M:%S"),
                    timezone
                );

                tokio::select! {
                    result = run_gateway_session(&token, &status) => {
                        if let Err(e) = result {
                            error!("Gateway exited with error: {}", e);
                            return Err(e);
                        }
                    }
                    result = shutdown_signal() => {
                        result?;
                        info!("Shutting down gracefully (SIGINT/SIGTERM received)");
                        break;
                    }
                    _ = tokio::time::sleep_until(instant_from_utc(window_end)) => {
                        info!("Active window ended. Disconnecting until the next scheduled start");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_gateway_session(token: &str, status: &str) -> Result<()> {
    let http_client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/122.0.0.0 Safari/537.36",
        )
        .build()?;

    info!("Scraping Discord build number...");
    let discord_info = scraper::fetch_discord_info(&http_client).await;
    info!("Using build number: {}", discord_info.build_number);

    let fingerprint = fingerprint::select_fingerprint(discord_info.build_number);
    info!(
        "Selected fingerprint: {} on {} (build {})",
        fingerprint.browser_version, fingerprint.os, discord_info.build_number
    );

    gateway::run_gateway(token.to_owned(), status.to_owned(), fingerprint).await
}

fn log_schedule(schedule: &Schedule) {
    match schedule {
        Schedule::Always => info!("Schedule: always active"),
        Schedule::Window {
            timezone,
            start,
            end,
        } => info!(
            "Schedule: active daily from {} to {} ({})",
            start.format("%H:%M"),
            end.format("%H:%M"),
            timezone
        ),
    }
}

fn instant_from_utc(target: chrono::DateTime<chrono::Utc>) -> Instant {
    let now = chrono::Utc::now();
    let delay = (target - now)
        .to_std()
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    Instant::now() + delay
}

async fn shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigterm = signal(SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
