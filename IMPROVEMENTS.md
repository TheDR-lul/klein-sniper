# 🚀 Klein-Sniper - Improvements & Recommendations

## ✅ Completed Improvements

### 1. **Scoring System (0-100)**
- ✅ Median-based analysis (more stable than average)
- ✅ Percentile ranking (10%, 25%, 75%, 90%)
- ✅ Multi-factor scoring:
  - **Percentile Bonus**: +30 for top 10%, +20 for top 25%
  - **Median Bonus**: +25 if 25%+ below median
  - **Freshness Bonus**: +15 for offers <1 hour old
  - **Volatility Penalty**: -20 in unstable price ranges

### 2. **Advanced Filtering**
- ✅ **Negative Keywords** (`exclude_keywords`) - filters out unwanted items
- ✅ **Required Keywords** (`require_all_keywords`) - precise matching
- ✅ **Spam Seller Detection** - filters suspicious sellers (many similar offers)
- ✅ **Seller Risk Assessment**:
  - ✅ Low risk (old account, few offers)
  - ⚠️ Medium risk (normal seller)
  - 🚫 High risk (new account, many offers)

### 3. **Smart Notifications**
- ✅ Compact format with emoji indicators
- ✅ Score-based emoji (🔥/⭐/✨/💎)
- ✅ Seller info with risk indicator
- ✅ Limited to 3 best offers per cycle
- ✅ No spam on first run (statistics collection only)

### 4. **Example Notification**
```
🔥 95★ | 290€ | iPhone 13 128GB Grün Top Gepflegt
📍 Berlin | ✅ Mitglied seit 2019 | 🔗 Link
```

## 🎯 Recommended Next Steps

### Priority 1 (High Impact, Low Effort)
1. **Price History Tracking** 📊
   - Track price changes over time
   - Detect price drops (seller becoming desperate)
   - Alert on significant price reductions

2. **Better Time Analysis** ⏰
   - Best times to find deals (early morning, late evening)
   - Day of week patterns
   - Seasonal trends

3. **Location Filtering** 📍
   - Add max distance from your location
   - Prefer nearby offers (easier pickup)
   - City/region whitelist/blacklist

### Priority 2 (Medium Impact, Medium Effort)
4. **Image Analysis** 🖼️
   - Download offer images
   - Basic quality check (blurry, stock photo detection)
   - Send preview image in notification

5. **Duplicate Detection** 🔍
   - Find reposted offers (same seller, same price)
   - Track offer lifecycle (how long it stays online)
   - Detect "professional sellers" masking as private

6. **Advanced Seller Analysis** 👤
   - Parse seller profile (rating, reviews)
   - Track seller's price changes
   - Blacklist problematic sellers

### Priority 3 (High Impact, High Effort)
7. **Price Prediction Model** 🤖
   - ML-based price forecasting
   - Predict if price will drop further
   - "Wait" vs "Buy Now" recommendations

8. **Multi-Platform Support** 🌐
   - eBay Kleinanzeigen alternatives
   - Vinted, Marktplaats, etc.
   - Unified scoring across platforms

9. **Web Dashboard** 💻
   - View all offers in browser
   - Interactive charts and statistics
   - Manage configuration via UI

## 🔧 Technical Improvements

### Code Quality
- ✅ Scoring system implemented
- ✅ Seller risk assessment
- ✅ Advanced filtering
- ⏳ Integration tests for storage/scraper
- ⏳ Connection pool for SQLite
- ⏳ Proxy support for IP rotation

### Security
- ⏳ Encrypt bot token in config
- ⏳ Add authentication for web dashboard
- ⏳ Rate limit protection

### Performance
- ⏳ Cache parsed HTML selectors
- ⏳ Parallel offer processing
- ⏳ Database query optimization

## 📋 Configuration Examples

### iPhone 13 (Strict Filtering)
```json
{
  "query": "iPhone 13",
  "match_keywords": ["iphone", "13"],
  "exclude_keywords": [
    "pro max", "mini", 
    "чехол", "hülle", "case", "cover",
    "defekt", "дефект", "kaputt", "broken",
    "display", "schutz", "folie", "panzerglas"
  ],
  "require_all_keywords": true,
  "min_price": 200.0,
  "max_price": 500.0
}
```

### Gaming Laptop (Flexible)
```json
{
  "query": "Gaming Laptop",
  "match_keywords": ["rtx", "4060", "4070", "4080"],
  "exclude_keywords": ["defekt", "broken", "parts only"],
  "require_all_keywords": false,
  "min_price": 800.0,
  "max_price": 2000.0
}
```

## 💡 Usage Tips

1. **Start Conservative**: Use high score threshold (70+) first
2. **Monitor for a Week**: Collect statistics before trusting scores
3. **Adjust Thresholds**: Fine-tune based on your needs
4. **Check Seller Manually**: Always verify high-risk sellers
5. **Act Fast**: Best deals disappear in minutes

## 🚨 Known Limitations

1. **Parsing Reliability**: Kleinanzeigen may change HTML structure
2. **Rate Limits**: Max 30 requests/minute (configurable)
3. **No Image Analysis**: Yet - only text-based scoring
4. **Single Market**: Only Kleinanzeigen.de supported
5. **Local Only**: Requires running 24/7 on your machine/server

## 📈 Statistics to Track

- Average time offers stay online
- Best time of day for new offers
- Price drop frequency
- Seller response rate
- Successful purchase rate

## 🎓 Learning Resources

- **Price Analysis**: Study percentiles vs standard deviation
- **Seller Patterns**: Track professional vs private sellers
- **Market Dynamics**: Understand supply/demand cycles
- **Negotiation**: Use scoring data to negotiate better

---

**Built with 🦀 Rust | Powered by Teloxide | Optimized for Speed**

