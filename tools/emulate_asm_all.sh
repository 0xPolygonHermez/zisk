#!/bin/bash

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

if [$DEBUG -eq 1 ]; then
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

# Create an empty input file
INPUT_FILE="/tmp/empty_input.bin"
touch $INPUT_FILE

# For all files
COUNTER=0
DIFF_PASSED_COUNTER=0
DIFF_FAILED_COUNTER=0
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

    # Varify the constraints for this file
    echo ""
    echo "Emulating file ${COUNTER} of ${MAX_COUNTER}: ${ELF_FILE}"

    # Transpile the ELF RISC-V file to Zisk, and then generate assembly file emu.asm
    cargo build --bin=riscv2zisk
    ./target/debug/riscv2zisk $ELF_FILE emulator-asm/src/emu.asm --gen=1

    # Compile the assembly emulator derived from this ELF file
    cd emulator-asm
    make

    # Execute it and save output
    touch empty_input.bin
    build/ziskemuasm -s --gen=1 -o --silent 2>&1|tee output &

    # Store the PID of the background process
    BG_PID=$!
    echo "Sleeping for 10 seconds to let the emulator server initialize..."
    sleep 10
    build/ziskemuasm -c -i empty_input.bin --gen=1 --shutdown
    echo "Sleeping for 5 seconds to let the emulator server complete..."
    sleep 5

    #echo "Killing the background process..."
    #kill $BG_PID

    # Compare output vs reference
    ELF_FILE_DIRECTORY=${ELF_FILE%%my.elf}
    REFERENCE_FILE="../${ELF_FILE_DIRECTORY}../ref/Reference-sail_c_simulator.signature"
    echo "Calling diff of ./output vs reference=$REFERENCE_FILE"
    if diff output $REFERENCE_FILE; then
        DIFF_PASSED_COUNTER=$((DIFF_PASSED_COUNTER+1))
        echo "After processing file ${ELF_FILE}..."
        echo "DIFF PASSED total passed=${DIFF_PASSED_COUNTER} total failed=${DIFF_FAILED_COUNTER}"
    else
        DIFF_FAILED_COUNTER=$((DIFF_FAILED_COUNTER+1))
        echo "After processing file ${ELF_FILE}..."
        echo "DIFF FAILED total passed=${DIFF_PASSED_COUNTER} total failed=${DIFF_FAILED_COUNTER}"
    fi

    # Go back to root directory
    cd ..
done

