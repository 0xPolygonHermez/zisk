# ZisK deployment

Same `zisk-worker` and `zisk-coordinator` binaries; many ways to deploy them.
Pick the one that matches your environment.

## Which path?

| You want to… | Look in | Notes |
|---|---|---|
| Run a single-host stack on a laptop, dev box, or small staging | [`docker/`](./docker/) | docker-compose. Workers + coordinator + Prometheus + Grafana, one command. **Dev-only:** Grafana ships with anonymous admin access — disable before any non-local deploy. |
| Install on bare-metal Linux/macOS hosts (one or two machines) | [`scripts/`](./scripts/) | `worker/install.sh` / `coordinator/install.sh` write a systemd unit (Linux) or launchd plist (macOS). Curl-pipe-able from a fresh server. |
| Roll out across a fleet of bare-metal / VM hosts | [`ansible/`](./ansible/) | Ansible roles wrap the same install scripts; idempotent, repeatable. |
| Run on a Kubernetes cluster | [`k8s/`](./k8s/) | Helm chart + slim Dockerfile. Horizontal autoscaling, queue-depth-aware (with KEDA / prometheus-adapter). |
| Sample worker.toml / coordinator.toml | [`config/`](./config/) | Same TOML format used by every path above. |

The Grafana dashboards (`grafana/`) and Prometheus scrape config (`prometheus/`)
are wired into the compose stack today, but the dashboard JSON is reusable from
any deploy that exposes the coordinator's `/metrics` port.

## Layout

```
distributed/deploy/
├── README.md          # this file — decision matrix
├── config/            # sample worker.toml / coordinator.toml (shared)
├── docker/            # docker-compose stack (compose.yaml + Dockerfiles)
├── scripts/           # bare-metal install.sh for systemd / launchd
├── ansible/           # multi-host orchestration of scripts/
├── k8s/               # Helm chart + slim Dockerfile for clusters
├── grafana/           # dashboards + provisioning (used by docker/ today)
└── prometheus/        # scrape config (used by docker/ today)
```

## Common tasks

### Bring up a local stack (dev)
```bash
docker compose -f distributed/deploy/docker/compose.yaml up --scale worker=4
```

### Install a worker on a fresh Linux host (curl-pipe)
```bash
curl -fsL https://raw.githubusercontent.com/0xPolygonHermez/zisk/main/distributed/deploy/scripts/worker/install.sh \
    | sudo bash -s -- --gpu --coordinator-url <URL>
```

### Apply the worker role across an inventory (Ansible)
```bash
ansible-playbook -i inventory.ini distributed/deploy/ansible/playbooks/install-worker.yml \
    -e zisk_worker_binary_src=./target/release/zisk-worker
```

### Deploy a worker fleet to k8s (Helm)
```bash
helm install zisk-worker distributed/deploy/k8s/charts/zisk-worker \
    -n zisk --create-namespace \
    --set coordinator.url=coordinator.zisk.svc.cluster.local:7000 \
    --set provingKey.pvc.claimName=zisk-bundle \
    --set replicaCount=8
```

### Uninstall (any path)
Each install path has its own `--uninstall` flow. To fully remove ZisK from a
host, run the per-service uninstall first, then `ziskup --uninstall --system`
to remove the shared bundle and zisk system user. See the README in each
subdirectory for details.
