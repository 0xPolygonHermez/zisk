//! LLVM diagnostics can reject a program even when the builder exits successfully.
#![cfg(unix)]
use std::{
    env, fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn checks_diagnostics_without_optional_search_tools() {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let directory =
        TempDir(env::temp_dir().join(format!("zisk-build-test-{}-{id}", std::process::id())));
    let root = &directory.0;
    fs::create_dir(root).unwrap();
    fs::create_dir(root.join("scripts")).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/build-sbf.sh");
    let script = root.join("scripts/build-sbf.sh");
    fs::copy(source, &script).unwrap();
    let builder = root.join(".cache/build-tools/bin/cargo-build-sbf");
    fs::create_dir_all(builder.parent().unwrap()).unwrap();
    let commands = root.join("commands");
    fs::create_dir(&commands).unwrap();
    let path = env::var_os("PATH").unwrap();
    for name in ["dirname", "mkdir", "tee", "grep"] {
        let executable = env::split_paths(&path)
            .map(|p| p.join(name))
            .find(|p| p.is_file())
            .expect("required POSIX command");
        symlink(executable, commands.join(name)).unwrap();
    }
    for (diagnostic, succeeds) in [
        ("Finished release build", true),
        ("Error: Function exceeds stack offset", false),
        ("LLVM ERROR: Branch target out of insn range", false),
    ] {
        fs::write(&builder, format!("#!/bin/bash\nif [[ \"$1\" == --version ]]; then echo 'cargo-build-sbf 4.6.0'; else echo '{diagnostic}'; fi\n")).unwrap();
        fs::set_permissions(&builder, fs::Permissions::from_mode(0o755)).unwrap();
        let output =
            Command::new("/bin/bash").arg(&script).env("PATH", &commands).output().unwrap();
        assert_eq!(
            output.status.success(),
            succeeds,
            "{diagnostic}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
