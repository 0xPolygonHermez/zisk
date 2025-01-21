Currently pil2-components tests can be launched with the following commands:

------------------------------------
SIMPLE

mkdir -p ./pil2-components/test/simple/build/ \
&& rm -rf pil2-components/test/simple/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/simple/simple.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/simple/build/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/simple/build/build.pilout \
     -b ./pil2-components/test/simple/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/simple/build/build.pilout \
     --path ./pil2-components/test/simple/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libsimple.so \
     --proving-key ./pil2-components/test/simple/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libsimple.so \
     --proving-key ./pil2-components/test/simple/build/provingKey \
     --output-dir ./pil2-components/test/simple/build/proofs

------------------------------------
CONNECTION

mkdir -p ./pil2-components/test/std/connection/build/ \
&& rm -rf pil2-components/test/connection/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/connection/connection.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/connection/build/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/connection/build/build.pilout \
     -b ./pil2-components/test/std/connection/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/connection/build/build.pilout \
     --path ./pil2-components/test/std/connection/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libconnection.so \
     --proving-key ./pil2-components/test/std/connection/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libconnection.so \
     --proving-key ./pil2-components/test/std/connection/build/provingKey \
     --output-dir ./pil2-components/test/std/connection/build/proofs

------------------------------------
DIFF BUSES

mkdir -p ./pil2-components/test/std/diff_buses/build/ \
&& rm -rf pil2-components/test/diff_buses/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/diff_buses/diff_buses.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/diff_buses/build/diff_buses.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/diff_buses/build/diff_buses.pilout \
     -b ./pil2-components/test/std/diff_buses/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/diff_buses/build/diff_buses.pilout \
     --path ./pil2-components/test/std/diff_buses/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libdiff_buses.so \
     --proving-key ./pil2-components/test/std/diff_buses/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libdiff_buses.so \
     --proving-key ./pil2-components/test/std/diff_buses/build/provingKey \
     --output-dir ./pil2-components/test/std/diff_buses/build/proofs

------------------------------------
DIRECT UPDATES

mkdir -p ./pil2-components/test/std/direct_update/build/ \
&& rm -rf pil2-components/test/direct_update/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/direct_update/direct_update.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/direct_update/build/direct_update.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/direct_update/build/direct_update.pilout \
     -b ./pil2-components/test/std/direct_update/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/direct_update/build/direct_update.pilout \
     --path ./pil2-components/test/std/direct_update/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libdirect_update.so \
     --proving-key ./pil2-components/test/std/direct_update/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libdirect_update.so \
     --proving-key ./pil2-components/test/std/direct_update/build/provingKey \
     --output-dir ./pil2-components/test/std/direct_update/build/proofs

------------------------------------
LOOKUP

mkdir -p ./pil2-components/test/std/lookup/build/ \
&& rm -rf pil2-components/test/lookup/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/lookup/lookup.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/lookup/build/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/lookup/build/build.pilout \
     -b ./pil2-components/test/std/lookup/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/lookup/build/build.pilout \
     --path ./pil2-components/test/std/lookup/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/liblookup.so \
     --proving-key ./pil2-components/test/std/lookup/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/liblookup.so \
     --proving-key ./pil2-components/test/std/lookup/build/provingKey \
     --output-dir ./pil2-components/test/std/lookup/build/proofs

------------------------------------
PERMUTATION

mkdir -p ./pil2-components/test/std/permutation/build/ \
&& rm -rf pil2-components/test/permutation/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/permutation/permutation.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/permutation/build/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/permutation/build/build.pilout \
     -b ./pil2-components/test/std/permutation/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/permutation/build/build.pilout \
     --path ./pil2-components/test/std/permutation/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/libpermutation.so \
     --proving-key ./pil2-components/test/std/permutation/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/libpermutation.so \
     --proving-key ./pil2-components/test/std/permutation/build/provingKey \
     --output-dir ./pil2-components/test/std/permutation/build/proofs

------------------------------------
RANGE CHECKS

mkdir -p ./pil2-components/test/std/range_check/build/ \
&& rm -rf pil2-components/test/range_check/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/range_check/build.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/range_check/build/build.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/range_check/build/build.pilout \
     -b ./pil2-components/test/std/range_check/build \
     -t ./pil2-stark/build/bctree \
&& cargo run --bin proofman-cli pil-helpers \
     --pilout ./pil2-components/test/std/range_check/build/build.pilout \
     --path ./pil2-components/test/std/range_check/rs/src -o \
&& cargo build \
&& cargo run --bin proofman-cli verify-constraints \
     --witness-lib ./target/debug/librange_check.so \
     --proving-key ./pil2-components/test/std/range_check/build/provingKey \
&& cargo run --bin proofman-cli prove \
     --witness-lib ./target/debug/librange_check.so \
     --proving-key ./pil2-components/test/std/range_check/build/provingKey \
     --output-dir ./pil2-components/test/std/range_check/build/proofs

------------------------------------
SPECIAL

mkdir -p ./pil2-components/test/std/special/build/ \
&& rm -rf pil2-components/test/special/build/proofs/ \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/special/expr_optimizations.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/special/build/expr_optimizations.pilout \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/special/intermediate_sums.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/special/build/intermediate_sums.pilout \
&& node ../pil2-compiler/src/pil.js ./pil2-components/test/std/special/table.pil \
     -I ./pil2-components/lib/std/pil \
     -o ./pil2-components/test/std/special/build/table.pilout \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/special/build/expr_optimizations.pilout \
     -b ./pil2-components/test/std/special/build \
     -t ./pil2-stark/build/bctree \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/special/build/intermediate_sums.pilout \
     -b ./pil2-components/test/std/special/build \
     -t ./pil2-stark/build/bctree \
&& node ../pil2-proofman-js/src/main_setup.js \
     -a ./pil2-components/test/std/special/build/table.pilout \
     -b ./pil2-components/test/std/special/build \
     -t ./pil2-stark/build/bctree