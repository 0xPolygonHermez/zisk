#!/bin/bash

source ./utils.sh
source ${HOME}/.bashrc

main() {
    current_step=1
    total_steps=1

    step "Installing ZisK from binaries..."
    ensure curl https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/ziskup/install.sh | bash
    source "${HOME}/.bashrc"
    ensure cargo-zisk --version || return 1
}

main || return 1