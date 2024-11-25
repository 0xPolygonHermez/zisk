#!/bin/bash

echo "Verify all ELF files found in a directory"

# Check that at least one argument has been passed
if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <dirname> [-l/--list -b/--begin <first_file> -e/--end <last_file>]"
    exit 1
fi

# Get the first argument as the directory path
if [[ "$1" != "" ]]; then
    DIR="$1"
else
    DIR=.
fi
shift
echo "Verifying ELF files found in directory ${DIR}"

# Parse optional arguments
LIST=0
BEGIN=0
END=0
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -l|--list) LIST=1 ;;
        -b|--begin) BEGIN=$2; shift; ;;
        -e|--end) END=$2; shift; ;;
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
    echo "Beginning at file ${END}";
fi

# If just listing, exit
if [ $LIST -eq 1 ]; then
    echo "Exiting after listing files";
    exit 0;
fi

# Record the number of files
MAX_COUNTER=${COUNTER}

# Create and empty input file
INPUT_FILE="/tmp/empty_input.bin"
touch $INPUT_FILE

# For all files
COUNTER=0
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

    # Varify the contraints for this file
    echo ""
    echo "Verifying file ${COUNTER} of ${MAX_COUNTER}: ${ELF_FILE}"
    (cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom $ELF_FILE -i $INPUT_FILE --proving-key ../zisk/build/provingKey)
done

