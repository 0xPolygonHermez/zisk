#![cfg(all(target_os = "zkvm", target_vendor = "zisk"))]

// TODO: Add all fcalls
pub fn diagnostic_fcall() {
    println!("diagnostic_fcall() success");
}
