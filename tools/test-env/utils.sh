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

# Helper to ensure a command runs successfully
# If it fails, it prints an error message and waits for user input
ensure() {
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
    echo "${RED}❌ Error: $1${RESET}" >&2
    press_any_key
    return 1
}

success() {
    echo "${BOLD}${GREEN}✅ $1${RESET}"
}

tolower() {
  echo "$1" | awk '{print tolower($0)}'
}

# load_env: Load environment variables from .env file, without overwriting existing ones
load_env() {
    local zisk_repo_dir=$1

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

        # Try to get the value from Cargo.toml (takes precedence)
        key_value=$(get_var_from_cargo_toml "$key") || return 1

        if [[ -n "$key_value" ]]; then
            # If defined in Cargo.toml, export it (overrides anything else)
            export "$key=$key_value"
            __env_print_lines+=(" - [Cargo] ${key} = ${key_value}")
        elif [[ -z "${!key}" ]]; then
            # If not already defined, set the value from the .env file if ZISK_GHA is not set
            if [[ "$ZISK_GHA" != "1" ]]; then
                export "$key=$value"
                __env_print_lines+=(" -  [.env] ${key} = ${value}")
            fi
        else
            # Already defined in the shell: keep current value
            __env_print_lines+=(" - [shell] ${key} = ${!key}")
        fi
    done < .env

    echo
    info "🔍 Environment variables:"
    for line in "${__env_print_lines[@]}"; do
        echo "$line"
    done
    echo
}

# confirm_continue: Ask the user for confirmation to continue
confirm_continue() {
    # If ZISK_GHA is set to 1, skip confirmation
    if [[ -z "$ZISK_GHA" || "$ZISK_GHA" != "1" ]]; then
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
    # If ZISK_GHA is set to 1, skip waiting for user input
    if [[ -z "$ZISK_GHA" || "$ZISK_GHA" != "1" ]]; then
        read -p "Press any key to continue..." -n1 -s
        echo
    fi
}

# is_proving_key_installed: Check if the proving key is installed
is_proving_key_installed() {
    if [[ -d "$HOME/.zisk/provingKey" ]]; then
        return 0
    else
        err "Proving key not installed. Please install it first."
        return 1    
    fi
}

# get_var_list: Returns the list of items (separated by commas) in the variable
#
# Parameters:
#   $1 (var_name) — Name of the environment variable containing comma-separated values
get_var_list() {
    local var_name="$1"
    local raw="${!var_name}"
    local item

    # if not defined or empty, return nothing
    [[ -z "${raw//[[:space:]]/}" ]] && return 0

    # separate by comma, trim spaces and emit each line
    IFS=',' read -ra parts <<< "$raw"
    for item in "${parts[@]}"; do
        # remove surrounding whitespace
        item="${item#"${item%%[![:space:]]*}"}"
        item="${item%"${item##*[![:space:]]}"}"
        printf '%s\n' "$item"
    done
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
        if [[ ! -f "${base_path}/${f}" ]]; then
            err "File not found: ${base_path}/${f}"
            return 1
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

# get_var_from_cargo_toml: Extracts a variable value from Cargo.toml
get_var_from_cargo_toml() {
    local var_name=$1
    local file="$(get_zisk_repo_dir)/Cargo.toml"

    # Guard clauses: file must exist and var_name must be non-empty
    [[ -f "$file" && -n "$var_name" ]] || { echo; return; }

    # Normalize the requested key to lowercase (portable on macOS and Linux)
    local var_lc
    var_lc="$(printf '%s' "$var_name" | tr '[:upper:]' '[:lower:]')"

    # Special case: pil2_proofman_branch
    # Assumption: the "proofman = { ... }" entry is a single line and contains "pil2-proofman" in the URL
    if [[ "$var_lc" == "pil2_proofman_branch" ]]; then
        # Find the single line starting with "proofman =" that references pil2-proofman
        local proof_line
        proof_line="$(LC_ALL=C grep -E '^[[:space:]]*proofman[[:space:]]*=' "$file")"

        echo "proof_line = $proof_line"

        if [[ -n "$proof_line" ]]; then
            local branch
            # Try to extract branch in three formats: "value", 'value', or unquoted value
            branch=$(printf '%s' "$proof_line" | LC_ALL=C sed -nE 's/.*branch[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p')
            [[ -z "$branch" ]] && branch=$(printf '%s' "$proof_line" | LC_ALL=C sed -nE "s/.*branch[[:space:]]*=[[:space:]]*'([^']*)'.*/\1/p")
            [[ -z "$branch" ]] && branch=$(printf '%s' "$proof_line" | LC_ALL=C sed -nE 's/.*branch[[:space:]]*=[[:space:]]*([^,}[:space:]]+).*/\1/p')

            # If a branch was found, print it and return
            if [[ -n "$branch" ]]; then
                echo "$branch"
                return
            fi
            # If no branch found, fall back to the standard variable lookup below
        fi
        # If no proofman line found, fall back to the standard variable lookup below
    fi

    # --- Standard behavior: look up a variable by name (lowercased), quoted with "..." or '...' ---
    # Escape regex special characters in the key for sed
    local escaped_var
    escaped_var=$(printf '%s' "$var_lc" | sed 's/[.[\*^$+?{}|()\\]/\\&/g')

    # First, try double-quoted value: key = "value"
    local value
    value=$(LC_ALL=C sed -nE "s/^[[:space:]]*${escaped_var}[[:space:]]*=[[:space:]]*\"([^\"]*)\".*/\1/p" "$file" | head -n1)

    # If not found, try single-quoted value: key = 'value'
    if [[ -z "$value" ]]; then
        value=$(LC_ALL=C sed -nE "s/^[[:space:]]*${escaped_var}[[:space:]]*=[[:space:]]*'([^']*)'.*/\1/p" "$file" | head -n1)
    fi

    # Print the value or an empty string if not found
    echo "$value"
}

# get_zisk_repo_dir: returns the ZisK repository directory
get_zisk_repo_dir() {
    if [[ -n "${ZISK_REPO_DIR}" ]]; then
        echo "${ZISK_REPO_DIR}"
    else
        echo "${WORKSPACE_DIR}/zisk"
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
    else
        err "${script} exited with code ${exit_code} after ${pretty}"
    fi

    # Always return success to keep the menu running
    return 0
}

# Sets PLATFORM based on the current system
get_platform || return 1
# Sets PROFILE and PREF_SHELL based on the current shell
get_shell_and_profile || return 1
# Ensure profile is loaded
touch $PROFILE
source "$PROFILE"

# Define directories
ZISK_DIR="$HOME/.zisk"
ZISK_BIN_DIR="$ZISK_DIR/bin"
WORKSPACE_DIR="${HOME}/workspace"
OUTPUT_DIR="${HOME}/output"

# Ensure directories exists
mkdir -p "${WORKSPACE_DIR}"
mkdir -p "$(get_zisk_repo_dir)"