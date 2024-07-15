#!/bin/bash

input_file="input.bin"
output_file="input_size.bin"

# Verify that the input file exists
if [ ! -f "$input_file" ]; then
    echo "Error: The input file '$input_file' does not exist."
    exit 1
fi

# Get the size of the input file in bytes
size=$(stat -c%s "$input_file")

# Convert the size to little endian format and write it to a binary file
printf "%08x" "$size" | tac -rs.. | xxd -r -p > "$output_file"
