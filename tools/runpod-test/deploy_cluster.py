#!/usr/bin/env python3

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

import requests


RUNPOD_API_URL = "https://rest.runpod.io/v1"

SSH_OPTIONS = [
    "-o",
    "StrictHostKeyChecking=no",
    "-o",
    "UserKnownHostsFile=/dev/null",
]

# Coordinator internal port (worker on pod 1 connects locally to this).
COORD_INTERNAL_PORT = 50051

# Remote destinations. $HOME expands inside remote shell / tmux commands; scp
# (SFTP mode) does not expand $HOME, so scp destinations use ~ instead.
REMOTE_ZISK_DIR = "$HOME/.zisk"
REMOTE_ZISK_DIR_SCP = "~/.zisk"
REMOTE_BIN_DIR = "$HOME/.zisk/bin"
REMOTE_PROVING_KEY_DIR = "$HOME/.zisk/provingKey"
REMOTE_LOG_DIR = "/workspace/logs"

# ethproofs-client, zec-reth and mpi_params.sh are copied here from --work-dir.
REMOTE_WORKSPACE_BIN_DIR = "/workspace/bin"

# Wait for each worker to register with the coordinator (by watching its log)
# before starting ethproofs-client.
REGISTRATION_MESSAGE = "Registration accepted: Registration successful"
REGISTRATION_TIMEOUT_SECONDS = 300

DEFAULT_WORK_DIR = "/workspace"
DEFAULT_LOCAL_ZISK_DIR = "~/.zisk"
DEFAULT_PROVING_KEY = "zisk-provingkey-pre-1.0.0-beta.tar.gz"
DEFAULT_RPC_HTTP_URL = "http://144.76.59.84:8545"
DEFAULT_RPC_WS_URL = "ws://144.76.59.84:8546"

# Files copied to /workspace/bin (relative to --work-dir).
MPI_PARAMS_SCRIPT = "zisk/tools/mpi_params.sh"
ETHPROOFS_CLIENT_BIN = "zisk-ethproofs/target/release/ethproofs-client"
ZEC_RETH_BIN = (
    "zisk-eth-client/bin/guests/stateless-validator-reth/"
    "target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth"
)


def runpod_headers(api_key: str) -> dict[str, str]:
    return {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }


def api_request(
    method: str,
    path: str,
    api_key: str,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    response = requests.request(
        method=method,
        url=f"{RUNPOD_API_URL}{path}",
        headers=runpod_headers(api_key),
        json=payload,
        timeout=60,
    )

    try:
        response.raise_for_status()
    except requests.HTTPError:
        print(f"RunPod API error: {response.status_code}", file=sys.stderr)
        print(response.text, file=sys.stderr)
        raise

    if not response.text:
        return {}

    return response.json()


def terminate_pod(api_key: str, pod_id: str) -> None:
    print(f"Terminating pod {pod_id}...")
    api_request("DELETE", f"/pods/{pod_id}", api_key)


def load_pods_info(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Pods info file not found: {path}")

    data = json.loads(path.read_text())
    pods = data.get("pods") if isinstance(data, dict) else data

    if not isinstance(pods, list) or len(pods) < 2:
        raise RuntimeError(f"Expected at least 2 pods in {path}")

    return pods


def ssh_argv(target: dict[str, Any], private_key: str, remote_command: str) -> list[str]:
    return [
        "ssh",
        "-tt",
        "-i",
        private_key,
        "-p",
        str(target["port"]),
        *SSH_OPTIONS,
        f"{target['user']}@{target['host']}",
        remote_command,
    ]


def scp_argv(
    target: dict[str, Any],
    private_key: str,
    local_files: list[Path],
    remote_dir: str,
    recursive: bool = False,
) -> list[str]:
    return [
        "scp",
        *(["-r"] if recursive else []),
        "-i",
        private_key,
        "-P",
        str(target["port"]),
        *SSH_OPTIONS,
        *[str(path) for path in local_files],
        f"{target['user']}@{target['host']}:{remote_dir}/",
    ]


def tar_stream_copy_argv(
    target: dict[str, Any],
    private_key: str,
    local_dir: Path,
    remote_dir: str,
    items: list[str],
) -> list[str]:
    """Copy the given items (relative to local_dir) into remote_dir by streaming
    a gzip'd tar over SSH (one connection, no per-file overhead — much faster
    than scp for many small files). No `ssh -tt`: a pty would corrupt the binary
    stream.
    """
    remote_cmd = f"tar -C {remote_dir} --no-same-owner -xzf -"
    ssh_part = shlex.join(
        [
            "ssh",
            "-i",
            private_key,
            "-p",
            str(target["port"]),
            *SSH_OPTIONS,
            f"{target['user']}@{target['host']}",
            remote_cmd,
        ]
    )

    tar_cmd = ["tar", "-C", str(local_dir), "-czf", "-", *items]
    tar_part = shlex.join(tar_cmd)

    # Show transfer rate/volume with pv when available (measures the compressed
    # bytes going to the network). Falls back to no progress if pv is missing.
    if shutil.which("pv"):
        pv_part = shlex.join(["pv", "-f", "-i", "2", "-b", "-a", "-r", "-t"])
        pipeline = f"{tar_part} | {pv_part} | {ssh_part}"
    else:
        pipeline = f"{tar_part} | {ssh_part}"

    return ["bash", "-c", pipeline]


def run_streaming(command: list[str], prefix: str) -> int:
    """Run a local subprocess, streaming combined output live with a per-pod
    prefix. Returns the exit code.

    Splits on both \\n and \\r so carriage-return progress updates (pv, scp)
    show up live instead of only at the end.
    """
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        stdin=subprocess.DEVNULL,
    )

    assert process.stdout is not None
    fd = process.stdout.fileno()
    buffer = b""

    while True:
        chunk = os.read(fd, 4096)
        if not chunk:
            break
        buffer += chunk
        # Treat CRLF as a single separator (ssh -tt turns \n into \r\n); lone \r
        # (progress updates) and \n are separators too.
        segments = re.split(rb"\r\n|\r|\n", buffer)
        buffer = segments.pop()
        for segment in segments:
            text = segment.decode("utf-8", "replace")
            # Skip blank/whitespace-only lines.
            if not text.strip():
                continue
            print(prefix + text, flush=True)

    if buffer:
        text = buffer.decode("utf-8", "replace")
        if text.strip():
            print(prefix + text, flush=True)

    return process.wait()


def build_pod_steps(
    target: dict[str, Any],
    private_key: str,
    local_zisk_dir: Path,
    workspace_bin_files: list[Path],
    proving_key: str,
    is_coord: bool,
    coord_host: str,
    coord_port: int,
    worker_id: int,
    rpc_http_url: str,
    rpc_ws_url: str,
    run_time: int,
) -> list[dict[str, Any]]:
    """Ordered list of steps (each a subprocess argv) to run on one pod."""
    index = target["index"]
    steps: list[dict[str, Any]] = []

    steps.append(
        {
            "desc": f"Create {REMOTE_LOG_DIR}",
            "cmd": ssh_argv(target, private_key, f"mkdir -p {REMOTE_LOG_DIR}"),
        }
    )
    steps.append(
        {
            "desc": "Kill any running tmux sessions (free the binaries)",
            "cmd": ssh_argv(target, private_key, "tmux kill-server || true"),
        }
    )
    steps.append(
        {
            "desc": "Wait 3s after killing tmux",
            "sleep": 3,
        }
    )
    steps.append(
        {
            "desc": f"Reset {REMOTE_ZISK_DIR}/bin and {REMOTE_ZISK_DIR}/zisk",
            "cmd": ssh_argv(
                target,
                private_key,
                # Replace only the dirs we copy; keep provingKey and cache.
                f"rm -rf {REMOTE_ZISK_DIR}/bin {REMOTE_ZISK_DIR}/zisk && "
                f"mkdir -p {REMOTE_ZISK_DIR}",
            ),
        }
    )
    steps.append(
        {
            "desc": f"Copy local {local_zisk_dir} to {REMOTE_ZISK_DIR} (tar stream)",
            "cmd": tar_stream_copy_argv(
                target,
                private_key,
                local_zisk_dir,
                REMOTE_ZISK_DIR_SCP,
                items=["bin", "zisk"],
            ),
        }
    )
    steps.append(
        {
            "desc": "Install zisk toolchain (cargo-zisk toolchain install)",
            "cmd": ssh_argv(
                target,
                private_key,
                # A non-interactive SSH shell doesn't load ~/.bashrc/profile, so
                # put cargo/rustup (and zisk) on PATH; cargo-zisk shells out to
                # rustup to link the toolchain.
                'export PATH="$HOME/.cargo/bin:$HOME/.zisk/bin:$PATH" && '
                f"{REMOTE_BIN_DIR}/cargo-zisk toolchain install",
            ),
        }
    )
    steps.append(
        {
            "desc": f"Ensure {REMOTE_WORKSPACE_BIN_DIR} exists",
            "cmd": ssh_argv(
                target, private_key, f"mkdir -p {REMOTE_WORKSPACE_BIN_DIR}"
            ),
        }
    )
    names = ", ".join(path.name for path in workspace_bin_files)
    steps.append(
        {
            "desc": f"Copy {names} to {REMOTE_WORKSPACE_BIN_DIR}",
            "cmd": scp_argv(
                target, private_key, workspace_bin_files, REMOTE_WORKSPACE_BIN_DIR
            ),
        }
    )
    steps.append(
        {
            "desc": "Make binaries executable",
            "cmd": ssh_argv(
                target,
                private_key,
                f"chmod +x {REMOTE_WORKSPACE_BIN_DIR}/*",
            ),
        }
    )

    if is_coord:
        steps.append(
            {
                "desc": "Start zisk-coordinator (tmux: coord)",
                "cmd": ssh_argv(
                    target,
                    private_key,
                    "tmux new-session -d -s coord "
                    f'"{REMOTE_BIN_DIR}/zisk-coordinator '
                    f'2>&1 | tee {REMOTE_LOG_DIR}/pod{index}-coord.log"',
                ),
            }
        )

    steps.append(
        {
            "desc": f"Download & extract proving key ({proving_key}) if missing",
            "cmd": ssh_argv(
                target,
                private_key,
                f"if [ -d {REMOTE_PROVING_KEY_DIR} ]; then "
                'echo "Proving key already present, skipping download"; '
                f"else mkdir -p {REMOTE_ZISK_DIR} && cd {REMOTE_ZISK_DIR} && "
                f'curl -L -#o "{proving_key}" '
                f'"https://storage.googleapis.com/zisk-setup/{proving_key}" && '
                f'tar --no-same-owner -xf "{proving_key}" -C {REMOTE_ZISK_DIR}; fi',
            ),
        }
    )

    # Sync: workers only start (and wait for registration) once the coordinator
    # on pod 1 has been launched.
    steps.append(
        {
            "desc": "Wait for coordinator to be started (barrier)",
            "barrier": True,
        }
    )

    steps.append(
        {
            "desc": f"Start zisk-worker under mpirun (tmux: worker) -> {coord_host}:{coord_port}",
            "cmd": ssh_argv(
                target,
                private_key,
                # Source mpi_params.sh (copied to /workspace/bin) to export
                # MPI_NP / MPI_PPR / MPI_RAYON_NUM_THREADS, then launch the worker
                # under mpirun inside bash -c. The \$MPI_* are escaped so the outer
                # login shell leaves them untouched — they expand only in the inner
                # bash, after the source. $HOME still expands in the login shell.
                # --allow-run-as-root is required because pods run as root.
                "tmux new-session -d -s worker "
                f"\"bash -c 'source {REMOTE_WORKSPACE_BIN_DIR}/mpi_params.sh && "
                "mpirun --allow-run-as-root -np \\$MPI_NP "
                "-map-by ppr:\\$MPI_PPR:numa --bind-to numa "
                "-x RAYON_NUM_THREADS=\\$MPI_RAYON_NUM_THREADS "
                f"{REMOTE_BIN_DIR}/zisk-worker -c http://{coord_host}:{coord_port} "
                f"--worker-id {worker_id} -k {REMOTE_PROVING_KEY_DIR} "
                "--unlock-mapped-memory --gpu "
                f"2>&1 | tee {REMOTE_LOG_DIR}/pod{index}-worker.log'\"",
            ),
        }
    )

    # Wait until this pod's worker has registered with the coordinator (watch its
    # log for a line containing the registration message, up to the timeout).
    worker_log = f"{REMOTE_LOG_DIR}/pod{index}-worker.log"
    steps.append(
        {
            "desc": (
                f"Wait for worker registration "
                f"(<= {REGISTRATION_TIMEOUT_SECONDS}s)"
            ),
            "cmd": ssh_argv(
                target,
                private_key,
                f"timeout {REGISTRATION_TIMEOUT_SECONDS} bash -c "
                f"'until grep -qF \"{REGISTRATION_MESSAGE}\" \"{worker_log}\" "
                "2>/dev/null; do sleep 2; done'",
            ),
        }
    )

    # Sync both pods: ethproofs-client (pod 1) only starts once every pod's
    # worker has registered.
    steps.append(
        {
            "desc": "Wait for all pods' workers to be registered (barrier)",
            "barrier": True,
        }
    )

    if is_coord:
        ethproofs_cmd = (
            f"{REMOTE_WORKSPACE_BIN_DIR}/ethproofs-client "
            "-c http://localhost:7000 "
            f"--input.folder {REMOTE_LOG_DIR}/inputs -n rpc --input.keep "
            f"-g {REMOTE_WORKSPACE_BIN_DIR}/zec-reth "
            f"--rpc.http-url {rpc_http_url} --rpc.ws-url {rpc_ws_url} "
            f"--hints.debug --hints.debug-folder {REMOTE_LOG_DIR}/hints "
            f"--run-time {run_time} --exit-on-error "
            f"--proof.csv {REMOTE_LOG_DIR}/proof.csv"
        )
        ethproofs_log = f"{REMOTE_LOG_DIR}/pod{index}-ethproofs-client.log"
        steps.append(
            {
                "desc": "Run ethproofs-client (foreground, streaming logs)",
                # tee saves the output to REMOTE_LOG_DIR while still streaming it
                # over SSH to this script's console. pipefail keeps the pipeline's
                # exit code as ethproofs-client's (not tee's), so --exit-on-error
                # failures still propagate.
                "cmd": ssh_argv(
                    target,
                    private_key,
                    "bash -c 'set -o pipefail; "
                    f"{ethproofs_cmd} 2>&1 | tee {ethproofs_log}'",
                ),
            }
        )

    return steps


def run_pod_steps(
    target: dict[str, Any],
    steps: list[dict[str, Any]],
    barrier: threading.Barrier | None = None,
) -> None:
    """Run a pod's steps sequentially. Raises RuntimeError on the first failure.

    On failure the shared barrier is aborted so the other pod does not hang
    waiting at it.
    """
    prefix = f"[pod{target['index']}] "

    try:
        for step in steps:
            print(f"\n{prefix}>>> {step['desc']}", flush=True)

            if "sleep" in step:
                time.sleep(step["sleep"])
                continue

            if step.get("barrier"):
                if barrier is not None:
                    barrier.wait()
                continue

            returncode = run_streaming(step["cmd"], prefix)

            if returncode != 0:
                raise RuntimeError(
                    f"pod{target['index']} step failed: {step['desc']} "
                    f"(exit code {returncode})"
                )
    except BaseException:
        # Break the barrier so a sibling pod waiting on it fails fast instead of
        # blocking forever.
        if barrier is not None:
            barrier.abort()
        raise


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Provision an already-deployed zisk cluster (from pods.json): copy "
            "the built binaries to /workspace/bin, install the proving key and "
            "start coordinator/worker/ethproofs."
        )
    )

    parser.add_argument(
        "--pods-file",
        default="pods.json",
        help="JSON file written by deploy_pods.py. Default: pods.json",
    )
    parser.add_argument(
        "--work-dir",
        default=DEFAULT_WORK_DIR,
        help=(
            "Local directory containing the built repos (for mpi_params.sh). "
            f"Default: {DEFAULT_WORK_DIR}"
        ),
    )
    parser.add_argument(
        "--local-zisk-dir",
        default=DEFAULT_LOCAL_ZISK_DIR,
        help=(
            "Local dist directory (bin/ and zisk/) whose contents are copied "
            f"to the pods' ~/.zisk. Default: {DEFAULT_LOCAL_ZISK_DIR}"
        ),
    )
    parser.add_argument(
        "--provingkey",
        default=DEFAULT_PROVING_KEY,
        help=(
            "Proving key file downloaded from the zisk-setup bucket. "
            f"Default: {DEFAULT_PROVING_KEY}"
        ),
    )
    parser.add_argument(
        "--rpc-http-url",
        default=DEFAULT_RPC_HTTP_URL,
        help=f"ethproofs-client --rpc.http-url. Default: {DEFAULT_RPC_HTTP_URL}",
    )
    parser.add_argument(
        "--rpc-ws-url",
        default=DEFAULT_RPC_WS_URL,
        help=f"ethproofs-client --rpc.ws-url. Default: {DEFAULT_RPC_WS_URL}",
    )
    parser.add_argument(
        "--run-time",
        type=int,
        default=60,
        help=(
            "Time (seconds) the test runs, passed to ethproofs-client "
            "--run-time. Default: 60"
        ),
    )
    parser.add_argument(
        "--ssh-key",
        default=str(Path.home() / ".ssh" / "id_ed25519"),
        help="Private SSH key path. Default: ~/.ssh/id_ed25519",
    )
    parser.add_argument(
        "--keep-on-error",
        action="store_true",
        help=(
            "Do NOT terminate the pods if something fails (for debugging). "
            "By default all pods are terminated on any error to avoid costs."
        ),
    )

    args = parser.parse_args()

    api_key = os.environ.get("RUNPOD_API_KEY")
    if not api_key:
        raise RuntimeError("Missing RUNPOD_API_KEY environment variable")

    private_key = str(Path(args.ssh_key).expanduser().resolve())
    work_dir = Path(args.work_dir).expanduser().resolve()

    pods = load_pods_info(Path(args.pods_file).expanduser())
    pod1, pod2 = pods[0], pods[1]

    def make_target(pod: dict[str, Any], index: int) -> dict[str, Any]:
        return {
            "index": index,
            "name": pod.get("name"),
            "host": pod.get("publicIp"),
            "port": pod.get("sshPort"),
            "user": pod.get("sshUser") or "root",
        }

    target1 = make_target(pod1, 1)
    target2 = make_target(pod2, 2)

    # pod 2's worker reaches the coordinator over the private global network
    # (<podId>.runpod.internal) on the internal 50051 port. The public IP does
    # not work between pods behind the same NAT.
    pod1_internal_host = pod1.get("internalHost") or f"{pod1.get('id')}.runpod.internal"

    # ~/.zisk/{bin,zisk} is copied to each pod; ethproofs-client, zec-reth and
    # mpi_params.sh go to /workspace/bin (pod 2 only needs mpi_params.sh).
    local_zisk_dir = Path(args.local_zisk_dir).expanduser().resolve()
    mpi_params = work_dir / MPI_PARAMS_SCRIPT
    ethproofs_client = work_dir / ETHPROOFS_CLIENT_BIN
    zec_reth = work_dir / ZEC_RETH_BIN

    pod1_workspace_bin = [ethproofs_client, zec_reth, mpi_params]
    pod2_workspace_bin = [mpi_params]

    # Fail early (without touching the pods) if something is missing locally.
    for required_dir in (local_zisk_dir / "bin", local_zisk_dir / "zisk"):
        if not required_dir.is_dir():
            raise FileNotFoundError(f"Local dir not found: {required_dir}")
    for required_file in (mpi_params, ethproofs_client, zec_reth):
        if not required_file.is_file():
            raise FileNotFoundError(f"File not found: {required_file}")

    steps1 = build_pod_steps(
        target=target1,
        private_key=private_key,
        local_zisk_dir=local_zisk_dir,
        workspace_bin_files=pod1_workspace_bin,
        proving_key=args.provingkey,
        is_coord=True,
        coord_host="127.0.0.1",
        coord_port=COORD_INTERNAL_PORT,
        worker_id=1,
        rpc_http_url=args.rpc_http_url,
        rpc_ws_url=args.rpc_ws_url,
        run_time=args.run_time,
    )
    steps2 = build_pod_steps(
        target=target2,
        private_key=private_key,
        local_zisk_dir=local_zisk_dir,
        workspace_bin_files=pod2_workspace_bin,
        proving_key=args.provingkey,
        is_coord=False,
        coord_host=pod1_internal_host,
        coord_port=COORD_INTERNAL_PORT,
        worker_id=2,
        rpc_http_url=args.rpc_http_url,
        rpc_ws_url=args.rpc_ws_url,
        run_time=args.run_time,
    )

    jobs = [
        {"target": target1, "steps": steps1},
        {"target": target2, "steps": steps2},
    ]

    # Barrier so ethproofs-client (pod 1) only starts once every pod's worker
    # has registered with the coordinator.
    barrier = threading.Barrier(len(jobs))

    try:
        print("\nProvisioning cluster on all pods (parallel)...\n")

        errors: list[str] = []
        with ThreadPoolExecutor(max_workers=len(jobs)) as executor:
            futures = {
                executor.submit(
                    run_pod_steps, job["target"], job["steps"], barrier
                ): job
                for job in jobs
            }
            for future in as_completed(futures):
                try:
                    future.result()
                except Exception as job_error:
                    errors.append(str(job_error))

        if errors:
            raise RuntimeError("; ".join(errors))

        print("\nCluster provisioned and started successfully on all pods.")

    except BaseException as error:
        # Catch everything (including KeyboardInterrupt) so pods are never left
        # running and billing after a failure.
        print(f"\nERROR: cluster provisioning failed: {error}", file=sys.stderr)

        if args.keep_on_error:
            print(
                "Pods left running (--keep-on-error). Terminate them manually "
                "to stop incurring costs.",
                file=sys.stderr,
            )
        else:
            print(
                f"\nTerminating {len(pods)} pod(s) to avoid costs...",
                file=sys.stderr,
            )
            for pod in pods:
                pod_id = pod.get("id")
                if not pod_id:
                    continue
                try:
                    terminate_pod(api_key, pod_id)
                except Exception as terminate_error:
                    print(
                        f"WARNING: failed to terminate pod {pod_id}: "
                        f"{terminate_error}. TERMINATE IT MANUALLY to avoid costs!",
                        file=sys.stderr,
                    )

        raise


if __name__ == "__main__":
    main()
