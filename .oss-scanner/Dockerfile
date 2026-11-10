# Build environment for Anthropic OSS Scanner (https://github.com/anthropics/oss-scanner).
#
# Self-contained version of tools/test-env/Dockerfile: the image is built with `docker build`
# (no systemd, no Docker daemon, no GPU). The scanner uses the checkout being scanned as the
# build context, so that checkout is copied to /src and built there with the test-env scripts.
#
# Network is only available while this file is built: everything the audit needs (crates,
# toolchains) is fetched here. The proving key is not installed: proving does not fit in the
# analysis environment and is not needed to audit the code.

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    bash \
    sudo \
    curl \
    gnupg \
    ca-certificates \
    git \
 && rm -rf /var/lib/apt/lists/*

# Non-root user with sudo privileges, as in tools/test-env/Dockerfile. /src and /pil2-proofman are
# created here because ziskuser cannot create directories in /.
RUN groupadd --gid 1000 ziskuser \
 && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash ziskuser \
 && echo "ziskuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers \
 && mkdir /src /pil2-proofman \
 && chown ziskuser:ziskuser /src /pil2-proofman

USER ziskuser

# utils.sh needs SHELL to pick the shell profile. The scanner runs as root with HOME=/root, so the
# Rust and ZisK locations are set explicitly instead of relying on ~/.bashrc or $HOME.
# ZISK_REPO_DIR makes the test-env scripts use /src instead of cloning ZisK.
ENV SHELL=/bin/bash \
    CARGO_HOME=/home/ziskuser/.cargo \
    RUSTUP_HOME=/home/ziskuser/.rustup \
    PATH=/home/ziskuser/.cargo/bin:/home/ziskuser/.zisk/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    PILFFLONK_JS=/home/ziskuser/.zisk/zisk/pilfflonk-js \
    ZISK_REPO_DIR=/src \
    ONLY_CPU=1 \
    DISABLE_ENV_CONFIRM=1 \
    ZISK_CI_NO_DOCKER_REBUILD=1

# System packages, Node.js, snarkjs and Rust (no CUDA). Only the scripts it needs are copied first,
# so this layer does not depend on the rest of the checkout. No .env file is created:
# configuration comes from the environment, so load_env never asks for confirmation.
COPY --chown=ziskuser:ziskuser tools/test-env/utils.sh tools/test-env/install_deps.sh /tmp/test-env/
RUN cd /tmp/test-env \
 && ./install_deps.sh \
 && sudo rm -rf /var/lib/apt/lists/* /tmp/test-env

# The checkout being scanned (.oss-scanner/Dockerfile.dockerignore keeps .git and the docs)
COPY --chown=ziskuser:ziskuser . /src

# pil2-proofman is part of what is audited. Clone it at the tag of the version Cargo.lock pins, next
# to /src as in the "Proofman Local development" block of Cargo.toml, and build ZisK against it
# through [patch.crates-io] in $CARGO_HOME/config.toml, so that /src/Cargo.toml stays untouched and
# changes made in /pil2-proofman are rebuilt. Only the crates.io packages in Cargo.lock that the
# clone provides are patched. The build fails if that tag cannot be found, so that pil2-proofman is
# never left out silently.
# Every Cargo.lock that pulls in pil2-proofman is re-resolved here, before anything is built: the
# guest build scripts rerun whenever their workspace's Cargo.lock changes, and a lock rewritten by
# the nested guest build itself would force every later build to rebuild all the guests.
WORKDIR /src
RUN version="$(sed -n '/^name = "proofman"$/{n;s/^version = "\(.*\)"$/\1/p;q}' Cargo.lock)" \
 && repo=https://github.com/0xPolygonHermez/pil2-proofman.git \
 && if git ls-remote --exit-code --tags "${repo}" "refs/tags/v${version}" > /dev/null; then \
      git clone --depth 1 --branch "v${version}" "${repo}" /pil2-proofman \
      && awk -v v="${version}" '/^name = /{n=$3} /^version = /{ver=$3} \
           /^source = "registry/{gsub(/"/,"",n); gsub(/"/,"",ver); if (ver == v) print n}' \
           Cargo.lock > /tmp/proofman-crates \
      && { echo "[patch.crates-io]"; \
           cargo metadata --no-deps --format-version 1 --manifest-path /pil2-proofman/Cargo.toml \
             | jq -r '.packages[] | "\(.name) \(.manifest_path)"' \
             | while read -r name manifest; do \
                 if grep -qx "${name}" /tmp/proofman-crates; then \
                   echo "${name} = { path = \"$(dirname "${manifest}")\" }"; \
                 fi; \
               done; } >> "${CARGO_HOME}/config.toml" \
      && rm /tmp/proofman-crates \
      && cat "${CARGO_HOME}/config.toml" \
      && cargo metadata --format-version 1 \
           | jq -e '[.packages[] | select(.name == "proofman") | .manifest_path | startswith("/pil2-proofman/")] == [true]' \
           > /dev/null \
      && for lock in $(git ls-files '*Cargo.lock'); do \
           if grep -q '^name = "proofman' "${lock}"; then \
             cargo metadata --format-version 1 --manifest-path "$(dirname "${lock}")/Cargo.toml" > /dev/null \
               || echo "WARNING: could not re-resolve ${lock}"; \
           fi; \
         done; \
    else \
      echo "ERROR: pil2-proofman has no tag v${version} (or ${repo} is unreachable)" >&2; \
      exit 1; \
    fi

# Build ZisK with the cpu-only feature, copy the binaries to ~/.zisk/bin and install the ZisK Rust
# toolchain. The test-env scripts source ./utils.sh, so they run from tools/test-env.
WORKDIR /src/tools/test-env
RUN ./build_zisk.sh

# pilfflonk's JS verifier, used by `cargo-zisk verify`: ziskup installs it from the release bundle,
# build_zisk.sh does not. Copy it from the pil2-proofman crate as release.yml does.
WORKDIR /src
RUN crate_dir="$(dirname "$(cargo metadata --format-version 1 \
      | jq -r '.packages[] | select(.name == "proofman-pilfflonk") | .manifest_path')")" \
 && mkdir -p "${PILFFLONK_JS}" \
 && cp -R "${crate_dir}/js/bin" "${crate_dir}/js/src" "${crate_dir}/js/package.json" "${PILFFLONK_JS}/" \
 && test -f "${PILFFLONK_JS}/bin/verify.js"

# Fetch every dependency and compile the workspace tests with the same profile, target and features
# as build_zisk.sh, so the offline audit can build and run them without downloading anything.
# No --locked: patching pil2-proofman to a local path rewrites Cargo.lock.
RUN cargo fetch \
 && (cd examples && cargo fetch) \
 && cargo test --release --target x86_64-unknown-linux-gnu --features cpu-only --no-run

# The scanner works as root with HOME=/root: point root's ~/.zisk to this installation and let git
# operate on a checkout owned by ziskuser.
USER root
RUN ln -s /home/ziskuser/.zisk /root/.zisk \
 && git config --system --add safe.directory '*'
USER ziskuser

# From here on everything has been fetched
ENV CARGO_NET_OFFLINE=true

WORKDIR /src
