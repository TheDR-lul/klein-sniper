# Multi-stage build для оптимизации размера образа

# Stage 1: Builder
FROM rust:1.85-slim AS builder

WORKDIR /app

# Установка зависимостей для сборки
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Копируем только файлы зависимостей для кэширования слоя
COPY Cargo.toml Cargo.lock ./

# Создаем dummy main.rs для кэширования зависимостей
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Копируем исходный код
COPY src ./src
COPY build.rs ./
COPY icon.ico ./

# Собираем финальный бинарник
RUN cargo build --release --bin klein-sniper

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Установка runtime зависимостей
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Копируем бинарник из builder stage
COPY --from=builder /app/target/release/klein-sniper /usr/local/bin/klein-sniper

# Создаем непривилегированного пользователя
RUN useradd -m -u 1000 sniper && \
    mkdir -p /app/data && \
    chown -R sniper:sniper /app

USER sniper

# Volume для хранения данных
VOLUME ["/app/data"]

# Переменные окружения по умолчанию
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=60s --timeout=10s --start-period=30s --retries=3 \
    CMD test -f /app/data.db || exit 1

# Запуск приложения
CMD ["klein-sniper"]

