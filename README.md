# dscrd-status

> ⚠️ **For educational/research purposes only.**  
> Using this tool is against Discord's [Terms of Service](https://discord.com/terms).  
> You acknowledge the risk of a **permanent ban** on any account used with this tool.

A lightweight Rust CLI daemon that connects to the Discord WebSocket Gateway and maintains a user's online presence status.

---

## Table of Contents

1. [Features](#features)
2. [Prerequisites](#prerequisites)
3. [How to Get Your Discord Token](#how-to-get-your-discord-token)
4. [Setup](#setup)
5. [Build](#build)
6. [Usage](#usage)
7. [Configuration — Environment Variables & `.env`](#configuration--environment-variables--env)
8. [How the Build Number Scraper Works](#how-the-build-number-scraper-works)
9. [Anti-Detection Notes](#anti-detection-notes)
10. [Project Structure](#project-structure)
11. [License](#license)

---

## Features

- **Persistent WebSocket connection** to Discord's Gateway API (v10)
- **Automatic heartbeat** with random jitter to appear human
- **Realistic browser fingerprint** (Chrome user-agent, OS properties)
- **Build number scraper** — always uses the latest Discord web client build number
- **Graceful reconnect** handling (OP 7 Reconnect, OP 9 Invalid Session, OP 6 Resume)
- **Periodic presence updates** (every 5 minutes) to reinforce online status
- **Clean shutdown** on SIGINT/SIGTERM
- **Full environment variable support** — every flag can be set in `.env` or the shell environment

---

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs/) | 1.70+ | Install via `rustup` |
| OpenSSL or LibreSSL | system | Required for TLS; usually pre-installed |
| Internet access | — | Required to connect to Discord |

Install Rust if you haven't already:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

---

## How to Get Your Discord Token

> ⚠️ **Never share your token with anyone.** It gives full access to your Discord account.

Your Discord user token is a credential that identifies your account to the Discord API.  
Here is how to retrieve it from the Discord web client:

### Step-by-step (Browser DevTools method)

1. Open [https://discord.com/app](https://discord.com/app) in your browser and log in.

2. Press **F12** (or `Cmd+Option+I` on macOS) to open Developer Tools.

3. Go to the **Network** tab.

4. In the filter box, type `api/v` to narrow down requests to the Discord API.

5. In Discord, click on any server, channel, or perform any action (send a message, switch channels, etc.) to generate a network request.

6. Click on one of the requests that appears (e.g. `messages`, `guilds`, `science`).

7. In the **Request Headers** section, find the **`Authorization`** header — the value is your token.

   It will look like one of these formats:
   - User accounts: a long string of characters and dots, e.g. `MTIzNDU2Nzg5.XXXXXX.XXXXXXXXXXXXXXXXXXXXXXXXXX`
   - Bot tokens start with `Bot ` — **this tool is for user tokens only**

8. Copy the token value (without any surrounding quotes).

### Alternative: Application Storage method

1. Open [https://discord.com/app](https://discord.com/app) and log in.
2. Open Developer Tools → **Application** tab (Chrome) or **Storage** tab (Firefox).
3. Expand **Local Storage** → `https://discord.com`.
4. Find the key named `token` — the value (inside quotes) is your token.

### ⚠️ Security reminder

- **Never paste your token into chat, GitHub, or any website.**
- If you suspect your token has been exposed, immediately **change your Discord password** (this invalidates the token) or use **Settings → Logout** from all devices.
- Treat it like a password.

---

## Setup

```bash
# 1. Clone the repository
git clone https://github.com/ffimnsr/dscrd-status.git
cd dscrd-status

# 2. Copy the example environment file
cp .env.example .env

# 3. Open .env and paste your Discord token
#    (replace "your_user_token_here" with the token you obtained above)
nano .env   # or: code .env, vim .env, etc.
```

---

## Build

```bash
# Debug build (faster compile, larger binary)
cargo build

# Release build (optimised, smaller binary — recommended for production use)
cargo build --release
```

The release binary will be at `target/release/dscrd-status`.

---

## Usage

```
USAGE:
    dscrd-status [OPTIONS]

OPTIONS:
    -t, --token <TOKEN>      Discord user token [env: DISCORD_TOKEN]
    -s, --status <STATUS>    Status to set: online, idle, dnd, invisible [env: DISCORD_STATUS] [default: online]
    -v, --verbose            Enable verbose logging [env: DISCORD_VERBOSE]
    -h, --help               Print help
    -V, --version            Print version
```

### Examples

```bash
# Quickest way — token in .env file, just run:
cargo run --release

# Pass the token directly on the command line
cargo run -- --token YOUR_TOKEN_HERE

# Use an inline environment variable
DISCORD_TOKEN=YOUR_TOKEN_HERE cargo run

# Set idle status with verbose logging
cargo run -- --token YOUR_TOKEN_HERE --status idle --verbose

# Run the compiled release binary
./target/release/dscrd-status --token YOUR_TOKEN_HERE

# Keep running in the background (Linux/macOS)
nohup ./target/release/dscrd-status &

# Or with systemd (see below)
```

### Running as a background service (systemd)

Create `/etc/systemd/system/dscrd-status.service`:

```ini
[Unit]
Description=Discord presence keeper
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/path/to/dscrd-status
EnvironmentFile=/path/to/dscrd-status/.env
ExecStart=/path/to/dscrd-status/target/release/dscrd-status
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Then enable and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now dscrd-status
sudo systemctl status dscrd-status
```

---

## Configuration — Environment Variables & `.env`

Every CLI flag can be set via an environment variable. The daemon reads a `.env` file in the current directory automatically on startup (via [`dotenv`](https://crates.io/crates/dotenv)); no extra steps required beyond creating the file.

### Priority order (highest → lowest)

1. **CLI flag** — `--token VALUE`
2. **Shell environment** — `export DISCORD_TOKEN=VALUE`
3. **`.env` file** — `DISCORD_TOKEN=VALUE` in `.env`
4. **Default value** (where applicable)

### Supported variables

| Environment Variable | CLI Flag | Required | Default | Description |
|---|---|---|---|---|
| `DISCORD_TOKEN` | `--token` | ✅ Yes | — | Your Discord user token |
| `DISCORD_STATUS` | `--status` | ❌ No | `online` | Presence status: `online`, `idle`, `dnd`, `invisible` |
| `DISCORD_VERBOSE` | `--verbose` | ❌ No | `false` | Set to `true` or `1` to enable debug logging |

### Example `.env` file

Copy `.env.example` to `.env` and edit it:

```bash
cp .env.example .env
```

```dotenv
# Required
DISCORD_TOKEN=your_user_token_here

# Optional — keep these commented out to use the defaults
#DISCORD_STATUS=online
#DISCORD_VERBOSE=false
```

---

## How the Build Number Scraper Works

Discord's Gateway rejects connections with an outdated `client_build_number`. The scraper:

1. Makes an HTTP GET to `https://discord.com/app` with a real Chrome User-Agent
2. Parses the HTML to find `<script>` tags referencing JS asset files (e.g. `/assets/abc123.js`)
3. Fetches up to 10 of those JS files and searches for the build number pattern:
   - `buildNumber:"(\d+)"`
   - `build_number:(\d+)`
   - `"BUILD_NUMBER","(\d+)"`
4. Caches the result in memory for the session lifetime
5. Falls back to a hardcoded recent build number (`308796`) if scraping fails

---

## Anti-Detection Notes

This daemon implements several measures to reduce the chance of automated detection:

| Measure | Implementation |
|---|---|
| **Heartbeat jitter** | ±500 ms random offset on every heartbeat interval |
| **Realistic fingerprint** | Randomly selects from a pool of real Chrome User-Agents (Windows, macOS, Linux) |
| **Scraped build number** | Always uses the latest Discord web client build number |
| **Human-like reconnect delays** | Random 1–5 s delay before reconnecting |
| **`capabilities` field** | Set to `8189` matching the Discord web client |
| **`client_state` field** | Present in Identify payload as the Discord web client sends it |
| **OP 6 Resume** | Resumes session after disconnect instead of re-identifying when possible |

---

## Project Structure

```
dscrd-status/
├── Cargo.toml           # Dependencies and binary target
├── Cargo.lock
├── .env.example         # Template — copy to .env and fill in your token
├── README.md
└── src/
    ├── main.rs          # CLI entry point (clap + tokio runtime)
    ├── gateway.rs       # WebSocket gateway connection + opcode handling
    ├── heartbeat.rs     # Heartbeat loop with jitter
    ├── presence.rs      # OP 3 Presence Update payload builder
    ├── scraper.rs       # Discord build number scraper
    └── fingerprint.rs   # Browser/OS fingerprint pool
```

---

## License

[MIT](LICENSE)
