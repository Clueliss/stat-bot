#!/bin/bash

set -e

cd /tmp/stat-bot
git pull

cargo build --release

rm /init || true
cp ./target/release/stat-bot /init
chmod +x /init

rm /update || true
cp ./deploy/update.sh /update
chmod +x /update

echo "success"
