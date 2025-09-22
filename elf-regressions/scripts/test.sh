#!/bin/bash

# TODO: Possibly simplify this. It likely doesn't need to have great display

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
readonly ELF_OUTPUT_DIR="${PROJECT_DIR}/elf-output"
readonly ZISK_DIR="${PROJECT_DIR}/.."
# Binary paths - can be overridden via environment variables
# Default to debug build locations for local development
readonly ZISKEMU="${ZISKEMU_PATH:-${ZISK_DIR}/target/debug/ziskemu}"
readonly CARGO_ZISK="${CARGO_ZISK_PATH:-${ZISK_DIR}/target/debug/cargo-zisk}"

# Colors for output
if [[ -t 1 ]]; then
    readonly RED='\033[0;31m'
    readonly GREEN='\033[0;32m'
    readonly YELLOW='\033[1;33m'
    readonly BLUE='\033[0;34m'
    readonly NC='\033[0m'
else
    readonly RED=''
    readonly GREEN=''
    readonly YELLOW=''
    readonly BLUE=''
    readonly NC=''
fi

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_cmd() {
    echo -e "${BLUE}[CMD]${NC} $*"
}

# Show usage
usage() {
    cat <<EOF
Usage: $0 [OPTIONS] <test-name>

Run tests on compiled ELF files using either rom-setup or ziskemu.

ARGUMENTS:
    test-name           Name of the test directory (e.g., jalr, simple_add, remainder)
                       Use 'all' to run on all available tests

OPTIONS:
    -r, --rom-setup     Run rom-setup on the ELF (default)
    -e, --emulator      Run ziskemu on the ELF
    -v, --verbose       Enable verbose output
    -s, --steps NUM     Maximum steps for emulator (no default, uses ziskemu's default)
    -o, --output DIR    Output directory for rom-setup (default: /tmp)
    -l, --list          List all available tests
    -h, --help          Show this help message

EXAMPLES:
    $0 simple_add                    # Run rom-setup on simple_add
    $0 -e simple_add                 # Run ziskemu on simple_add
    $0 -e -v -s 100 jalr            # Run ziskemu with verbose and 100 steps
    $0 -r -v remainder              # Run rom-setup with verbose output
    $0 -l                           # List all available tests
    $0 all                          # Run rom-setup on all tests
    $0 -r -v all                    # Run rom-setup on all tests with verbose

EOF
}

# List available tests
list_tests() {
    log_info "Available tests:"
    echo
    for elf in "${ELF_OUTPUT_DIR}"/*.elf; do
        if [[ -f "$elf" ]]; then
            basename "$elf" .elf | sed 's/^/  - /'
        fi
    done | sort -u
}

# Find ELF file for test
find_elf() {
    local test_name="$1"
    
    # Check for exact match with .elf extension
    local exact_elf="${ELF_OUTPUT_DIR}/${test_name}.elf"
    if [[ -f "$exact_elf" ]]; then
        echo "$exact_elf"
        return 0
    fi
    
    log_error "No ELF file found for test: ${test_name}"
    log_warn "Looking for: ${exact_elf}"
    log_warn "Run ./build.sh first to compile the tests"
    return 1
}

# Run rom-setup
run_rom_setup() {
    local elf="$1"
    local output_dir="$2"
    local verbose="$3"
    
    log_info "Running rom-setup on: $(basename "$elf")"
    log_info "Output directory: ${output_dir}"
    
    local cmd="${CARGO_ZISK} rom-setup --elf ${elf} --output-dir ${output_dir}"
    if [[ "$verbose" == "true" ]]; then
        cmd="${cmd} -v"
    fi
    
    log_cmd "$cmd"
    echo
    
    if [[ "$verbose" == "true" ]]; then
        # Show full output when verbose
        eval "$cmd" 2>&1
    else
        # Capture output and only show on error
        if output=$(eval "$cmd" 2>&1); then
            echo "$output" | grep -E "INFO:|Root hash:|successfully completed" || true
            log_info "✓ ROM setup completed successfully"
        else
            log_error "ROM setup failed:"
            echo "$output"
            return 1
        fi
    fi
}

# Run ziskemu
run_emulator() {
    local elf="$1"
    local steps="$2"
    local verbose="$3"
    
    log_info "Running ziskemu on: $(basename "$elf")"
    
    local cmd="${ZISKEMU} -e ${elf}"
    
    # Only add steps if provided
    if [[ -n "$steps" ]]; then
        log_info "Max steps: ${steps}"
        cmd="${cmd} -n ${steps}"
    fi
    
    if [[ "$verbose" == "true" ]]; then
        cmd="${cmd} -v"
    fi
    
    log_cmd "$cmd"
    echo
    
    eval "$cmd"
    
    log_info "✓ Emulator execution completed"
}

# Main function
main() {
    # Default values
    local mode="rom-setup"
    local verbose="false"
    local steps=""
    local output_dir="/tmp"
    local test_name=""
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -r|--rom-setup)
                mode="rom-setup"
                shift
                ;;
            -e|--emulator)
                mode="emulator"
                shift
                ;;
            -v|--verbose)
                verbose="true"
                shift
                ;;
            -s|--steps)
                steps="$2"
                shift 2
                ;;
            -o|--output)
                output_dir="$2"
                shift 2
                ;;
            -l|--list)
                list_tests
                exit 0
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                test_name="$1"
                shift
                ;;
        esac
    done
    
    # Check if test name provided
    if [[ -z "$test_name" ]]; then
        log_error "No test name provided"
        echo
        usage
        exit 1
    fi
    
    # Handle 'all' - run on all tests
    if [[ "$test_name" == "all" ]]; then
        log_info "Running ${mode} on all tests..."
        echo
        
        local failed_tests=()
        local passed_tests=()
        
        for elf in "${ELF_OUTPUT_DIR}"/*.elf; do
            if [[ -f "$elf" ]]; then
                local test_name=$(basename "$elf" .elf)
                
                echo -e "${BLUE}════════════════════════════════════════${NC}"
                echo -e "${BLUE}Testing: ${test_name}${NC}"
                echo -e "${BLUE}════════════════════════════════════════${NC}"
                
                # Run the test
                if [[ "$mode" == "rom-setup" ]]; then
                    if run_rom_setup "$elf" "$output_dir" "$verbose"; then
                        passed_tests+=("$test_name")
                    else
                        failed_tests+=("$test_name")
                    fi
                else
                    if run_emulator "$elf" "$steps" "$verbose"; then
                        passed_tests+=("$test_name")
                    else
                        failed_tests+=("$test_name")
                    fi
                fi
                
                echo
            fi
        done
        
        # Summary
        echo -e "${BLUE}════════════════════════════════════════${NC}"
        echo -e "${BLUE}Summary${NC}"
        echo -e "${BLUE}════════════════════════════════════════${NC}"
        
        if [[ ${#passed_tests[@]} -gt 0 ]]; then
            echo -e "${GREEN}Passed (${#passed_tests[@]}):${NC}"
            for test in "${passed_tests[@]}"; do
                echo -e "  ${GREEN}✓${NC} $test"
            done
        fi
        
        if [[ ${#failed_tests[@]} -gt 0 ]]; then
            echo -e "${RED}Failed (${#failed_tests[@]}):${NC}"
            for test in "${failed_tests[@]}"; do
                echo -e "  ${RED}✗${NC} $test"
            done
            exit 1
        else
            echo -e "${GREEN}All tests passed!${NC}"
        fi
        
        exit 0
    fi
    
    # Check prerequisites
    if [[ "$mode" == "rom-setup" ]] && [[ ! -f "$CARGO_ZISK" ]]; then
        log_error "cargo-zisk not found at: ${CARGO_ZISK}"
        log_warn "Run: cargo build --bin cargo-zisk"
        exit 1
    fi
    
    if [[ "$mode" == "emulator" ]] && [[ ! -f "$ZISKEMU" ]]; then
        log_error "ziskemu not found at: ${ZISKEMU}"
        log_warn "Run: cargo build --bin ziskemu"
        exit 1
    fi
    
    # Find ELF file
    elf=$(find_elf "$test_name")
    if [[ $? -ne 0 ]]; then
        exit 1
    fi
    
    # Run the appropriate mode
    case "$mode" in
        rom-setup)
            run_rom_setup "$elf" "$output_dir" "$verbose"
            ;;
        emulator)
            run_emulator "$elf" "$steps" "$verbose"
            ;;
    esac
}

# Run main function
main "$@"