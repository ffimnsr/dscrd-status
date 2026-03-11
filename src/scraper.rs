use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;
use tracing::{debug, warn};

const FALLBACK_BUILD_NUMBER: u64 = 508471;
const DISCORD_LOGIN_URL: &str = "https://discord.com/login";

static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
static BUILD_RE: OnceLock<Regex> = OnceLock::new();

fn script_re() -> &'static Regex {
    SCRIPT_RE
        .get_or_init(|| Regex::new(r#"(/assets/[^"'<>[:space:]]+\.js)"#).expect("script regex is valid"))
}

fn build_re() -> &'static Regex {
    BUILD_RE.get_or_init(|| {
        Regex::new(
            r#"(?:YA\("buildNumber","(\d+)"\)|buildNumber:"(\d+)"|build_number:(\d+)|"BUILD_NUMBER","(\d+)")"#,
        )
        .expect("build number regex is valid")
    })
}

pub struct DiscordInfo {
    pub build_number: u64,
}

/// Scrape the Discord web client to find the latest build number.
/// Falls back to a hardcoded recent build number if scraping fails.
pub async fn fetch_discord_info(client: &reqwest::Client) -> DiscordInfo {
    match try_scrape_build_number(client).await {
        Ok(build_number) => {
            debug!("Scraped Discord build number: {}", build_number);
            DiscordInfo { build_number }
        }
        Err(e) => {
            warn!(
                "Failed to scrape Discord build number: {}. Using fallback: {}",
                e, FALLBACK_BUILD_NUMBER
            );
            DiscordInfo {
                build_number: FALLBACK_BUILD_NUMBER,
            }
        }
    }
}

async fn try_scrape_build_number(client: &reqwest::Client) -> Result<u64> {
    let html = client
        .get(DISCORD_LOGIN_URL)
        .send()
        .await?
        .text()
        .await?;

    let mut seen_assets = HashSet::new();
    let asset_paths: Vec<String> = script_re()
        .captures_iter(&html)
        .filter_map(|cap| {
            let path = cap[1].to_string();
            seen_assets.insert(path.clone()).then_some(path)
        })
        .collect();

    debug!("Found {} JS asset(s) on Discord login page", asset_paths.len());

    for path in &asset_paths {
        let url = format!("https://discord.com{path}");
        debug!("Fetching JS asset: {}", url);

        let js = match client.get(&url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    debug!("Failed to read JS asset {}: {}", path, e);
                    continue;
                }
            },
            Err(e) => {
                debug!("Failed to fetch JS asset {}: {}", path, e);
                continue;
            }
        };

        if let Some(caps) = build_re().captures(&js) {
            let build_str = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .or_else(|| caps.get(4))
                .map(|m| m.as_str());

            if let Some(s) = build_str {
                if let Ok(n) = s.parse::<u64>() {
                    return Ok(n);
                }
            }
        }
    }

    anyhow::bail!("Build number not found in any JS asset")
}
