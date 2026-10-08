#!/bin/bash

set -u

# crates.io throttles publishes in two independent buckets: creating a crate name
# ("new") and uploading a version of a name that already exists ("update"). Each
# allows a burst and then one publish per interval, so they are tracked apart.
# Both buckets belong to the crates.io token rather than to a repo, so a run that
# publishes the two repos shares them.
NEW_BURST=5
NEW_WAIT_SECONDS=630     # 10 minutos y 30 segundos
UPDATE_BURST=30
UPDATE_WAIT_SECONDS=70   # 1 minuto y 10 segundos

RETRY_WAIT_SECONDS=60    # 1 minuto

# PROOFMAN_CRATES and ZISK_CRATES (the publish order) and the shared helpers.
source "$(dirname "${BASH_SOURCE[0]}")/publish_common.sh"

declare_optional_option --proofman-repo-path \
"Path to the pil2-proofman repository its crates are
published from. They go first when both repos are given,
since the ZisK crates depend on them."

declare_optional_option --zisk-repo-path \
"Path to the ZisK repository its crates are published
from. Unrelated to where these scripts live: both can sit
in different checkouts, or anywhere else."

declare_option --version \
"Version to publish. Must match the [workspace.package]
version in the Cargo.toml of every given repo. Crates
already published with it are skipped (crates.io versions
are immutable), so a run can be repeated to finish the
rest."

parse_args "$@"

# Either repo can be published on its own, but publishing neither is not a run.
if [ -z "$PROOFMAN_REPO_PATH" ] && [ -z "$ZISK_REPO_PATH" ]; then
    echo "ERROR: at least one of --proofman-repo-path or --zisk-repo-path is required."
    usage
fi

# ------------------------------------------------------------
# Repos
# ------------------------------------------------------------

# check_repo_version <VAR>
#
# Checks the repo held by <VAR>, if any, against --version. cargo publish takes
# the version from the manifest, not from this script, so a --version
# disagreeing with the workspace one would check a version on crates.io and
# upload another.
check_repo_version() {
    local path="${!1}"
    local version

    if [ -z "$path" ]; then
        return 0
    fi

    version=$(workspace_version "$path")

    if [ -z "$version" ]; then
        echo "ERROR: could not read [workspace.package] version from $path/Cargo.toml"
        exit 1
    fi

    if [ "$VERSION" != "$version" ]; then
        echo "ERROR: --version $VERSION does not match the workspace version" \
             "$version in $path/Cargo.toml."
        echo "Bump [workspace.package] version in Cargo.toml first."
        exit 1
    fi
}

resolve_repo_path --proofman-repo-path PROOFMAN_REPO_PATH
resolve_repo_path --zisk-repo-path ZISK_REPO_PATH

check_repo_version PROOFMAN_REPO_PATH
check_repo_version ZISK_REPO_PATH

# ------------------------------------------------------------
# Work list
#
# One flat list of crates in publish order, each paired with the repo it is
# published from: the proofman crates first, since the ZisK ones depend on them.
# ------------------------------------------------------------

WORK_CRATES=()
WORK_REPOS=()
WORK_GROUPS=()

# add_crates <group> <repo> <crate>...
add_crates() {
    local group="$1"
    local repo="$2"
    shift 2
    local crate

    for crate in "$@"; do
        WORK_CRATES+=("$crate")
        WORK_REPOS+=("$repo")
        WORK_GROUPS+=("$group")
    done
}

if [ -n "$PROOFMAN_REPO_PATH" ]; then
    add_crates "Proofman" "$PROOFMAN_REPO_PATH" "${PROOFMAN_CRATES[@]}"
fi

if [ -n "$ZISK_REPO_PATH" ]; then
    add_crates "ZisK" "$ZISK_REPO_PATH" "${ZISK_CRATES[@]}"
fi

# ------------------------------------------------------------
# Countdown
# ------------------------------------------------------------

countdown() {
    local seconds="$1"
    local message="$2"

    while [ "$seconds" -gt 0 ]; do
        local minutes=$((seconds / 60))
        local secs=$((seconds % 60))

        printf "\r\033[K%s %02d:%02d" \
            "$message" \
            "$minutes" \
            "$secs"

        sleep 1
        seconds=$((seconds - 1))
    done

    printf "\r\033[K%s 00:00\n" "$message"
}

# ------------------------------------------------------------
# Publish crate with one retry
#
# cargo publish picks the workspace from the current directory, so every crate is
# published from a subshell sitting in its own repo.
# ------------------------------------------------------------

publish_crate() {
    local repo="$1"
    local crate="$2"

    echo
    echo "Publishing $crate $VERSION..."
    echo

    if (cd "$repo" && cargo publish -p "$crate"); then
        return 0
    fi

    echo
    echo "WARNING: Failed to publish $crate."
    echo

    countdown "$RETRY_WAIT_SECONDS" \
        "Retrying $crate | Attempt 2/2 | Waiting:"

    echo
    echo "Retrying $crate $VERSION..."
    echo

    if (cd "$repo" && cargo publish -p "$crate"); then
        return 0
    fi

    echo
    echo "ERROR: Failed to publish $crate after 2 attempts."
    return 1
}

# ------------------------------------------------------------
# Check all crates
#
# Each pending crate is classified as new/update here, since that decides which
# rate limit applies to it while publishing.
# ------------------------------------------------------------

PENDING_CRATES=()
PENDING_REPOS=()
PENDING_KINDS=()
PUBLISHED_CRATES=()
NEW_COUNT=0
UPDATE_COUNT=0

HEADER=("Checking crates.io for version $VERSION")

if [ -n "$PROOFMAN_REPO_PATH" ]; then
    HEADER+=("Proofman repo: $PROOFMAN_REPO_PATH")
fi

if [ -n "$ZISK_REPO_PATH" ]; then
    HEADER+=("ZisK repo: $ZISK_REPO_PATH")
fi

section "${HEADER[@]}"

group=""

for ((i=0; i<${#WORK_CRATES[@]}; i++)); do

    crate="${WORK_CRATES[$i]}"

    if [ "${WORK_GROUPS[$i]}" != "$group" ]; then
        group="${WORK_GROUPS[$i]}"
        echo
        echo "${group} crates:"
        echo
    fi

    is_published "$crate"
    status=$?

    if [ "$status" -eq 2 ]; then
        echo "Stopping because crates.io check failed."
        exit 1
    fi

    # Already on crates.io with this exact version: nothing to do, versions are
    # immutable there. Skipping lets a broken run be repeated as is.
    if [ "$status" -eq 0 ]; then
        echo "SKIP  $crate $VERSION"
        PUBLISHED_CRATES+=("$crate")
        continue
    fi

    crate_exists "$crate"
    status=$?

    if [ "$status" -eq 2 ]; then
        echo "Stopping because crates.io check failed."
        exit 1
    fi

    if [ "$status" -eq 0 ]; then
        kind="update"
        UPDATE_COUNT=$((UPDATE_COUNT + 1))
    else
        kind="new"
        NEW_COUNT=$((NEW_COUNT + 1))
    fi

    echo "TODO  $crate $VERSION ($kind)"
    PENDING_CRATES+=("$crate")
    PENDING_REPOS+=("${WORK_REPOS[$i]}")
    PENDING_KINDS+=("$kind")

done

# ------------------------------------------------------------
# Summary
# ------------------------------------------------------------

PUBLISHED_COUNT=${#PUBLISHED_CRATES[@]}
PENDING_COUNT=${#PENDING_CRATES[@]}

section "Summary"
echo
echo "Total crates:       ${#WORK_CRATES[@]}"
echo "Already published:  $PUBLISHED_COUNT"
echo "To publish:         $PENDING_COUNT ($NEW_COUNT new, $UPDATE_COUNT update)"
echo

if [ "$PENDING_COUNT" -eq 0 ]; then
    echo "All crates are already published with $VERSION."
    exit 0
fi

echo "Crates to publish:"
for ((i=0; i<PENDING_COUNT; i++)); do
    echo "  - ${PENDING_CRATES[$i]} (${PENDING_KINDS[$i]})"
done

echo

# ------------------------------------------------------------
# Publish pending crates
#
# One token bucket per kind: NEW_BURST publishes back to back, then one every
# NEW_WAIT_SECONDS; likewise UPDATE_BURST / UPDATE_WAIT_SECONDS. The buckets are
# independent, so an update never consumes the allowance for a new crate, but
# each one is shared by both repos. Each wait happens before its publish, so
# nothing is spent after the last one.
# ------------------------------------------------------------

TOTAL=$PENDING_COUNT
NEW_USED=0
UPDATE_USED=0

for ((i=0; i<TOTAL; i++)); do

    crate="${PENDING_CRATES[$i]}"
    repo="${PENDING_REPOS[$i]}"
    kind="${PENDING_KINDS[$i]}"
    pending_left=$((TOTAL - i))

    if [ "$kind" = "new" ]; then
        burst=$NEW_BURST
        used=$NEW_USED
        wait_seconds=$NEW_WAIT_SECONDS
    else
        burst=$UPDATE_BURST
        used=$UPDATE_USED
        wait_seconds=$UPDATE_WAIT_SECONDS
    fi

    if [ "$used" -lt "$burst" ]; then
        echo
        echo "Within the $kind burst ($((used + 1))/$burst). No wait."
        echo "Next crate: $crate"
        echo "Pending:    $pending_left"
        echo
    else
        echo
        echo "The $kind burst ($burst) is spent."
        echo "Next crate: $crate"
        echo "Pending:    $pending_left"
        echo

        countdown "$wait_seconds" \
            "Next: $crate ($kind) | Pending: $pending_left | Waiting:"

        echo
    fi

    section \
        "Publishing [$((i + 1))/$TOTAL] $crate $VERSION ($kind)" \
        "Repo: $repo"

    if ! publish_crate "$repo" "$crate"; then
        section "ERROR: Publishing stopped at $crate"
        echo
        echo "Rerun to continue with the remaining crates."
        exit 1
    fi

    echo
    echo "OK: $crate $VERSION published successfully."

    if [ "$kind" = "new" ]; then
        NEW_USED=$((NEW_USED + 1))
    else
        UPDATE_USED=$((UPDATE_USED + 1))
    fi

done

section "All pending crates have been published successfully."
