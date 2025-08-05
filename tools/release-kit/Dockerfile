FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Install base packages
RUN apt update && apt install -y bash sudo curl gnupg

# Copy helper scripts to /tmp
COPY utils.sh install_deps.sh /tmp/

# Create a non-root user with sudo privileges
RUN groupadd --gid 1000 ziskuser \
 && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash ziskuser \
 && echo "ziskuser ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Patch install_deps.sh to source utils.sh from /tmp, then run it as ziskuser
RUN chmod +x /tmp/install_deps.sh \
 #&& sed -i 's|source ./utils.sh|source /tmp/utils.sh|' /tmp/install_deps.sh \
 && cd /tmp/ \
 && sudo -u ziskuser ./install_deps.sh \
 && rm -f ./utils.sh ./install_deps.sh

# Create output directory and set ownership
RUN mkdir -p /home/ziskuser/output \
 && chown ziskuser:ziskuser /home/ziskuser/output

# Copy project files and adjust permissions
COPY . /home/ziskuser/zisk-release-kit/
RUN chown -R ziskuser:ziskuser /home/ziskuser \
 && find /home/ziskuser/zisk-release-kit -type f -exec chmod +x {} \;

# Switch to the non-root user and set working directory
USER ziskuser
WORKDIR /home/ziskuser/zisk-release-kit

