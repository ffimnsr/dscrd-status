# dscrd-status

> ⚠️ **For educational/research purposes only.** Using this tool is against Discord's [Terms of Service](https://discord.com/terms). You acknowledge the risk of a permanent ban on any account used with this tool.

A lightweight Rust CLI daemon that connects to the Discord WebSocket Gateway and maintains a user's online presence status.

---

## Features

- **Persistent WebSocket connection** to Discord's Gateway API (v10)
- **Automatic heartbeat** with random jitter to appear human
- **Realistic browser fingerprint** (Chrome user-agent, OS properties)
- **Build number scraper** — always uses the latest Discord web client build number
- **Graceful reconnect** handling (OP 7 Reconnect, OP 9 Invalid Session, OP 6 Resume)
- **Periodic presence updates** (every 5 minutes) to reinforce online status
- **Clean shutdown** on SIGINT/SIGTERM

---

## Build

```bash
cargo build --release
```

The compiled binary will be at `target/release/dscrd-status`.

---

## Usage

```
USAGE:
    dscrd-status [OPTIONS]

OPTIONS:
    -t, --token <TOKEN>      Discord user token (or set DISCORD_TOKEN env var)
    -s, --status <STATUS>    Status to set: online, idle, dnd [default: online]
    -v, --verbose            Enable verbose (debug) logging
    -h, --help               Print help
    -V, --version            Print version
```

### Examples

```bash
# Using a command-line flag
cargo run -- --token YOUR_TOKEN_HERE

# Using an environment variable
export DISCORD_TOKEN=YOUR_TOKEN_HERE
cargo run

# Set idle status with verbose logging
cargo run -- --token YOUR_TOKEN_HERE --status idle --verbose

# Using the release binary
./target/release/dscrd-status --token YOUR_TOKEN_HERE
```

---

## Environment Variables

Copy `.env.example` to `.env` and fill in your token:

```bash
cp .env.example .env
```

| Variable | Description |
|---|---|
| `DISCORD_TOKEN` | Your Discord user token |

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
| **Heartbeat jitter** | ±500ms random offset on every heartbeat interval |
| **Realistic fingerprint** | Randomly selects from a pool of real Chrome User-Agents (Windows, macOS, Linux) |
| **Scraped build number** | Always uses the latest Discord web client build number |
| **Human-like reconnect delays** | Random 1–5s delay before reconnecting |
| **`capabilities` field** | Set to `8189` matching the Discord web client |
| **`client_state` field** | Present in Identify payload as Discord web client sends it |
| **OP 6 Resume** | Resumes session after disconnect instead of re-identifying when possible |

---

## Project Structure

```
dscrd-status/
├── Cargo.toml
├── Cargo.lock
├── .env.example
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

MIT
