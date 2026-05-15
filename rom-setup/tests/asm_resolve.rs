#[cfg(not(target_os = "macos"))]
use rom_setup::{ensure_ziskclib, resolve_emulator_asm, EmulatorAsmSource};

#[test]
#[cfg(not(target_os = "macos"))]
fn resolve_picks_workspace_when_in_workspace() {
    let (path, source) = resolve_emulator_asm().expect("resolution");
    assert!(path.exists(), "resolved path must exist: {}", path.display());
    assert!(matches!(source, EmulatorAsmSource::Workspace));
    assert!(path.ends_with("emulator-asm"));
}

#[test]
#[cfg(not(target_os = "macos"))]
fn ensure_ziskclib_succeeds_in_workspace() {
    let (path, source) = resolve_emulator_asm().expect("resolution");
    ensure_ziskclib(&path, source).expect("ziskclib must be locatable or buildable");
}

#[test]
#[cfg(not(target_os = "macos"))]
fn ensure_ziskclib_finds_installed_prebuilt_lib() {
    let home = match std::env::var_os("HOME") {
        Some(h) => std::path::PathBuf::from(h),
        None => return,
    };
    let installed_emu = home.join(".zisk/zisk/emulator-asm");
    let installed_lib = home.join(".zisk/bin/libziskclib.a");
    if !installed_emu.exists() || !installed_lib.exists() {
        eprintln!("skipping: ~/.zisk/ install layout not present");
        return;
    }
    ensure_ziskclib(&installed_emu, EmulatorAsmSource::Installed)
        .expect("installed libziskclib.a must be locatable");
}
