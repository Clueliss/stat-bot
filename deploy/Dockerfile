FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"
ENV QT_QPA_PLATFORM=offscreen

VOLUME ["/data"]

RUN dnf install gcc g++ git openssl-devel cmake make freetype-devel expat-devel qt5-qtbase-devel qt5-qtbase-gui -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
RUN sh /tmp/rustup.sh -y --default-toolchain=nightly

RUN cd /tmp && git clone https://github.com/Clueliss/stat-bot
RUN sh /tmp/stat-bot/deploy/update.sh

RUN cd /tmp && git clone https://github.com/Clueliss/StatsGraphing
RUN cd /tmp/StatsGraphing && qmake-qt5 && make && cp ./stat-graphing /usr/local/bin && chmod +x /usr/local/bin/stat-graphing

CMD ["/init", "--settings-file", "/data/stat-bot.conf", "--graphing-tool-path", "/usr/local/bin/stat-graphing"]
