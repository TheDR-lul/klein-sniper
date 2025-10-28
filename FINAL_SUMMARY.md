# 🎯 Klein-Sniper - Final Summary

## ✅ Что сделано сегодня

### 1. **Scoring система (0-100 баллов)** 📊
- ✅ Медиана вместо среднего (устойчивее к выбросам)
- ✅ Процентили (10%, 25%, 75%, 90%)
- ✅ Multi-factor scoring:
  - Percentile Bonus: +30 (top 10%), +20 (top 25%)
  - Median Bonus: +25 (25%+ ниже медианы)
  - Freshness Bonus: +15 (<1ч), +5 (<6ч)
  - Volatility Penalty: -20 (нестабильные цены)

### 2. **Улучшенная фильтрация** 🎯
```json
{
  "match_keywords": ["iphone", "13"],
  "exclude_keywords": ["pro max", "mini", "чехол", "case", "defekt"],
  "require_all_keywords": true
}
```

**Отсеивает:**
- iPhone 13 Pro Max/Mini (если ищешь базовый)
- Чехлы, аксессуары, защитные стекла
- Сломанные/дефектные товары
- Спам от подозрительных продавцов

### 3. **Оценка риска продавца** 🛡️
- ✅ **Low Risk**: Старый аккаунт (3+ года) + мало офферов (≤3)
- ⚠️ **Medium Risk**: Обычный продавец
- 🚫 **High Risk**: Новый аккаунт (<1 год) + много офферов (>5)

### 4. **Компактные уведомления** 💬
**Старый формат (8 строк):**
```
💎 Great Deal Found! (Score: 65/100)
📦 Model: iPhone 13
💰 Price: 350.00 €
📊 Quality Score: 65/100
...
```

**Новый формат (2 строки):**
```
🔥 95★ | 290€ | iPhone 13 128GB Grün Top
📍 Berlin | ✅ Mitglied seit 2019 | 🔗 Link
```

### 5. **Telegram команды управления** 🎮
```
/settings - текущие настройки
/cleardb - очистить базу данных
/minscore 70 - установить мин. балл
/maxdist 50 - макс расстояние (км)
/dbstats - статистика БД
/refresh - ручной перезапуск
/top5 - топ 5 офферов
```

### 6. **Антиспам** 🚫
- ✅ Первый запуск БЕЗ уведомлений (только сбор статистики)
- ✅ Макс 3 уведомления за цикл (лучшие офферы)
- ✅ Проверка дубликатов (is_notified)
- ✅ Фильтр подозрительных продавцов

### 7. **Улучшенный .gitignore** 📝
```
logs/
*.log
data.db*
test_*.html
docs/ (generated)
```

## 🚀 Как использовать

### Первый запуск
```bash
# 1. Настроить config.json
{
  "telegram_bot_token": "YOUR_TOKEN",
  "telegram_chat_id": YOUR_ID,
  "models": [{
    "query": "iPhone 13",
    "match_keywords": ["iphone", "13"],
    "exclude_keywords": ["pro max", "mini", "чехол", "case"],
    "require_all_keywords": true,
    "min_price": 200.0,
    "max_price": 500.0
  }]
}

# 2. Запустить
cargo run --release

# 3. Дождаться первого цикла (собирает статистику, НЕ шлет уведомления)
# 4. Со второго цикла начнут приходить уведомления на топ офферы
```

### Telegram команды
```
/help - список команд
/settings - текущие настройки
/top5 - топ 5 офферов
/dbstats - статистика БД
/cleardb - очистить БД (осторожно!)
/refresh - ручной перезапуск поиска
```

## 📊 Пример работы

### Цикл 1 (Первый запуск)
```
⏭️ First run for model 'iPhone 13' - collecting data only
✅ Initial statistics saved. Next cycle will check for deals.
```

### Цикл 2+
```
📊 Extended Stats:
   Median: 350.00 €
   P10: 290.00 € | P25: 320.00 €
   
🏆 #1 Score: 95 | Price: 290€ [P:30 M:25 F:15]
🏆 #2 Score: 85 | Price: 320€ [P:20 M:15 F:15]
🏆 #3 Score: 75 | Price: 340€ [P:20 M:5 F:15]

Found 15 potentially good offers (scored >= 60)
📬 Sent 3 notification(s) - reached limit
```

### Уведомление в Telegram
```
🔥 95★ | 290€ | Apple iPhone 13 128GB Grün Top Gepflegt
📍 Berlin | ✅ Mitglied seit 2019 | 🔗 Link
```

## 🎯 Рекомендации по настройке

### Conservative (Только топовые сделки)
```json
{
  "deviation_threshold": 0.20,
  "min_price_delta": 50.0,
  "exclude_keywords": [много стоп-слов]
}
```
**Результат:** Мало уведомлений, но все реально выгодные

### Balanced (Рекомендуемая)
```json
{
  "deviation_threshold": 0.15,
  "min_price_delta": 30.0,
  "exclude_keywords": [основные стоп-слова]
}
```
**Результат:** 1-3 уведомления каждые 3 минуты, хороший баланс

### Aggressive (Все потенциальные сделки)
```json
{
  "deviation_threshold": 0.10,
  "min_price_delta": 20.0,
  "exclude_keywords": [минимум стоп-слов]
}
```
**Результат:** Много уведомлений, нужно самому фильтровать

## 📈 Что еще можно добавить

### Priority 1 (Быстро & Полезно)
1. **История цен** - отслеживать снижение цены оффера
2. **Временной анализ** - лучшее время для поиска (утро/вечер)
3. **Фильтр расстояния** - только в радиусе 50км
4. **Batch уведомления** - одно сообщение с 3 офферами

### Priority 2 (Сложнее)
5. **Картинки** - показывать фото в уведомлении
6. **Дубликаты** - тот же оффер, новая цена = алерт
7. **Quiet hours** - не беспокоить ночью
8. **Webhooks** - интеграция с другими сервисами

### Priority 3 (Advanced)
9. **ML предсказание** - когда цена упадет
10. **Multi-platform** - eBay, Vinted, etc.
11. **Web dashboard** - красивый UI для статистики

## 🐛 Known Issues

1. ~~Спамит уведомлениями~~ ✅ FIXED (лимит 3 за цикл)
2. ~~При первом запуске шлет на все~~ ✅ FIXED (has_previous_stats)
3. ~~Нет информации о продавце~~ ✅ FIXED (seller risk)
4. ~~Длинные уведомления~~ ✅ FIXED (компактный формат)

## 📝 TODO (Остались)

- [ ] Integration тесты
- [ ] Шифрование токена
- [ ] Quiet hours (тихие часы)
- [ ] Connection pool для SQLite
- [ ] Proxy поддержка
- [ ] Health check endpoint
- [ ] ML предсказание цен

## 💡 Tips & Tricks

1. **Первый запуск**: Дождись 2-3 циклов для накопления статистики
2. **exclude_keywords**: Добавляй все что видишь лишнего
3. **Seller risk**: ✅ = доверяй, 🚫 = осторожно
4. **Score 90+**: Хватай немедленно! 
5. **Score 70-89**: Хорошая сделка
6. **Score 60-69**: Средняя сделка

## 🎮 Быстрые команды

```
/settings - проверить настройки
/top5 - посмотреть лучшие офферы
/cleardb - если что-то сломалось
/refresh - если нужно срочно проверить
```

## 🔥 Best Practices

1. Запускай 24/7 на сервере/VPS
2. Проверяй Telegram каждые 30 мин
3. Действуй быстро на score 90+
4. Добавляй стоп-слова по мере находок
5. Периодически чисть БД (/cleardb)

---

**Built with 🦀 Rust**
**Powered by Teloxide**
**Optimized for Speed**

🚀 **Happy Hunting!**

