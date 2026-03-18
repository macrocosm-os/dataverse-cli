# Dataverse CLI

Rust CLI for querying social media data from X/Twitter and Reddit via the Macrocosmos SN13 API (Bittensor Data Universe subnet).

## Quick Reference

- Binary: `dv`
- Language: Rust 2021 edition
- Build: `cargo build` / `cargo build --release`
- Run: `cargo run -- <command>`
- Test: `cargo test`

## Architecture

```
src/
  main.rs              Entry point — parse CLI, dispatch to command
  api/
    mod.rs             Re-exports
    client.rs          HTTP client (reqwest) for Macrocosmos gRPC-Web JSON API
    types.rs           Request/response structs (serde)
  cmd/
    mod.rs             CLI definition (clap derive), GlobalOpts, command dispatch
    search.rs          `dv search` — OnDemandData queries
    gravity.rs         `dv gravity` — Gravity data collection task management
    auth.rs            `dv auth` / `dv status` — API key setup and validation
  config/
    mod.rs             Config file (~/.config/dataverse/config.toml), API key resolution
  display/
    mod.rs             Output formatting (table/JSON/CSV), nested field extraction
```

## API Details

- **Base URL**: `https://constellation.api.cloud.macrocosmos.ai`
- **Protocol**: gRPC-Web JSON transcoding over HTTPS (HTTP/2 required — ALB returns 464 on HTTP/1.1)
- **Auth**: `Authorization: Bearer <api_key>` header
- **Services**:
  - `sn13.v1.Sn13Service/OnDemandData` — real-time social data queries
  - `gravity.v1.GravityService/*` — large-scale data collection tasks

## Key Design Decisions

- **HTTP/2 required**: The Macrocosmos ALB target group requires HTTP/2. reqwest must have the `http2` feature enabled; `http1_only()` will cause 464 errors.
- **Nested response fields**: X post data nests under `user.*` and `tweet.*` (e.g., `user.username`, `tweet.like_count`). The display layer uses dot-path extraction.
- **Stdout/stderr discipline**: Data goes to stdout, diagnostics/errors to stderr.
- **Config file permissions**: 0600 on Unix.
- **API key resolution order**: `--api-key` flag > `MC_API` env > `MACROCOSMOS_API_KEY` env > config file.

## Commands

| Command | Description |
|---------|-------------|
| `dv search <source> -k <keywords>` | Search X or Reddit posts |
| `dv gravity create -p <platform> -t <topic>` | Create data collection task |
| `dv gravity status [task_id]` | List/check gravity tasks |
| `dv gravity build <crawler_id>` | Build dataset from crawler |
| `dv gravity dataset <dataset_id>` | Check dataset status |
| `dv gravity cancel <task_id>` | Cancel gravity task |
| `dv gravity cancel-dataset <dataset_id>` | Cancel dataset build |
| `dv auth` | Configure API key interactively |
| `dv status` | Check API key and connection |

## Global Flags

- `-o, --output <table|json|csv>` — output format
- `--api-key <key>` — override API key
- `--dry-run` — preview API request without executing
- `--timeout <seconds>` — request timeout (default: 120)
