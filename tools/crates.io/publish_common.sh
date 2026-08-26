#!/bin/bash
#
# Shared data and helpers for the crates.io scripts in this directory. Sourced,
# not executed.
#
# Defines:
#   PROOFMAN_CRATES     every published pil2-proofman crate, in dependency
#                       order: a crate never comes before one it depends on,
#                       which is the order publish_crates.sh uploads them in
#   ZISK_CRATES         the same for the ZisK crates, published after the
#                       proofman ones because they depend on them
#   declare_option      declares one mandatory command line option
#   declare_optional_option
#                       declares one optional command line option
#   parse_args          parses the declared options
#   resolve_repo_path   checks a repo path option and makes it absolute
#   workspace_version   reads the [workspace.package] version of a repo
#   section             prints a ==== delimited header
#   crates_io_get       queries the crates.io API

PROOFMAN_CRATES=(
  "pil2-pilout"
  "proofman-macros"
  "proofman-starks-src"
  "proofman-exps-codegen"
  "proofman-util"
  "proofman-starks-lib-c"
  "proofman-fields"
  "proofman-common"
  "proofman-curves"
  "proofman-verifier"
  "pil2-stark-recurser"
  "proofman-hints"
  "proofman-witness"
  "pil2-stark-setup"
  "pil2-std-lib"
  "proofman"
)

ZISK_CRATES=(
  "zisk-definitions"
  "zisk-lib-c"
  "zisk-lib-float"
  "zisk-pil"
  "zisk-program-macros"
  "zisk-stream"
  "zisk-verifier"
  "zisk-zkvm-interface"
  "zisk-circuit"
  "zisk-precomp-helpers"
  "zisk-recurser"
  "ziskos"
  "ziskos-hints"
  "zisk-core"
  "zisk-common"
  "zisk-riscv"
  "zisk-sm-frequent-ops"
  "zisk-cluster-common"
  "zisk-coordinator-api"
  "zisk-sm-binary"
  "zisk-sm-mem-common"
  "zisk-transpiler-common"
  "zisk-coordinator-client"
  "zisk-precomp-hints"
  "zisk-sm-arith"
  "zisk-sm-mem"
  "zisk-sm-mem-planner"
  "zisk-transpiler-riscv"
  "zisk-asm-runner"
  "zisk-frops-analyzer"
  "zisk-precomp-common"
  "ziskemu"
  "zisk-precomp-arith-eq"
  "zisk-precomp-big-int"
  "zisk-precomp-blake2"
  "zisk-precomp-dma"
  "zisk-precomp-evm"
  "zisk-precomp-keccakf"
  "zisk-precomp-poseidon"
  "zisk-precomp-sha256f"
  "zisk-sm-main"
  "zisk-sm-rom"
  "zisk-precomp-arith-eq-384"
  "zisk-rom-setup"
  "zisk-build"
  "zisk-executor"
  "zisk-prover-backend"
  "zisk-sdk"
  "cargo-zisk"
)

SEPARATOR="============================================================"

# ------------------------------------------------------------
# Output
# ------------------------------------------------------------

# section <line>...
#
# Prints the given lines inside a ==== block, preceded by a blank line. The
# caller adds the trailing blank line when it wants one.
section() {
    echo
    echo "$SEPARATOR"
    printf '%s\n' "$@"
    echo "$SEPARATOR"
}

# ------------------------------------------------------------
# Arguments
#
# Every option a script takes is declared the same way, with declare_option when
# it is mandatory or declare_optional_option when it is not. Each script only
# declares the ones it needs: the scripts share --version (with a different help
# text, since its meaning differs), and only publish_crates.sh takes --version,
# while the repo paths are optional everywhere because either repo can be worked
# on its own.
#
# The declarations are the single source for parsing and for the usage message,
# so an option cannot be accepted without being documented, or the other way
# round.
# ------------------------------------------------------------

OPTION_FLAGS=()      # --version, --zisk-repo-path, ...
OPTION_VARS=()       # VERSION, ZISK_REPO_PATH, ...: variable each value lands in
OPTION_HELPS=()
OPTION_REQUIRED=()   # 1 = mandatory, 0 = optional

HELP_COLUMN=22       # width of the "--flag" column in the usage message

# declare_option <--flag> <help-text>
#
# Declares one mandatory option. Its value ends up in the variable named after
# the flag (--zisk-repo-path -> ZISK_REPO_PATH), which usage() also shows as the
# placeholder. The help text may span several lines; usage() indents the
# continuation lines.
declare_option() {
    declare_option_with_kind 1 "$1" "$2"
}

# declare_optional_option <--flag> <help-text>
#
# Same as declare_option, but the option may be left out. Its variable is then
# left empty and what that means is up to the script; usage() shows the option
# in brackets.
declare_optional_option() {
    declare_option_with_kind 0 "$1" "$2"
}

# declare_option_with_kind <1|0> <--flag> <help-text>
declare_option_with_kind() {
    local required="$1"
    local flag="$2"
    local var

    var=$(printf '%s' "${flag#--}" | tr 'a-z-' 'A-Z_')

    OPTION_FLAGS+=("$flag")
    OPTION_VARS+=("$var")
    OPTION_HELPS+=("$3")
    OPTION_REQUIRED+=("$required")

    printf -v "$var" '%s' ""
}

usage() {
    local i line first synopsis=""

    for ((i=0; i<${#OPTION_FLAGS[@]}; i++)); do
        if [ "${OPTION_REQUIRED[$i]}" -eq 1 ]; then
            synopsis+=" ${OPTION_FLAGS[$i]} <${OPTION_VARS[$i]}>"
        else
            synopsis+=" [${OPTION_FLAGS[$i]} <${OPTION_VARS[$i]}>]"
        fi
    done

    {
        echo "usage: $(basename "$0")$synopsis"
        echo

        for ((i=0; i<${#OPTION_FLAGS[@]}; i++)); do
            first=1
            while IFS= read -r line; do
                if [ "$first" -eq 1 ]; then
                    printf '  %-*s' "$HELP_COLUMN" "${OPTION_FLAGS[$i]}"
                    first=0
                else
                    printf '  %-*s' "$HELP_COLUMN" ""
                fi
                printf '%s\n' "$line"
            done <<< "${OPTION_HELPS[$i]}"
        done
    } >&2

    exit 1
}

# parse_args "$@"
#
# Fills the variable of every declared option, or exits with the usage message
# if a mandatory argument is missing, empty or unknown. Only checks that the values are
# there: what they have to mean is up to each script.
parse_args() {
    local i flag var matched

    while [ $# -gt 0 ]; do
        if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
            usage
        fi

        matched=0

        for ((i=0; i<${#OPTION_FLAGS[@]}; i++)); do
            flag="${OPTION_FLAGS[$i]}"
            var="${OPTION_VARS[$i]}"

            case "$1" in
                "$flag")
                    if [ $# -lt 2 ]; then
                        echo "ERROR: $flag requires a value."
                        usage
                    fi
                    printf -v "$var" '%s' "$2"
                    shift 2
                    matched=1
                    ;;
                "$flag"=*)
                    printf -v "$var" '%s' "${1#*=}"
                    shift
                    matched=1
                    ;;
            esac

            if [ "$matched" -eq 1 ]; then
                break
            fi
        done

        if [ "$matched" -eq 0 ]; then
            echo "ERROR: unknown argument: $1"
            usage
        fi
    done

    for ((i=0; i<${#OPTION_FLAGS[@]}; i++)); do
        var="${OPTION_VARS[$i]}"

        if [ "${OPTION_REQUIRED[$i]}" -eq 1 ] && [ -z "${!var}" ]; then
            echo "ERROR: ${OPTION_FLAGS[$i]} is mandatory."
            usage
        fi
    done
}

# ------------------------------------------------------------
# Repos
# ------------------------------------------------------------

# resolve_repo_path <--flag> <VAR>
#
# Checks that the repo path held by <VAR> is a workspace root and rewrites it as
# an absolute one, or does nothing when the flag was not given: every script
# here takes the repos as optional options.
resolve_repo_path() {
    local flag="$1"
    local var="$2"
    local path="${!var}"

    if [ -z "$path" ]; then
        return 0
    fi

    if [ ! -d "$path" ]; then
        echo "ERROR: $flag $path is not a directory."
        exit 1
    fi

    path=$(cd "$path" && pwd)

    if [ ! -f "$path/Cargo.toml" ]; then
        echo "ERROR: $path does not look like a repo root (no Cargo.toml)."
        exit 1
    fi

    printf -v "$var" '%s' "$path"
}

# workspace_version <repo>
#
# Prints the [workspace.package] version of the repo, or nothing when the root
# Cargo.toml holds no version at all.
workspace_version() {
    sed -nE 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' \
        "$1/Cargo.toml" | head -n1
}

# ------------------------------------------------------------
# crates.io queries
#   0 = yes, 1 = no, 2 = query failed
# ------------------------------------------------------------

# crates_io_get <api-path> <what>
#
# <api-path> is appended to https://crates.io/api/v1/crates/, so "<crate>" asks
# whether the crate name exists and "<crate>/<version>" whether that exact
# version does. <what> only names the query in the error message.
crates_io_get() {
    local path="$1"
    local what="$2"
    local http_code

    http_code=$(curl -sS \
        -o /dev/null \
        -w "%{http_code}" \
        -H "User-Agent: zisk-release-script/1.0" \
        -H "Accept: application/json" \
        "https://crates.io/api/v1/crates/${path}")

    case "$http_code" in
        200)
            return 0
            ;;
        404)
            return 1
            ;;
        *)
            echo
            echo "ERROR: Could not check $what on crates.io."
            echo "HTTP status: $http_code"
            return 2
            ;;
    esac
}

# Does this exact crate version exist? Uses VERSION.
is_published() {
    crates_io_get "$1/$VERSION" "$1 $VERSION"
}

# Does the crate name exist at all (i.e. would publishing be an update)?
crate_exists() {
    crates_io_get "$1" "$1"
}
