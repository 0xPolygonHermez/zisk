#!/usr/bin/env python3
"""Build the pinned freestanding Nitro helper; outputs stay under target/."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BROTLI_REV = "f4153a09f87cbb9c826d8fc12c74642bb2d879ea"
BROTLI_URL = f"https://codeload.github.com/google/brotli/tar.gz/{BROTLI_REV}"
BROTLI_SHA = "6d6cacce05086b7debe75127415ff9c3661849f564fe2f5f3b0383d48aa4ed77"
LOG_URL = "https://git.musl-libc.org/cgit/musl/plain/src/math/log2.c?h=v1.1.19"
LOG_SHA = "f80ab2145f156b8a986cd1303b9753d10631e75a03502a7067c4a38c4849484e"

def run(args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)

def sha(data):
    return hashlib.sha256(data).hexdigest()

def download(url, digest):
    data = urllib.request.urlopen(url, timeout=60).read()
    if sha(data) != digest:
        raise SystemExit(f"source checksum mismatch: {url}")
    return data

def source(out, repo):
    directory = out / "brotli"
    # Re-extract the pinned archive so locally edited cache files cannot enter a build.
    archive = out / "brotli.tar.gz"
    if repo:
        revision = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
        dirty = subprocess.check_output(["git", "-C", str(repo), "status", "--porcelain", "--untracked-files=all"])
        if revision != BROTLI_REV or dirty:
            raise SystemExit("--brotli-source must be the exact clean pinned Brotli commit")
        data = subprocess.check_output(["git", "-C", str(repo), "archive", BROTLI_REV])
    else:
        if not archive.exists():
            archive.write_bytes(download(BROTLI_URL, BROTLI_SHA))
        data = archive.read_bytes()
        if sha(data) != BROTLI_SHA:
            raise SystemExit("cached Brotli archive checksum mismatch")
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir()
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        for item in tar.getmembers():
            if not repo:
                item.name = "/".join(item.name.split("/")[1:])
            if item.name:
                tar.extract(item, directory, filter="data")
    return directory

def find_tool(env, candidates):
    if os.getenv(env):
        return os.environ[env]
    for tool in candidates:
        path = shutil.which(tool)
        if path:
            return path
    raise SystemExit(f"set {env} to an LLVM tool with the RISC-V backend")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--brotli-source", type=Path, help="clean git checkout at pinned commit (otherwise downloads verified archive)")
    parser.add_argument("--out", type=Path, default=ROOT / "target/nitro-runtime")
    parser.add_argument("--test", action="store_true", help="run native helper controls before cross-compiling")
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    brotli = source(out, args.brotli_source)
    sources = sorted((brotli / "c/common").glob("*.c")) + sorted((brotli / "c/dec").glob("*.c")) + sorted((brotli / "c/enc").glob("*.c"))
    include = brotli / "c/include"
    log = out / "log2.c"
    if not log.exists():
        log.write_bytes(download(LOG_URL, LOG_SHA))
    if sha(log.read_bytes()) != LOG_SHA:
        raise SystemExit("cached musl log2 checksum mismatch")
    if args.test:
        native = [os.getenv("HOST_CC", "cc"), "-O2", "-ffp-contract=off", "-DNDEBUG", "-DNITRO_RUNTIME_HOST_TEST", "-I", HERE, "-I", include]
        native += [HERE / "brotli-tests.c", HERE / "brotli.c", HERE / "libc.c", *sources, log, "-lm", "-o", out / "brotli-tests"]
        run(native)
        run([out / "brotli-tests"])
    cc = find_tool("NITRO_CC", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = find_tool("NITRO_LD", ["ld.lld", "/opt/homebrew/opt/llvm/bin/ld.lld"])
    flags = ["--target=riscv64-unknown-elf", "-march=rv64imfd", "-mabi=lp64", "-mcmodel=medany", "-mno-relax", "-msmall-data-limit=0", "-O2", "-ffreestanding", "-fno-builtin", "-fno-stack-protector", "-fno-unwind-tables", "-fno-asynchronous-unwind-tables", "-ffp-contract=off", "-ffunction-sections", "-fdata-sections", "-DNDEBUG", "-DBROTLI_BUILD_PORTABLE", "-I", ROOT / "ziskasm/lang/c/include", "-I", HERE / "include", "-I", HERE, "-I", include, f"-ffile-prefix-map={ROOT}=."]
    runtime_sources = [HERE / name for name in ("runtime.c", "state.c", "brotli.c", "libc.c")]
    objects = []
    for i, src in enumerate([*runtime_sources, *sources, log]):
        obj = out / f"{i:02d}-{src.stem}.o"
        run([cc, *flags, "-c", src, "-o", obj])
        objects.append(obj)
    thunk = out / "zkvm_calls.o"
    run([cc, "--target=riscv64-unknown-elf", "-march=rv64imfd_zicsr", "-mabi=lp64", "-mno-relax", "-c", ROOT / "ziskasm/lang/c/src/zkvm_calls.s", "-o", thunk])
    objects.append(thunk)
    elf = out / "nitro-runtime.elf"
    run([linker, "--no-relax", "--gc-sections", "-T", HERE / "runtime.ld", "-Map", out / "nitro-runtime.map", "-o", elf, *objects])
    metadata = {"brotli_revision": BROTLI_REV, "musl_log2_sha256": LOG_SHA, "clang": subprocess.check_output([cc, "--version"], text=True).splitlines()[0], "ld": subprocess.check_output([linker, "--version"], text=True).strip(), "target": "riscv64-unknown-elf", "march": "rv64imfd", "abi": "lp64", "elf_sha256": sha(elf.read_bytes())}
    (out / "build.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))
    print(elf)

if __name__ == "__main__":
    main()
