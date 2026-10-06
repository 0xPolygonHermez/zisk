#!/bin/bash

set -u

# PROOFMAN_CRATES and ZISK_CRATES (the publish lists) and the shared helpers.
source "$(dirname "${BASH_SOURCE[0]}")/publish_common.sh"

declare_optional_option --proofman-repo-path \
"Path to the pil2-proofman repository to check
PROOFMAN_CRATES against."

declare_optional_option --zisk-repo-path \
"Path to the ZisK repository to check ZISK_CRATES
against."

parse_args "$@"

# Either repo can be checked on its own, but checking neither is not a run.
if [ -z "$PROOFMAN_REPO_PATH" ] && [ -z "$ZISK_REPO_PATH" ]; then
    echo "ERROR: at least one of --proofman-repo-path or --zisk-repo-path is required."
    usage
fi

resolve_repo_path --proofman-repo-path PROOFMAN_REPO_PATH
resolve_repo_path --zisk-repo-path ZISK_REPO_PATH

for command in cargo jq; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "ERROR: $command is required but was not found in PATH."
        exit 1
    fi
done

# ------------------------------------------------------------
# Checks
#
# What the repo says is read with cargo metadata rather than with cargo publish
# --dry-run: the dry run refuses to run at all while a dependency points at a
# git repo instead of a version, which is how the ZisK workspace sits until the
# switch to the crates.io dependencies is made right before a release.
#
# A crate is meant to be published unless its manifest opts out with
# publish = false, which cargo metadata reports as an empty publish list.
# ------------------------------------------------------------

# fail <reason> [detail]...
#
# Stops at the first crate that does not add up. This is a gate: whatever the
# rest of the crates or the other repo say, the lists have to be fixed before
# anything is published, so there is nothing to gain by carrying on.
fail() {
    local reason="$1"
    shift

    echo
    echo "ERROR: $reason"

    if [ $# -gt 0 ]; then
        echo
        printf '%s\n' "$@"
    fi

    exit 1
}

# listed <crate> <newline separated crates>
listed() {
    printf '%s\n' "$2" | grep -qxF -- "$1"
}

# depends_on <crate> <"crate<TAB>dependency" lines>
depends_on() {
    printf '%s\n' "$2" | awk -F'\t' -v crate="$1" '$1 == crate { print $2 }'
}

# check_crates <group> <list-name> <repo> <crate>...
#
# Compares the crates the repo marks as publishable with the ones the list
# holds, and checks that the list order can be published: every crate has to
# come after the workspace crates it depends on. Only the crates in the list are
# considered dependencies here, since anything outside it is reported already.
#
# The list is walked in its own order, printing each crate as it is taken on, so
# that the run shows how far it got and which crate the error below it belongs
# to.
check_crates() {
    local group="$1"
    local list_name="$2"
    local repo="$3"
    shift 3
    local expected=("$@")

    local metadata publishable expected_list dependencies seen crate dependency

    section \
        "$group crates" \
        "Repo: $repo"
    echo

    if ! metadata=$(cd "$repo" && cargo metadata --no-deps --format-version 1 2>&1); then
        fail "cargo metadata failed in $repo" "$metadata"
    fi

    publishable=$(printf '%s' "$metadata" |
        jq -r '.packages[] | select(.publish != []) | .name' | sort)

    expected_list=$(printf '%s\n' "${expected[@]}")

    # Pairs of "crate<TAB>dependency" for the workspace crates each publishable
    # crate depends on. Dev dependencies are left out: cargo strips the ones
    # without a version and they do not constrain the publish order.
    dependencies=$(printf '%s' "$metadata" | jq -r '
        .packages[]
        | select(.publish != [])
        | .name as $crate
        | .dependencies[]
        | select(.kind == null or .kind == "build")
        | "\($crate)\t\(.name)"' | sort -u)

    echo "Marked to publish in the repo: $(printf '%s\n' "$publishable" | grep -c .)"
    echo "Listed in $list_name: ${#expected[@]}"
    echo

    # Every crate in the list has to be one the repo publishes, only once, and
    # after everything it depends on. Since the list is walked in order, a
    # dependency that is in the list but has not been seen yet is one this crate
    # would be published before.
    seen=""

    for crate in "${expected[@]}"; do
        printf '  %-35s ' "$crate"

        if ! listed "$crate" "$publishable"; then
            fail "$crate is listed in $list_name but the repo does not publish it"
        fi

        if listed "$crate" "$seen"; then
            fail "$crate is listed twice in $list_name"
        fi

        for dependency in $(depends_on "$crate" "$dependencies"); do
            if listed "$dependency" "$expected_list" &&
               ! listed "$dependency" "$seen"; then
                fail "$crate comes before $dependency in $list_name, which it depends on"
            fi
        done

        seen="$seen$crate"$'\n'
        echo "OK"
    done

    # And the other way round: a crate the repo publishes that nobody added to
    # the list would be left behind by publish_crates.sh.
    while IFS= read -r crate; do
        if ! listed "$crate" "$expected_list"; then
            fail "$crate is marked to publish in the repo but is missing from $list_name"
        fi
    done <<< "$publishable"

    echo
    echo "  ✓ $list_name matches the crates the repo publishes, in dependency order"
}

if [ -n "$PROOFMAN_REPO_PATH" ]; then
    check_crates "Proofman" "PROOFMAN_CRATES" "$PROOFMAN_REPO_PATH" \
        "${PROOFMAN_CRATES[@]}"
fi

if [ -n "$ZISK_REPO_PATH" ]; then
    check_crates "ZisK" "ZISK_CRATES" "$ZISK_REPO_PATH" "${ZISK_CRATES[@]}"
fi

section "All checks passed."
