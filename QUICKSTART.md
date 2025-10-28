# KleinSniper - Quick Start Guide

Get KleinSniper up and running in 5 minutes! 🚀

## ⚡ Quick Setup

### 1. Get Your Telegram Credentials

#### Bot Token
1. Open Telegram and search for [@BotFather](https://t.me/botfather)
2. Send `/newbot` and follow instructions
3. Copy the bot token (format: `123456:ABC-DEF1234...`)

#### Chat ID
1. Search for [@userinfobot](https://t.me/userinfobot)
2. Start the bot
3. Copy your ID number

### 2. Configure the App

Copy example config:
```bash
cp config.example.json config.json
```

Edit `config.json` and update:
```json
{
  "telegram_bot_token": "YOUR_TOKEN_HERE",
  "telegram_chat_id": YOUR_CHAT_ID_HERE
}
```

### 3. Run

```bash
# Development mode
cargo run

# Production mode
cargo run --release
```

## 📱 Test the Bot

Once running, send these commands to your bot:

```
/ping        → Check if bot is alive
/status      → See current status
/help        → List all commands
```

## 🔍 Add Your First Search

Edit `config.json` and add a model:

```json
{
  "models": [
    {
      "query": "iPhone 13",
      "category_id": "k0c173",
      "min_price": 200.0,
      "max_price": 500.0,
      "deviation_threshold": 0.15,
      "min_price_delta": 20.0,
      "match_keywords": ["iphone", "13", "pro"]
    }
  ]
}
```

**Important:** All fields are required!

## 🎯 Common Categories

| Category | ID | Example |
|----------|-----|---------|
| All | `k0` | Everything |
| Mobile Phones | `k0c173` | iPhone, Samsung |
| Computers | `k0c161` | Laptops, PCs |
| Gaming | `k0c225` | PS5, Xbox |
| Photo & Video | `k0c279` | Cameras, Lenses |

## 🐛 Troubleshooting

### Error: missing field `min_price_delta`

Add this to your model config:
```json
{
  "min_price_delta": 20.0  // ← Add this
}
```

### Error: Invalid bot token

Token must contain `:` character:
```
✅ 8191253017:AAG8uiNV12E5Uz265TEtZf5JW6ec771RhOw
❌ 8191253017AAG8uiNV12E5Uz265TEtZf5JW6ec771RhOw
```

### Bot doesn't respond

1. Check bot token is correct
2. Check chat ID is correct
3. Make sure you've started the bot in Telegram first

## 📚 Next Steps

- 📖 Read [CONFIGURATION_GUIDE](docs/CONFIGURATION_GUIDE.md) for detailed config options
- 🏗️ Check [ARCHITECTURE](docs/ARCHITECTURE.md) to understand how it works
- 🎨 Review [STYLEGUIDE](STYLEGUIDE.md) if you want to contribute

## 🐳 Docker (Optional)

```bash
docker-compose up -d
```

Check logs:
```bash
docker-compose logs -f
```

## 💡 Pro Tips

1. **Start with conservative settings**
   - `check_interval_seconds: 180` (3 minutes)
   - `max_pages: 10`
   - `requests_per_minute: 20`

2. **Use specific keywords**
   - Bad: `["phone"]` → Too broad
   - Good: `["iphone", "13", "pro"]` → Specific

3. **Set realistic price ranges**
   - Check actual prices on Kleinanzeigen first
   - Don't make range too wide

4. **Monitor first runs**
   - Check what offers are detected
   - Adjust thresholds if needed

## 🎓 Understanding the Output

When running, you'll see:
```
INFO Processing model: iPhone 13
INFO Fetching page 1: https://...
INFO Parsed 20 items from page 1
INFO Found 2 good offers
INFO Checking offer: 2123456 — 299.00 €
INFO Sending Telegram notification...
```

Good deal notifications will appear in your Telegram chat! 📬

---

**Need help?** Check out:
- [Full Documentation](docs/) - All guides and references
- [README.md](README.md) - Feature overview
- [Issue Tracker](https://github.com/yourusername/klein-sniper/issues) - Report bugs

**Happy hunting! 🎯**


