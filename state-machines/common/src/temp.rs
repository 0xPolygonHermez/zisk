use std::error::Error;

#[derive(Debug)]
pub struct EmuTrace {
    pub a: u64,
    pub b: u64,
}

pub trait Emulator {
    fn emulate(
        &self,
        freq: usize,
        callback: impl Fn(Vec<EmuTrace>),
    ) -> Result<Vec<u8>, Box<dyn Error>>;
}

pub struct MockEmulator {}

impl Emulator for MockEmulator {
    fn emulate(
        &self,
        freq: usize,
        callback: impl Fn(Vec<EmuTrace>),
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut inputs = Vec::with_capacity(freq);

        for i in 0..(1 << 6) + 3 {
            inputs.push(EmuTrace { a: i, b: i + 1 });

            if inputs.len() == freq {
                callback(std::mem::take(&mut inputs));
            }
        }

        if !inputs.is_empty() {
            println!("Emulator: Flushing Remaining inputs {:?}", inputs);
            callback(inputs);
        }

        Ok(vec![1, 2, 3, 4, 5])
    }
}
