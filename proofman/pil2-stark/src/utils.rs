use libloading::{Library, Symbol};
use proofman_common::WitnessManager;
use std::path::PathBuf;

pub fn load_plugin<F>(path: PathBuf) -> Result<Box<dyn WitnessManager<F>>, libloading::Error> {
    let library = unsafe { Library::new(path)? };

    let plugin: Symbol<fn() -> Box<dyn WitnessManager<F>>> = unsafe { library.get(b"create_plugin")? };
    Ok(plugin())
}
