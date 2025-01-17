#!/bin/bash

# Check that at least one argument has been passed
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <dirname> [-l/--list -b/--begin <first_file> -e/--end <last_file> -d/--debug]"
    exit 1
fi

# Initialize variables
list=0
begin=0
end=0
debug=0
input_path=""
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
LIST=0
BEGIN=0
END=0
DEBUG=0
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -l|--list) LIST=1 ;;
        -b|--begin) BEGIN=$2; shift; ;;
        -e|--end) END=$2; shift; ;;
        -d|--debug) DEBUG=1 ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

ELF_FILES=`find $DIR -type f -name my.elf |sort`

# List files with their corresponding index
COUNTER=0
for ELF_FILE in $ELF_FILES
do
    COUNTER=$((COUNTER+1))
    echo "File ${COUNTER}: ${ELF_FILE}"
done

# Log begin and end options, if provided
if [ $BEGIN -ne 0 ]; then
    echo "Beginning at file ${BEGIN}";
fi
if [ $END -ne 0 ]; then
    echo "Ending at file ${END}";
fi

# Function to verify an ELF file with one or more input files
verify_elf_with_inputs() {
    local elf_file=$1
    local input_path=$2
    local input_counter=0
    local input_total=0

    echo "Verifying ELF file: $elf_file"

# Create an empty input file
INPUT_FILE="/tmp/empty_input.bin"
touch $INPUT_FILE

        if [[ $debug -eq 1 ]]; then
            # Run with debug flag
            (cargo build --release && cd ../pil2-proofman; \
            cargo run --release --bin proofman-cli verify-constraints \
            --witness-lib ../zisk/target/release/libzisk_witness.so \
            --rom "$elf_file" -i "$default_input" \
            --proving-key ../zisk/build/provingKey -d)
        else
            # Run without debug flag
            (cargo build --release && cd ../pil2-proofman; \
            cargo run --release --bin proofman-cli verify-constraints \
            --witness-lib ../zisk/target/release/libzisk_witness.so \
            --rom "$elf_file" -i "$default_input" \
            --proving-key ../zisk/build/provingKey)
        fi
    elif [[ -f $input_path ]]; then
        # Single input file provided
        echo "Using input file: $input_path"

        if [[ $debug -eq 1 ]]; then
            # Run with debug flag
            (cargo build --release && cd ../pil2-proofman; \
            cargo run --release --bin proofman-cli verify-constraints \
            --witness-lib ../zisk/target/release/libzisk_witness.so \
            --rom "$elf_file" -i "$input_path" \
            --proving-key ../zisk/build/provingKey -d)
        else
            # Run without debug flag
            (cargo build --release && cd ../pil2-proofman; \
            cargo run --release --bin proofman-cli verify-constraints \
            --witness-lib ../zisk/target/release/libzisk_witness.so \
            --rom "$elf_file" -i "$input_path" \
            --proving-key ../zisk/build/provingKey)
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
                (cargo build --release && cd ../pil2-proofman; \
                cargo run --release --bin proofman-cli verify-constraints \
                --witness-lib ../zisk/target/release/libzisk_witness.so \
                --rom "$elf_file" -i "$input_file" \
                --proving-key ../zisk/build/provingKey -d)
            else
                # Run without debug flag
                (cargo build --release && cd ../pil2-proofman; \
                cargo run --release --bin proofman-cli verify-constraints \
                --witness-lib ../zisk/target/release/libzisk_witness.so \
                --rom "$elf_file" -i "$input_file" \
                --proving-key ../zisk/build/provingKey)
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

    # Varify the contraints for this file
    echo ""
    echo "Verifying file ${COUNTER} of ${MAX_COUNTER}: ${ELF_FILE}"

    if [ $DEBUG -eq 1 ]; then
        # Run with debug flag
        (cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom $ELF_FILE -i $INPUT_FILE --proving-key ../zisk/build/provingKey -d)
    else
        # Run without debug flag
        (cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom $ELF_FILE -i $INPUT_FILE --proving-key ../zisk/build/provingKey)
    fi
done

