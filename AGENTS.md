# Dataverse CLI — Agent Integration Guide

Binary: `dv`

Dataverse CLI provides real-time social media data from X/Twitter and Reddit via the Bittensor SN13 decentralized data network (Macrocosmos).

## Machine-Readable Command Catalog

```bash
dv commands
```

Emits a full JSON schema of all commands, flags, types, defaults, enums, examples, and API endpoint mappings. Use this to discover capabilities programmatically.

## Quick Start for Agents

```bash
# Set API key (get free at https://app.macrocosmos.ai/account?tab=api-keys)
export MC_API="your-key"

# Always use -o json for structured output
dv -o json search x -k bittensor -l 10

# Preview request without executing (no API call, no credits consumed)
dv --dry-run search x -k bitcoin -l 5
```

## Output Modes

| Flag | Format | Use Case |
|------|--------|----------|
| `-o table` | Human-readable table (default) | Interactive terminal |
| `-o json` | JSON array to stdout | Agent parsing, piping to jq |
| `-o csv` | CSV to stdout | Data analysis, spreadsheets |

**Convention:** Data goes to stdout, diagnostics/errors go to stderr. Safe to pipe stdout.

## Response Structure (JSON mode)

### Search (`dv -o json search ...`)

Returns a JSON array of post objects. Each post has:

```json
{
  "datetime": "2026-03-17T14:53:00Z",
  "source": "x",
  "text": "Post content...",
  "uri": "https://x.com/user/status/123",
  "user": {
    "username": "handle",
    "display_name": "Display Name",
    "followers_count": 1234,
    "user_blue_verified": true
  },
  "tweet": {
    "id": "123456",
    "like_count": 42,
    "retweet_count": 5,
    "reply_count": 3,
    "view_count": 1000,
    "hashtags": ["#bittensor"],
    "language": "en"
  }
}
```

### Dry Run (`dv --dry-run ...`)

Returns the exact HTTP request that would be sent:

```json
{
  "method": "POST",
  "url": "https://constellation.api.cloud.macrocosmos.ai/sn13.v1.Sn13Service/OnDemandData",
  "headers": { "authorization": "Bearer 87ae...c800", "content-type": "application/json" },
  "body": { "source": "X", "keywords": ["bittensor"], "limit": 10 }
}
```

## API Key Resolution Order

1. `--api-key` CLI flag
2. `MC_API` environment variable
3. `MACROCOSMOS_API_KEY` environment variable
4. Config file (`~/.config/dataverse/config.toml` on Linux, `~/Library/Application Support/dataverse/config.toml` on macOS)

## Common Patterns

```bash
# Search and extract specific fields with jq
dv -o json search x -k bittensor -l 50 | jq '.[].text'

# Get top posts by engagement
dv -o json search x -k bitcoin -l 100 | jq 'sort_by(-.tweet.like_count) | .[0:5]'

# Search multiple keywords (OR mode)
dv -o json search x -k bittensor,tao,subnet -l 100

# Search multiple keywords (AND mode — all must match)
dv -o json search x -k bittensor,subnet --mode all -l 50

# Export to CSV for analysis
dv -o csv search x -k crypto -l 1000 > crypto_posts.csv

# Gravity: create task, monitor, build dataset
dv gravity create -p x -k bittensor -n "Bittensor collection"

# IMPORTANT: List ALL tasks (omit task_id). ALWAYS use --crawlers for record counts.
dv gravity status --crawlers

# Check a specific task (only when you already know the ID)
dv gravity status multicrawler-abc123 --crawlers

# Build dataset when enough records collected (NOTE: this stops the crawler)
dv gravity build crawler-0-multicrawler-abc123 --max-rows 50000

# Check dataset build progress
dv gravity dataset dataset-abc123
```

## Gravity Workflow Tips (READ THIS)

1. **Always use `--crawlers` flag** with `dv gravity status` — without it, Records and Size columns are empty
2. **List all tasks first** — run `dv gravity status --crawlers` without a task_id to see everything
3. **Don't build too early** — if a task has very few records, the dataset will contain "No valid data". Let it collect for hours/days first
4. **Building stops the crawler** — `dv gravity build` deregisters the crawler from the network. Only build when you have enough data
5. **Crawler IDs** follow the pattern `crawler-N-multicrawler-UUID` where N is the crawler index (usually 0)
6. **Dataset IDs** follow the pattern `dataset-UUID`

## Error Handling

Errors go to stderr. Non-zero exit code on failure. In JSON mode, check stderr for error details. Common transient errors:

- **464/503/504**: SN13 miner network temporarily unavailable — retry after a few seconds
- **401**: Invalid API key
- **timeout**: The decentralized network can take 10-60 seconds to respond

## Limitations

- The SN13 network is decentralized and can be intermittently slow or unavailable
- Default time window is 24 hours if `--from` is not specified
- Maximum 5 keywords and 5 usernames per query
- Reddit does not support `--usernames` filtering
- Gravity is large-scale data collection that can run up to 7 days on the network
