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
        --active-start <TIME>
                             Daily local start time like 09:00 [env: DISCORD_ACTIVE_START]
        --active-end <TIME>  Daily local end time like 18:00 [env: DISCORD_ACTIVE_END]
        --timezone <TZ>      IANA timezone like America/Chicago [env: DISCORD_TIMEZONE]
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

# Stay active only during business hours in Chicago
cargo run -- --token YOUR_TOKEN_HERE --active-start 09:00 --active-end 17:30 --timezone America/Chicago

# Overnight window example
cargo run -- --token YOUR_TOKEN_HERE --active-start 22:00 --active-end 06:00 --timezone Asia/Taipei

# Run the compiled release binary
./target/release/dscrd-status --token YOUR_TOKEN_HERE

# Keep running in the background (Linux/macOS)
nohup ./target/release/dscrd-status &

# Or with systemd (see below)
```

### Running as a background service (systemd)

This repository includes ready-to-install systemd assets:

- `packaging/systemd/dscrd-status.service`
- `packaging/systemd/dscrd-status.env.example`
- `packaging/systemd/dscrd-status-refresh.service`
- `packaging/systemd/dscrd-status-refresh.timer`
- `packaging/systemd/install-user-systemd.sh`

Install them like this:

```bash
# Build and install the binary
cargo build --release
sudo install -Dm755 target/release/dscrd-status /usr/local/bin/dscrd-status

# Create a dedicated service account
sudo useradd --system --home /var/lib/dscrd-status --create-home --shell /usr/sbin/nologin dscrd-status

# Install the env file and edit it
sudo install -d /etc/dscrd-status
sudo install -m600 packaging/systemd/dscrd-status.env.example /etc/dscrd-status/dscrd-status.env
sudo editor /etc/dscrd-status/dscrd-status.env

# Install the unit
sudo install -Dm644 packaging/systemd/dscrd-status.service /etc/systemd/system/dscrd-status.service
sudo install -Dm644 packaging/systemd/dscrd-status-refresh.service /etc/systemd/system/dscrd-status-refresh.service
sudo install -Dm644 packaging/systemd/dscrd-status-refresh.timer /etc/systemd/system/dscrd-status-refresh.timer

# Enable and start it
sudo systemctl daemon-reload
sudo systemctl enable --now dscrd-status
sudo systemctl enable --now dscrd-status-refresh.timer
sudo systemctl status dscrd-status
sudo systemctl list-timers dscrd-status-refresh.timer
```

The service stays running under systemd and the daemon itself enforces the daily active window. Outside that window it sleeps and waits for the next start time, so the configured timezone stays accurate even for overnight schedules.

The refresh timer restarts the daemon once per day so the CLI re-scrapes Discord's current build number on startup. The packaged timer defaults to `05:00:00` in the server's local timezone:

```ini
[Timer]
OnCalendar=*-*-* 05:00:00
```

Change that time by editing `/etc/systemd/system/dscrd-status-refresh.timer`, then reload systemd:

```bash
sudo editor /etc/systemd/system/dscrd-status-refresh.timer
sudo systemctl daemon-reload
sudo systemctl restart dscrd-status-refresh.timer
```

Example: restart daily at 03:30 local time:

```ini
[Timer]
OnCalendar=*-*-* 03:30:00
```

### Running as a user service

If you do not want a system-wide unit under `/etc/systemd/system`, use the installer script:

```bash
cargo build --release
./packaging/systemd/install-user-systemd.sh
```

That script:

- installs user units into `~/.config/systemd/user`
- creates `~/.config/dscrd-status/dscrd-status.env` if it does not exist
- runs `systemctl --user daemon-reload`
- enables and starts `dscrd-status.service`
- enables and starts the daily refresh timer by default

Optional flags:

```bash
./packaging/systemd/install-user-systemd.sh \
  --binary /absolute/path/to/dscrd-status \
  --env-file ~/.config/dscrd-status/dscrd-status.env \
  --refresh-time 03:30:00
```

If you only want the service and not the daily restart timer:

```bash
./packaging/systemd/install-user-systemd.sh --no-enable-timer
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
| `DISCORD_ACTIVE_START` | `--active-start` | ❌ No | — | Daily local start time in `HH:MM` or `HH:MM:SS` |
| `DISCORD_ACTIVE_END` | `--active-end` | ❌ No | — | Daily local end time in `HH:MM` or `HH:MM:SS` |
| `DISCORD_TIMEZONE` | `--timezone` | ❌ No | `UTC` when a window is set | IANA timezone such as `America/Chicago` or `Asia/Taipei` |

If you set `DISCORD_ACTIVE_START`, you must also set `DISCORD_ACTIVE_END`, and vice versa. Overnight windows are supported. For example, `22:00` to `06:00` means the daemon connects at 10:00 PM and disconnects at 6:00 AM every day in the chosen timezone.

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

# Optional schedule
#DISCORD_ACTIVE_START=09:00
#DISCORD_ACTIVE_END=18:00
#DISCORD_TIMEZONE=America/Chicago
```

---

## How the Build Number Scraper Works

Discord's Gateway rejects connections with an outdated `client_build_number`. The scraper:

1. Makes an HTTP GET to `https://discord.com/login` with a real Chrome User-Agent
2. Parses the HTML to find `<script>` tags referencing JS asset files (e.g. `/assets/web.b583426249e4da72.js`)
3. Fetches those JS files and searches for the build number pattern:
   - `YA("buildNumber","(\d+)")`
   - `buildNumber:"(\d+)"`
   - `build_number:(\d+)`
   - `"BUILD_NUMBER","(\d+)"`
4. Returns the first matching build number
5. Falls back to a hardcoded recent build number (`508471`) if scraping fails

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
