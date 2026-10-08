#!/bin/bash

set -u

# PROOFMAN_CRATES and ZISK_CRATES (the publish order) and the shared helpers.
source "$(dirname "${BASH_SOURCE[0]}")/publish_common.sh"

declare_option --version \
"Version to look for on crates.io. Any version may be
checked, published or not: unlike publish_crates.sh this
one only reads crates.io, so it needs no repo checkout."

parse_args "$@"

section \
    "Checking Proofman and ZisK crates" \
    "Version: $VERSION"

PUBLISHED=()
NOT_PUBLISHED=()
ERRORS=()

# check_crates <group> <crate>...
#
# Checks one group of crates and adds each of them to the list matching its
# result. Groups are checked in publish order, the proofman crates first, since
# the ZisK ones depend on them.
check_crates() {
    local group="$1"
    shift
    local crate status

    echo
    echo "${group} crates:"
    echo

    for crate in "$@"; do
        printf "%-35s " "$crate"

        is_published "$crate"
        status=$?

        case "$status" in
            0)
                echo "PUBLISHED"
                PUBLISHED+=("$crate")
                ;;
            1)
                echo "NOT PUBLISHED"
                NOT_PUBLISHED+=("$crate")
                ;;
            *)
                echo "ERROR"
                ERRORS+=("$crate")
                ;;
        esac
    done
}

check_crates "Proofman" "${PROOFMAN_CRATES[@]}"
check_crates "ZisK" "${ZISK_CRATES[@]}"

section "Summary"
echo
echo "Total:          $((${#PROOFMAN_CRATES[@]} + ${#ZISK_CRATES[@]}))"
echo "Published:      ${#PUBLISHED[@]}"
echo "Not published:  ${#NOT_PUBLISHED[@]}"
echo "Errors:         ${#ERRORS[@]}"
echo

if [ "${#PUBLISHED[@]}" -gt 0 ]; then
    echo "-------------------- PUBLISHED -----------------------------"
    for crate in "${PUBLISHED[@]}"; do
        echo "  ✓ $crate"
    done
    echo
fi

if [ "${#NOT_PUBLISHED[@]}" -gt 0 ]; then
    echo "------------------ NOT PUBLISHED ----------------------------"
    for crate in "${NOT_PUBLISHED[@]}"; do
        echo "  ✗ $crate"
    done
    echo
fi

if [ "${#ERRORS[@]}" -gt 0 ]; then
    echo "----------------------- ERRORS ------------------------------"
    for crate in "${ERRORS[@]}"; do
        echo "  ! $crate"
    done
    echo
fi

echo "$SEPARATOR"
