# 🕵️‍♂️ KleinSniper

**KleinSniper** is a Rust-based marketplace analyzer that scrapes [Kleinanzeigen.de](https://www.kleinanzeigen.de/) for interesting offers, detects price anomalies, and sends Telegram notifications about deals.

## ✨ Features

- 🔍 Scrapes all pages of Kleinanzeigen search results
- 💰 Calculates average price and standard deviation
- 📉 Detects underpriced items based on configurable thresholds
- 📦 Normalizes offer models by keywords
- 📊 Saves offer statistics to SQLite
- 📬 Sends alerts via Telegram bot
- ⚙️ Configurable via `config.json`
- 🗑 Automatically removes outdated or deleted offers from the database

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

Edit the `config.json`:

```json
{
  "telegram_bot_token": "your-bot-token",
  "telegram_chat_id": 123456789,
  "check_interval_seconds": 60,
  "models": [
    {
      "query": "rog ally",
      "category_id": "k0",
      "deviation_threshold": 0.2,
      "min_price_delta": 100,
      "min_price": 240,
      "max_price": 800,
      "match_keywords": ["z1 extreme", "extreme"]
    },
    {
      "query": "rog ally",
      "category_id": "k0",
      "deviation_threshold": 0.2,
      "min_price_delta": 100,
      "min_price": 200,
      "max_price": 800,
      "match_keywords": ["z1"]
    }
  ]
}
```

- `deviation_threshold` — percent below average price to trigger notification
- `min_price_delta` — absolute price delta below average to trigger notification
- `match_keywords` — filters only offers containing these words

---

## 💬 Telegram Commands

Once the bot is running, send these commands:

- `/ping` – check bot status
- `/status` – show system status
- `/last` – show last offer
- `/top5` – show top 5 cheapest offers
- `/avg` – show average prices per model
- `/refresh` – manually trigger scraping
- `/uptime` – show uptime
- `/help` – show commands list
- `/config` – show cconfig

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
git clone https://github.com/yourname/klein-sniper
cd klein-sniper
cargo run --release
```

> Make sure `config.json` is in the root directory.

---

## 📜 License

Creative Commons Attribution-NonCommercial 4.0 International (CC BY-NC 4.0) – 
You may use, share, and adapt the code for non-commercial purposes as long as you provide attribution.
See the [LICENSE](./LICENSE) file for more details.
