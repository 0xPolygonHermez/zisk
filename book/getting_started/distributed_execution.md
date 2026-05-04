# Distributed Execution

Generating a ZisK proof means proving the full execution trace of a
program. For real workloads, that trace is too large and too slow to
prove on a single machine. A ZisK cluster splits the trace into
**segments**, proves each segment in parallel on separate machines,
and aggregates the results into a single final proof. Throughput and
latency scale with the number of machines you give it.

This guide covers the three things you need to run distributed
proving: the cluster's architecture, a single-host quickstart that
gets a job through the binaries, and the production path that
deploys the same binaries on bare Linux hosts with systemd.

---

## Architecture

A ZisK cluster is two binaries: a single `zisk-coordinator` and one
or more `zisk-worker` instances.

```
                    ┌─────────────────────────┐
                    │    Host application     │
                    │     (RemoteClient)      │
                    └────────────┬────────────┘
                                 │
                                 │ gRPC :7000
                                 │ prove request
                                 ▼
    ╔════════════════════════════════════════════════════════╗
    ║                    ZisK cluster                        ║
    ║                                                        ║
    ║         ┌──────────────────────────────────┐           ║
    ║         │        zisk-coordinator          │           ║
    ║         │     :7000   :50051   :9090       │           ║
    ║         └───┬──────────┬──────────┬────────┘           ║
    ║             │          │          │                    ║
    ║      assign │   assign │   assign │                    ║
    ║    segments │ segments │ segments │                    ║
    ║             ▼          ▼          ▼                    ║
    ║      ┌──────────┐ ┌──────────┐ ┌──────────┐            ║
    ║      │ worker 1 │ │ worker 2 │ │ worker 3 │            ║
    ║      └─────┬────┘ └─────┬────┘ └─────┬────┘            ║
    ║            │            │            │                 ║
    ║    segment │    segment │    segment │                 ║
    ║      proof │      proof │      proof │                 ║
    ║            ▼            ▼            ▼                 ║
    ║         ┌──────────────────────────────┐               ║
    ║         │      Aggregation tree        │               ║
    ║         └──────────────┬───────────────┘               ║
    ╚════════════════════════│═══════════════════════════════╝
                             │
                             ▼
                    ┌─────────────────┐
                    │   Final proof   │
                    └────────┬────────┘
                             │
                             │ return proof
                             ▼
                    ┌─────────────────────────┐
                    │    Host application     │
                    └─────────────────────────┘
```

### The coordinator

The coordinator is the only stateful process in the cluster. It
exposes a public gRPC interface that hosts use to submit proof
requests, poll job status, and retrieve results. From the host's
point of view, the coordinator is the only endpoint it ever talks
to; workers are an invisible implementation detail.

Internally, the coordinator splits each job into segments, assigns
them to workers, and returns the final proof. It also caches the
proving keys derived from each uploaded guest ELF, so subsequent
jobs for the same program skip the expensive setup step.

### Workers

Workers are the proving processes. Each worker connects outbound to
the coordinator and waits for segment assignments. Workers are
stateless across jobs, holding only the segments they are currently
proving. You can add, remove, or restart them without touching the
coordinator or losing cluster state.

The first worker to send its partial proof to the coordinator is
automatically promoted to aggregator for that job. The aggregator
collects the remaining segment proofs and assembles the final proof,
then returns it to the coordinator.

### Proving pipeline

Once a job is submitted, the coordinator selects workers from the
available pool and runs three phases:

1. **Partial contributions.** Each assigned worker processes its
   segments and returns partial challenges. The coordinator collects
   them and derives a single global challenge.
2. **Prove.** The coordinator broadcasts the global challenge to all
   workers. Each worker computes its partial proofs and returns
   them.
3. **Aggregation.** As partial proofs arrive, the coordinator builds
   an opportunistic binary aggregation tree, folding proofs in as
   they land. The first worker to deliver its partial proof is
   promoted to aggregator and assembles the final proof.

```
Client            Coordinator            Workers
     │                    │                    │
     │  prove(request)    │                    │
     ├───────────────────>│                    │
     │                    │  assign segments   │
     │                    ├───────────────────>│
     │                    │                    │
     │       ╔════════════╧════════════════════╧════════════╗
     │       ║   Phase 1: Partial contributions             ║
     │       ╚════════════╤════════════════════╤════════════╝
     │                    │ partial challenges │
     │                    │<───────────────────┤
     │                    │                    │
     │       ╔════════════╧════════════════════╧════════════╗
     │       ║   Phase 2: Prove                             ║
     │       ╚════════════╤════════════════════╤════════════╝
     │                    │  global challenge  │
     │                    ├───────────────────>│
     │                    │                    │
     │                    │   partial proofs   │
     │                    │<───────────────────┤
     │                    │                    │
     │       ╔════════════╧════════════════════╧════════════╗
     │       ║   Phase 3: Aggregation                       ║
     │       ║   ┌──────────────────────────────┐           ║
     │       ║   │  First worker to reply       │           ║
     │       ║   │  becomes aggregator          │           ║
     │       ║   └──────────────────────────────┘           ║
     │       ╚════════════╤════════════════════╤════════════╝
     │                    │     aggregate      │
     │                    ├───────────────────>│
     │                    │                    │
     │                    │    final proof     │
     │                    │<───────────────────┤
     │   return proof     │                    │
     │<───────────────────┤                    │
     │                    │                    │
     ▼                    ▼                    ▼
```

---

## Quickstart: single-host cluster

This brings up one coordinator and one worker on the same machine,
then submits a real proving job. It is the smallest deployment that
exercises the production binaries end-to-end.

### Prerequisites

- Linux x86_64
- Rust toolchain (`cargo --version` should work)
- ~32 GB free RAM (Assembly emulator preallocates large shared regions)
- Zisk installed. Follow installation guide.

Clone the repo:

```bash
git clone https://github.com/0xPolygonHermez/zisk.git
cd zisk
```
### Start the coordinator

```bash
zisk-coordinator
```

The coordinator binds three default ports on startup:

| Port  | Purpose                                                     |
| ---   | ---                                                         |
| 7000  | Client-facing gRPC API. Host applications connect here.     |
| 50051 | Worker-facing gRPC port. Workers connect here.              |
| 9090  | Prometheus metrics endpoint and `/health` liveness probe.   |

If the coordinator exits with `Address already in use`, override the
offending port:

```bash
zisk-coordinator --api-port 8000 --cluster-port 60000 --metrics-port 5245
```

### Start a worker

In a second terminal:

```bash
zisk-worker --config distributed/deploy/config/worker.toml --gpu
```

`worker.toml` points the worker at `http://127.0.0.1:50051`, advertises
ten compute units, and sets the log level to debug. On a successful
handshake:

```
INFO connecting to coordinator http://127.0.0.1:50051
INFO registered as worker <random-uuid> (capacity 10)
INFO heartbeat ok
```

The coordinator logs the matching side:

```
INFO worker registered: <random-uid> capacity=10
```

### Health check

With the coordinator and worker both running, verify the cluster in
two steps: a liveness probe and an end-to-end proving job.

**Liveness probe.** In a third terminal:

```bash
curl http://127.0.0.1:9090/health
```

A healthy coordinator returns `200 OK` with an empty body.

**Smoke-test proof.** Submit a real job from the included example:

```bash
cd examples/sha-hasher/host
cargo run --release --bin prove-remote
```

The `prove-remote` binary builds a `ProverClient::remote("http://127.0.0.1:7000")`,
uploads the guest ELF, and waits for the final proof. End-to-end:
the coordinator splits the trace into segments and hands them to the
worker, the worker produces STARK proofs, and the coordinator
aggregates them into the final proof. Terminals 1 and 2 show the
matching coordinator and worker activity.

---

## Deployment with scripts

This section deploys the same two binaries on bare Linux hosts under
systemd, the canonical path for a ZisK cluster.

### Prerequisites

- Linux x86_64
- ~32 GB free RAM (Assembly emulator preallocates large shared regions)
- Zisk installed in each machine. Follow installation guide.

Clone the repo:

```bash
git clone https://github.com/0xPolygonHermez/zisk.git
cd zisk
```

### Install the coordinator

On the coordinator host:

```bash
sudo distributed/deploy/scripts/coordinator/install.sh
```

The script:

- Creates the `zisk-coordinator` system user
- Drops the binary at `/usr/local/bin/zisk-coordinator`
- Writes the config at `/etc/zisk/coordinator.toml` (mode `640`)
- Creates the working directory at `/var/lib/zisk`
- Installs the systemd unit and runs `systemctl enable --now`

Verify the service:

```bash
sudo systemctl status zisk-coordinator
sudo journalctl -u zisk-coordinator -f
```

You should see the same two "listening on" lines from the
quickstart. If the service is `failed`, shows the
underlying error (most often a port conflict or missing config
field).

#### Configure the coordinator

Every setting is optional; the binary falls back to a built-in
default for anything you leave out.

Override precedence (later wins): built-in defaults → config file →
`ZISK_COORDINATOR_*` environment variables → CLI flags.

Edit `/etc/zisk/coordinator.toml`:

**`[service]`** — coordinator identity.

| Setting       | Default              | Notes                                                            |
| ---           | ---                  | ---                                                              |
| `name`        | `"ZisK Coordinator"` | Shown in logs and status output.                                 |
| `environment` | `development`        | One of `development`, `staging`, `production`. Use `production`. |

**`[server]`** — client-facing gRPC API.

| Setting                     | Default     | Notes                                                                  |
| ---                         | ---         | ---                                                                    |
| `host`                      | `0.0.0.0`   | Listen address. Bind to a specific interface to restrict access.       |
| `port`                      | `7000`      | Client gRPC port. CLI: `--api-port`, env: `ZISK_COORDINATOR_API_PORT`. |
| `shutdown_timeout_seconds`  | `30`        | Drain time after a shutdown signal before forced exit.                 |

**`[coordinator]`** — worker-facing port and core tuning.

| Setting       | Default | Notes                                                                       |
| ---           | ---     | ---                                                                         |
| `port`        | `50051` | Worker gRPC port. CLI: `--cluster-port`, env: `ZISK_COORDINATOR_CLUSTER_PORT`. |
| `config_file` | (none)  | Optional path to a coordinator-core tuning file.                            |

**`[metrics]`** — Prometheus endpoint.

| Setting   | Default   | Notes                                                                     |
| ---       | ---       | ---                                                                       |
| `enabled` | `true`    | Set `false` to disable `/metrics`. `/health` stays available either way.  |
| `host`    | `0.0.0.0` | Listen address for the scrape endpoint.                                   |
| `port`    | `9090`    | Scrape port. CLI: `--metrics-port`, env: `ZISK_COORDINATOR_METRICS_PORT`. |

**`[logging]`** — what gets logged and where.

| Setting     | Default  | Notes                                                                              |
| ---         | ---      | ---                                                                                |
| `level`     | `info`   | `trace`, `debug`, `info`, `warn`, `error`. `RUST_LOG` takes precedence.            |
| `format`    | `pretty` | `pretty`, `json` (production aggregators), or `compact`.                           |
| `file_path` | (none)   | Rotating daily log file. Leave unset on systemd hosts; journald captures stdout.   |

After editing:

```bash
sudo systemctl restart zisk-coordinator
```

### Install workers

Workers need the proving keys on local disk before they can start.
On each worker host, download and extract them first. Alternatively, use `mv` instead of `cp` to 
relocate the proving key without duplicating it. Be aware that other tools (such as `cargo-zisk`) relying on the 
original location will no longer find it.

```bash
cargo-zisk check-setup --gpu
sudo cp $HOME/.zisk/provingKey /var/lib/zisk-worker/
sudo chown zisk-worker:zisk-worker -R /var/lib/zisk-worker/provingKey
```

Then run the installer, pointing it at the extracted directory:

```bash
sudo distributed/deploy/scripts/worker/install.sh \
    --proving-key /var/lib/zisk-worker/provingKey --gpu
```

If you skip `--proving-key`, the installer falls back to
`/var/lib/zisk-worker/provingKey`; place (or symlink) the extracted
keys there if you prefer the default. The --gpu flag can be ommited 
if want to work with cpu.

The worker starts immediately, but its default coordinator URL
(`http://127.0.0.1:50051`) points at localhost — it will keep retrying
this address until you redirect it to your real coordinator.

#### Point the worker at the coordinator

Edit `/etc/zisk/worker.toml` and set the coordinator URL:

```toml
[coordinator]
url = "http://<coordinator-host>:50051"
```

```bash
sudo systemctl restart zisk-worker
sudo journalctl -u zisk-worker -f
```

Workers retry the connection every `reconnect_interval_seconds`
(default 5) until the coordinator answers. Start order does not
matter.

#### Configure the worker

Annotated example: `distributed/crates/worker/config/prod.toml`.

Override precedence: built-in defaults → config file →
`ZISK_WORKER__*` environment variables (double underscore as table
separator) → CLI flags.

Edit `/etc/zisk/worker.toml`:

**`[worker]`** — identity, capacity, on-disk location.

| Setting                          | Default                        | Notes                                                                                                                |
| ---                              | ---                            | ---                                                                                                                  |
| `worker_id`                      | random UUID                    | Pin to e.g. the hostname so log correlation works at scale.                                                          |
| `compute_capacity.compute_units` | `10`                           | Start at one unit per physical CPU core (minus two for OS overhead), plus one per GPU stream.                        |
| `environment`                    | `development`                  | `development` or `production`.                                                                                       |
| `inputs_folder`                  | `/var/lib/zisk-worker/inputs`  | Where the worker writes intermediate input files. Override only for a faster disk or separate partition.             |

**`[coordinator]`** — registration target.

| Setting | Default                    | Notes                                              |
| ---     | ---                        | ---                                                |
| `url`   | `http://127.0.0.1:50051`   | gRPC URL of the coordinator's worker-facing port.  |

**`[connection]`** — reaction to network trouble.

| Setting                      | Default | Notes                                                                |
| ---                          | ---     | ---                                                                  |
| `reconnect_interval_seconds` | `5`     | Backoff between reconnect attempts when the coordinator is unreachable. |
| `heartbeat_timeout_seconds`  | `30`    | How long to wait for a heartbeat before treating the connection dead.   |

**`[logging]`** — same shape as the coordinator's `[logging]`
table.

After editing:

* In Linux:

```bash
sudo systemctl restart zisk-worker
```

### Add more workers

Run the install script on as many hosts as you want. All workers
register against the same coordinator and receive work proportional
to their advertised capacity.

```
   ┌──────────────────────────────┐
   │      Application host        │
   │  ┌────────────────────────┐  │
   │  │     host program       │  │
   │  │     (RemoteClient)     │  │
   │  └───────────┬────────────┘  │
   └──────────────│───────────────┘
                  │
                  │ :7000
                  ▼
   ┌──────────────────────────────┐
   │      Coordinator host        │
   │  ┌────────────────────────┐  │
   │  │    zisk-coordinator    │  │
   │  │  :7000  :50051  :9090  │  │
   │  └───────────▲────────────┘  │
   └──────────────│───────────────┘
                  │
        ┌─────────┼─────────┐
        │ :50051  │ :50051  │ :50051
        │         │         │
   ┌────┴────┐┌───┴─────┐┌──┴──────┐
   │ Worker  ││ Worker  ││ Worker  │
   │ host A  ││ host B  ││ host C  │
   │(32 unit)││(32 unit)││(16 unit)│
   │┌───────┐││┌───────┐││┌───────┐│
   ││zisk-  ││││zisk-  ││││zisk-  ││
   ││worker ││││worker ││││worker ││
   │└───────┘││└───────┘││└───────┘│
   └─────────┘└─────────┘└─────────┘
```

### CLI references

A handful of operational knobs are CLI-only and not exposed in the
TOML:

| Flag                            | Default               | Description                              |
| ---                             | ---                   | ---                                      |
| `--proving-key`                 | `~/.zisk/provingKey`  | Path to the proving-key folder           |
| `--elf`                         | (none)                | Path to the ELF file                     |
| `--shared-tables`               | `false`               | Share tables when running in a cluster   |
| `--verify-constraints`          | `false`               | Verify constraints after witness gen     |
| `-n`, `--number-threads-witness`| (none)                | Threads for witness computation          |
| `-g`, `--gpu`                   | `false`               | Enable GPU mode (CUDA build only)        |
| `-t`, `--max-streams`           | (none)                | Maximum GPU streams                      |

CLI flags override the config file for one-off testing:

```bash
zisk-coordinator --api-port 8000 --cluster-port 60000 --log-level debug
zisk-worker --coordinator-url http://prod-coord:50051 --compute-capacity 32
```
