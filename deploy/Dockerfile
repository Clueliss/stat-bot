FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN=YOUR="TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"

RUN dnf install gcc git openssl-devel -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
RUN sh /tmp/rustup.sh -y --default-toolchain=nightly

RUN git clone https://github.com/Clueliss/stat-bot /tmp/stat-bot

RUN cd /tmp/stat-bot && cargo build --release

RUN cp /tmp/stat-bot/target/release/stat-bot /init

CMD ["/init", "--file", "/stats.json"]
