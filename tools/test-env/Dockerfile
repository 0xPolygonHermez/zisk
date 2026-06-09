FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV container=docker

# Define build-time argument
ARG CUDA=false

# Install base packages
#
# docker.io provides the Docker Engine (daemon + CLI) so that builds run inside
# this container can themselves invoke `docker` (e.g. lib-float/build.rs runs
# `make docker` to regenerate ziskfloat.elf reproducibly). This is real
# Docker-in-Docker: the daemon runs alongside systemd (PID 1) and shares this
# container's filesystem, so bind mounts like `-v $(CURDIR):/work` resolve
# correctly. It only works because the container is started --privileged.
RUN apt update && apt install -y \
    bash \
    sudo \
    curl \
    gnupg \
    ca-certificates \
    systemd \
    systemd-sysv \
    dbus \
    docker.io \
 && systemctl enable docker.service

# Force the `vfs` storage driver for the in-container Docker daemon. Nested
# overlay2 (overlay-on-overlay) is unreliable in Docker-in-Docker, whereas vfs
# always works. The performance/disk cost is negligible here because the only
# image ever built inside the container is the small lib-float builder.
COPY daemon.json /etc/docker/daemon.json

# Copy helper scripts to /tmp
COPY utils.sh install_deps.sh /tmp/

# Create a non-root user with sudo privileges. Add it to the `docker` group
# (created by the docker.io package above) so `build_zisk.sh`, which runs as
# ziskuser, can talk to the Docker daemon without sudo.
RUN groupadd --gid 1000 ziskuser \
 && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash ziskuser \
 && echo "ziskuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers \
 && usermod -aG docker ziskuser

# Patch install_deps.sh to source utils.sh from /tmp, then run it as ziskuser
RUN chmod +x /tmp/install_deps.sh \
 && cd /tmp/ \
 && if [ "$CUDA" = "true" ]; then \
      sudo -u ziskuser ./install_deps.sh --cuda; \
    else \
      sudo -u ziskuser ./install_deps.sh; \
    fi \
 && rm -f ./utils.sh ./install_deps.sh

# Create output directory and set ownership
RUN mkdir -p /home/ziskuser/output \
 && chown ziskuser:ziskuser /home/ziskuser/output

# Copy project files and adjust permissions
COPY . /home/ziskuser/scripts/

RUN chown -R ziskuser:ziskuser /home/ziskuser \
 && find /home/ziskuser/scripts -type f -exec chmod +x {} \;

WORKDIR /home/ziskuser/scripts

STOPSIGNAL SIGRTMIN+3

CMD ["/sbin/init"]