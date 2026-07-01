# Kubernetes deploy — zisk-worker

Run zisk-workers as horizontally-scaled pods that pull work off a coordinator's
queue. Complements `distributed/deploy/scripts/worker/install.sh` (host-level
systemd) and `distributed/deploy/docker/` (single-node compose).

This is a **skeleton**. It's enough to ship workers to a cluster; production
operators will likely tune resources, add autoscaling on a real signal, and
pick a proving-key strategy that matches their storage profile.

## Layout

```
k8s/
├── Dockerfile.worker           # slim production image (no install.sh, no ziskup)
├── README.md
└── charts/
    └── zisk-worker/            # Helm chart
        ├── Chart.yaml
        ├── values.yaml
        └── templates/
            ├── _helpers.tpl
            ├── configmap.yaml
            ├── deployment.yaml   # Deployment OR StatefulSet (workloadKind switch)
            ├── hpa.yaml
            ├── serviceaccount.yaml
            └── NOTES.txt
```

The coordinator is **not** in this chart — assume it's deployed separately
(it's not horizontally scaled; one coordinator per cluster is the model).

## Build the worker image

The slim Dockerfile expects a **pre-built** binary in `./bin/zisk-worker`.
Build the binary first (in CI or locally), then build the image:

```bash
# Build the binary outside the image (faster, smaller, no CUDA toolkit etc).
cargo build --release -p zisk-worker

# Stage it for the docker build context.
mkdir -p bin
cp target/release/zisk-worker bin/

# Build a 200MB-ish image instead of the dev image's multi-GB.
docker build -f distributed/deploy/k8s/Dockerfile.worker -t zisk-worker:0.17.0 .
docker push zisk-worker:0.17.0
```

For a GPU variant, build the binary against `cargo build --features gpu` (or
whatever your build flow uses) and push under a different tag (`:0.17.0-gpu`).

## Pick a proving-key strategy

The proving key is GBs. The chart supports three patterns via
`values.provingKey.strategy`:

| Strategy | When to use | Tradeoffs |
|---|---|---|
| `pvc` (default) | Production, multi-replica | Best perf and bandwidth, but operator must provision a `ReadOnlyMany` PVC and seed it once. |
| `image` | Air-gapped or small clusters | Simple. Image is multi-GB; node image pulls are slow. Bake the bundle in via your own Dockerfile build that copies it. |
| `download` | One-off testing, single replica | Each pod re-downloads from GCS on start. Slow + bandwidth-wasteful at scale. |

### Seeding a PVC (one-time)

```bash
kubectl create pvc zisk-bundle \
    --storage-class=<your-rwx-class> \
    --access-modes=ReadOnlyMany \
    --size=80Gi

# Run a one-off pod to populate it. Either:
#   - `kubectl run --rm -it ... -- ziskup --system --prefix /opt/zisk` then
#     copy the contents into the PVC, or
#   - extract the release tarball into the PVC mount directly.
```

The PVC's contents must match the standard layout `/opt/zisk/{bin,zisk,
provingKey,provingKeySnark?}` — same as a host install via ziskup.

## Install the chart

```bash
helm install zisk-worker distributed/deploy/k8s/charts/zisk-worker \
    --namespace zisk \
    --create-namespace \
    --set coordinator.url=coordinator.zisk.svc.cluster.local:7000 \
    --set image.tag=0.17.0 \
    --set replicaCount=4 \
    --set provingKey.pvc.claimName=zisk-bundle
```

Or from a values file:

```yaml
# values.prod.yaml
coordinator:
  url: coordinator.zisk.svc.cluster.local:7000
image:
  repository: registry.example/zisk-worker
  tag: 0.17.0
replicaCount: 8
provingKey:
  strategy: pvc
  pvc:
    claimName: zisk-bundle
worker:
  gpu: true
  computeCapacity: 4
resources:
  limits:
    nvidia.com/gpu: 1
nodeSelector:
  node-role.example/gpu: "true"
```

```bash
helm install zisk-worker distributed/deploy/k8s/charts/zisk-worker \
    -n zisk -f values.prod.yaml
```

## Scaling

Three knobs:

1. **Static**: `--set replicaCount=N` (or via values file).
2. **HPA on CPU**: `--set autoscaling.enabled=true` — coarse approximation.
3. **HPA on coordinator queue**: enable HPA, then add a custom external
   metric that exposes coordinator queue depth (via prometheus-adapter or
   KEDA). Replace the CPU metric in `templates/hpa.yaml` with your custom
   metric. CPU-based scaling reacts to the *symptom* (workers are busy);
   queue-depth scaling reacts to the *cause* (work is piling up).

## What's intentionally NOT in this chart

- **Coordinator deployment** — different shape (singleton, has gRPC ports
  to expose, often wants an Ingress). Out of scope here.
- **Health probes** — the worker doesn't expose an HTTP health endpoint
  today. When it does, add `readinessProbe` / `livenessProbe` to
  `deployment.yaml`.
- **Network policies** — depends on cluster policy model.
- **Service / Ingress** — workers don't accept inbound traffic; they dial
  the coordinator. No Service needed.
- **MPI multi-process per pod** — possible via [kubeflow/mpi-operator] but
  adds a lot of moving parts. The chart runs one worker per pod, scaling
  horizontally instead.

[kubeflow/mpi-operator]: https://github.com/kubeflow/mpi-operator
