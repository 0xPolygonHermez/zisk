# git-utils.sh

# Log a message with a specific level (color)
log_level() {
    local level=$1
    local message=$2
    local color=""

    case $level in
        info) color="\e[1;32m" ;;  # Green
        warn) color="\e[1;33m" ;;  # Yellow
        err)  color="\e[1;31m" ;;  # Red
        *) color="\e[1;m" ;;       # Default
    esac

    echo -e "${color}$message\e[0m"
}

# Log a message with info level
log_info() {
    log_level "info" "$1"
}

# Log a message with warn level
log_warn() {
    log_level "warn" "$1"
}

# Log a message with error level
log_err() {
    log_level "err" "$1"
}

# Log a message with no level
log() {
    log_level "" "$1"
}

# Execute a git command and print the output. Exits if the command fails.
exec_git() {
    local command="$1"
    local error_message="$2"

    output=$(eval "$command" 2>&1)

    # Exit if the command fails
    if [[ $? -ne 0 ]]; then
        # Print the error output
        echo "$output"
        # Print the error message
        log_err ${error_message}
        
        exit 1
    fi

    # Print the output if it's not empty
    if [[ -n "$output" ]]; then
        echo "$output"
    fi    
}
