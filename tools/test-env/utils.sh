#!/bin/bash

# Export PATH to include ZisK binaries
export PATH="$PATH:$HOME/.zisk/bin"

# Colors
if [ -t 1 ]; then
    BOLD=$(tput bold)
    GREEN=$(tput setaf 2)
    RED=$(tput setaf 1)
    YELLOW=$(tput setaf 3)
    RESET=$(tput sgr0)
else
    BOLD=""
    GREEN=""
    RED=""
    YELLOW=""
    RESET=""
fi

# Ensure a command runs successfully echoing the command
ensure() {
    echo -e "${YELLOW}▶ Executing:${RESET} $*"
    if ! "$@"; then
        echo "${RED}❌ Error: command failed -> $*${RESET}" >&2
        press_any_key
        return 1
    fi
}

# Ensure a command runs successfully without echoing the command
ensure_no_echo() {
    if ! "$@"; then
        echo "${RED}❌ Error: command failed -> $*${RESET}" >&2
        press_any_key
        return 1
    fi
}

step() {
    echo "${BOLD}${GREEN}[${current_step}/${total_steps}] $1${RESET}"

    current_step=$(( ${current_step} + 1 ))
}

info() {
    echo "$1"
}

warn() {
    echo "${BOLD}${YELLOW}🚨  $1${RESET}"
}

err() {
    local message="$1"
    local skip_press_any_key="${2:-false}"

    echo "${RED}❌ Error: ${message}${RESET}" >&2
    if [[ "${skip_press_any_key}" != "true" ]]; then
        press_any_key
    fi
    return 1
}

success() {
    echo "${BOLD}${GREEN}✅ $1${RESET}"
}

tolower() {
  echo "$1" | awk '{print tolower($0)}'
}

# load_env: Load environment variables from .env file, without overwriting existing ones
#
# Arguments:
#   $1…$n (optional) — Names of the variables to process. When provided, only
#       those variables are loaded from .env; any other keys are skipped. When
#       omitted, every variable in .env is processed.
load_env() {
    # Optional allow-list of variable names to process
    local -a __wanted_vars=("$@")

    # Check if .env file exists
    if [[ ! -f ".env" ]]; then
        info "Skipping loading .env file as it does not exist"
        return 0
    fi

    info "📦 Loading environment variables from .env"

    # We'll collect printable lines with the source of each variable
    local -a __env_print_lines=()

    # Loop through each line in the .env file
    while IFS='=' read -r key value; do
        # Skip comments and empty lines
        if [[ -z "$key" || "$key" =~ ^# ]]; then
            continue
        fi

        # If an allow-list was provided, skip variables not in it (except control vars)
        if (( ${#__wanted_vars[@]} > 0 )) && [[ "$key" != "DISABLE_ENV_CONFIRM" ]]; then
            local __is_wanted=0
            local __w
            for __w in "${__wanted_vars[@]}"; do
                if [[ "$__w" == "$key" ]]; then
                    __is_wanted=1
                    break
                fi
            done
            if (( __is_wanted == 0 )); then
                continue
            fi
        fi

        # Precedence (highest first): already-set env var, then .env, then Cargo.toml.
        if [[ -n "${!key}" ]]; then
            # Already defined in the shell/CI environment: keep current value.
            __env_print_lines+=(" - [shell] ${key} = ${!key}")
        elif [[ -n "$value" ]] && ! is_gha; then
            # Value from .env (skipped under ZISK_GHA, where the environment and
            # Cargo.toml drive configuration).
            export "$key=$value"
            __env_print_lines+=(" -  [.env] ${key} = ${value}")
        else
            # Fall back to Cargo.toml.
            key_value=$(get_var_from_cargo_toml "$key") || return 1
            if [[ -n "$key_value" ]]; then
                export "$key=$key_value"
                __env_print_lines+=(" - [Cargo] ${key} = ${key_value}")
            fi
        fi
    done < .env

    echo
    info "🔍 Environment variables:"
    for line in "${__env_print_lines[@]}"; do
        echo "$line"
    done
    echo

    # Skip confirming env variables if DISABLE_ENV_CONFIRM is set to 1
    if [[ "$DISABLE_ENV_CONFIRM" == "1" ]]; then
        info "Skipping confirming env variables as DISABLE_ENV_CONFIRM is set to 1"
        return 0
    else
        confirm_continue || return 1
    fi
}

# confirm_continue: Ask the user for confirmation to continue
confirm_continue() {
    # If ZISK_GHA is set, skip confirmation
    if ! is_gha; then
        read -p "Do you want to continue? [Y/n] " answer
        answer=${answer:-y}

        if [[ "$answer" != [Yy]* ]]; then
            echo "Aborted."
            return 1
        fi
    fi
}

# press_any_key: Wait for user to press any key
press_any_key() {
    # If ZISK_GHA is set, skip waiting for user input
    if ! is_gha; then
        read -p "Press any key to continue..." -n1 -s
        echo
    fi
}

# is_proving_key_installed: Check if the proving key is installed
is_proving_key_installed() {
    if [[ -d "$HOME/.zisk/provingKey" ]]; then
        return 0
    else
        err "Proving Key not installed. Please install it first."
        return 1
    fi
}

# is_gha: Check if the script is running in a GitHub Actions environment
is_gha() {
    [[ "${ZISK_GHA:-}" == "1" ]]
}

# get_var_list_to_array: fills a bash array with items from a comma-separated env var
# Usage: get_var_list_to_array <dest_array_name> <ENV_VAR_NAME>
get_var_list_to_array() {
    local __dest="$1"
    local __varname="$2"
    local raw="${!__varname}"

    # If empty or only whitespace, set empty array and return
    if [[ -z "${raw//[[:space:]]/}" ]]; then
        eval "$__dest=()"
        return 0
    fi

    local -a __tmp=()
    local item
    IFS=',' read -ra __parts <<< "$raw"
    for item in "${__parts[@]}"; do
        # trim surrounding whitespace
        item="${item#"${item%%[![:space:]]*}"}"
        item="${item%"${item##*[![:space:]]}"}"
        [[ -n "$item" ]] && __tmp+=("$item")
    done
    # assign by name
    eval "$__dest=(\"\${__tmp[@]}\")"
}

# verify_files_exist: Ensure that all specified files exist under a given base path
#
# Arguments:
#   $1 (base_path) — Directory path where input files are located
#   $2…$n (files) — Filenames (relative to base_path) to check for existence
#
# Example:
#   verify_files_exist "/home/user/inputs" file1.bin file2.bin file3.bin
verify_files_exist() {
    local base_path="$1"
    shift
    local files=("$@")

    for f in "${files[@]}"; do
        if [[ "${f}" != "empty" ]]; then # skip "empty", since this indicates that no input file is needed
            if [[ ! -f "${base_path}/${f}" ]]; then
                err "File not found: ${base_path}/${f}"
                return 1
            fi
        fi
    done
    return 0
}

# get_shell_and_profile: Sets PROFILE and PREF_SHELL based on the current shell
get_shell_and_profile() {
  case "${SHELL}" in
    */zsh)
      PROFILE=${ZDOTDIR:-${HOME}}/.zshenv
      PREF_SHELL="zsh"
      ;;
    */bash)
      PROFILE=${HOME}/.bashrc
      PREF_SHELL="bash"
      ;;
    */fish)
      PROFILE=${HOME}/.config/fish/config.fish
      PREF_SHELL="fish"
      ;;
    */ash)
      PROFILE=${HOME}/.profile
      PREF_SHELL="ash"
      ;;
    *)
      err "shell ${SHELL} is not supported"
      exit 1
      ;;
  esac
}

# get_platform: Sets PLATFORM based on the current system
get_platform() {
    uname_s=$(uname -s)
    PLATFORM=$(tolower "${ZISKUP_PLATFORM:-${uname_s}}")
}

# get_var_from_cargo_toml: Extracts a variable value from Cargo.toml (with "gha_" prefix)
get_var_from_cargo_toml() {
    local var_name=$1
    local file="$(get_zisk_repo_dir)/Cargo.toml"

    # Guard clauses: file must exist and var_name must be non-empty
    [[ -f "$file" && -n "$var_name" ]] || { echo; return; }

    # Normalize the requested key to lowercase (portable on macOS and Linux)
    local var_lc
    var_lc="$(printf '%s' "$var_name" | tr '[:upper:]' '[:lower:]')"

    # Always add prefix "gha_"
    local prefixed_var="gha_${var_lc}"

    # Escape regex special characters for sed
    local escaped_prefixed
    escaped_prefixed=$(printf '%s' "$prefixed_var" | sed 's/[.[\*^$+?{}|()\\]/\\&/g')

    local value=""
    # Try double-quoted value: key = "value"
    value=$(LC_ALL=C sed -nE "s/^[[:space:]]*${escaped_prefixed}[[:space:]]*=[[:space:]]*\"([^\"]*)\".*/\1/p" "$file" | head -n1)

    # If not found, try single-quoted value: key = 'value'
    [[ -z "$value" ]] && value=$(LC_ALL=C sed -nE "s/^[[:space:]]*${escaped_prefixed}[[:space:]]*=[[:space:]]*'([^']*)'.*/\1/p" "$file" | head -n1)

    echo "$value"
}

# get_zisk_repo_dir: returns the ZisK repository directory
get_zisk_repo_dir() {
    if [[ -n "${ZISK_REPO_DIR:-}" ]]; then
        echo "${ZISK_REPO_DIR}"
    else
        echo "${WORKSPACE_DIR:-${PWD}}/zisk"
    fi
}


# patch_cargo_dep: Repoint a git dependency in a Cargo.toml to a local path.
# Comments out the existing `<crate> = { git = ... }` line and inserts (idempotently)
# a `<crate> = { path = "<local_path>" }` entry right after it.
# Relies on the SED_PARAMS global set up by the caller.
# Usage: patch_cargo_dep <cargo_toml> <crate_name> <local_path>
patch_cargo_dep() {
    local cargo_toml="$1"
    local crate="$2"
    local dep_path="$3"
    # Optional: the real crate name at dep_path, when it differs from the dependency KEY
    # used in cargo_toml. Emitted as Cargo's `package = ` rename so the manifest keeps its
    # original key (and import name) while resolving to the renamed local crate.
    local package="${4:-}"

    if [[ ! -f "${cargo_toml}" ]]; then
        err "Cargo.toml not found: ${cargo_toml}"
        return 1
    fi
    if [[ ! -f "${dep_path}/Cargo.toml" ]]; then
        err "Local path for '${crate}' not found: ${dep_path}/Cargo.toml. Make sure the ZisK repo is available."
        return 1
    fi

    # Escape regex-special characters in the crate name for sed/grep patterns.
    local crate_re
    crate_re=$(printf '%s' "${crate}" | sed 's/[.[\*^$+?{}|()\/]/\\&/g')

    local new_line="${crate} = { path = \"${dep_path}\" }"
    if [[ -n "${package}" ]]; then
        new_line="${crate} = { path = \"${dep_path}\", package = \"${package}\" }"
    fi

    # Comment out the git dependency line and add a local path entry right below it, in a
    # single substitution. The `# &` keeps the original line as a comment; the `\<newline>`
    # form (a backslash followed by a real newline) is portable across GNU and BSD/macOS sed.
    # Idempotent: on reruns the git line is already commented, so it no longer matches.
    # Expand SED_PARAMS defensively: `${SED_PARAMS[@]+"${SED_PARAMS[@]}"}` yields
    # nothing (instead of erroring) when it is unset, so the function is safe under
    # `set -u`. The fallback below then fills in GNU/BSD defaults.
    local sed_params=(${SED_PARAMS[@]+"${SED_PARAMS[@]}"})
    if [[ ${#sed_params[@]} -eq 0 ]]; then
        if [[ "$(uname -s)" == "Darwin" ]]; then
            sed_params=(-i "" -E)
        else
            sed_params=(-i -E)
        fi
    fi

    ensure sed "${sed_params[@]}" \
        "s~^${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*git.*~# &\\
${new_line}~" \
        "${cargo_toml}" || return 1

    # Verify the patch was applied correctly.
    if ! grep -qE "^#[[:space:]]*${crate_re}[[:space:]]*=[[:space:]]*[{][[:space:]]*git" "${cargo_toml}"; then
        err "Failed to comment '${crate} = { git = ... }' line in ${cargo_toml}"
        return 1
    fi
    if ! grep -qF "${new_line}" "${cargo_toml}"; then
        err "Failed to add ${crate} path entry pointing to ${dep_path} in ${cargo_toml}"
        return 1
    fi
}

# format_duration_ms: format milliseconds to HH:MM:SS.mmm
format_duration_ms() {
    local ms=$1
    local h=$(( ms / 3600000 ))
    ms=$(( ms % 3600000 ))
    local m=$(( ms / 60000 ))
    ms=$(( ms % 60000 ))
    local s=$(( ms / 1000 ))
    local rem_ms=$(( ms % 1000 ))
    printf "%02d:%02d:%02d.%03d" "$h" "$m" "$s" "$rem_ms"
}

# now_ns: get current time in nanoseconds (fallback to seconds*1e9 if not supported)
now_ns() {
    local n
    n=$(date +%s%N 2>/dev/null)
    if [[ -z "$n" || "$n" =~ [^0-9] ]]; then
        n="$(date +%s)000000000"
    fi
    printf "%s" "$n"
}

# run_timed: execute a .sh script and measure its execution time
# Usage: run_timed "./script.sh"
run_timed() {
    local script="$1"

    if [[ -z "$script" ]]; then
        err "no script provided to run_timed"
        return 1
    fi
    if [[ ! -f "$script" ]]; then
        err "script not found: $script"
        return 1
    fi

    local start_ns end_ns elapsed_ns elapsed_ms exit_code

    # Record start time
    start_ns=$(now_ns)

    # Execute script
    "$script"
    exit_code=$?

    # Record end time
    end_ns=$(now_ns)
    elapsed_ns=$(( end_ns - start_ns ))
    elapsed_ms=$(( elapsed_ns / 1000000 ))

    local pretty
    pretty=$(format_duration_ms "$elapsed_ms")

    # Show execution time and exit code
    if [[ $exit_code -eq 0 ]]; then
        info "🕒 Finished ${script} in ${pretty} (exit code 0)"
    fi

    # Always return success to keep the menu running
    return 0
}

# Sets PLATFORM based on the current system
get_platform || return 1
# Sets PROFILE and PREF_SHELL based on the current shell
get_shell_and_profile || return 1

source "$HOME/.cargo/env"

# Define directories
ZISK_DIR="$HOME/.zisk"
ZISK_BIN_DIR="$ZISK_DIR/bin"
WORKSPACE_DIR="${WORKSPACE_DIR:-${HOME}/workspace}"
OUTPUT_DIR="${HOME}/output"

# Ensure directories exists
ensure_no_echo mkdir -p "${WORKSPACE_DIR}"
ensure_no_echo mkdir -p "$(get_zisk_repo_dir)"
ensure_no_echo mkdir -p "${OUTPUT_DIR}"
