FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"
ENV QT_QPA_PLATFORM=offscreen

VOLUME ["/data"]

RUN dnf install gcc g++ git openssl-devel cmake make qt5-qtbase-devel qt5-qtbase-gui -y && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh && \
    sh /tmp/rustup.sh -y --default-toolchain=nightly && \
    mkdir /tmp/stat-bot

COPY . /tmp/stat-bot

RUN sh /tmp/stat-bot/deploy/update.sh && \
    cd /tmp && \
    git clone https://github.com/Clueliss/StatsGraphing && \
    cd StatsGraphing && \
    qmake-qt5 && \
    make && \
    cp ./stat-graphing /usr/local/bin && \
    chmod +x /usr/local/bin/stat-graphing

CMD ["/init", "--settings-file", "/data/stat-bot.conf", "--graphing-tool-path", "/usr/local/bin/stat-graphing"]
