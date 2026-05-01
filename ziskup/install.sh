#!/usr/bin/env bash

# Reference: https://github.com/foundry-rs/foundry/blob/master/foundryup/install

set -e

echo Installing ziskup...

BIN_URL="https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/ziskup"

# In --system mode, bootstrap ziskup to a temp file (running as root, we don't
# want to pollute /root/.zisk/bin). The canonical ziskup will end up inside the
# bundle (e.g. /opt/zisk/bin/ziskup) once the tarball is extracted.
SYSTEM_MODE=false
for arg in "$@"; do
  if [[ "$arg" == "--system" ]]; then
    SYSTEM_MODE=true
    break
  fi
done

if $SYSTEM_MODE; then
  BIN_PATH=$(mktemp /tmp/ziskup-bootstrap.XXXXXX)
  CLEANUP_BOOTSTRAP=true
else
  BASE_DIR=${XDG_CONFIG_HOME:-$HOME}
  ZISK_DIR=${ZISK_DIR-"$BASE_DIR/.zisk"}
  ZISK_BIN_DIR="$ZISK_DIR/bin"
  BIN_PATH="$ZISK_BIN_DIR/ziskup"
  CLEANUP_BOOTSTRAP=false
  mkdir -p "$ZISK_BIN_DIR"
fi

curl -# -L "$BIN_URL" -o "$BIN_PATH"
chmod +x "$BIN_PATH"

echo && echo "Running ziskup..."
"$BIN_PATH" "$@"
RUN_STATUS=$?

if $CLEANUP_BOOTSTRAP; then
  rm -f "$BIN_PATH"
fi

exit $RUN_STATUS
