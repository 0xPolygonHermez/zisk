use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

use asm_runner::AsmRunnerRomH;

pub struct RomAsmWorker {
    handle: Option<JoinHandle<AsmRunnerRomH>>,
}

impl RomAsmWorker {
    const SHM_DEFAULT_SIZE: u64 = 1 << 30; // 1 GiB

    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn launch_task(&mut self, asm_path: PathBuf, input_data_path: Option<PathBuf>) {
        let handle = thread::spawn(move || {
            AsmRunnerRomH::run(
                &asm_path,
                input_data_path.as_deref(),
                Self::SHM_DEFAULT_SIZE,
                asm_runner::AsmRunnerOptions::default(),
            )
        });
        self.handle = Some(handle);
    }

    pub fn wait_for_task(&mut self) -> AsmRunnerRomH {
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap()
        } else {
            panic!("No task to wait for");
        }
    }
}
