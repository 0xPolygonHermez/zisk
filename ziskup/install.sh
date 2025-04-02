#!/usr/bin/env bash

# Reference: https://github.com/foundry-rs/foundry/blob/master/foundryup/install

set -e

echo Installing ziskup...

BASE_DIR=${XDG_CONFIG_HOME:-$HOME}
ZISK_DIR=${ZISK_DIR-"$BASE_DIR/.zisk"}
ZISK_BIN_DIR="$ZISK_DIR/bin"

BIN_URL="https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/ziskup"
BIN_PATH="$ZISK_BIN_DIR/ziskup"

# Create the .zisk bin directory and ziskup script if it doesn't exist.
mkdir -p $ZISK_BIN_DIR
curl -# -L $BIN_URL -o $BIN_PATH
chmod +x $BIN_PATH

echo && echo "Running ziskup..."
$BIN_PATH

