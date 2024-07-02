use libloading::{Library, Symbol};
use crate::WitnessManagerAPI;
use std::path::PathBuf;

pub fn load_plugin<'a, F>(path: PathBuf) -> Result<Box<dyn WitnessManagerAPI<'a, F>>, libloading::Error> {
    let library = unsafe { Library::new(path)? };

    let plugin: Symbol<fn() -> Box<dyn WitnessManagerAPI<'a, F>>> = unsafe { library.get(b"create_plugin")? };

    Ok(plugin())
}
