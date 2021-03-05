FROM rustlang/rust:nightly-buster-slim AS builder
RUN apt-get update
RUN apt-get install pkg-config libssl-dev cmake make g++ libfreetype6-dev libexpat1-dev libfontconfig1-dev -y

WORKDIR /usr/src/stat-bot
COPY Cargo.toml ./
COPY ./src ./src
RUN cargo build --release


FROM debian:buster-slim

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"

VOLUME ["/data"]

RUN apt-get update
RUN apt-get install libssl-dev ca-certificates libexpat1-dev libfreetype6 fonts-liberation2 libfontconfig1-dev -y

RUN mkdir /tmp/stat-bot
COPY . /tmp/stat-bot

COPY --from=builder /usr/src/stat-bot/target/release/stat-bot /usr/local/bin/
RUN chmod +x /usr/local/bin/stat-bot

CMD ["/usr/local/bin/stat-bot", "--settings-file", "/data/stat-bot.conf"]
