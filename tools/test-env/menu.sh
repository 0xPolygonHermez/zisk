#!/bin/bash

source ./utils.sh

current_dir=$(pwd)

# Main menu loop
while true; do
    echo "========================================="
    echo "          ZisK Test Menu          "
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

    # Prompt for user selection
    read -p "Select an option [1-12]: " option
    echo

    case $option in
        1)
            nano .env
            ;;
        2)
            run_timed "./build_zisk.sh"
            ;;
        3)
            run_timed "./build_setup.sh"
            ;;
        4)
            run_timed "./package_setup.sh"
            ;;
        5)
            run_timed "./install_zisk_bin.sh"
            ;;
        6)
            run_timed "./test_sha_hasher.sh"
            ;;
        7)
            run_timed "./test_pp.sh"
            ;;
        8)
            run_timed "./test_eth_block.sh"
            ;;
        9)
            run_timed "./install_setup_public.sh"
            ;;
        10)
            run_timed "./install_setup_local.sh"
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

    # Always go back to original directory after running scripts
    cd "$current_dir" || {
        err "Failed to change directory to $current_dir"
        exit 1
    }
done
