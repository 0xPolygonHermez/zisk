#!/bin/bash -x

echo "Verify all ELF files found in a directory"

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <dirname>"
    exit 1
fi

if [[ "$1" != "" ]]; then
    DIR="$1"
else
    DIR=.
fi

echo "Verifying ELF files found in directory ${DIR}"

ELF_FILES=`find $DIR -name my.elf`
INPUT_FILE="/tmp/empty_input.bin"
touch $INPUT_FILE

COUNTER=0

for ELF_FILE in $ELF_FILES
do
    echo ""
    echo "Verifying file ${COUNTER}: ${ELF_FILE}"
    (cargo build --release && cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom $ELF_FILE -i $INPUT_FILE --proving-key ../zisk/build/provingKey)
    COUNTER=$((COUNTER+1))
done

