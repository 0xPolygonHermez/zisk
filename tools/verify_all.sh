#!/bin/bash

# Check that at least two arguments have been passed
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <pk_dir> <dirname|elf_file> [-i|--inputs <input_dir|input_file>] [-le|--list-elfs] [-li|--list-inputs] [-b|--begin <first_file>] [-e|--end <last_file>]"
    exit 1
fi

# Initialize variables
list_elfs=0
list_inputs=0
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
        -i|--inputs) input_path="$2"; shift ;;
        -le|--list-elfs) list_elfs=1 ;;
        -li|--list-inputs) list_inputs=1 ;;
        -b|--begin) begin=$2; shift ;;
        -e|--end) end=$2; shift ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done

list_elf_files() {
    local target_dir=$1
    
    if [[ $elf_mode -eq 1 ]]; then
        echo "Verifying ELF file: $elf_file"
        return
    fi
    
    echo "Verifying ELF files in directory: $target_dir"
    elf_files=$(find "$target_dir" -type f -name "*.elf" | sort)
    if [[ -z "$elf_files" ]]; then
        echo "No ELF files found in directory: $target_dir"
        return
    fi
    
    counter=0
    for elf_file in $elf_files; do
        counter=$((counter + 1))
        echo "File $counter: $elf_file"
    done
    echo "Total ELF files found: $counter"
}

list_input_files() {
    local input_path=$1
    
    if [[ -z $input_path ]]; then
        echo "No input path provided"
        return
    elif [[ -f $input_path ]]; then
        echo "Input file: $input_path"
        return
    elif [[ -d $input_path ]]; then
        echo "Listing input files in directory: $input_path"
        input_files=$(find "$input_path" -type f | sort)
        if [[ -z "$input_files" ]]; then
            echo "No input files found in directory: $input_path"
            return
        fi
        
        counter=0
        for input_file in $input_files; do
            counter=$((counter + 1))
            echo "Input $counter: $input_file"
        done
        echo "Total input files found: $counter"
    else
        echo "Invalid input path: $input_path"
    fi
}

# Handle listing options first (before building or processing)
if [ $list_elfs -eq 1 ]; then
    if [[ $elf_mode -eq 0 ]]; then
        list_elf_files "$dir"
    else
        list_elf_files "$elf_file"
    fi
    exit 0
fi

if [ $list_inputs -eq 1 ]; then
    list_input_files "$input_path"
    exit 0
fi

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

# Build the project
echo "Building project..."
if ! cargo build --release; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Build successful"
echo ""

if [[ $elf_mode -eq 0 ]]; then
    # Logic for multiple ELF files in a directory

    # Find ELF files in the directory
    elf_files=$(find "$dir" -type f -name "*.elf" | sort)

    # List ELF files found
    list_elf_files "$dir"

    # Log begin and end options, if provided
    if [ $begin -ne 0 ]; then
        echo "Beginning at file $begin"
    fi
    if [ $end -ne 0 ]; then
        echo "Ending at file $end"
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

        if (cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
        --elf "$elf_file" \
        --proving-key "$proving_key"); then
            record_result "$elf_file" "PASSED" "$counter"
        else
            record_result "$elf_file" "FAILED" "$counter"
        fi

        echo ""
    done

    # Print final report for directory mode
    print_final_report
else
    # Logic for single ELF file with input directory or file
    input_counter=0
    input_total=0

    if [[ -z $input_path ]]; then
        # No input path provided
        echo "Verifying ELF file: \"$elf_file\" with no inputs"

        if (cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
        --elf "$elf_file" \
        --proving-key "$proving_key"); then
            record_result "$elf_file" "PASSED"
        else
            record_result "$elf_file" "FAILED"
        fi

    elif [[ -f $input_path ]]; then
        # Single input file provided
        echo "Verifying ELF file: \"$elf_file\" with input file: \"$input_path\""

        if (cargo run --release --bin cargo-zisk verify-constraints \
        --emulator \
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

        echo "Verifying ELF file \"$elf_file\" with $input_total input files from directory \"$input_path\":"
        for input_file in $input_files; do
            input_counter=$((input_counter + 1))
            echo "Input $input_counter: $input_file"
        done
        echo ""

        input_counter=0
        for input_file in $input_files; do
            input_counter=$((input_counter + 1))

            echo "Verifying input $input_counter of $input_total: \"$input_file\""

            if (cargo run --release --bin cargo-zisk verify-constraints \
            --emulator \
            --elf "$elf_file" \
            --input "$input_file" \
            --proving-key "$proving_key"); then
                record_result "$input_file" "PASSED" "$input_counter"
            else
                record_result "$input_file" "FAILED" "$input_counter"
            fi

            echo ""
        done
    else
        echo "Invalid input path: $input_path"
        exit 1
    fi

    # Print final report for single file mode
    print_final_report
fi