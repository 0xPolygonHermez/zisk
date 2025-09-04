#!/bin/bash

set -euo pipefail

# Configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
readonly DOCKER_IMAGE="riscv-asm-builder"
OUTPUT_DIR="elf-output"
CLEAN_BUILD=0
VERBOSE=0
readonly ASSEMBLER_FLAGS="-march=rv64imac"
readonly LINKER_FLAGS="-Ttext=0x80000000"

# Colors for output (if terminal supports it)
if [[ -t 1 ]]; then
    readonly RED='\033[0;31m'
    readonly GREEN='\033[0;32m'
    readonly YELLOW='\033[1;33m'
    readonly NC='\033[0m'
else
    readonly RED=''
    readonly GREEN=''
    readonly YELLOW=''
    readonly NC=''
fi

# Logging functions
log_info() {
    echo -e "${GREEN}$*${NC}"
}

log_warn() {
    echo -e "${YELLOW}$*${NC}"
}

log_error() {
    echo -e "${RED}$*${NC}" >&2
}

log_success() {
    echo -e "  ${GREEN}✓${NC} $*"
}

log_failure() {
    echo -e "  ${RED}✗${NC} $*" >&2
}

# Show usage information
usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Build all RISC-V ELF files from assembly sources.

OPTIONS:
    -h, --help      Show this help message
    -o, --output    Output directory (default: $OUTPUT_DIR)
    -c, --clean     Clean output directory before building
    -v, --verbose   Enable verbose output

DESCRIPTION:
    This script compiles all .s assembly files found in subdirectories
    into RISC-V ELF executables using a Docker container for convenience.

EXAMPLES:
    $0                    # Build all assembly files
    $0 --clean            # Clean and rebuild
    $0 -o custom_output   # Use custom output directory

EOF
}

# Parse command line arguments
parse_args() {
    local output_dir="$OUTPUT_DIR"
    local clean_build=0
    local verbose=0

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                usage
                exit 0
                ;;
            -o|--output)
                output_dir="$2"
                shift 2
                ;;
            -c|--clean)
                clean_build=1
                shift
                ;;
            -v|--verbose)
                verbose=1
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    # Export for use in functions
    OUTPUT_DIR="$output_dir"
    CLEAN_BUILD="$clean_build"
    VERBOSE="$verbose"
}

# Build Docker image
build_docker_image() {
    log_info "Building Docker image for RISC-V assembly compilation..."
    
    if ! docker build -f "$PROJECT_DIR/Dockerfile.build" -t "$DOCKER_IMAGE" "$PROJECT_DIR"; then
        log_error "Failed to build Docker image"
        exit 1
    fi
}

# Prepare output directory
prepare_output_directory() {
    if [[ "$CLEAN_BUILD" -eq 1 ]] && [[ -d "$OUTPUT_DIR" ]]; then
        log_warn "Cleaning existing output directory..."
        rm -rf "$OUTPUT_DIR"
    fi
    
    mkdir -p "$OUTPUT_DIR"
}

# Compile a single assembly file
# This is ran in a loop to compile all assembly files
compile_assembly_file() {
    local asm_file="$1"
    local dir_name="$2"
    local base_name
    local output_file
    local relative_asm_file
    local linker_flags
    local linker_script
    
    base_name="$(basename "$asm_file" .s)"
    output_file="${OUTPUT_DIR}/${dir_name}_${base_name}.elf"
    # Get relative path for use inside container
    relative_asm_file="${dir_name}/${base_name}.s"
    
    echo "  Compiling: $asm_file"
    
    # Check for custom linker script in the directory
    # If not just use the default 0x8000000000 from LINKER_FLAGS
    linker_script="$(find "${dir_name}" -maxdepth 1 -name "*.ld" -type f 2>/dev/null | head -1)"
    if [[ -n "$linker_script" && -f "$linker_script" ]]; then
        local linker_script_name="$(basename "$linker_script")"
        linker_flags="-T ${dir_name}/${linker_script_name}"
        log_warn "  Using custom linker script: $linker_script_name"
    else
        linker_flags="$LINKER_FLAGS"
    fi
    
    # Create compile command
    local compile_cmd="cd /workspace && \
        riscv64-unknown-elf-as $ASSEMBLER_FLAGS '$relative_asm_file' -o '/tmp/$base_name.o' && \
        riscv64-unknown-elf-ld $linker_flags '/tmp/$base_name.o' -o '$output_file'"
    
    if [[ "$VERBOSE" -eq 1 ]]; then
        # TODO: possibly make this log_info 
        log_warn "  Command: $compile_cmd"
    fi
    
    # Run compilation in Docker
    if docker run --rm -v "$(pwd):/workspace" "$DOCKER_IMAGE" bash -c "$compile_cmd" 2>/dev/null; then
        if [[ -f "$output_file" ]]; then
            log_success "Created: $output_file"
            return 0
        fi
    fi
    
    log_failure "Failed to compile $asm_file"
    return 1
}

# Copy prebuilt ELF files to output directory
copy_prebuilt_elfs() {
    local prebuilt_dir="${PROJECT_DIR}/prebuilt-elfs"
    local copied_count=0
    
    if [[ -d "$prebuilt_dir" ]]; then
        log_info "Copying prebuilt ELF files..."
        
        # Find all .elf files in prebuilt directory using a for loop
        for elf_file in "$prebuilt_dir"/*.elf; do
            # Check if the glob actually matched files
            if [[ ! -f "$elf_file" ]]; then
                continue
            fi
            
            local elf_basename
            elf_basename="$(basename "$elf_file")"
            local dest_file="${OUTPUT_DIR}/${elf_basename}"
            
            if cp "$elf_file" "$dest_file" 2>/dev/null; then
                log_success "Copied: $elf_basename"
                copied_count=$((copied_count + 1))
            else
                log_failure "Failed to copy: $elf_basename"
            fi
        done
        
        if [[ $copied_count -gt 0 ]]; then
            log_info "Copied $copied_count prebuilt ELF files"
        fi
    fi
}

# Process a directory containing assembly files
process_directory() {
    local dir="$1"
    local dir_name
    local success_count=0
    local fail_count=0
    
    dir_name="$(basename "$dir")"
    
    # Find all .s files in the directory
    local asm_files
    mapfile -t asm_files < <(find "$dir" -maxdepth 1 -name "*.s" -type f 2>/dev/null)
    
    if [[ ${#asm_files[@]} -eq 0 ]]; then
        return 0
    fi
    
    log_warn "Processing directory: $dir_name (${#asm_files[@]} files)"
    
    for asm_file in "${asm_files[@]}"; do
        if compile_assembly_file "$asm_file" "$dir_name"; then
            ((success_count++))
        else
            ((fail_count++))
        fi
    done
    
    if [[ $fail_count -gt 0 ]]; then
        log_warn "  Directory $dir_name: $success_count succeeded, $fail_count failed"
        return 1
    fi
    
    return 0
}

# Find and process all directories with assembly files
process_all_directories() {
    local total_dirs=0
    local failed_dirs=0
    
    log_info "Compiling assembly files to ELF..."
    
    # Find all directories (excluding output and hidden directories)
    while IFS= read -r -d '' dir; do
        # Skip output directory and hidden directories
        if [[ "$(basename "$dir")" == "$OUTPUT_DIR" ]] || [[ "$(basename "$dir")" == .* ]]; then
            continue
        fi
        
        if process_directory "$dir"; then
            ((total_dirs++))
        else
            ((total_dirs++))
            ((failed_dirs++))
        fi
    done < <(find "$PROJECT_DIR" -mindepth 1 -maxdepth 1 -type d -print0)
    
    if [[ $total_dirs -eq 0 ]]; then
        log_warn "No directories with assembly files found"
        return 1
    fi
    
    if [[ $failed_dirs -gt 0 ]]; then
        log_warn "Processed $total_dirs directories, $failed_dirs had failures"
        return 1
    fi
    
    log_info "Successfully processed $total_dirs directories"
    return 0
}

# Display build results
show_results() {
    log_info "Build complete!"
    echo "ELF files are in: $OUTPUT_DIR/"
    
    if [[ -d "$OUTPUT_DIR" ]]; then
        local file_count
        file_count=$(find "$OUTPUT_DIR" -name "*.elf" -type f | wc -l)
        echo "Total ELF files generated: $file_count"
        
        if [[ "$VERBOSE" -eq 1 ]]; then
            ls -la "$OUTPUT_DIR/"
        fi
    fi
}

main() {
    cd "$PROJECT_DIR"
    
    parse_args "$@"
    
    trap 'log_error "Build interrupted"' INT TERM
    
    build_docker_image
    prepare_output_directory
    
    # Copy prebuilt ELF files first
    copy_prebuilt_elfs
    
    if process_all_directories; then
        show_results
        exit 0
    else
        show_results
        exit 1
    fi
}

# Run main function
main "$@"