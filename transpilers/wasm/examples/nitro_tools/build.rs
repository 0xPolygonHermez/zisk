use super::*;
use serde_json::json;

pub const BROTLI: &str = "f4153a09f87cbb9c826d8fc12c74642bb2d879ea";
const BROTLI_SHA: &str = "6d6cacce05086b7debe75127415ff9c3661849f564fe2f5f3b0383d48aa4ed77";
const LOG_SHA: &str = "f80ab2145f156b8a986cd1303b9753d10631e75a03502a7067c4a38c4849484e";
const DICTIONARY_SHA: &str = "9681d04f40f0960dbc44f57fdd523ad5b829b43b9f65878fb9e80f964e273672";

#[derive(clap::Args)]
pub struct Args {
    /// Clean Git checkout at the pinned commit; otherwise download a verified archive.
    #[arg(long)]
    brotli_source: Option<PathBuf>,
    #[arg(long)]
    out: Option<PathBuf>,
    /// Run native codec controls before cross-compiling.
    #[arg(long)]
    test: bool,
    #[arg(long)]
    stylus_library: Option<PathBuf>,
}

fn download(path: &Path, url: &str, digest: &str) -> Result<()> {
    if !path.exists() {
        let temporary = path.with_extension("download");
        run(Command::new("curl")
            .args(["--fail", "--location", "--retry", "3", "--max-time", "60", url, "--output"])
            .arg(&temporary))?;
        check_sha(&temporary, digest)?;
        fs::rename(temporary, path)?;
    }
    check_sha(path, digest)
}

fn brotli_source(out: &Path, repo: Option<&Path>) -> Result<PathBuf> {
    let directory = out.join("brotli");
    fresh_dir(&directory)?;
    if let Some(repo) = repo {
        let revision = text(Command::new("git").arg("-C").arg(repo).args(["rev-parse", "HEAD"]))?;
        let dirty = text(Command::new("git").arg("-C").arg(repo).args([
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]))?;
        anyhow::ensure!(
            revision == BROTLI && dirty.is_empty(),
            "--brotli-source must be the exact clean pinned Brotli commit"
        );
        git_archive(repo, BROTLI, &directory)?;
    } else {
        let archive = out.join("brotli.tar.gz");
        download(
            &archive,
            &format!("https://codeload.github.com/google/brotli/tar.gz/{BROTLI}"),
            BROTLI_SHA,
        )?;
        run(Command::new("tar")
            .arg("-xf")
            .arg(archive)
            .args(["--strip-components=1", "-C"])
            .arg(&directory))?;
    }
    Ok(directory)
}

pub fn build(args: Args) -> Result<()> {
    let here = runtime();
    let root = root();
    let library = args.stylus_library.unwrap_or_else(|| {
        root.join(
            "target/nitro-stylus/cargo/riscv64ima-zisk-zkvm-elf/release/libnitro_stylus_runtime.a",
        )
    });
    let library =
        library.canonicalize().context("missing Stylus library; run nitro_tools stylus first")?;
    let out = output_dir(args.out, "nitro-runtime")?;
    let dictionary = here.join("stylus-dictionary/stylus-program-11.lz");
    check_sha(&dictionary, DICTIONARY_SHA)?;
    let bytes = fs::read(dictionary)?;
    let rows: Vec<String> = bytes
        .chunks(32)
        .map(|row| row.iter().map(u8::to_string).collect::<Vec<_>>().join(","))
        .collect();
    fs::write(
        out.join("stylus-dictionary.h"),
        format!("static const uint8_t stylus_dictionary[] = {{\n{}\n}};\n", rows.join(",\n")),
    )?;
    let brotli = brotli_source(&out, args.brotli_source.as_deref())?;
    let mut codec = Vec::new();
    for group in ["common", "dec", "enc"] {
        codec.extend(sources(&brotli.join("c").join(group), "c")?);
    }
    let include = brotli.join("c/include");
    let log = out.join("log2.c");
    download(&log, "https://git.musl-libc.org/cgit/musl/plain/src/math/log2.c?h=v1.1.19", LOG_SHA)?;
    if args.test {
        run(Command::new(std::env::var_os("HOST_CC").unwrap_or_else(|| "cc".into()))
            .args(["-O2", "-ffp-contract=off", "-DNDEBUG", "-DNITRO_RUNTIME_HOST_TEST", "-I"])
            .arg(&here)
            .arg("-I")
            .arg(&include)
            .arg("-I")
            .arg(&out)
            .args([here.join("brotli-tests.c"), here.join("brotli.c"), here.join("libc.c")])
            .args(&codec)
            .arg(&log)
            .args(["-lm", "-o"])
            .arg(out.join("brotli-tests")))?;
        run(&mut Command::new(out.join("brotli-tests")))?;
        println!("Native Brotli controls passed");
    }
    let cc = tool("NITRO_CC", &["/opt/homebrew/opt/llvm/bin/clang", "clang"])?;
    let ld = tool("NITRO_LD", &["ld.lld", "/opt/homebrew/opt/llvm/bin/ld.lld"])?;
    let flags = [
        "--target=riscv64-unknown-elf",
        "-march=rv64imfd",
        "-mabi=lp64",
        "-mcmodel=medany",
        "-mno-relax",
        "-msmall-data-limit=0",
        "-O2",
        "-ffreestanding",
        "-fno-builtin",
        "-fno-stack-protector",
        "-fno-unwind-tables",
        "-fno-asynchronous-unwind-tables",
        "-ffp-contract=off",
        "-ffunction-sections",
        "-fdata-sections",
        "-DNDEBUG",
        "-DBROTLI_BUILD_PORTABLE",
    ];
    let mut all: Vec<_> =
        ["runtime.c", "state.c", "brotli.c", "libc.c"].iter().map(|name| here.join(name)).collect();
    all.extend(codec);
    all.push(log);
    let mut objects = Vec::new();
    for (i, source) in all.iter().enumerate() {
        let object =
            out.join(format!("{i:02}-{}.o", source.file_stem().unwrap().to_string_lossy()));
        run(Command::new(&cc)
            .args(flags)
            .arg("-I")
            .arg(root.join("ziskasm/lang/c/include"))
            .arg("-I")
            .arg(here.join("include"))
            .arg("-I")
            .arg(&here)
            .arg("-I")
            .arg(&include)
            .arg(format!("-ffile-prefix-map={}=.", root.display()))
            .arg("-I")
            .arg(&out)
            .args(["-DBROTLI_BUILD_ENC_EXTRA_API", "-DBROTLI_HAVE_LOG2=1", "-c"])
            .arg(source)
            .arg("-o")
            .arg(&object))?;
        objects.push(object);
    }
    let thunk = out.join("zkvm_calls.o");
    run(Command::new(&cc)
        .args([
            "--target=riscv64-unknown-elf",
            "-march=rv64imfd_zicsr",
            "-mabi=lp64",
            "-mno-relax",
            "-c",
        ])
        .arg(root.join("ziskasm/lang/c/src/zkvm_calls.s"))
        .arg("-o")
        .arg(&thunk))?;
    objects.push(thunk);
    let elf = out.join("nitro-runtime.elf");
    run(Command::new(&ld)
        .args(["--no-relax", "--gc-sections", "-T"])
        .arg(here.join("runtime.ld"))
        .arg("-Map")
        .arg(out.join("nitro-runtime.map"))
        .arg("-o")
        .arg(&elf)
        .args(objects)
        .arg(&library))?;
    write_json(
        &out.join("build.json"),
        &json!({
            "brotli_revision": BROTLI, "musl_log2_sha256": LOG_SHA,
            "clang": text(Command::new(&cc).arg("--version"))?.lines().next().unwrap_or(""),
            "ld": text(Command::new(&ld).arg("--version"))?,
            "target": "riscv64-unknown-elf", "march": "rv64imfd", "abi": "lp64",
            "elf_sha256": sha(&elf)?,
            "stylus_dictionary_revision": "62f0625ad9bdb4e1ff42f08688429fc33618f181",
            "stylus_dictionary_sha256": DICTIONARY_SHA, "stylus_library_sha256": sha(&library)?
        }),
    )?;
    println!("{}", elf.display());
    Ok(())
}
