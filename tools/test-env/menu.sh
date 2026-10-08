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
    echo " 4) Build dylib files (macOS)"
    echo " 5) Build zec guest ELF (ZEC_GUEST)"
    echo " 6) Upload setup"
    echo " 7) Install ZisK from binaries"
    echo " 8) Test sha_hasher"
    echo " 9) Test Ethereum block"
    echo "10) Test EthProofs"
    echo "11) Test ELF diagnostic"
    echo "12) Test docs examples"
    echo "13) Test quickstart"
    echo "14) Install setup from public packages"
    echo "15) Install setup from local packages"
    echo "16) Shell"
    echo "17) Exit"
    echo

    # Prompt for user selection
    read -p "Select an option [1-17]: " option
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
            run_timed "./build_dylib.sh"
            ;;
        5)
            run_timed "./build_zec_guest.sh"
            ;;
        6)
            run_timed "./upload_setup.sh"
            ;;
        7)
            run_timed "./install_zisk_bin.sh"
            ;;
        8)
            run_timed "./test_sha_hasher.sh"
            ;;
        9)
            run_timed "./test_eth_block.sh"
            ;;
        10)
            run_timed "./test_ethproofs.sh"
            ;;
        11)
            run_timed "./test_diagnostic.sh"
            ;;
        12)
            run_timed "./test_examples.sh"
            ;;
        13)
            run_timed "./test_quickstart.sh"
            ;;
        14)
            run_timed "./install_setup_public.sh"
            ;;
        15)
            run_timed "./install_setup_local.sh"
            ;;
        16)
            info "Open shell"
            bash -i
            ;;
        17)
            info "Exiting ZisK Release Kit. Goodbye!"
            exit
            ;;
        *)
            info "Invalid selection. Please enter a number between 1 and 17."
            ;;
    esac

    echo

    # Always go back to original directory after running scripts
    cd "$current_dir" || {
        err "Failed to change directory to $current_dir"
        exit 1
    }
done
