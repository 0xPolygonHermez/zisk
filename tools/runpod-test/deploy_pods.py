#!/usr/bin/env python3

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

import requests


RUNPOD_API_URL = "https://rest.runpod.io/v1"

GPU_TYPES = [
    "NVIDIA GeForce RTX 4090",
    "NVIDIA GeForce RTX 5090",
]

POD_PORTS = [
    "22/tcp",      # SSH
    "50051/tcp",  # cluster coordinator
    "7000/tcp",   # zisk-coordinator
    "9090/tcp",   # metrics
]

# RunPod only supports allowlists (no exclude), so we allow all data centers
# except the excluded ones. Full list from the RunPod create-pod API docs.
ALL_DATA_CENTER_IDS = [
    "EU-RO-1", "CA-MTL-1", "EU-SE-1", "US-IL-1", "EUR-IS-1", "EU-CZ-1",
    "US-TX-3", "EUR-IS-2", "US-KS-2", "US-GA-2", "US-WA-1", "US-TX-1",
    "CA-MTL-3", "EU-NL-1", "US-TX-4", "US-CA-2", "US-NC-1", "OC-AU-1",
    "US-DE-1", "EUR-IS-3", "CA-MTL-2", "AP-JP-1", "EUR-NO-1", "EU-FR-1",
    "US-KS-3", "US-GA-1",
]
EXCLUDED_DATA_CENTER_IDS = {"EUR-IS-2", "US-IL-1", "US-CA-2"}
DATA_CENTER_IDS = [
    dc for dc in ALL_DATA_CENTER_IDS if dc not in EXCLUDED_DATA_CENTER_IDS
]

SSH_OPTIONS = [
    "-o",
    "StrictHostKeyChecking=no",
    "-o",
    "UserKnownHostsFile=/dev/null",
]


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


def create_pod(
    api_key: str,
    name: str,
    image_name: str,
    container_disk_gb: int,
    volume_gb: int,
    volume_mount_path: str,
    cloud_type: str,
    env: dict[str, str],
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "name": name,
        "imageName": image_name,

        "cloudType": cloud_type,
        "computeType": "GPU",

        # 2 GPUs per pod
        "gpuCount": 2,
        "gpuTypeIds": GPU_TYPES,
        "gpuTypePriority": "availability",

        # Allowed data centers (all except the excluded ones, e.g. EUR-IS-2).
        "dataCenterIds": DATA_CENTER_IDS,

        # Public ports
        "ports": POD_PORTS,
        "supportPublicIp": True,

        # Private inter-pod network (<podId>.runpod.internal) so pods can reach
        # each other without going through the public IP (which fails between
        # pods behind the same NAT).
        "globalNetworking": True,

        # False = reserved/on-demand (not spot/interruptible)
        "interruptible": False,

        # Storage
        "containerDiskInGb": container_disk_gb,
        "volumeInGb": volume_gb,
        "volumeMountPath": volume_mount_path,

        # Environment variables
        "env": env,
    }

    print(f"Creating pod {name}...")
    pod = api_request("POST", "/pods", api_key, payload)

    if "id" not in pod:
        raise RuntimeError(f"Unexpected create pod response: {pod}")

    return pod


def get_pod(api_key: str, pod_id: str) -> dict[str, Any]:
    return api_request("GET", f"/pods/{pod_id}", api_key)


def get_public_port(pod: dict[str, Any], internal_port: int) -> int | None:
    port_mappings = pod.get("portMappings") or {}

    value = (
        port_mappings.get(str(internal_port))
        or port_mappings.get(internal_port)
    )

    if value is None:
        return None

    return int(value)


def print_pod_ports(pod: dict[str, Any]) -> None:
    public_ip = pod.get("publicIp")

    print(f"Public IP: {public_ip}")

    for internal_port in [22, 50051, 7000, 9090]:
        public_port = get_public_port(pod, internal_port)

        if public_port:
            print(f"  {internal_port}/tcp -> {public_ip}:{public_port}")
        else:
            print(f"  {internal_port}/tcp -> not mapped yet")


def save_pods_info(
    pods: list[dict[str, Any]],
    ssh_user: str,
    output_path: Path,
) -> None:
    entries = []

    for pod in pods:
        pod_id = pod.get("id")
        entries.append(
            {
                "id": pod_id,
                "name": pod.get("name"),
                "publicIp": pod.get("publicIp"),
                # Private DNS name on the global (inter-pod) network.
                "internalHost": f"{pod_id}.runpod.internal" if pod_id else None,
                "sshPort": get_public_port(pod, 22),
                "sshUser": ssh_user,
                "ports": {
                    str(port): get_public_port(pod, port)
                    for port in [22, 50051, 7000, 9090]
                },
            }
        )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps({"pods": entries}, indent=2))
    print(f"\nSaved pods info to {output_path}")


def wait_for_pod_ports(
    api_key: str,
    pod_id: str,
    timeout_seconds: int,
    poll_seconds: int = 10,
) -> dict[str, Any]:
    deadline = time.time() + timeout_seconds
    required_ports = [22, 50051, 7000, 9090]

    while time.time() < deadline:
        pod = get_pod(api_key, pod_id)

        public_ip = pod.get("publicIp")
        desired_status = pod.get("desiredStatus")
        machine_id = pod.get("machineId")

        mapped_ports = {
            port: get_public_port(pod, port)
            for port in required_ports
        }

        print(
            f"Pod {pod_id}: "
            f"desiredStatus={desired_status}, "
            f"machineId={machine_id}, "
            f"publicIp={public_ip}, "
            f"ports={mapped_ports}"
        )

        if public_ip and all(mapped_ports.values()):
            return pod

        time.sleep(poll_seconds)

    raise TimeoutError(f"Pod {pod_id} did not expose all required ports")


def wait_for_ssh_ready(
    host: str,
    port: int,
    user: str,
    private_key: str,
    timeout_seconds: int,
) -> None:
    deadline = time.time() + timeout_seconds

    while time.time() < deadline:
        command = [
            "ssh",
            "-i",
            private_key,
            "-p",
            str(port),
            *SSH_OPTIONS,
            "-o",
            "ConnectTimeout=10",
            f"{user}@{host}",
            "echo ssh-ready",
        ]

        result = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        if result.returncode == 0 and "ssh-ready" in result.stdout:
            print(f"SSH ready on {host}:{port}")
            return

        print(f"Waiting for SSH on {host}:{port}...")
        time.sleep(10)

    raise TimeoutError(f"SSH was not ready on {host}:{port}")


def terminate_pod(api_key: str, pod_id: str) -> None:
    print(f"Terminating pod {pod_id}...")
    api_request("DELETE", f"/pods/{pod_id}", api_key)


def parse_env_vars(env_items: list[str]) -> dict[str, str]:
    env: dict[str, str] = {}

    for item in env_items:
        if "=" not in item:
            raise ValueError(f"Invalid --env value: {item}. Use KEY=VALUE")

        key, value = item.split("=", 1)
        env[key] = value

    return env


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Create two RunPod pods with 2 GPUs each from a Docker image and "
            "wait until they are reachable over SSH. Provisioning is done "
            "separately."
        )
    )

    parser.add_argument(
        "--image",
        default="ziskvm/zisk-runpod:latest",
        help=(
            "Docker image to run on each pod. The image must support SSH. "
            "Default: ziskvm/zisk-runpod:latest"
        ),
    )
    parser.add_argument(
        "--output",
        default="pods.json",
        help=(
            "File where pod connection info (IP, SSH port) is saved for "
            "run_command.py. Default: pods.json"
        ),
    )
    parser.add_argument(
        "--ssh-key",
        default=str(Path.home() / ".ssh" / "id_ed25519"),
        help="Private SSH key path. Default: ~/.ssh/id_ed25519",
    )
    parser.add_argument(
        "--ssh-user",
        default="root",
        help="SSH user. Default: root",
    )
    parser.add_argument(
        "--name-prefix",
        default="zisk-cluster-test",
        help="Pod name prefix. Default: zisk-cluster-test",
    )
    parser.add_argument(
        "--container-disk-gb",
        type=int,
        default=100,
        help="Container disk size in GB. Default: 100",
    )
    parser.add_argument(
        "--volume-gb",
        type=int,
        default=50,
        help="Pod volume size in GB. Default: 50",
    )
    parser.add_argument(
        "--volume-mount-path",
        default="/workspace",
        help="Volume mount path. Default: /workspace",
    )
    parser.add_argument(
        "--cloud-type",
        choices=["SECURE", "COMMUNITY"],
        default="SECURE",
        help="RunPod cloud type. Default: SECURE",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=900,
        help="Timeout in seconds for pod/SSH readiness. Default: 900",
    )
    parser.add_argument(
        "--keep-on-error",
        action="store_true",
        help=(
            "Do NOT terminate the pods if something fails (for debugging). "
            "By default all created pods are terminated on any error to avoid "
            "incurring costs."
        ),
    )
    parser.add_argument(
        "--env",
        action="append",
        default=[],
        help="Extra env var for the pod, format KEY=VALUE. Can be repeated.",
    )

    args = parser.parse_args()

    api_key = os.environ.get("RUNPOD_API_KEY")
    if not api_key:
        raise RuntimeError("Missing RUNPOD_API_KEY environment variable")

    private_key = str(Path(args.ssh_key).expanduser().resolve())

    env = parse_env_vars(args.env)

    created_pods: list[str] = []

    try:
        pod_ids: list[str] = []

        for index in range(1, 3):
            pod_name = f"{args.name_prefix}-{index}"

            pod = create_pod(
                api_key=api_key,
                name=pod_name,
                image_name=args.image,
                container_disk_gb=args.container_disk_gb,
                volume_gb=args.volume_gb,
                volume_mount_path=args.volume_mount_path,
                cloud_type=args.cloud_type,
                env={
                    **env,
                    "POD_INDEX": str(index),
                    "POD_COUNT": "2",
                },
            )

            pod_id = pod["id"]
            created_pods.append(pod_id)
            pod_ids.append(pod_id)

            print(f"Created pod {index}: {pod_id}")

        final_pods: list[dict[str, Any]] = []

        for index, pod_id in enumerate(pod_ids, start=1):
            print(f"\nWaiting for pod {index} ports...")
            pod = wait_for_pod_ports(
                api_key=api_key,
                pod_id=pod_id,
                timeout_seconds=args.timeout,
            )

            print(f"\nPod {index} ports:")
            print_pod_ports(pod)

            final_pods.append(pod)

        save_pods_info(
            pods=final_pods,
            ssh_user=args.ssh_user,
            output_path=Path(args.output).expanduser(),
        )

        for index, pod in enumerate(final_pods, start=1):
            public_ip = pod["publicIp"]
            ssh_port = get_public_port(pod, 22)

            if ssh_port is None:
                raise RuntimeError(f"SSH port not found for pod {index}")

            print(f"\nWaiting for SSH on pod {index}...")
            wait_for_ssh_ready(
                host=public_ip,
                port=ssh_port,
                user=args.ssh_user,
                private_key=private_key,
                timeout_seconds=args.timeout,
            )

        # Pods are up and reachable over SSH at this point.
        print("\nAll pods created and reachable over SSH.")
        print("\nCreated pods:")

        for index, pod in enumerate(final_pods, start=1):
            print(f"\nPod {index}:")
            print(f"  ID: {pod['id']}")
            print_pod_ports(pod)

    except BaseException as error:
        # Catch everything (including KeyboardInterrupt) so pods are never left
        # running and billing after a failure.
        print(f"\nERROR: deployment failed: {error}", file=sys.stderr)

        if args.keep_on_error:
            print(
                "Pods left running (--keep-on-error). Terminate them manually "
                "to stop incurring costs.",
                file=sys.stderr,
            )
        elif created_pods:
            print(
                f"\nTerminating {len(created_pods)} pod(s) to avoid costs...",
                file=sys.stderr,
            )
            for pod_id in created_pods:
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
