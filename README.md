# dv — Dataverse CLI

A fast Rust CLI for querying real-time social media data from X/Twitter and Reddit, powered by the [Bittensor SN13](https://docs.macrocosmos.ai) decentralized data network.

> [!NOTE]
> Dataverse CLI is currently in Beta.
> We'd love your feedback — please open an [issue](https://github.com/macrocosm-os/dataverse-cli/issues) or submit a PR.

## Features at a Glance

- **Real-Time Search** — Query X/Twitter and Reddit posts by keyword, username, or URL via decentralized Bittensor miners
- **Large-Scale Collection** — Gravity tasks collect data continuously for up to 7 days across the miner network
- **Dataset Export** — Build downloadable Parquet datasets from collected data
- **Multiple Output Formats** — Table, JSON, and CSV output for terminal, scripting, and analysis
- **Agent/LLM Friendly** — `dv commands` emits a full JSON schema of all commands for tool integration
- **Dry-Run Mode** — Preview exact API requests without executing or consuming credits
- **Secure Config** — API keys stored with 0600 permissions, masked in output

---

## Install

### Cargo (Rust)

```sh
cargo install dataverse
```

### From Source

```sh
git clone https://github.com/macrocosm-os/dataverse-cli
cd dataverse-cli
cargo install --path .
```

### Manual

Download the binary for your platform from [Releases](https://github.com/macrocosm-os/dataverse-cli/releases), and place `dv` in your `$PATH`.

---

## Setup

Get a free API key at [app.macrocosmos.ai](https://app.macrocosmos.ai/account?tab=api-keys), then:

```sh
# Interactive setup (recommended — input is masked)
dv auth

# Or via environment variable
export MC_API=your-api-key

# Verify configuration
dv status
```

API key resolution order: `--api-key` flag > `MC_API` env > `MACROCOSMOS_API_KEY` env > config file.

---

## Global Flags

```sh
# JSON output (for scripting and agents)
dv -o json search x -k bitcoin -l 10
dv -o json search x -k bitcoin -l 100 | jq '.[0].tweet.like_count'

# CSV export
dv -o csv search x -k bitcoin -l 1000 > bitcoin_posts.csv

# Dry-run mode (shows the API request without executing it)
dv --dry-run search x -k bitcoin -l 10

# Custom timeout
dv --timeout 180 search x -k bitcoin -l 500
```

All data commands support `-o json` and `-o csv`. Diagnostics go to stderr; stdout is always clean data.

---

## Commands

### `dv search` — Real-Time Social Data

Search X/Twitter or Reddit posts in real-time via the Bittensor SN13 miner network.

```sh
# Search X by keyword
dv search x -k bitcoin -l 10
dv search x -k bitcoin,ethereum -l 50 --from 2025-01-01

# Search by username (X only)
dv search x -u elonmusk -l 20

# Multiple keywords with AND mode
dv search x -k bittensor,subnet --mode all -l 50

# Search Reddit
dv search reddit -k r/MachineLearning -l 25

# Search by URL
dv search x --url "https://x.com/user/status/123456"
```

| Flag | Default | Description |
|------|---------|-------------|
| `source` | — | **Required.** `x`, `twitter`, or `reddit` |
| `-k, --keywords` | — | Keywords, comma-separated (up to 5). For Reddit, first item is subreddit |
| `-u, --usernames` | — | Usernames, comma-separated (up to 5, X only) |
| `--from` | 24h ago | Start date (YYYY-MM-DD or ISO 8601) |
| `--to` | now | End date (YYYY-MM-DD or ISO 8601) |
| `-l, --limit` | 100 | Max results (1–1000) |
| `--mode` | any | Keyword match mode: `any` (OR) or `all` (AND) |
| `--url` | — | Search by URL instead of keywords |

---

### `dv gravity create` — Start Data Collection

Create a Gravity task that collects social data from the Bittensor miner network for up to 7 days.

```sh
dv gravity create -p x -t '#bittensor' -n "TAO tracker"
dv gravity create -p x -k bitcoin -n "Bitcoin collection"
dv gravity create -p reddit -t 'r/MachineLearning' -k transformer
dv gravity create -p x -t '$BTC' --email me@example.com
```

| Flag | Default | Description |
|------|---------|-------------|
| `-p, --platform` | — | **Required.** `x`, `twitter`, or `reddit` |
| `-t, --topic` | — | Topic to track. X: `#hashtag` or `$cashtag`. Reddit: `r/subreddit` |
| `-k, --keyword` | — | Additional keyword filter |
| `-n, --name` | — | Task name |
| `--email` | — | Notification email on completion |

---

### `dv gravity status` — Monitor Tasks

List all tasks or check a specific task. **Always use `--crawlers`** to see record counts and data sizes.

```sh
# List all tasks with collection stats
dv gravity status --crawlers

# Check a specific task
dv gravity status multicrawler-abc123 --crawlers
```

| Flag | Default | Description |
|------|---------|-------------|
| `task_id` | — | Omit to list all tasks |
| `--crawlers` | false | Include record counts and data sizes |

---

### `dv gravity build` — Build Dataset

Build a downloadable Parquet dataset from a crawler.

> **Warning:** This stops the crawler and deregisters it from the network. Only build when you have enough data.

```sh
dv gravity build crawler-0-multicrawler-abc123
dv gravity build crawler-0-multicrawler-abc123 --max-rows 50000
```

| Flag | Default | Description |
|------|---------|-------------|
| `crawler_id` | — | **Required.** Crawler ID |
| `--max-rows` | 10000 | Maximum rows in dataset |

---

### `dv gravity dataset` — Dataset Status

Check dataset build progress and get download links.

```sh
dv gravity dataset dataset-abc123
dv -o json gravity dataset dataset-abc123
```

---

### `dv gravity cancel` / `dv gravity cancel-dataset`

```sh
dv gravity cancel multicrawler-abc123
dv gravity cancel-dataset dataset-abc123
```

---

### `dv auth` — Configure API Key

```sh
dv auth
```

Interactive setup that validates your key against the SN13 network and saves to config.

---

### `dv status` — Check Connection

```sh
dv status
```

Shows API key source and tests connectivity to the SN13 network.

---

## Agent / LLM Integration

Dataverse CLI is designed for use by AI agents and LLMs.

```sh
# Full JSON schema of all commands, flags, types, and examples
dv commands
```

The hidden `dv commands` outputs a machine-readable catalog for tool integration. See [AGENTS.md](AGENTS.md) for the full integration guide including response schemas, workflow tips, and common patterns.

---

## Gravity Workflow

```
1. Create task     →  dv gravity create -p x -k bitcoin -n "my task"
2. Monitor         →  dv gravity status --crawlers
3. Wait            →  Let miners collect data (hours to days)
4. Build dataset   →  dv gravity build crawler-0-multicrawler-... --max-rows 50000
5. Check progress  →  dv gravity dataset dataset-...
6. Download        →  Parquet files with download URLs
```

> **Tip:** Don't build too early. If a task has very few records, the dataset will be empty. Let it collect for at least a few hours.

---

## Development

```sh
cargo build
cargo test
cargo build --release
```

---

## Tech Stack

| Crate | Purpose |
|-------|---------|
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing with derive API |
| [reqwest](https://github.com/seanmonstar/reqwest) | Async HTTP/2 client with rustls |
| [serde](https://serde.rs) | JSON serialization/deserialization |
| [tokio](https://tokio.rs) | Async runtime |
| [tabled](https://github.com/zhiburt/tabled) | Terminal table formatting |
| [colored](https://github.com/mackwic/colored) | Terminal colors |
| [dialoguer](https://github.com/console-rs/dialoguer) | Interactive prompts |

---

## License

MIT — see [LICENSE](LICENSE).
