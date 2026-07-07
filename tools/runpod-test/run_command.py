#!/usr/bin/env python3

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any


def load_pods_info(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(
            f"Pods info file not found: {path}. Run deploy_pods.py first."
        )

    data = json.loads(path.read_text())
    pods = data.get("pods") if isinstance(data, dict) else data

    if not isinstance(pods, list) or not pods:
        raise RuntimeError(f"No pods found in {path}")

    return pods


def select_pods(
    pods: list[dict[str, Any]],
    name_prefix: str | None,
    pod_ids: list[str],
) -> list[dict[str, Any]]:
    if pod_ids:
        by_id = {pod.get("id"): pod for pod in pods}
        selected = []
        for pod_id in pod_ids:
            pod = by_id.get(pod_id)
            if pod is None:
                raise RuntimeError(f"Pod not found: {pod_id}")
            selected.append(pod)
        return selected

    if not name_prefix:
        return pods

    selected = [
        pod
        for pod in pods
        if str(pod.get("name", "")).startswith(name_prefix)
    ]

    if not selected:
        raise RuntimeError(f"No pods found with name prefix '{name_prefix}'")

    return selected


def run_remote_command(
    host: str,
    port: int,
    user: str,
    private_key: str,
    command: str,
    connect_timeout: int,
) -> tuple[int, str, str]:
    ssh_command = [
        "ssh",
        "-i",
        private_key,
        "-p",
        str(port),
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        f"ConnectTimeout={connect_timeout}",
        f"{user}@{host}",
        command,
    ]

    result = subprocess.run(
        ssh_command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.returncode, result.stdout, result.stderr


def run_on_pod(
    pod: dict[str, Any],
    default_user: str | None,
    private_key: str,
    command: str,
    connect_timeout: int,
) -> dict[str, Any]:
    name = pod.get("name")
    public_ip = pod.get("publicIp")
    ssh_port = pod.get("sshPort")
    ssh_user = default_user or pod.get("sshUser") or "root"

    if not public_ip or not ssh_port:
        return {
            "name": name,
            "ok": False,
            "returncode": None,
            "stdout": "",
            "stderr": "missing publicIp / sshPort in pods file",
            "target": None,
        }

    target = f"{public_ip}:{ssh_port}"
    print(f"Launching on {name} ({target})...")

    returncode, stdout, stderr = run_remote_command(
        host=public_ip,
        port=int(ssh_port),
        user=ssh_user,
        private_key=private_key,
        command=command,
        connect_timeout=connect_timeout,
    )

    return {
        "name": name,
        "ok": returncode == 0,
        "returncode": returncode,
        "stdout": stdout,
        "stderr": stderr,
        "target": target,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Run a command over SSH on the RunPod pods created by "
            "deploy_pods.py, using the info saved in pods.json."
        )
    )

    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help="Command to run on each pod (everything after the options).",
    )
    parser.add_argument(
        "--pods-file",
        default="pods.json",
        help="JSON file written by deploy_pods.py. Default: pods.json",
    )
    parser.add_argument(
        "--name-prefix",
        default=None,
        help="Only target pods whose name starts with this prefix.",
    )
    parser.add_argument(
        "--pod-id",
        action="append",
        default=[],
        help="Target a specific pod id. Can be repeated. Overrides --name-prefix.",
    )
    parser.add_argument(
        "--ssh-key",
        default=str(Path.home() / ".ssh" / "id_ed25519"),
        help="Private SSH key path. Default: ~/.ssh/id_ed25519",
    )
    parser.add_argument(
        "--ssh-user",
        default=None,
        help="SSH user. Default: the user saved in pods.json (or root).",
    )
    parser.add_argument(
        "--connect-timeout",
        type=int,
        default=15,
        help="SSH connection timeout in seconds. Default: 15",
    )

    args = parser.parse_args()

    command = args.command
    # argparse.REMAINDER keeps a leading "--" if present; drop it.
    if command and command[0] == "--":
        command = command[1:]

    if not command:
        parser.error("No command provided. Example: run_command.py -- nvidia-smi")

    import shlex
    command_str = shlex.join(command)

    private_key = str(Path(args.ssh_key).expanduser().resolve())

    pods = load_pods_info(Path(args.pods_file).expanduser())
    selected = select_pods(pods, args.name_prefix, args.pod_id)

    print(f"Selected {len(selected)} pod(s):")
    for pod in selected:
        print(f"  {pod.get('name')} ({pod.get('id')})")
    print()

    # Launch the command on every pod in parallel, then wait for all of them.
    with ThreadPoolExecutor(max_workers=len(selected)) as executor:
        results = list(
            executor.map(
                lambda pod: run_on_pod(
                    pod=pod,
                    default_user=args.ssh_user,
                    private_key=private_key,
                    command=command_str,
                    connect_timeout=args.connect_timeout,
                ),
                selected,
            )
        )

    failures: list[str] = []

    for result in results:
        name = result["name"]
        target = result["target"]
        header = f"{name} ({target})" if target else str(name)

        print(f"\n=== {header} ===")

        if result["stdout"]:
            print(result["stdout"].rstrip("\n"))
        if result["stderr"]:
            print(result["stderr"].rstrip("\n"), file=sys.stderr)

        if result["ok"]:
            print(f"[{name}] OK")
        else:
            print(
                f"[{name}] FAILED (exit code {result['returncode']})",
                file=sys.stderr,
            )
            failures.append(str(name))

    if failures:
        raise SystemExit(
            f"\nCommand failed on {len(failures)} pod(s): {', '.join(failures)}"
        )

    print("\nCommand finished successfully on all selected pods.")


if __name__ == "__main__":
    main()
