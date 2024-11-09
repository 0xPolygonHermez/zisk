#!/bin/bash
node --max-old-space-size=65536 ../pil2-compiler/src/pil.js pil/zisk.pil -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines -o pil/zisk.pilout
node --max-old-space-size=65536 ../pil2-proofman-js/src/main_setup.js -a pil/zisk.pilout -b build -t ../pil2-proofman/pil2-stark/build/bctree
(cd ../pil2-proofman; cargo run --bin proofman-cli pil-helpers --pilout ../zisk/pil/zisk.pilout --path ../zisk/pil/src/ -o)
RUSTFLAGS="-L native=/home/zkronos73/devel/zisk2/pil2-proofman/pil2-stark/lib"  RUST_BACKTRACE=1 cargo build --release
(cd ../pil2-proofman; RUST_BACKTRACE=1 cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input_one_segment.bin --proving-key ../zisk/build/provingKey)