use super::*;
use regex::Regex;
use serde_json::json;
use std::collections::BTreeSet;

const NITRO: &str = "62f0625ad9bdb4e1ff42f08688429fc33618f181";
const WASMER: &str = "d72d6c4f02549f3f443b0f0653f39787695a29b4";
const SOFTFLOAT: &str = "7bf03222ad094ec3441f5c3935eeb1b41ee470ba";
const TARGET_LEXICON: &str = "b36201fb3efd8babeb625c26217bfb4fb7af32a5";
const SHA3: &str = "08098df5827b672d604e2dc440446b86064b99f2";
const TARGET: &str = "riscv64ima-zisk-zkvm-elf";
const LIBRARIES: [&str; 5] = ["forward", "soft-float", "wasi_stub", "user_host", "program_exec"];

#[derive(clap::Args)]
pub struct Args {
    /// Local Git object source containing the pinned Nitro commit.
    #[arg(long)]
    nitro_source: PathBuf,
    #[arg(long)]
    wasmer_source: Option<PathBuf>,
    #[arg(long)]
    target_lexicon_source: Option<PathBuf>,
    #[arg(long)]
    sha3_source: Option<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    test: bool,
    #[arg(long)]
    prepare_only: bool,
}

fn common_dir(source: &Path) -> Result<PathBuf> {
    let common = PathBuf::from(text(
        Command::new("git").arg("-C").arg(source).args(["rev-parse", "--git-common-dir"]),
    )?);
    Ok(source.join(common).canonicalize()?)
}

fn cached_source(explicit: Option<PathBuf>, prefix: &str, revision: &str) -> Result<PathBuf> {
    let candidates = if let Some(path) = explicit {
        vec![path]
    } else {
        let home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cargo"));
        let mut candidates = fs::read_dir(home.join("git/db"))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        candidates.retain(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix));
        candidates.sort();
        candidates
    };
    for path in candidates {
        if run(Command::new("git").arg("-C").arg(&path).args([
            "cat-file",
            "-e",
            &format!("{revision}^{{commit}}"),
        ]))
        .is_ok()
        {
            return Ok(path);
        }
    }
    bail!("pinned {prefix} commit is missing; provide its source checkout")
}

fn replace_once(text: &str, pattern: &str, replacement: &str) -> Result<String> {
    let regex = Regex::new(pattern)?;
    anyhow::ensure!(
        regex.find_iter(text).count() == 1,
        "pinned source declaration changed: {pattern}"
    );
    Ok(regex.replace(text, regex::NoExpand(replacement)).into_owned())
}

fn prepare(args: &Args, out: &Path) -> Result<PathBuf> {
    let here = runtime().join("stylus");
    let source = args.nitro_source.canonicalize()?;
    let common = common_dir(&source)?;
    let nitro = out.join("nitro");
    fresh_dir(&nitro)?;
    git_archive(&source, NITRO, &nitro)?;
    fs::copy(nitro.join("Cargo.lock"), out.join("canonical.Cargo.lock"))?;
    let wasmer =
        args.wasmer_source.clone().unwrap_or_else(|| common.join("modules/crates/tools/wasmer"));
    git_archive(&wasmer, WASMER, &nitro.join("crates/tools/wasmer"))?;
    let manifest = nitro.join("crates/tools/wasmer/Cargo.toml");
    let mut content = fs::read_to_string(&manifest)?;
    for (before, after) in [
        ("corosensei = \"0.3.0\"", "corosensei = \"=0.3.3\""),
        ("which = \"8.0.0\"", "which = \"=8.0.2\""),
    ] {
        content = replace_once(&content, &regex::escape(before), after)?;
    }
    fs::write(manifest, content)?;
    let brotli = if source.join("brotli/.git").exists() {
        source.join("brotli")
    } else {
        common.join("modules/brotli")
    };
    git_archive(&brotli, build::BROTLI, &nitro.join("brotli"))?;
    git_archive(
        &common.join("modules/crates/wasm-libraries/soft-float/SoftFloat"),
        SOFTFLOAT,
        &nitro.join("crates/wasm-libraries/soft-float/SoftFloat"),
    )?;
    run(Command::new("git").arg("apply").arg(here.join("interpreter.patch")).current_dir(&nitro))?;
    let manifest = nitro.join("Cargo.toml");
    let members = [
        "arbutil",
        "brotli",
        "caller-env",
        "validation",
        "prover",
        "wasm-libraries/forward",
        "wasm-libraries/wasi-stub",
        "wasm-libraries/user-host-trait",
        "wasm-libraries/user-host",
        "wasm-libraries/program-exec",
    ]
    .map(|name| format!("crates/{name}"));
    let replacement = format!(
        "members = {}\ndefault-members = [\"crates/prover\"]",
        serde_json::to_string(&members)?
    );
    let content = replace_once(
        &fs::read_to_string(&manifest)?,
        r"(?s)members = \[.*?\]\ndefault-members = \[.*?\]",
        &replacement,
    )?;
    let content = replace_once(
        &content,
        r"(?s)\[patch.crates-io\].*?\n\[profile.release\]",
        "[profile.release]",
    )?;
    fs::write(
        &manifest,
        format!(
            "{content}\n[patch.crates-io]\ntarget-lexicon = {{ path = \"../target-lexicon\" }}\n"
        ),
    )?;
    for (source, prefix, revision, name) in [
        (args.target_lexicon_source.clone(), "target-lexicon-", TARGET_LEXICON, "target-lexicon"),
        (args.sha3_source.clone(), "zisk-patch-hashes-", SHA3, "sha3-patch"),
    ] {
        let source = cached_source(source, prefix, revision)?;
        let destination = out.join(name);
        fresh_dir(&destination)?;
        git_archive(&source, revision, &destination)?;
    }
    run(Command::new("git")
        .arg("apply")
        .arg(here.join("target-lexicon.patch"))
        .current_dir(out.join("target-lexicon")))?;
    Ok(nitro)
}

fn cargo(out: &Path, wasm: &Path, flags: &str, toolchain: &str) -> Command {
    let mut command = Command::new("cargo");
    command.arg(format!("+{toolchain}"));
    for key in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"]
    {
        command.env_remove(key);
    }
    command
        .env("CARGO_TARGET_DIR", out.join("cargo"))
        .env("NITRO_STYLUS_WASM_DIR", wasm)
        .env("RUSTFLAGS", flags);
    command
}

fn registry_versions(lock: &str, registry_only: bool) -> Result<BTreeSet<(String, String)>> {
    let lock: toml::Value = toml::from_str(lock)?;
    let packages = lock["package"].as_array().context("lock must contain packages")?;
    packages
        .iter()
        .filter(|p| {
            !registry_only
                || p.get("source")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|s| s.starts_with("registry+"))
        })
        .map(|p| {
            Ok((
                p["name"].as_str().context("missing package name")?.to_owned(),
                p["version"].as_str().context("missing package version")?.to_owned(),
            ))
        })
        .collect()
}

fn check_lock(canonical: &str, selected: &str) -> Result<()> {
    let allowed = registry_versions(canonical, false)?;
    let actual = registry_versions(selected, true)?;
    anyhow::ensure!(
        actual.is_subset(&allowed),
        "support-library registry versions changed: {:?}",
        actual.difference(&allowed).collect::<Vec<_>>()
    );
    Ok(())
}

fn brotli_archives(
    nitro: &Path,
    out: &Path,
    arch: &str,
    clang: &Path,
    ar: &Path,
) -> Result<PathBuf> {
    let folder = out.join(format!("brotli-{arch}"));
    fs::create_dir_all(&folder)?;
    let target: &[&str] = if arch == "wasm" {
        &["--target=wasm32-wasip1"]
    } else {
        &[
            "--target=riscv64-unknown-elf",
            "-march=rv64im",
            "-mabi=lp64",
            "-mcmodel=medany",
            "-mno-relax",
            "-msmall-data-limit=0",
        ]
    };
    for group in ["common", "dec", "enc"] {
        let mut objects = Vec::new();
        for source in sources(&nitro.join("brotli/c").join(group), "c")? {
            let object =
                folder.join(format!("{group}-{}.o", source.file_stem().unwrap().to_string_lossy()));
            run(Command::new(clang)
                .args(target)
                .args([
                    "-O2",
                    "-ffreestanding",
                    "-fno-builtin",
                    "-fno-stack-protector",
                    "-fno-unwind-tables",
                    "-fno-asynchronous-unwind-tables",
                    "-ffp-contract=off",
                    "-DNDEBUG",
                    "-DBROTLI_BUILD_PORTABLE",
                    "-DBROTLI_BUILD_ENC_EXTRA_API",
                    "-DBROTLI_HAVE_LOG2=1",
                    "-I",
                ])
                .arg(runtime().join("include"))
                .arg("-I")
                .arg(nitro.join("brotli/c/include"))
                .arg("-c")
                .arg(source)
                .arg("-o")
                .arg(&object))?;
            objects.push(object);
        }
        let archive = folder.join(format!("libbrotli{group}-static.a"));
        if archive.exists() {
            fs::remove_file(&archive)?;
        }
        run(Command::new(ar).arg("rcs").arg(&archive).args(&objects))?;
        let members = text(Command::new(ar).arg("t").arg(&archive))?;
        let expected: Vec<_> =
            objects.iter().map(|p| p.file_name().unwrap().to_string_lossy()).collect();
        anyhow::ensure!(
            members.lines().eq(expected.iter().map(|s| s.as_ref())),
            "unexpected members in {}",
            archive.display()
        );
    }
    Ok(folder)
}

fn support_wasm(nitro: &Path, out: &Path, clang: &Path, ar: &Path) -> Result<PathBuf> {
    let wasm = out.join("wasm");
    fs::create_dir_all(&wasm)?;
    let libraries = brotli_archives(nitro, out, "wasm", clang, ar)?;
    let flags = format!(
        "-C target-cpu=mvp -C debuginfo=0 -C strip=debuginfo -L native={}",
        libraries.display()
    );
    run(cargo(out, &wasm, &flags, "1.93.0")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(nitro.join("Cargo.toml")))?;
    check_lock(
        &fs::read_to_string(out.join("canonical.Cargo.lock"))?,
        &fs::read_to_string(nitro.join("Cargo.lock"))?,
    )?;
    run(cargo(out, &wasm, &flags, "1.93.0")
        .args(["build", "--locked", "--release", "--manifest-path"])
        .arg(nitro.join("Cargo.toml"))
        .args([
            "--target",
            "wasm32-wasip1",
            "-p",
            "user-host",
            "-p",
            "program-exec",
            "--features",
            "user-host/native-activation",
        ]))?;
    for name in ["user_host", "program_exec"] {
        fs::copy(
            out.join(format!("cargo/wasm32-wasip1/release/{name}.wasm")),
            wasm.join(format!("{name}.wasm")),
        )?;
    }
    run(cargo(out, &wasm, &flags, "1.93.0")
        .args(["build", "--locked", "--release", "--manifest-path"])
        .arg(nitro.join("Cargo.toml"))
        .args(["--target", "wasm32-unknown-unknown", "-p", "wasi-stub"]))?;
    fs::copy(
        out.join("cargo/wasm32-unknown-unknown/release/wasi_stub.wasm"),
        wasm.join("wasi_stub.wasm"),
    )?;
    let generator = out.join("generator");
    fs::create_dir_all(generator.join("src"))?;
    fs::write(generator.join("Cargo.toml"), "[package]\nname=\"nitro-forward-generator\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[workspace]\n[dependencies]\nforward={path=\"../nitro/crates/wasm-libraries/forward\"}\nwat=\"=1.245.1\"\n")?;
    fs::copy(runtime().join("stylus/generator.Cargo.lock"), generator.join("Cargo.lock"))?;
    fs::write(generator.join("src/main.rs"), "fn main() { let mut wat = Vec::new(); forward::forward(&mut wat).unwrap(); let wasm = wat::parse_bytes(&wat).unwrap(); std::fs::write(std::env::args().nth(1).unwrap(), wasm).unwrap(); }\n")?;
    run(cargo(out, &wasm, "", "1.93.0")
        .args(["run", "--locked", "--release", "--manifest-path"])
        .arg(generator.join("Cargo.toml"))
        .arg("--")
        .arg(wasm.join("forward.wasm")))?;
    let soft = nitro.join("crates/wasm-libraries/soft-float");
    let build = soft.join("SoftFloat/build/Wasm-Clang");
    let compile = format!("{} -c -Werror-implicit-function-declaration -DSOFTFLOAT_FAST_INT64 --target=wasm32-unknown-unknown -DSOFTFLOAT_ROUND_ODD -DINLINE_LEVEL=5 -DSOFTFLOAT_FAST_DIV32TO16 -DSOFTFLOAT_FAST_DIV64TO32 -I. -I../../source/8086 -I../../source/include -O2 -o $@", shell_quote(clang));
    run(Command::new("make")
        .args([
            "-j4",
            &format!("COMPILE_C={compile}"),
            &format!("MAKELIB={} crs $@", shell_quote(ar)),
        ])
        .current_dir(&build))?;
    let mut objects = sources(&build, "o")?;
    let mut bindings = String::new();
    for name in ["bindings32.c", "bindings64.c"] {
        let source = soft.join(name);
        let object = source.with_extension("o");
        run(Command::new(clang)
            .args(["--target=wasm32-unknown-unknown", "-O2", "-I"])
            .arg(soft.join("SoftFloat/source/include"))
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object))?;
        objects.push(object);
        bindings.push_str(&fs::read_to_string(source)?);
    }
    let regex = Regex::new(r"\b(wavm__[a-zA-Z0-9_]+)\s*\(")?;
    let exports: BTreeSet<_> = regex.captures_iter(&bindings).map(|c| c[1].to_owned()).collect();
    run(Command::new(tool("NITRO_WASM_LD", &["wasm-ld", "/opt/homebrew/opt/llvm/bin/wasm-ld"])?)
        .args(["--no-entry", "-o"])
        .arg(wasm.join("soft-float.wasm"))
        .args(objects)
        .args(exports.iter().map(|name| format!("--export={name}"))))?;
    // Normalize reference types and padded table-index LEBs as Nitro's own build does.
    for name in LIBRARIES {
        let module = wasm.join(format!("{name}.wasm"));
        let wat = module.with_extension("wat");
        run(Command::new("wasm2wat").arg(&module).arg("-o").arg(&wat))?;
        run(Command::new("wat2wasm").arg(wat).arg("-o").arg(module))?;
    }
    Ok(wasm)
}

// Make expands this command before passing it to a shell.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('$', "$$").replace('\'', "'\\''"))
}

pub fn build(args: Args) -> Result<()> {
    let out = output_dir(args.out.clone(), "nitro-stylus")?;
    println!("Archiving pinned Nitro and support sources");
    let nitro = prepare(&args, &out)?;
    let here = runtime().join("stylus");
    let mut metadata = json!({"nitro": NITRO, "wasmer": WASMER, "brotli": build::BROTLI, "softfloat": SOFTFLOAT,
        "target_lexicon": TARGET_LEXICON, "sha3": SHA3, "interpreter_patch_sha256": sha(&here.join("interpreter.patch"))?, "target_lexicon_patch_sha256": sha(&here.join("target-lexicon.patch"))?});
    write_json(&out.join("sources.json"), &metadata)?;
    if args.prepare_only {
        return Ok(());
    }
    for chain in ["1.93.0", "zisk"] {
        run(Command::new("rustup").args(["run", chain, "rustc", "--version"]))?;
    }
    let clang = tool("NITRO_CC", &["/opt/homebrew/opt/llvm/bin/clang", "clang"])?;
    let ar = tool("NITRO_AR", &["llvm-ar", "/opt/homebrew/opt/llvm/bin/llvm-ar"])?;
    println!("Building canonical support Wasm libraries");
    let wasm = support_wasm(&nitro, &out, &clang, &ar)?;
    let helper = out.join("helper");
    fresh_dir(&helper)?;
    fs::create_dir_all(helper.join("src"))?;
    for source in sources(&here.join("src"), "rs")? {
        fs::copy(&source, helper.join("src").join(source.file_name().unwrap()))?;
    }
    for name in ["Cargo.toml", "bridge.wat", "build.rs", "Cargo.lock"] {
        fs::copy(here.join(name), helper.join(name))?;
    }
    if args.test {
        println!("Running Stylus host controls");
        let result = run(cargo(&out, &wasm, "", "1.93.0")
            .args(["test", "--locked", "--release", "--manifest-path"])
            .arg(helper.join("Cargo.toml")))?;
        print!("{}", String::from_utf8_lossy(&result.stdout));
    }
    let native = brotli_archives(&nitro, &out, "riscv", &clang, &ar)?;
    let flags = format!(
        "-C target-feature=-c -C code-model=medium -C relocation-model=static -L native={}",
        native.display()
    );
    println!("Building the RISC-V Stylus static library");
    run(cargo(&out, &wasm, &flags, "zisk")
        .args(["build", "--locked", "--release", "--lib", "--target", TARGET, "--manifest-path"])
        .arg(helper.join("Cargo.toml")))?;
    let library = out.join(format!("cargo/{TARGET}/release/libnitro_stylus_runtime.a"));
    metadata["library_sha256"] = sha(&library)?.into();
    let mut digests = serde_json::Map::new();
    for name in LIBRARIES {
        digests.insert(name.into(), sha(&wasm.join(format!("{name}.wasm")))?.into());
    }
    metadata["wasm_sha256"] = digests.into();
    metadata["rustc_zisk"] = text(Command::new("rustc").args(["+zisk", "-vV"]))?.into();
    metadata["rustc_wasm"] = text(Command::new("rustc").args(["+1.93.0", "-vV"]))?.into();
    metadata["clang"] = text(Command::new(&clang).arg("--version"))?.into();
    metadata["wasm2wat"] = text(Command::new("wasm2wat").arg("--version"))?.into();
    metadata["wat2wasm"] = text(Command::new("wat2wasm").arg("--version"))?.into();
    metadata["helper_lock_sha256"] = sha(&helper.join("Cargo.lock"))?.into();
    metadata["support_lock_sha256"] = sha(&nitro.join("Cargo.lock"))?.into();
    write_json(&out.join("build.json"), &metadata)?;
    println!("{}", library.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pruning_cannot_upgrade_registry_versions() {
        let canonical = "[[package]]\nname='test'\nversion='1.0.0'\n";
        let selected = format!("{canonical}source='registry+https://example.com'\n");
        assert!(check_lock(canonical, &selected).is_ok());
        assert!(check_lock(canonical, &selected.replace("1.0.0", "1.0.1")).is_err());
    }
    #[test]
    fn source_edits_reject_missing_or_ambiguous_declarations() {
        assert_eq!(replace_once("a=1", "a=1", "a=2").unwrap(), "a=2");
        assert!(replace_once("a=1 a=1", "a=1", "a=2").is_err());
        assert!(replace_once("a=2", "a=1", "a=2").is_err());
    }
}
