#!/bin/bash

source ./git-utils.sh

# Check if the script has the required parameters
if [[ $# -ne 3 ]]; then
    log_err "Error invalid parameters"
    log "Usage: $0 <FROM_VERSION> <TO_VERSION> <WORKING_DIR>"
    exit 1
fi

FROM_VERSION=$1
TO_VERSION=$2
ZISK_RUST_DIR="$(realpath "$3")/rust"
REMOTE_NAME="upstream"
CURRENT_FOLDER=$(pwd)

log_info "Updating Rust version from ${FROM_VERSION} to ${TO_VERSION}"

if [ ! -d "$ZISK_RUST_DIR" ]; then
    # Cloning Zisk Rust repository
    log_info "Cloning Zisk Rust repository in directory ${ZISK_RUST_DIR} (this will take some minutes)"
    exec_git \
        "git clone git@github.com:0xPolygonHermez/rust.git ${ZISK_RUST_DIR}" \
        "Failed to clone Zisk Rust repository"

    # Change directory to Zisk Rust repository
    cd ${ZISK_RUST_DIR}

else
    log_info "Zisk Rust repository already exists in directory ${ZISK_RUST_DIR}"
    # Change directory to Zisk Rust repository
    cd ${ZISK_RUST_DIR}
fi

# Check if we are in ZISK_RUST_DIR directory
if [ "$(pwd)" != "$ZISK_RUST_DIR" ]; then
    log_err "\e[1;31mError changing directory to '${ZISK_RUST_DIR}'"
    exit 1
fi

# Check and add the remote if it doesn't exist
log_info "Checking if remote '${REMOTE_NAME}' exists"
if ! git remote | grep -q "^${REMOTE_NAME}$"; then
    log "Remote '${REMOTE_NAME}' does not exist. Adding it"
    exec_git \
        "git remote add ${REMOTE_NAME} git@github.com:rust-lang/rust.git" \
        "Failed to add remote '${REMOTE_NAME}'"
else
    log "Remote '${REMOTE_NAME}' already exists."
fi

# Fetch remote
log_info "Fetching remote '${REMOTE_NAME}'"
exec_git \
    "git fetch ${REMOTE_NAME}" \
    "Failed to fetch remote '${REMOTE_NAME}' branches"

# Create and checkout new branch zisk-rust-${TO_VERSION} from upstream/stable branch
log_info "Creating and check out new branch 'zisk-rust-${TO_VERSION}' from 'upstream/stable' branch"
exec_git \
    "git checkout -b zisk-rust-${TO_VERSION} ${REMOTE_NAME}/stable" \
    "Failed to create new branch 'zisk-rust-${TO_VERSION}'"

# Checkout 'zisk' branch
log_info "Checking out 'zisk' branch"
exec_git \
    "git checkout zisk" \
    "Failed to checkout 'zisk' branch"
# Verify current branch is zisk
current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "zisk" ]]; then
    log_err "\e[1;31mCurrent branch is not 'zisk'."
    exit 1
fi

# List cherry picks
log_info "List of cherry picks to apply:"
exec_git \
    "git log --oneline --no-decorate --reverse ${FROM_VERSION}..HEAD" \
    "Failed to list cherry picks"
# Store the list of cherry-pick commits to apply in an array
commits_array=$(exec_git \
    "git log --oneline --no-decorate --reverse ${FROM_VERSION}..HEAD" \
    "Failed to get cherry picks")
IFS=$'\n' read -d '' -r -a commits_array <<< "$commits_array"
log_warn "Press a key to continue..."
read -n1 -s

# Checkout 'zisk-rust-${TO_VERSION}' branch
log_info "Checking out 'zisk-rust-${TO_VERSION}' branch"
exec_git \
    "git checkout zisk-rust-${TO_VERSION}" \
    "Failed to checkout 'zisk-rust-${TO_VERSION}' branch"
# Verify current branch is zisk-rust-${TO_VERSION}
current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" != "zisk-rust-${TO_VERSION}" ]]; then
    log_err "\e[1;31mCurrent branch is not 'zisk-rust-${TO_VERSION}'."
    exit 1
fi

# Apply cherry picks to branch zisk-rust-${TO_VERSION}
for line in "${commits_array[@]}"; do
    commit=$(echo "$line" | awk '{print $1}')
    msg=$(echo "$line" | cut -d' ' -f2-)
    
    log_info "Applying cherry pick for commit: ${msg} (${commit})"
    output=$(git cherry-pick $commit -n 2>&1)
    if ! [[ $? -eq 0 ]]; then
        if [[ "$output" == *"CONFLICT"* ]]; then
            printf "%s\n" "$output"
            log_warn "\e[1;33mThe are CONFLICTS, please resolve them and after press a key to continue"
            read -n1 -s
        else
            log_err "\e[1;31mFailed to apply cherry pick for commit: ${msg} (${commit})"
            printf "%s\n" "$output"
            exit 1
        fi
    fi
done

# Final instructions
echo
log_info "Now test build Zisk tool chain using the rust code in the directory ${ZISK_RUST_DIR} and new branch 'zisk-rust-${TO_VERSION}'"
log_info "When successfully tested, execute the following command to commit/merge the changes to 'zisk' branch and generate the release:"
echo
log "./release-rust.sh ${TO_VERSION} <RELEASE_VERSION> <WORKING_DIR>"
log "Example: ./release-rust.sh ${TO_VERSION} 0.5.0 $(realpath "$3")"
echo
