use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

/// Browser/OS fingerprint properties sent in the OP 2 Identify payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub os: String,
    pub browser: String,
    pub device: String,
    pub system_locale: String,
    pub browser_user_agent: String,
    pub browser_version: String,
    pub os_version: String,
    pub referrer: String,
    pub referring_domain: String,
    pub referrer_current: String,
    pub referring_domain_current: String,
    pub release_channel: String,
    pub client_build_number: u64,
    pub client_event_source: Option<String>,
}

struct UserAgentProfile {
    user_agent: &'static str,
    browser_version: &'static str,
    os: &'static str,
    os_version: &'static str,
}

static UA_POOL: &[UserAgentProfile] = &[
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        browser_version: "122.0.0.0",
        os: "Windows",
        os_version: "10",
    },
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        browser_version: "123.0.0.0",
        os: "Windows",
        os_version: "10",
    },
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        browser_version: "122.0.0.0",
        os: "Mac OS X",
        os_version: "10.15.7",
    },
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        browser_version: "123.0.0.0",
        os: "Mac OS X",
        os_version: "10.15.7",
    },
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        browser_version: "122.0.0.0",
        os: "Linux",
        os_version: "",
    },
    UserAgentProfile {
        user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        browser_version: "123.0.0.0",
        os: "Linux",
        os_version: "",
    },
];

/// Select a random fingerprint profile from the pool for this session.
pub fn select_fingerprint(build_number: u64) -> Fingerprint {
    let mut rng = rand::thread_rng();
    let profile = UA_POOL.choose(&mut rng).unwrap_or(&UA_POOL[0]);

    Fingerprint {
        os: profile.os.to_string(),
        browser: "Chrome".to_string(),
        device: String::new(),
        system_locale: "en-US".to_string(),
        browser_user_agent: profile.user_agent.to_string(),
        browser_version: profile.browser_version.to_string(),
        os_version: profile.os_version.to_string(),
        referrer: String::new(),
        referring_domain: String::new(),
        referrer_current: String::new(),
        referring_domain_current: String::new(),
        release_channel: "stable".to_string(),
        client_build_number: build_number,
        client_event_source: None,
    }
}
