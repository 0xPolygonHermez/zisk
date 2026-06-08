# Reproducible build environment for ziskfloat.elf / libziskfloat.a
#
# Pinned by digest so the tag `22.04` cannot silently move under us.
# ubuntu:22.04 (jammy) as of this commit:
FROM ubuntu:22.04@sha256:4f838adc7181d9039ac795a7d0aba05a9bd9ecd480d294483169c5def983b64d

# Toolchain versions that actually determine the output bytes. The
# binutils-riscv64-unknown-elf package provides riscv64-unknown-elf-{ar,ld,
# strip,objdump}; gcc-riscv64-unknown-elf provides the compiler. `make` is not
# pinned because it only orchestrates commands and never affects output bytes.
ARG GCC_RISCV_VERSION=10.2.0-0ubuntu1
ARG BINUTILS_RISCV_VERSION=2.35.1-0ubuntu1

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        make \
        binutils-riscv64-unknown-elf=${BINUTILS_RISCV_VERSION} \
        gcc-riscv64-unknown-elf=${GCC_RISCV_VERSION} && \
    rm -rf /var/lib/apt/lists/*

# Neutralize any timestamp gcc/ar/ld might otherwise embed. The host build.rs
# also exports this, and `make docker` forwards it, but set a default here too
# so a manual `docker run` is reproducible as well.
ENV SOURCE_DATE_EPOCH=0

WORKDIR /work
