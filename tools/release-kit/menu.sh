#!/bin/bash

source ./utils.sh

current_dir=$(pwd)

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
        nano .env
        ;;
        2)
        ./build_zisk.sh || :
        ;;
        3)
        ./build_setup.sh || :
        ;;
        4)
        ./package_setup.sh || :
        ;;
        5)
        ./install_zisk_bin.sh || :
        ;;
        6)
        ./test_sha_hasher.sh || :
        ;;
        7)
        ./test_pp.sh || :
        ;;  
        8)
        ./test_eth_block.sh || :
        ;;        
        9)
        ./install_setup_public.sh || :
        ;;
        10)
        ./install_setup_local.sh || :
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
        info "Invalid selection. Please enter a number between 1 and 12."
        ;;
    esac

    echo

    cd "$current_dir" || {
        err "Failed to change directory to $current_dir"
        exit 1
    }
done
