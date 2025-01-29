#!/bin/bash

source ./git-utils.sh

# Check if the script has the required parameters
if [[ $# -ne 3 ]]; then
    log_err "Error invalid parameters"
    log "Usage: $0 <RUST_VERSION> <RELEASE_VERSION> <WORKING_DIR>"
    exit 1
fi

TO_VERSION=$1
RELEASE_VERSION=$2
ZISK_RUST_DIR="$(realpath "$3")/rust"

# Change directory to Zisk Rust repository
cd ${ZISK_RUST_DIR}

# Check if we are in ZISK_RUST_DIR directory
if [ "$(pwd)" != "$ZISK_RUST_DIR" ]; then
    log_err "\e[1;31mError changing directory to '${ZISK_RUST_DIR}'"
    exit 1
fi

# Verify current branch is zisk-rust-${TO_VERSION}
current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "zisk-rust-${TO_VERSION}" ]]; then
    log_err "Current branch is not 'zisk-rust-${TO_VERSION}'"
    exit 1
fi

unstaged_files=$(git diff --name-only)
if ! [[ -z "$unstaged_files" ]]; then
    log_info "Unstaged files in the working directory:"
    echo "$unstaged_files"
    read -p "$(log_warn 'There are unstaged files in the working directory. Continue? [y/N] ')" response
    if [[ "$response" != "y" && "$response" != "yes" ]]; then
        log "Exiting..."
        exit 1
    fi
fi

log_info "Committing all staged files to branch 'zisk-rust-${TO_VERSION}'"
log "Commit message 'Update Rust to version ${TO_VERSION}'"
exec_git \
    "git commit -m 'Update Rust to version ${TO_VERSION}'" \
    "Failed to commit changes"

# Push zisk-rust-${TO_VERSION} branch to origin/zisk
log_info "Pushing 'zisk-rust-${TO_VERSION}' branch to origin/zisk"
exec_git \
    "git push origin zisk-rust-${TO_VERSION}:refs/heads/zisk --force" \
    "Failed to push 'zisk-rust-${TO_VERSION}' branch to origin/zisk"

#Checkout zisk branch
log_info "Checking out 'zisk' branch"
exec_git \
    "git checkout zisk" \
    "Failed to checkout 'zisk' branch"
# Verify current branch is zisk-rust-${TO_VERSION}
current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "zisk" ]]; then
    log_err "Current branch is not 'zisk'"
    exit 1
fi

# Resetting local zisk branch
log_info "Resetting local 'zisk' branch"
exec_git \
    "git reset --hard origin/zisk" \
    "Failed to pull 'zisk' branch"

log_info "Tagging branch 'zisk' with tag 'zisk-${RELEASE_VERSION}'"
# Create tag
exec_git \
    "git tag zisk-${RELEASE_VERSION}" \
    "Failed to create local tag 'zisk-${RELEASE_VERSION}'"
# Push tag to origin
exec_git \
    "git push origin zisk-${RELEASE_VERSION}" \
    "Failed to push tag 'zisk-${RUST-ZISK-VERSION}' to origin"

echo
log_info "Done! Now the release GHA should be running, check it"
echo
 