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
        read -p "Press any key to continue..." -n1 -s
        echo
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
    read -p "Press any key to continue..." -n1 -s
    echo
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
    # If ZISK_GHA is set to 1, skip loading .env file
    if [[ -z "$ZISK_GHA" || "$ZISK_GHA" != "1" ]]; then
        # Check if .env file exists
        if [[ ! -f ".env" ]]; then
            info "Skipping loading .env file as it does not exist"
            return 0
        fi

        info "📦 Loading environment variables from .env"

        # Loop through each line in the .env file
        while IFS='=' read -r key value; do
            # Skip comments and empty lines
            if [[ -z "$key" || "$key" =~ ^# ]]; then
                continue
            fi

            # Check if the variable is already defined
            if [[ -z "${!key}" ]]; then
                # If not defined, set the value from the .env file
                export "$key=$value"
            else
                info "Variable '$key' is already defined with value '${!key}', skipping..."
            fi
        done < .env

        echo
        info "🔍 Environment variables:"
        # List variables that were set
        grep -vE '^\s*#' .env | grep -vE '^\s*$' | while IFS='=' read -r key _; do
            echo "  - ${key} = ${!key}"
        done
        echo
    else
        info "Skipping loading .env file since ZISK_GHA is set to 1"
    fi
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
    local zisk_repo_dir=$1
    local var_name=$2
    
    # Check if Cargo.toml exists
    if [ -f "${zisk_repo_dir}/Cargo.toml" ]; then
        # Extract the value of the variable from Cargo.toml
        local value=$(grep -oP "(?<=${var_name} = \")[^\"]+" "${zisk_repo_dir}/Cargo.toml")

        # If the value is found, return it, else return empty string
        if [ -n "$value" ]; then
            echo "$value"
        else
            echo
        fi
    else
        # If the file doesn't exist, return an error message
        err "Cargo.toml not found at ${zisk_repo_dir}/Cargo.toml"
        return 1
    fi
}

# Sets PLATFORM based on the current system
get_platform || return 1
# Sets PROFILE and PREF_SHELL based on the current shell
get_shell_and_profile || return 1
# Ensure profile is loaded
touch $PROFILE
source "$PROFILE"

# Define ZisK directories
ZISK_DIR="$HOME/.zisk"
ZISK_BIN_DIR="$ZISK_DIR/bin"
WORKSPACE_DIR="${HOME}/workspace"
DEFAULT_ZISK_REPO_DIR="${WORKSPACE_DIR}/zisk"
