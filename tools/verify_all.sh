#!/bin/bash

# Check that at least one argument has been passed
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <dirname|elf_file> [-l|--list] [-b|--begin <first_file>] [-e|--end <last_file>] [-d|--debug] [-i|--inputs <input_dir|input_file>] [-k|--proving-key <pk_dir>]"
    exit 1
fi

# Initialize variables
list=0
begin=0
end=0
debug=0
input_path=""
proving_key=""
elf_mode=0
default_input="/tmp/empty_input.bin" # Default input file

# Check if the first argument is a file or directory
if [[ -f "$1" ]]; then
    elf_file="$1"
    elf_mode=1
elif [[ -d "$1" ]]; then
    dir="$1"
else
    echo "Invalid input. The first argument must be a directory or an ELF file."
    exit 1
fi
shift

# Parse optional arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -l|--list) list=1 ;;
        -b|--begin) begin=$2; shift ;;
        -e|--end) end=$2; shift ;;
        -d|--debug) debug=1 ;;
        -i|--inputs) input_path="$2"; shift ;;
        -k|--proving-key) proving_key="$2"; shift ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

# Ensure the default input file exists
if [[ ! -f $default_input ]]; then
    touch "$default_input"
fi

# Function to verify an ELF file with one or more input files
verify_elf_with_inputs() {
    local elf_file=$1
    local input_path=$2
    local input_counter=0
    local input_total=0

    echo "Verifying ELF file: $elf_file"

    if [[ -z $input_path ]]; then
        # No input path provided, use the default input file
        echo "Using default input file: $default_input"

        if [[ $debug -eq 1 ]]; then
            # Run with debug flag
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$default_input" \
            --proving-key "$proving_key" --emulator --debug)
        else
            # Run without debug flag
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$default_input" \
            --proving-key "$proving_key" --emulator)
        fi
    elif [[ -f $input_path ]]; then
        # Single input file provided
        echo "Using input file: $input_path"

        if [[ $debug -eq 1 ]]; then
            # Run with debug flag
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$input_path" \
            --proving-key "$proving_key" --emulator --debug)
        else
            # Run without debug flag
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$input_path" \
            --proving-key "$proving_key" --emulator)
        fi
    elif [[ -d $input_path ]]; then
        # Directory of input files provided
        input_files=$(find "$input_path" -type f | sort)
        input_total=$(echo "$input_files" | wc -l)
        if [[ -z "$input_files" ]]; then
            echo "No input files found in directory: $input_path"
            return
        fi

        for input_file in $input_files; do
            input_counter=$((input_counter + 1))
            echo "Using input $input_counter of $input_total: $input_file"

            if [[ $debug -eq 1 ]]; then
                # Run with debug flag
                (cargo build --release && target/release/cargo-zisk verify-constraints \
                --witness-lib target/release/libzisk_witness.so \
                --elf "$elf_file" \
                --input "$input_file" \
                --proving-key "$proving_key" --emulator --debug)
            else
                # Run without debug flag
                (cargo build --release && target/release/cargo-zisk verify-constraints \
                --witness-lib target/release/libzisk_witness.so \
                --elf "$elf_file" \
                --input "$input_file" \
                --proving-key "$proving_key" --emulator)
            fi
        done
    else
        echo "Invalid input path: $input_path"
        exit 1
    fi
}

# Logic for multiple ELF files in a directory
if [[ $elf_mode -eq 0 ]]; then
    echo "Verifying ELF files found in directory: $dir"

    # Find ELF files in the directory
    elf_files=$(find "$dir" -type f -name my.elf | sort)
    if [[ -z "$elf_files" ]]; then
        echo "No ELF files found in directory: $dir"
        exit 0
    fi

    # List files with their corresponding index
    counter=0
    for elf_file in $elf_files; do
        counter=$((counter + 1))
        echo "File $counter: $elf_file"
    done

    # Log begin and end options, if provided
    if [ $begin -ne 0 ]; then
        echo "Beginning at file $begin"
    fi
    if [ $end -ne 0 ]; then
        echo "Ending at file $end"
    fi

    # If just listing, exit
    if [ $list -eq 1 ]; then
        echo "Exiting after listing files"
        exit 0
    fi

    # Record the number of files
    max_counter=$counter

    # For all ELF files
    counter=0
    for elf_file in $elf_files; do
        counter=$((counter + 1))

        # Skip files lower than begin
        if [ $begin -ne 0 ] && [ $counter -lt $begin ]; then
            continue
        fi

        # Skip files higher than end
        if [ $end -ne 0 ] && [ $counter -gt $end ]; then
            continue
        fi

        echo ""
        echo "Verifying file $counter of $max_counter: $elf_file"

        if [ $debug -eq 1 ]; then
            # Run with debug flag
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$default_input" \
            --proving-key "$proving_key" --emulator --debug)
        else
            # Run without debug flag
            echo "Input($default_input)"
            (cargo build --release && target/release/cargo-zisk verify-constraints \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$default_input" \
            --proving-key "$proving_key" --emulator)
        fi
    done
else
    # Logic for single ELF file with input directory or file
    verify_elf_with_inputs "$elf_file" "$input_path"
fi
