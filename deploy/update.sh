#!/bin/bash

set -e

cd /tmp
git clone https://github.com/Clueliss/stat-bot
cd ./stat-bot

cargo build --release

rm /init
cp ./target/release/stat-bot /init
chmod +x /init

rm /update
cp ./deploy/update.sh /update
chmod +x /update

echo "success"
