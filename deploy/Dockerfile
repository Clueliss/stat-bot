FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"

VOLUME ["/data"]

RUN dnf install gcc g++ git openssl-devel cmake make freetype-devel expat-devel -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
RUN sh /tmp/rustup.sh -y --default-toolchain=nightly

RUN cd /tmp && git clone https://github.com/Clueliss/stat-bot
RUN sh /tmp/stat-bot/deploy/update.sh

CMD ["/init", "--settings-file", "/data/stat-bot.conf", "--graphing-tool-path", "/usr/local/bin/stat-graphing"]
