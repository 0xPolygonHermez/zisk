#!/bin/bash

source ./utils.sh
export -f ensure

# Copy all *.dylib files from a source directory (including subdirectories)
# to a destination directory while preserving the same directory structure.
#
# Usage:
#   copy_dylibs /path/to/source /path/to/destination
#
copy_dylibs() {
    local SRC_DIR="$1"
    local DEST_DIR="$2"

    # Find all *.dylib files under SRC_DIR
    find "$SRC_DIR" -type f -name "*.dylib" -exec sh -c '
      src="$1"
      dest="$2"
      shift 2

      # Loop over each found file
      for f do
        # Remove the SRC_DIR prefix to get the relative path
        rel="${f#"$src"/}"

        # Destination directory (without the filename)
        target_dir="$dest/$(dirname "$rel")"

        # Print log message
        echo "Copying: $f -> $target_dir"

        # Create the corresponding directory in DEST_DIR
        mkdir -p "$target_dir"

        # Copy the file to the destination, preserving structure
        cp "$f" "$target_dir/"
      done
    ' sh "$SRC_DIR" "$DEST_DIR" {} +
}

main () {
    info "▶️  Running $(basename "$0") script..."

    current_step=1
    total_steps=6

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1
    confirm_continue || return 0

    cd "$(get_zisk_repo_dir)"

    # If ZISK_SETUP_FILE is not set or empty, define it using version from cargo-zisk
    if [[ -z "$ZISK_SETUP_FILE" ]]; then
        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        ZISK_SETUP_FILE="zisk-provingkey-pre-${major}.${minor}.0.tar.gz"
    fi

    info "Using setup file: ${ZISK_SETUP_FILE}"

    step "Downloading public proving key ${ZISK_SETUP_FILE}..."
    ensure curl -L -#o "${ZISK_SETUP_FILE}" "https://storage.googleapis.com/zisk-setup/${ZISK_SETUP_FILE}" || return 1

    step "Renaming build/provingKey directory to build/provingKey-macos..."
    ensure mv build/provingKey "build/provingKey-macos" || return 1

    step "Extracting public proving ${ZISK_SETUP_FILE} to build/provingKey..."
    ensure rm -rf "build/provingKey"
    ensure mkdir -p "build"
    ensure tar -xf "${ZISK_SETUP_FILE}" -C "build" || return 1

    step "Adding macos libraries to build/provingKey..."
    copy_dylibs build/provingKey-macos build/provingKey

    step "Deleting downloaded public proving key..."
    rm -rf "${ZISK_SETUP_FILE}"

    success "Full proving key generated successfully!"
}

main