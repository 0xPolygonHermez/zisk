# RunPod ZisK Cluster Test

Tooling to spin up a 2-pod ZisK proving cluster on [RunPod](https://runpod.io) and run the ZisK Ethproofs test end to end.

## Overview

The deployment is split into independent steps:

| # | Command | Where | What it does |
|---|---------|-------|--------------|
| 1 | `build_docker.sh` | local | Build the Ubuntu 22.04 + CUDA + SSH image. |
| 2 | `push_docker.sh` | local | Push the image to Docker Hub (`ziskvm/zisk-runpod`). |
| 3 | `build_binaries.sh` | local | Clone + build the zisk binaries into `./work`. |
| 4 | `deploy_pods.py` | local | Create the 2 RunPod pods and wait until reachable (writes `pods.json`). |
| 5 | `deploy_cluster.py` | local | Copy binaries + proving key to the pods and start the cluster. |

Teardown: `terminate_pods.py`. Ad-hoc commands: `run_command.py`.

## Prerequisites

- **`RUNPOD_API_KEY`** exported in your shell (used by `deploy_pods.py`,
  `deploy_cluster.py`, `terminate_pods.py`).
- An **SSH public key** registered in your RunPod account (Settings → SSH Public
  Keys). RunPod injects it into the pods so the scripts can connect. The matching
  private key defaults to `~/.ssh/id_ed25519` (override with `--ssh-key`).
- **Docker** + `docker login` (for steps 1–2).
- Python deps for the scripts: `requests`.

---

## 1. Build the Docker image

```bash
./build_docker.sh
```

Builds `zisk-runpod:latest` from [`Dockerfile`](Dockerfile) (build context is
`../test-env`, which holds `install_deps.sh` + `utils.sh`). The image installs
system deps, Rust, CUDA, `tmux` and an SSH server.

## 2. Push the image to Docker Hub

```bash
docker login
./push_docker.sh
```

Tags and pushes the image as `ziskvm/zisk-runpod:latest` (the default
`--image` used by `deploy_pods.py`).

> Only needed when the image changes (new deps, Dockerfile edits).

## 3. Build the binaries

```bash
./build_binaries.sh --zisk-branch <branch> [--work-dir ./work]
```

Clones and builds into `--work-dir` (default `./work`):

- `zisk` (`zisk-coordinator`, `zisk-worker`, `cargo-zisk`)
- `zisk-ethproofs` (`ethproofs-client`)
- `zisk-eth-client` (`zec-reth` guest ELF)

The `zisk-ethproofs` and `zisk-eth-client` branches are read automatically from
`<work-dir>/zisk/Cargo.toml` (`gha_zisk_ethproofs_branch` /
`gha_zisk_eth_client_branch`).

## 4. Deploy the pods

```bash
python3 deploy_pods.py
```

Creates two pods (`zisk-cluster-test-1`, `zisk-cluster-test-2`) with 2 GPUs each
from the `ziskvm/zisk-runpod:latest` image, waits until their ports and SSH are
ready, and writes connection info to `pods.json`.

Useful options: `--image`, `--name-prefix`, `--cloud-type`, `--container-disk-gb`,
`--volume-gb`, `--timeout`, `--output`. On any error (or Ctrl+C) the created pods
are **terminated automatically** to avoid costs; use `--keep-on-error` to keep
them for debugging.

## 5. Provision and start the cluster

```bash
python3 deploy_cluster.py [--work-dir ./work]
```

Reads `pods.json` and, in parallel across pods (sequential within each pod),
streaming logs prefixed with `[pod1]` / `[pod2]`:

1. Copies the binaries from `--work-dir` to `/workspace/bin` on each pod
   (pod 1: coordinator, cargo-zisk, ethproofs-client, zec-reth · pod 2: worker).
2. Downloads and extracts the proving key into `/workspace`.
3. Starts, in `tmux`, `zisk-coordinator` (pod 1) and `zisk-worker` (both pods).
4. Runs `ethproofs-client` on pod 1 in the foreground (streaming its logs);
   `--exit-on-error` makes it fail fast.

Pod 1's worker connects to the coordinator locally (`127.0.0.1:50051`); pod 2's
worker connects to pod 1's external IP and mapped port.

Useful options: `--provingkey`, `--rpc-http-url`, `--rpc-ws-url`, `--ssh-key`,
`--keep-on-error`. As with `deploy_pods.py`, any error terminates the pods
unless `--keep-on-error` is set.

---

## Teardown

Terminate all pods listed in `pods.json`:

```bash
python3 terminate_pods.py
```

## Running ad-hoc commands

Run a command over SSH on the pods from `pods.json` (in parallel, logs grouped
per pod):

```bash
python3 run_command.py -- "tmux ls"
python3 run_command.py --pod-id <ID> -- "tmux capture-pane -pt coord -S -100"
```

## Files

- [`Dockerfile`](Dockerfile) — pod image (Ubuntu 22.04 + deps + CUDA + SSH + tmux).
- [`build_docker.sh`](build_docker.sh) / [`push_docker.sh`](push_docker.sh) — build/push the image.
- [`build_binaries.sh`](build_binaries.sh) — build the zisk binaries into `./work`.
- [`deploy_pods.py`](deploy_pods.py) — create the pods (writes `pods.json`).
- [`deploy_cluster.py`](deploy_cluster.py) — provision + start the cluster.
- [`terminate_pods.py`](terminate_pods.py) — terminate the pods.
- [`run_command.py`](run_command.py) — run ad-hoc SSH commands on the pods.
