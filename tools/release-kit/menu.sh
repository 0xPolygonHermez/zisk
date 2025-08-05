#!/bin/bash

source ./utils.sh

WORK_DIR=$(pwd)

# Loop until user chooses to exit
while true; do
    echo "========================================="
    echo "          ZisK Release Kit Menu          "
    echo "========================================="
    echo " 1) Edit environment variables"
    echo " 2) Build ZisK from source"
    echo " 3) Build setup from source"
    echo " 4) Package setup outcome"
    echo " 5) Install ZisK from binaries"
    echo " 6) Test sha_hasher"
    echo " 7) Test pessimistic proof"
    echo " 8) Test Ethereum block"
    echo " 9) Install setup from public packages"
    echo "10) Install setup from local packages"
    echo "11) Shell"
    echo "12) Exit"
    echo

    # Prompt for input
    read -p "Select an option [1-12]: " option
    echo

    case $option in
        1)
        info "Opening .env file with nano..."
        nano .env
        ;;
        2)
        info "Running build_zisk.sh..."
        bash -i ./build_zisk.sh || :
        ;;
        3)
        info "Running build_setup.sh..."
        bash -i ./build_setup.sh || :
        ;;
        4)
        info "Running package_setup.sh..."
        bash -i ./package_setup.sh || :
        ;;
        5)
        info "Running install_zisk_bin.sh..."
        bash -i ./install_zisk_bin.sh || :
        ;;
        6)
        info "Running test_sha_hasher.sh..."
        bash -i ./test_sha_hasher.sh || :
        ;;
        7)
        info "Running test_pp.sh"
        bash -i ./test_pp.sh || :
        ;;  
        8)
        info "Running test_eth_block.sh"
        bash -i ./test_eth_block.sh || :
        ;;        
        9)
        info "Running install_setup_public.sh..."
        bash -i ./install_setup_public.sh || :
        ;;
        10)
        info "Running install_setup_local.sh..."
        bash -i ./install_setup_local.sh || :
        ;;  
        11)
        info "Open shell"
        bash -i
        ;;   
        12)
        info "Exiting ZisK Release Kit. Goodbye!"
        exit
        ;;
        *)
        info "Invalid selection. Please enter a number between 1 and 11."
        ;;
    esac

    echo

    cd "$WORK_DIR" || {
        err "Failed to change directory to $WORK_DIR"
        exit 1
    }
done
