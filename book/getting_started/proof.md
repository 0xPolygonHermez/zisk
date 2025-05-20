## Steps to verify constraints or generate proof 

compile pils:
```
node ../pil2-compiler/src/pil.js pil/fork_0/pil/zisk.pil -I lib/std/pil -o pil/fork_0/pil/zisk.pilout
```

generate "structs" for different airs:
`(cd ../pil2-proofman; cargo run --bin proofman-cli pil-helpers --pilout ../zisk/pil/fork_0/pil/zisk.pilout --path ../zisk/pil/fork_0/src/ -o)`

prepare "fast tools" (only first time):
`(cd ../zkevm-prover && git switch develop_rust_lib && git submodule init &&  git submodule update && make -j bctree && make starks_lib -j)`

setup for pil, this step is necessary **only when pil change**:
`node ../pil2-proofman-js/src/main_setup.js -a pil/fork_0/pil/zisk.pilout -b build -t ../zkevm-prover/build/bctree`

this step should be done once and is optional. Edit file pil2-proofman/provers/starks-lib-c/Cargo.toml to remove "no_lib_link" from line 12:
`nano ../pil2-proofman/provers/starks-lib-c/Cargo.toml`

compile witness computation library (libzisk_witness.so). If you haven't nightly mode as default, must add +nightly when do build.
`cargo build --release`

In the following steps to verify constraints or generate prove, select one of these inputs:
- input.bin: large number of sha
- input_one_segment.bin: only one sha
- input_two_segments.bin: 512 shas

To **verify constraints** use: 
`(cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey)`

To **generate proof** use: 
`(cd ../pil2-proofman; cargo run --release --bin proofman-cli verify-constraints --witness-lib ../zisk/target/release/libzisk_witness.so --rom ../zisk/emulator/benches/data/my.elf -i ../zisk/emulator/benches/data/input.bin --proving-key ../zisk/build/provingKey)`

## Steps to compile a verifiable rust program

### Setup
Install qemu:
`sudo apt-get install qemu-system`
Add tokens to access repos:
```
export GITHUB_ACCESS_TOKEN=....
export ZISK_TOKEN=....
```
### Create new hello_world project
Create project with toolchain:
```bash
cargo-zisk sdk new hello_world
cd hello_world
```

Compile and execute in **riscv mode**:
`cargo-zisk run --release`

Compile and execute in **zisk mode**:
`cargo-zisk run --release --sim`

Execute with ziskemu:
`ziskemu -i build/input.bin -x -e target/riscv64ima-zisk-zkvm-elf/release/fibonacci`

### Update toolchain
```
ziskup
```
If ziskup fails, could update ziskemu manually.

### Update ziskemu manually
```bash
cd zisk
git pull
cargo install --path emulator
cp ~/.cargo/bin/ziskemu ~/.zisk/bin/
```

```bash
ziskemu -i build/input.bin -x -e target/riscv64ima-zisk-zkvm-elf/debug/fibonacci
```
