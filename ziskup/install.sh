#!/usr/bin/env bash

# Reference: https://github.com/foundry-rs/foundry/blob/master/foundryup/install

set -e

echo Installing ziskup...

BASE_DIR=${XDG_CONFIG_HOME:-$HOME}
ZISK_DIR=${ZISK_DIR-"$BASE_DIR/.zisk"}
ZISK_BIN_DIR="$ZISK_DIR/bin"

BIN_URL="https://raw.githubusercontent.com/0xPolygonHermez/zisk/develop/ziskup/ziskup"
BIN_PATH="$ZISK_BIN_DIR/ziskup"

# Create the .zisk bin directory and ziskup binary if it doesn't exist.
mkdir -p $ZISK_BIN_DIR
curl -# -L $BIN_URL -o $BIN_PATH
chmod +x $BIN_PATH

# Store the correct profile file (i.e. .profile for bash or .zshenv for ZSH).
case $SHELL in
*/zsh)
    PROFILE=${ZDOTDIR-"$HOME"}/.zshenv
    PREF_SHELL=zsh
    ;;
*/bash)
    PROFILE=$HOME/.bashrc
    PREF_SHELL=bash
    ;;
*/fish)
    PROFILE=$HOME/.config/fish/config.fish
    PREF_SHELL=fish
    ;;
*/ash)
    PROFILE=$HOME/.profile
    PREF_SHELL=ash
    ;;
*)
    echo "ziskup: could not detect shell, manually add ${ZISK_BIN_DIR} to your PATH."
    exit 1
    ;;
esac

# Only add ziskup if it isn't already in PATH.
if [[ ":$PATH:" != *":${ZISK_BIN_DIR}:"* ]]; then
    # Add the ziskup directory to the path and ensure the old PATH variables remain.
    echo >>$PROFILE && echo "export PATH=\"\$PATH:$ZISK_BIN_DIR\"" >>$PROFILE
fi

# Warn MacOS users that they may need to manually install libusb via Homebrew:
if [[ "$OSTYPE" =~ ^darwin ]] && [[ ! -f /usr/local/opt/libusb/lib/libusb-1.0.0.dylib && ! -f /opt/homebrew/opt/libusb/lib/libusb-1.0.0.dylib ]]; then
    echo && echo "warning: libusb not found. You may need to install it manually on MacOS via Homebrew (brew install libusb)."
fi

# Warn MacOS users that they may need to manually install opensll via Homebrew:
if [[ "$OSTYPE" =~ ^darwin ]] && [[ ! -f /usr/local/opt/openssl/lib/libssl.3.dylib && ! -f /opt/homebrew/opt/openssl/lib/libssl.3.dylib ]]; then
    echo && echo "warning: libusb not found. You may need to install it manually on MacOS via Homebrew (brew install openssl)."
fi

echo && echo "Detected your preferred shell is ${PREF_SHELL} and added ziskup to PATH. Run 'source ${PROFILE}' or start a new terminal session to use ziskup."
echo "Then, simply run 'ziskup' to install ZISK."
