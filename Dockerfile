FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"
ENV QT_QPA_PLATFORM=offscreen

VOLUME ["/data"]

RUN dnf install rust cargo gcc g++ git openssl-devel cmake make qt5-qtbase-devel qt5-qtbase-gui -y

RUN mkdir /tmp/stat-bot
COPY . /tmp/stat-bot

RUN cd /tmp/stat-bot && \
    cargo build --release && \
    rm /init || true > /dev/null && \
    cp ./target/release/stat-bot /init && \
    chmod +x /init && \
    cp ./deploy/stat-bot.conf /data

RUN cd /tmp && git clone https://github.com/Clueliss/StatsGraphing
RUN cd /tmp/StatsGraphing && \
    qmake-qt5 && \
    make && \
    cp ./stat-graphing /usr/local/bin \
    && chmod +x /usr/local/bin/stat-graphing

CMD ["/init", "--settings-file", "/data/stat-bot.conf", "--graphing-tool-path", "/usr/local/bin/stat-graphing"]
