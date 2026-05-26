# Inherits the pinned riscv64 toolchain from lib-float/c/Dockerfile.build.
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive
ENV container=docker

ARG CUDA=false

RUN apt update && apt install -y \
    bash \
    sudo \
    curl \
    gnupg \
    systemd \
    systemd-sysv \
    dbus

COPY utils.sh install_deps.sh /tmp/

RUN groupadd --gid 1000 ziskuser \
 && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash ziskuser \
 && echo "ziskuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

RUN chmod +x /tmp/install_deps.sh \
 && cd /tmp/ \
 && if [ "$CUDA" = "true" ]; then \
      sudo -u ziskuser ./install_deps.sh --cuda; \
    else \
      sudo -u ziskuser ./install_deps.sh; \
    fi \
 && rm -f ./utils.sh ./install_deps.sh

RUN riscv64-unknown-elf-gcc --version | head -1

RUN mkdir -p /home/ziskuser/output \
 && chown ziskuser:ziskuser /home/ziskuser/output

COPY . /home/ziskuser/scripts/

RUN chown -R ziskuser:ziskuser /home/ziskuser \
 && find /home/ziskuser/scripts -type f -exec chmod +x {} \;

WORKDIR /home/ziskuser/scripts

STOPSIGNAL SIGRTMIN+3

CMD ["/sbin/init"]