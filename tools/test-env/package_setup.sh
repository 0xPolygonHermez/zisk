#!/bin/bash

source ./utils.sh
export -f ensure

# Package the setup artifacts produced under <zisk-repo>/build into tarballs in
# ${OUTPUT_DIR}. Always produced (provingKey is required):
#   provingKey/                              -> zisk-provingkey-<VER>.tar.gz   (+ .md5)
#   provingKey/.../vadcop_final.verkey.bin   -> zisk-verifykey-<VER>.tar.gz    (+ .md5)
# Produced when present (recursive setup / setup-snark):
#   circom/                                  -> zisk-circuits-<VER>.tar.gz     (+ .md5)
#   provingKeySnark/                         -> zisk-provingkey-plonk-<VER>.tar.gz (+ .md5)
#
# <VER> = PACKAGE_SETUP_VERSION.
#
# Env vars:
#   SETUP_ADD_DYLIBS=1     merge macOS *.dylib into build/provingKey before
#                          packing. Source of the dylibs:
#                            - SETUP_DYLIB_DIR set  -> copy *.dylib straight from
#                              that directory (already extracted; used by CI).
#                            - SETUP_DYLIB_DIR unset -> extract the macOS tarball
#                              ${OUTPUT_DIR}/macos/${PROVINGKEY_FILE} first, then
#                              copy from ${OUTPUT_DIR}/macos/provingKey.
#   PACKAGE_SETUP_UPLOAD=1 after writing the tarballs to ${OUTPUT_DIR}, also
#                          upload them to gs://zisk-setup via `gcloud storage`
#                          (requires gcloud auth). Off by default — packaging is
#                          local-only unless this is set.

BUCKET="gs://zisk-setup"

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

write_md5() {
    local file="$1"
    if command -v md5sum >/dev/null 2>&1; then
        md5sum "$file"
    elif command -v md5 >/dev/null 2>&1; then
        md5 -r "$file"
    else
        echo "no md5 utility found (need md5sum or md5)" >&2
        return 1
    fi
}

pack_dir() {
    local src="$1" tarball="$2"
    shift 2
    # Remaining args are extra tar options (e.g. --exclude globs).
    if [[ ! -d "${src}" ]]; then
        warn "skipping ${tarball} — ${src}/ not found in $(pwd)"
        return 0
    fi
    ensure tar -czvf "${tarball}" "$@" "${src}/" || return 1
    write_md5 "${tarball}" > "${tarball}.md5" || { err "md5 failed for ${tarball}"; return 1; }
    ARTIFACTS+=("${tarball}" "${tarball}.md5")
}

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    # base steps: load env, pack provingKey, pack verifyKey, move = 4. The count
    # is cosmetic and finalized just below — ZISK_REPO_DIR may come from .env, so
    # the repo (and the build-dir probes) must be resolved AFTER load_env.
    current_step=1
    total_steps=4

    step "Loading environment variables..."
    load_env || return 1

    ZISK_REPO="$(get_zisk_repo_dir)"
    ensure cd "${ZISK_REPO}" || return 1

    if [[ "${SETUP_ADD_DYLIBS:-0}" == "1" ]]; then
      if [[ -n "${SETUP_DYLIB_DIR:-}" ]]; then total_steps=$((total_steps + 1)); else total_steps=$((total_steps + 2)); fi
    fi
    [[ -d "build/circom" ]] && total_steps=$((total_steps + 1))
    [[ -d "build/provingKeySnark" ]] && total_steps=$((total_steps + 1))
    [[ "${PACKAGE_SETUP_UPLOAD:-0}" == "1" ]] && total_steps=$((total_steps + 1))

    PROVINGKEY_FILE="zisk-provingkey-${PACKAGE_SETUP_VERSION}.tar.gz"
    VERIFYKEY_FILE="zisk-verifykey-${PACKAGE_SETUP_VERSION}.tar.gz"
    CIRCUITS_FILE="zisk-circuits-${PACKAGE_SETUP_VERSION}.tar.gz"
    SNARK_FILE="zisk-provingkey-plonk-${PACKAGE_SETUP_VERSION}.tar.gz"

    if [[ "$SETUP_ADD_DYLIBS" == "1" ]]; then
      if [[ -n "${SETUP_DYLIB_DIR:-}" ]]; then
        [[ -d "${SETUP_DYLIB_DIR}" ]] || { err "SETUP_DYLIB_DIR=${SETUP_DYLIB_DIR} not found"; return 1; }
        if [[ -d "${SETUP_DYLIB_DIR}/provingKey" ]]; then
          step "Adding macos libraries from ${SETUP_DYLIB_DIR}/provingKey to build/provingKey..."
          copy_dylibs "${SETUP_DYLIB_DIR}/provingKey" build/provingKey
          if [[ -d "${SETUP_DYLIB_DIR}/provingKeySnark" ]]; then
            step "Adding macos snark libraries from ${SETUP_DYLIB_DIR}/provingKeySnark to build/provingKeySnark..."
            copy_dylibs "${SETUP_DYLIB_DIR}/provingKeySnark" build/provingKeySnark
          fi
        else
          step "Adding macos libraries from ${SETUP_DYLIB_DIR} to build/provingKey..."
          copy_dylibs "${SETUP_DYLIB_DIR}" build/provingKey
        fi
      else
        step "Extracting macos proving key to ${OUTPUT_DIR}/macos/provingKey..."
        rm -rf "${OUTPUT_DIR}/macos/provingKey"
        ensure tar --warning=no-unknown-keyword --no-xattrs --no-acls --no-selinux --no-overwrite-dir \
          --exclude '._*' \
          -xf "${OUTPUT_DIR}/macos/${PROVINGKEY_FILE}" \
          -C "${OUTPUT_DIR}/macos" || return 1

        step "Adding macos libraries to build/provingKey..."
        copy_dylibs ${OUTPUT_DIR}/macos/provingKey build/provingKey
      fi
    fi

    ensure cd build || return 1

    ARTIFACTS=()

    step "Compress proving key..."
    [[ -d provingKey ]] || { err "build/provingKey not found — run the setup first"; return 1; }
    pack_dir provingKey "${PROVINGKEY_FILE}" \
      --exclude='*.consttree' \
      --exclude='*.consttree_gpu' \
      --exclude='*.const_gpu' || return 1

    step "Compress verify key..."
    ensure tar -czvf "${VERIFYKEY_FILE}" \
      provingKey/zisk/vadcop_final/vadcop_final.verkey.bin || return 1
    write_md5 "${VERIFYKEY_FILE}" > "${VERIFYKEY_FILE}.md5" || { err "md5 failed for ${VERIFYKEY_FILE}"; return 1; }
    ARTIFACTS+=("${VERIFYKEY_FILE}" "${VERIFYKEY_FILE}.md5")

    if [[ -d circom ]]; then
      step "Compress circom circuits..."
      pack_dir circom "${CIRCUITS_FILE}" || return 1
    fi

    if [[ -d provingKeySnark ]]; then
      step "Compress snark proving key..."
      pack_dir provingKeySnark "${SNARK_FILE}" || return 1
    fi

    step "Move files to output folder..."
    for f in "${ARTIFACTS[@]}"; do
      rm -rf "${OUTPUT_DIR}/${f}"
      ensure mv "${f}" "${OUTPUT_DIR}" || return 1
    done

    if [[ "${PACKAGE_SETUP_UPLOAD:-0}" == "1" ]]; then
      step "Uploading artifacts to ${BUCKET}/..."
      command -v gcloud >/dev/null || { err "gcloud not found in PATH (needed for PACKAGE_SETUP_UPLOAD=1)"; return 1; }
      ( cd "${OUTPUT_DIR}" && ensure gcloud storage cp "${ARTIFACTS[@]}" "${BUCKET}/" ) || return 1
    fi

    cd "${current_dir}"

    success "ZisK setup packaged successfully!"
}

main
