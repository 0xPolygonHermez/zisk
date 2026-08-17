# Ubuntu 22.04 image for RunPod that installs ZisK dependencies (with GPU/CUDA
# support) by running install_deps.sh --cuda, and exposes SSH.
#
# The build context must be tools/test-env (where utils.sh + install_deps.sh
# live), while this Dockerfile lives in tools/runpod-test.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
# utils.sh derives the shell profile from $SHELL.
ENV SHELL=/bin/bash

# Minimal prerequisites; install_deps.sh installs everything else itself.
# openssh-server is needed so RunPod pods are reachable over SSH.
RUN apt-get update && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        curl \
        wget \
        sudo \
        openssh-server \
        tmux \
    && rm -rf /var/lib/apt/lists/*

# Allow key-based root login over SSH.
RUN mkdir -p /var/run/sshd \
 && sed -i 's/#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config

# install_deps.sh sources ./utils.sh, so both must sit in the working directory.
# These paths are relative to the build context (tools/test-env).
WORKDIR /tmp
COPY utils.sh install_deps.sh ./

# Install all dependencies, including CUDA/GPU support.
RUN chmod +x install_deps.sh && ./install_deps.sh --cuda

# Make Rust/Cargo and CUDA available in interactive shells.
ENV PATH="/root/.cargo/bin:/usr/local/cuda/bin:${PATH}"

# Startup script: inject the SSH public key RunPod passes in $PUBLIC_KEY,
# start sshd, and keep the container alive.
RUN printf '%s\n' \
        '#!/bin/bash' \
        'set -e' \
        'mkdir -p /root/.ssh && chmod 700 /root/.ssh' \
        'if [ -n "$PUBLIC_KEY" ]; then' \
        '  echo "$PUBLIC_KEY" >> /root/.ssh/authorized_keys' \
        '  chmod 600 /root/.ssh/authorized_keys' \
        'fi' \
        '# Generate host keys if missing and start the SSH daemon.' \
        'ssh-keygen -A' \
        '/usr/sbin/sshd' \
        '# Keep the container (and therefore the pod) running.' \
        'sleep infinity' \
        > /start.sh \
 && chmod +x /start.sh

# SSH (22), cluster coordinator (50051), zisk-coordinator (7000), metrics (9090).
EXPOSE 22 50051 7000 9090

CMD ["/start.sh"]
