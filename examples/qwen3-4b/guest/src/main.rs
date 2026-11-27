#![no_main]
mod bf16_projection;
use qwen_model::{layer, stages};
use ziskos::zisklib::sha256 as sha;

ziskos::entrypoint!(main);

fn commit_capture<T>(digest: &[u8; 32], buffer: Vec<T>) {
    // Keep the buffer alive for the emulator to read after halt.
    let buffer = core::mem::ManuallyDrop::new(buffer);
    ziskos::io::commit_slice(digest);
    ziskos::io::commit_slice(&(buffer.as_ptr() as u64).to_le_bytes());
    ziskos::io::commit_slice(&(core::mem::size_of_val(buffer.as_slice()) as u64).to_le_bytes());
}

fn main() {
    let data = ziskos::io::read_slice();
    if data.starts_with(layer::MAGIC) {
        let result = layer::execute_batched(data, sha, &mut bf16_projection::project).unwrap();
        commit_capture(&layer::digest_of(data, &result.receipt), result.output);
    } else if data.starts_with(stages::head::MAGIC) {
        let result = stages::head::execute_with(data, sha, &mut |x, w, out| {
            bf16_projection::project(x, stages::DIM as usize, &[w], &mut [out])
        })
        .unwrap();
        commit_capture(&result.digest, result.logits);
    } else if data.starts_with(stages::embed::MAGIC) {
        let result = stages::embed::execute(data, sha).unwrap();
        commit_capture(&result.digest, result.output);
    } else {
        panic!("unknown benchmark input");
    }
}
