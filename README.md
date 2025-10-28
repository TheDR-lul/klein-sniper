# 🕵️‍♂️ KleinSniper

**KleinSniper** is a high-performance, production-ready Rust application that monitors [Kleinanzeigen.de](https://www.kleinanzeigen.de/) for great deals, detects price anomalies, and sends real-time Telegram notifications.

## ✨ Features

### Core Functionality
- 🔍 **Multi-page scraping** with automatic pagination
- 💰 **Statistical analysis** - average price, standard deviation, RSI
- 📉 **Smart deal detection** based on configurable thresholds
- 📦 **Model normalization** by keywords
- 📊 **SQLite storage** with automatic cleanup
- 📬 **Telegram bot** with rich HTML formatting

### Reliability & Performance
- 🔄 **Graceful shutdown** handling (Ctrl+C support)
- ⚙️ **Retry logic** with exponential backoff
- 🛡️ **Rate limiting** to avoid IP blocks
- 💾 **Database indexes** for fast queries
- ⚡ **Cached selectors** for optimal parsing
- 🚨 **Panic notifications** sent to Telegram

### Production Features
- 🐳 **Docker support** with multi-stage builds
- 📈 **Application metrics** and statistics
- 🧹 **Auto-cleanup** of old database records
- 📊 **Data export** to CSV/JSON
- 🔍 **Database statistics** command
- 🎯 **Configurable retention** periods

---

## 🚀 How It Works

1. Loads your search configuration from `config.json`
2. Periodically scrapes the search result pages for each configured model
3. Filters offers by price range and keywords
4. Computes average price & standard deviation for each model
5. Compares each offer to model stats — if a deal is found:
   - sends notification via Telegram
   - saves to database and marks as notified
6. Prunes offers that no longer exist on the marketplace

---

## 📦 Requirements

- Rust (stable)
- Telegram bot token and chat ID
- Kleinanzeigen search knowledge 😎

---

## ⚙️ Configuration

Create `config.json` with full customization:

```json
{
  "telegram_bot_token": "YOUR_BOT_TOKEN",
  "telegram_chat_id": 123456789,
  "check_interval_seconds": 120,
  
  "models": [
    {
      "query": "rog ally",
      "category_id": "k0",
      "deviation_threshold": 0.2,
      "min_price_delta": 100,
      "min_price": 240,
      "max_price": 800,
      "match_keywords": ["z1 extreme", "extreme"]
    }
  ],
  
  "scraper": {
    "max_pages": 20,
    "delay_seconds": 2,
    "max_retries": 3,
    "requests_per_minute": 30,
    "user_agents": [
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36..."
    ]
  },
  
  "analyzer": {
    "volatility_threshold": 20.0,
    "price_step": 50
  },
  
  "database": {
    "offer_retention_days": 30,
    "stats_retention_days": 90,
    "auto_cleanup_on_startup": true
  }
}
```

**Model Settings:**
- `deviation_threshold` - Percent below average to trigger (0.2 = 20%)
- `min_price_delta` - Absolute price difference required
- `match_keywords` - Filter offers by keywords

**Scraper Settings:**
- `max_retries` - Number of retry attempts (0 = no retry)
- `requests_per_minute` - Rate limit for requests
- `delay_seconds` - Delay between page fetches

**Database Settings:**
- `offer_retention_days` - Auto-delete offers older than X days (0 = keep forever)
- `auto_cleanup_on_startup` - Run cleanup when app starts

---

## 💬 Telegram Commands

| Command | Description |
|---------|-------------|
| `/ping` | Check bot connection |
| `/status` | Show analyzer status |
| `/help` | List all commands |
| `/last` | Show last great deal |
| `/top5` | Top 5 cheapest offers |
| `/avg` | Average prices by model |
| `/config` | Show current configuration |
| `/refresh` | Manually trigger check |
| `/uptime` | Service uptime |
| `/dbstats` | Database statistics |
| `/stats` | Application metrics |

---

## 🧠 Architecture Overview

```text
├── main.rs
├── config/         # config loader and model configs
├── scraper/        # HTML fetcher (with pagination handling)
├── parser/         # Extracts offer details from HTML
├── analyzer/       # Computes statistics, detects deals
├── normalizer/     # Normalizes model titles based on keywords
├── notifier/       # Telegram integration
└── storage/        # SQLite logic (offers, stats, notified)
```

---

## 🛠️ Run Locally

```bash
# Clone repository
git clone https://github.com/yourname/klein-sniper
cd klein-sniper

# Create config.json (see Configuration section)
cp config.example.json config.json
# Edit config.json with your bot token and chat ID

# Run in release mode
cargo run --release
```

### Docker Deployment

```bash
# Build and run with docker-compose
docker-compose up -d

# View logs
docker-compose logs -f klein-sniper

# Stop
docker-compose down
```

### Environment Variables

```bash
# Set log level (default: info)
export RUST_LOG=debug

# Run
cargo run --release
```

---

## 📜 License

Creative Commons Attribution-NonCommercial 4.0 International (CC BY-NC 4.0) – 
You may use, share, and adapt the code for non-commercial purposes as long as you provide attribution.
See the [LICENSE](./LICENSE) file for more details.
