I set a nightly toolchain to use Plonky2...

$ rustup override set nightly
override toolchain for '/Users/xpinsach/dev/pil2-proofman' set to 'nightly-aarch64-apple-darwin'

to be able to use the nightly features of Rust (as Plonky2 need)

NOTE: Back to stable toolchain !!!!
rustup help toolchain

Command to generate protobuffer parsing RUST code:
protoc --rust_out=experimental-codegen=enabled,kernel=upb:. pilout.proto

TraceColSegment
column: String,
row_from: usize,
row_to: usize,
buffer_idx: usize,
offset: usize,
next: usize,
last: bool

TraceColSegments
add_trace_layout(&mut self, trace_layout: &TraceLayout, TraceStoreType: TraceStoreType)-> Option<usize>
add_col_segment(&mut self, col: String, elem_0: usize, elem_1: usize, buffer: u32, offset: usize, next: usize) -> u32
find_col_segments(&self, col: String) -> Option<&TraceColSegment>
find_col_segment(&self, elem_idx: usize) -> Option<&TraceColSegment>
find_last_segment(&self, col: String) -> Option<&TraceColSegment>

TraceBuffer
buffer: Vec<u8>,
size: usize,

TraceBuffers
create_buffer(&mut self, size: usize) -> Option<usize>

TraceColInfo
column: String,
total_elements: usize,
element_bytes: usize,

TraceMemory
add_trace(&mut self, trace_layout: &TraceLayout, TraceStoreType: TraceStoreType, Trace: Vec[u8])-> Option<usize>

Context
add_trace(&mut self, subproof_id: usize, air_id: usize, trace: Vec<u8>) -> Option<usize>



