FROM docker.io/fedora

ENV STAT_BOT_DISCORD_TOKEN="YOUR TOKEN HERE"
ENV PATH="/root/.cargo/bin:${PATH}"

VOLUME ["/data/"]

RUN dnf install gcc git openssl-devel -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
RUN sh /tmp/rustup.sh -y --default-toolchain=nightly

COPY ./deploy/update.sh /update
RUN chmod +x /update

RUN /update

CMD ["/init", "--file", "/data/stats.json"]
