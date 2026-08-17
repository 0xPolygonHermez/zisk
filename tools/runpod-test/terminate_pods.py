#!/usr/bin/env python3

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

import requests


RUNPOD_API_URL = "https://rest.runpod.io/v1"


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

    if not isinstance(pods, list) or not pods:
        raise RuntimeError(f"No pods found in {path}")

    return pods


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Terminate all RunPod pods listed in pods.json."
    )

    parser.add_argument(
        "--pods-file",
        default="pods.json",
        help="JSON file written by deploy_pods.py. Default: pods.json",
    )

    args = parser.parse_args()

    api_key = os.environ.get("RUNPOD_API_KEY")
    if not api_key:
        raise RuntimeError("Missing RUNPOD_API_KEY environment variable")

    pods = load_pods_info(Path(args.pods_file).expanduser())

    print(f"Terminating {len(pods)} pod(s) from {args.pods_file}...")

    failures: list[str] = []

    for pod in pods:
        pod_id = pod.get("id")
        name = pod.get("name")

        if not pod_id:
            print(f"WARNING: skipping entry without id: {pod}", file=sys.stderr)
            continue

        try:
            terminate_pod(api_key, pod_id)
        except Exception as terminate_error:
            print(
                f"WARNING: failed to terminate pod {name} ({pod_id}): "
                f"{terminate_error}. TERMINATE IT MANUALLY to avoid costs!",
                file=sys.stderr,
            )
            failures.append(str(name or pod_id))

    if failures:
        raise SystemExit(
            f"\nFailed to terminate {len(failures)} pod(s): {', '.join(failures)}"
        )

    print("\nAll pods terminated successfully.")


if __name__ == "__main__":
    main()
