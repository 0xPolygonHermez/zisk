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

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    if [[ "$SETUP_ADD_DYLIBS" == "1" ]]; then
      total_steps=7
    else  
      total_steps=5
    fi

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 0

    cd "$(get_zisk_repo_dir)"

    PROVINGKEY_FILE="zisk-provingkey-${PACKAGE_SETUP_VERSION}.tar.gz"
    VERIFYKEY_FILE="zisk-verifykey-${PACKAGE_SETUP_VERSION}.tar.gz"

    if [[ "$SETUP_ADD_DYLIBS" == "1" ]]; then
      step "Extracting macos proving key to ${OUTPUT_DIR}/macos/provingKey..."
      rm -rf "${OUTPUT_DIR}/macos/provingKey"
      ensure tar --warning=no-unknown-keyword --no-xattrs --no-acls --no-selinux --no-overwrite-dir \
        --exclude '._*' \
        -xf "${OUTPUT_DIR}/macos/${PROVINGKEY_FILE}" \
        -C "${OUTPUT_DIR}/macos" || return 1

      step "Adding macos libraries to build/provingKey..."
      copy_dylibs ${OUTPUT_DIR}/macos/provingKey build/provingKey
    fi

    step "Compress proving key..."
    cd build
    ensure tar -czvf "${PROVINGKEY_FILE}" provingKey/ || return 1

    step "Compress verify key..."
    ensure tar -czvf "${VERIFYKEY_FILE}" \
      provingKey/zisk/vadcop_final/vadcop_final.starkinfo.json \
      provingKey/zisk/vadcop_final/vadcop_final.verkey.json \
      provingKey/zisk/vadcop_final/vadcop_final.verifier.bin || return 1

    step "Generate checksums..."
    ensure md5sum "${PROVINGKEY_FILE}" > "${PROVINGKEY_FILE}.md5" || return 1
    ensure md5sum "${VERIFYKEY_FILE}" > "${VERIFYKEY_FILE}.md5" || return 1

    step "Move files to output folder..."
    rm -rf "${OUTPUT_DIR}/${PROVINGKEY_FILE}"
    ensure mv "${PROVINGKEY_FILE}" "${OUTPUT_DIR}" || return 1
    rm -rf "${OUTPUT_DIR}/${VERIFYKEY_FILE}"
    ensure mv "${VERIFYKEY_FILE}" "${OUTPUT_DIR}" || return 1
    rm -rf "${OUTPUT_DIR}/${PROVINGKEY_FILE}.md5"
    ensure mv "${PROVINGKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1
    rm -rf "${OUTPUT_DIR}/${VERIFYKEY_FILE}.md5"
    ensure mv "${VERIFYKEY_FILE}.md5" "${OUTPUT_DIR}" || return 1

    cd "${current_dir}"

    success "ZisK setup packaged successfully!"
}

main
