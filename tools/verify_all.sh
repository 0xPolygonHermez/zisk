#!/bin/bash

# Check that at least two arguments have been passed
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <pk_dir> <dirname|elf_file> [-l|--list] [-b|--begin <first_file>] [-e|--end <last_file>] [-i|--inputs <input_dir|input_file>]"
    exit 1
fi

# Initialize variables
list=0
begin=0
end=0
input_path=""
proving_key=""
elf_mode=0
passed_counter=0
failed_counter=0

# Arrays to track results for final report
declare -a tested_files
declare -a test_results
declare -a test_indexes

# First argument is the proving key
proving_key="$1"
shift

# Validate that proving key directory exists
if [[ ! -d "$proving_key" ]]; then
    echo "Error: Proving key directory does not exist: $proving_key"
    exit 1
fi

# Second argument is the ELF file or directory (mandatory)
if [[ -f "$1" ]]; then
    elf_file="$1"
    elf_mode=1
elif [[ -d "$1" ]]; then
    dir="$1"
else
    echo "Invalid input. The second argument must be a directory or an ELF file."
    exit 1
fi
shift

# Parse optional arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -l|--list) list=1 ;;
        -b|--begin) begin=$2; shift ;;
        -e|--end) end=$2; shift ;;
        -i|--inputs) input_path="$2"; shift ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

# Function to record test result
record_result() {
    local file=$1
    local result=$2
    local index=${3:-""}
    
    tested_files+=("$file")
    test_results+=("$result")
    test_indexes+=("$index")
    
    if [ "$result" = "PASSED" ]; then
        passed_counter=$((passed_counter + 1))
        echo "✅ VERIFICATION PASSED - total passed=${passed_counter} total failed=${failed_counter}"
    else
        failed_counter=$((failed_counter + 1))
        echo "❌ VERIFICATION FAILED - total passed=${passed_counter} total failed=${failed_counter}"
    fi
}

# Function to verify an ELF file with one or more input files
verify_elf_with_inputs() {
    local elf_file=$1
    local input_path=$2
    local input_counter=0
    local input_total=0

    if [[ -z $input_path ]]; then
        # No input path provided
        echo "Verifying ELF file: \"$elf_file\" with no inputs"

        if (cargo build --release && cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
        --witness-lib target/release/libzisk_witness.so \
        --elf "$elf_file" \
        --proving-key "$proving_key"); then
            record_result "$elf_file" "PASSED"
        else
            record_result "$elf_file" "FAILED"
        fi

    elif [[ -f $input_path ]]; then
        # Single input file provided
        echo "Verifying ELF file: \"$elf_file\" with input file: \"$input_path\""

        if (cargo build --release && cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
        --witness-lib target/release/libzisk_witness.so \
        --elf "$elf_file" \
        --input "$input_path" \
        --proving-key "$proving_key"); then
            record_result "$elf_file" "PASSED"
        else
            record_result "$elf_file" "FAILED"
        fi

    elif [[ -d $input_path ]]; then
        # Directory of input files provided
        input_files=$(find "$input_path" -type f | sort)
        input_total=$(echo "$input_files" | wc -l)
        if [[ -z "$input_files" ]]; then
            echo "No input files found in directory: $input_path"
            return
        fi

        echo "Verifying ELF file: \"$elf_file\" with $input_total input files from directory: \"$input_path\""
        local all_passed=true
        for input_file in $input_files; do
            input_counter=$((input_counter + 1))
            echo "Verifying input $input_counter of $input_total: \"$input_file\""

            if ! (cargo build --release && cargo run --release --bin cargo-zisk verify-constraints \
            --emulator \
            --witness-lib target/release/libzisk_witness.so \
            --elf "$elf_file" \
            --input "$input_file" \
            --proving-key "$proving_key"); then
                all_passed=false
            fi
        done

        if [ "$all_passed" = true ]; then
            record_result "$elf_file" "PASSED"
        else
            record_result "$elf_file" "FAILED"
        fi
    else
        echo "Invalid input path: $input_path"
        exit 1
    fi
}

print_final_report() {
    echo ""
    echo "======================================"
    echo "           FINAL REPORT"
    echo "======================================"
    echo "Total files processed: $((passed_counter + failed_counter))"
    echo "Passed: ${passed_counter}"
    echo "Failed: ${failed_counter}"
    echo ""

    if [ ${#tested_files[@]} -gt 0 ]; then
        echo "Detailed Results:"
        echo "=================="
        for i in "${!tested_files[@]}"; do
            if [ "${test_results[$i]}" = "PASSED" ]; then
                echo "✅ ${test_indexes[$i]} ${tested_files[$i]}"
            else
                echo "❌ ${test_indexes[$i]} ${tested_files[$i]}"
            fi
        done
        echo ""
    fi

    echo "Total files processed: $((passed_counter + failed_counter)): ✅${passed_counter} passed, ❌${failed_counter} failed"

    if [ $failed_counter -eq 0 ]; then
        echo "✅ All ELF files verified successfully."
    else
        echo "❌ ${failed_counter} ELF files have failed verification."
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
        echo "Verifying ELF file $counter of $max_counter: \"$elf_file\" with no inputs"

        if (cargo build --release && cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
        --witness-lib target/release/libzisk_witness.so \
        --elf "$elf_file" \
        --proving-key "$proving_key"); then
            record_result "$elf_file" "PASSED" "$counter"
        else
            record_result "$elf_file" "FAILED" "$counter"
        fi
    done

    # Print final report for directory mode
    print_final_report
else
    # Logic for single ELF file with input directory or file
    verify_elf_with_inputs "$elf_file" "$input_path"

    # Print final report for single file mode
    print_final_report
fi
