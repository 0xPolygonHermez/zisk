#!/bin/bash
#
# Package and upload the ZisK setup to gs://zisk-setup. The setup must be built
# beforehand (build_setup.sh) — this script requires build/provingKey, computes
# the setup hash via `setup_build.sh --print-hash`, and skips the upload when the
# bucket already holds that hash. The hash is published as the <name>.hash sidecar.
#
# Artifacts (<VER> = SETUP_VERSION, <SFX> = SETUP_NAME_SUFFIX, empty by default):
#   provingKey/                            -> zisk-provingkey-<VER><SFX>.tar.gz       (+ .md5)
#   provingKey/.../vadcop_final.verkey.bin -> zisk-verifykey-<VER><SFX>.tar.gz        (+ .md5)
#   circom/            (if present)        -> zisk-circuits-<VER><SFX>.tar.gz         (+ .md5)
#   provingKeySnark/   (if present)        -> zisk-provingkey-plonk-<VER><SFX>.tar.gz (+ .md5)
#
# Every tarball present is uploaded. circom/ comes out of the recursive setup;
# provingKeySnark/ only when INCLUDE_SNARK=1.
#
# Env vars:
#   SETUP_VERSION       version tag <VER> in the tarball names.
#   SETUP_NAME_SUFFIX   suffix after <VER> in every artifact name, .hash sidecar
#                       included, so a setup built with a non-default hash family
#                       publishes next to the default one (e.g. "-blake3").
#   FORCE_UPLOAD        upload even if the bucket .hash already matches (no gate).
#   SETUP_ADD_DYLIBS    merge macOS *.dylib into build/provingKey before packing
#                       (from SETUP_DYLIB_DIR, else the macOS tarball in ${OUTPUT_DIR}/macos).
#   SETUP_PRUNE_LOCAL   delete this run's artifacts from ${OUTPUT_DIR} once
#                       uploaded. For CI, where that directory is a shared volume.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/utils.sh"

BUCKET="gs://zisk-setup"

# Copy all *.dylib under $1 into $2, preserving the directory structure.
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

# copy_dylibs mkdir -p's its destination, so without this a lost build/<tree>
# silently becomes a new one holding just the dylibs — packed and uploaded as-is.
require_tree() {
    local dir="$1"
    [[ -d "${dir}" ]] || { err "${dir} not found — refusing to merge macOS dylibs into a missing tree"; return 1; }
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
    # Empty, or nothing but dylibs, means the Linux setup went missing (see
    # require_tree); packing it would replace a complete tarball in the bucket.
    if ! find "${src}" -type f ! -name '*.dylib' -print -quit | grep -q .; then
        err "${src}/ has no non-dylib files — refusing to pack ${tarball}"
        return 1
    fi
    ensure tar -czvf "${tarball}" "$@" "${src}/" || return 1
    write_md5 "${tarball}" > "${tarball}.md5" || { err "md5 failed for ${tarball}"; return 1; }
    ARTIFACTS+=("${tarball}" "${tarball}.md5")
}

main() {
    info "▶️  Running $(basename "$0") script..."

    command -v gcloud >/dev/null || { err "gcloud not found in PATH (needed to read/upload the setup)"; return 1; }

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ZISK_REPO_DIR SETUP_VERSION SETUP_NAME_SUFFIX SETUP_ADD_DYLIBS FORCE_UPLOAD \
        SETUP_DYLIB_DIR SETUP_PRUNE_LOCAL || return 1

    ZISK_REPO="$(get_zisk_repo_dir)"
    ensure cd "${ZISK_REPO}" || return 1

    current_dir=$(pwd)

    # The setup must already be built (by build_setup.sh).
    [[ -d build/provingKey ]] || { err "build/provingKey not found — run build_setup.sh first"; return 1; }

    # Compute the setup hash and gate on the bucket. --print-hash runs frops +
    # compute_input_hash only (no compile-pil / setup) and prints the 64-hex hash
    # as its sole stdout line, so the same hasher that keyed the build is reused.
    info "Computing setup hash..."
    SETUP_HASH="$("${SCRIPT_DIR}/setup_build.sh" --print-hash --build-dir build)" || return 1
    [[ -n "${SETUP_HASH}" ]] || { err "failed to compute setup hash"; return 1; }
    info "Setup hash: ${SETUP_HASH}"

    # HASH_FILE follows PROVINGKEY_FILE, so the gate below reads the sidecar of
    # this exact variant.
    SETUP_NAME_SUFFIX="${SETUP_NAME_SUFFIX:-}"
    PROVINGKEY_FILE="zisk-provingkey-${SETUP_VERSION}${SETUP_NAME_SUFFIX}.tar.gz"
    VERIFYKEY_FILE="zisk-verifykey-${SETUP_VERSION}${SETUP_NAME_SUFFIX}.tar.gz"
    CIRCUITS_FILE="zisk-circuits-${SETUP_VERSION}${SETUP_NAME_SUFFIX}.tar.gz"
    SNARK_FILE="zisk-provingkey-plonk-${SETUP_VERSION}${SETUP_NAME_SUFFIX}.tar.gz"
    HASH_FILE="${PROVINGKEY_FILE%.tar.gz}.hash"
    info "Publishing as ${PROVINGKEY_FILE} (gate: ${HASH_FILE})"

    if [[ "${FORCE_UPLOAD:-0}" != "1" ]]; then
      local remote_hash
      remote_hash="$(gcloud storage cat "${BUCKET}/${HASH_FILE}" 2>/dev/null | tr -d '[:space:]' || true)"
      if [[ "${remote_hash}" == "${SETUP_HASH}" ]]; then
        success "${PROVINGKEY_FILE} already in ${BUCKET} (hash matches), nothing to do."
        return 0
      fi
    fi

    current_step=1
    total_steps=5
    if [[ "${SETUP_ADD_DYLIBS:-0}" == "1" ]]; then
      if [[ -n "${SETUP_DYLIB_DIR:-}" ]]; then
        total_steps=$((total_steps + 1))
        # Extra step only when both dylib subdirs exist (snark step is nested).
        [[ -d "${SETUP_DYLIB_DIR}/provingKey" && -d "${SETUP_DYLIB_DIR}/provingKeySnark" ]] && total_steps=$((total_steps + 1))
      else
        total_steps=$((total_steps + 2))
      fi
    fi

    [[ -d "build/circom" ]] && total_steps=$((total_steps + 1))
    [[ -d "build/provingKeySnark" ]] && total_steps=$((total_steps + 1))
    [[ "${SETUP_PRUNE_LOCAL:-0}" == "1" ]] && total_steps=$((total_steps + 1))

    if [[ "$SETUP_ADD_DYLIBS" == "1" ]]; then
      if [[ -n "${SETUP_DYLIB_DIR:-}" ]]; then
        [[ -d "${SETUP_DYLIB_DIR}" ]] || { err "SETUP_DYLIB_DIR=${SETUP_DYLIB_DIR} not found"; return 1; }
        if [[ -d "${SETUP_DYLIB_DIR}/provingKey" ]]; then
          step "Adding macos libraries from ${SETUP_DYLIB_DIR}/provingKey to build/provingKey..."
          require_tree build/provingKey || return 1
          copy_dylibs "${SETUP_DYLIB_DIR}/provingKey" build/provingKey
          if [[ -d "${SETUP_DYLIB_DIR}/provingKeySnark" ]]; then
            step "Adding macos snark libraries from ${SETUP_DYLIB_DIR}/provingKeySnark to build/provingKeySnark..."
            require_tree build/provingKeySnark || return 1
            copy_dylibs "${SETUP_DYLIB_DIR}/provingKeySnark" build/provingKeySnark
          fi
        else
          step "Adding macos libraries from ${SETUP_DYLIB_DIR} to build/provingKey..."
          require_tree build/provingKey || return 1
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
        require_tree build/provingKey || return 1
        copy_dylibs "${OUTPUT_DIR}/macos/provingKey" build/provingKey
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

    step "Uploading artifacts to ${BUCKET}/..."
    ( cd "${OUTPUT_DIR}" && ensure gcloud storage cp "${ARTIFACTS[@]}" "${BUCKET}/" ) || return 1

    # Publish the <name>.hash sidecar (content = SETUP_HASH) — the gate file.
    step "Uploading proving key hash sidecar ${HASH_FILE}..."
    printf '%s' "${SETUP_HASH}" > "${OUTPUT_DIR}/${HASH_FILE}" || { err "failed to write ${HASH_FILE}"; return 1; }
    ( cd "${OUTPUT_DIR}" && ensure gcloud storage cp "${HASH_FILE}" "${BUCKET}/" ) || return 1

    # Already in the bucket. In CI ${OUTPUT_DIR} is shared with the setup cache and
    # the ptau, so these must not pile up run after run.
    if [[ "${SETUP_PRUNE_LOCAL:-0}" == "1" ]]; then
      step "Removing uploaded artifacts from ${OUTPUT_DIR}..."
      for f in "${ARTIFACTS[@]}" "${HASH_FILE}"; do
        rm -f "${OUTPUT_DIR}/${f}"
      done
    fi

    cd "${current_dir}"

    success "ZisK setup packaged and uploaded successfully!"
}

main
