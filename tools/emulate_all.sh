#!/bin/bash

set -e

source "$HOME/.cargo/env"

echo "Emulate in assembly all ELF files found in a directory"

# Check that at least one argument has been passed
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <dirname> [-l/--list -b/--begin <first_file> -e/--end <last_file> -d/--debug]"
    exit 1
fi

# Get the first argument as the directory path
if [[ "$1" != "" ]]; then
    DIR="$1"
else
    DIR=.
fi
shift
echo "Emulating ELF files found in directory ${DIR}"

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

if [ $DEBUG -eq 1 ]; then
    echo "Debug mode enabled";
    set -x;  # Enable debugging output
else
    set +x;  # Disable debugging output
fi

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

# If just listing, exit
if [ $LIST -eq 1 ]; then
    echo "Exiting after listing files";
    exit 0;
fi

# Record the number of files
MAX_COUNTER=${COUNTER}

# Build ZisK
echo "Building ZisK..."
cargo build

# Create an empty input file
INPUT_FILE="/tmp/empty_input.bin"
touch $INPUT_FILE

# For all files
COUNTER=0
PASSED_COUNTER=0
FAILED_COUNTER=0
# Arrays to track results for final report
declare -a TESTED_FILES
declare -a TEST_RESULTS
declare -a TEST_INDEXES
for ELF_FILE in $ELF_FILES
do
    # Increase file counter
    COUNTER=$((COUNTER+1))

    # Skip files lower than BEGIN
    if [ $BEGIN -ne 0 ] && [ $COUNTER -lt $BEGIN ]; then
        continue;
    fi

    # Skip files higher than END
    if [ $END -ne 0 ] && [ $COUNTER -gt $END ]; then
        continue;
    fi

    # Emulate this file
    echo ""
    echo "Emulating file ${COUNTER} of ${MAX_COUNTER}: ${ELF_FILE}"

    # Execute it and save output
    ./target/debug/ziskemu -e $ELF_FILE -i $INPUT_FILE 2>&1|tee output

    # Compare output vs reference
    REFERENCE_FILE=${ELF_FILE%%my.elf}../ref/Reference-sail_c_simulator.signature
    cp $REFERENCE_FILE .
    echo "Calling diff of output vs reference=${REFERENCE_FILE}"
    ELF_FILE_DIRECTORY=${ELF_FILE%%my.elf}
    if diff output $REFERENCE_FILE; then
        PASSED_COUNTER=$((PASSED_COUNTER+1))
        echo "After processing file ${ELF_FILE} against reference ${REFERENCE_FILE} ..."
        echo "✅ DIFF PASSED total passed=${PASSED_COUNTER} total failed=${FAILED_COUNTER}"
        # Record result for final report
        TESTED_FILES+=("$ELF_FILE")
        TEST_RESULTS+=("PASSED")
        TEST_INDEXES+=("$COUNTER")
    else
        FAILED_COUNTER=$((FAILED_COUNTER+1))
        echo "After processing file ${ELF_FILE} against reference ${REFERENCE_FILE} ..."
        echo "❌ DIFF FAILED total passed=${PASSED_COUNTER} total failed=${FAILED_COUNTER}"
        # Record result for final report
        TESTED_FILES+=("$ELF_FILE")
        TEST_RESULTS+=("FAILED")
        TEST_INDEXES+=("$COUNTER")
    fi
done

# Print final report
echo ""
echo "======================================"
echo "           FINAL REPORT"
echo "======================================"
echo "Total files processed: $((PASSED_COUNTER + FAILED_COUNTER))"
echo "Passed: ${PASSED_COUNTER}"
echo "Failed: ${FAILED_COUNTER}"
echo ""

if [ ${#TESTED_FILES[@]} -gt 0 ]; then
    echo "Detailed Results:"
    echo "=================="
    for i in "${!TESTED_FILES[@]}"; do
        if [ "${TEST_RESULTS[$i]}" = "PASSED" ]; then
            echo "✅ ${TEST_INDEXES[$i]} ${TESTED_FILES[$i]}"
        else
            echo "❌ ${TEST_INDEXES[$i]} ${TESTED_FILES[$i]}"
        fi
    done
    echo ""
fi

echo "Total files processed: $((PASSED_COUNTER + FAILED_COUNTER)): ✅${PASSED_COUNTER} passed, ❌${FAILED_COUNTER} failed"

if [ $FAILED_COUNTER -eq 0 ]; then
    echo "✅ All ELF files processed successfully."
else
    echo "❌ ${FAILED_COUNTER} ELF files have failed."
    exit 1
fi

