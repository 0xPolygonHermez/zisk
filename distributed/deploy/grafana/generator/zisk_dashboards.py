#!/usr/bin/env python3
"""Writes ZisK's Grafana dashboards as provisioning JSON, and the Prometheus recording rules they read.

    zisk_dashboards.py [<dashboards dir> <rules file>]

writes, by default, ../dashboards/ and ../../prometheus/rules/zisk-proof-time.yml next to it.

Every series comes from the coordinator's /metrics (`job="zisk-coordinator"`): its own metrics, and
what its workers report about their hosts (`coordinator_worker_*`). The host rows and the GPUs
dashboard read node_exporter (`job="node"`) and nvidia_gpu_exporter (`job="nvidia_gpu_exporter"`),
whose targets carry a `host` label. Every target may carry a `cluster` label; the dashboards'
Cluster selector filters on it.

The JSON is generated: change this script and run it again, rather than editing the JSON.
"""

import copy
import json
import os
import sys

DS = {"type": "prometheus", "uid": "prometheus"}

# The coordinator's series, in the selected cluster.
C = 'job="zisk-coordinator",cluster=~"$cluster"'
# A host's series, in the selected hosts.
H = 'host=~"$host"'


class Layout:
    def __init__(self):
        self.y = 0
        self.x = 0
        self.row_h = 0
        self.panels = []
        self.next_id = 1

    def row(self, title):
        self.newline()
        self.panels.append({"type": "row", "title": title, "id": self._id(), "collapsed": False,
                            "gridPos": {"x": 0, "y": self.y, "w": 24, "h": 1}, "panels": []})
        self.y += 1

    def newline(self):
        if self.x:
            self.y += self.row_h
        self.x, self.row_h = 0, 0

    def add(self, panel, w, h):
        if self.x + w > 24:
            self.newline()
        panel["id"] = self._id()
        panel["gridPos"] = {"x": self.x, "y": self.y, "w": w, "h": h}
        panel.setdefault("datasource", DS)
        self.panels.append(panel)
        self.x += w
        self.row_h = max(self.row_h, h)

    def _id(self):
        self.next_id += 1
        return self.next_id - 1


def targets(exprs, instant=False, fmt=None):
    out = []
    for i, (expr, legend) in enumerate(exprs):
        t = {"refId": chr(65 + i), "expr": expr, "legendFormat": legend, "datasource": DS}
        if instant:
            t["instant"], t["range"] = True, False
        if fmt:
            t["format"] = fmt
        out.append(t)
    return out


def timeseries(title, exprs, unit="short", stack=False, bars=False, desc=""):
    custom = {"drawStyle": "bars" if bars else "line", "fillOpacity": 60 if bars else 15,
              "lineWidth": 1, "showPoints": "auto", "spanNulls": False,
              "stacking": {"mode": "normal" if stack else "none", "group": "A"}}
    return {"type": "timeseries", "title": title, "description": desc, "targets": targets(exprs),
            "fieldConfig": {"defaults": {"unit": unit, "custom": custom}, "overrides": []},
            "options": {"legend": {"displayMode": "table", "placement": "bottom", "calcs": ["lastNotNull", "max"]},
                        "tooltip": {"mode": "multi", "sort": "desc"}}}


# No value at all (DOWN, none, nothing running, No data) takes the thresholds' base colour; any value
# takes the next step's. So the base is neutral and every value is green from the second step on.
NO_VALUE_COLOR = "text"
ANY_VALUE = -1e300


def stat(title, exprs, unit="short", desc="", decimals=None, red_above=None):
    steps = [{"color": NO_VALUE_COLOR, "value": None}, {"color": "green", "value": ANY_VALUE}]
    if red_above is not None:
        steps.append({"color": "red", "value": red_above})
    defaults = {"unit": unit, "thresholds": {"mode": "absolute", "steps": steps}}
    if decimals is not None:
        defaults["decimals"] = decimals
    return {"type": "stat", "title": title, "description": desc, "targets": targets(exprs),
            "fieldConfig": {"defaults": defaults, "overrides": []},
            "options": {"reduceOptions": {"calcs": ["lastNotNull"], "fields": "", "values": False},
                        "colorMode": "value", "graphMode": "area", "textMode": "value_and_name", "justifyMode": "auto"}}


def bargauge(title, exprs, unit="short", desc="", decimals=None):
    defaults = {"unit": unit, "min": 0, "thresholds": {"mode": "absolute", "steps": [{"color": "blue", "value": None}]}}
    if decimals is not None:
        defaults["decimals"] = decimals
    return {"type": "bargauge", "title": title, "description": desc, "targets": targets(exprs, instant=True),
            "fieldConfig": {"defaults": defaults, "overrides": []},
            "options": {"orientation": "horizontal", "displayMode": "gradient", "showUnfilled": True,
                        "reduceOptions": {"calcs": ["lastNotNull"], "fields": "", "values": False}}}


def table(title, expr, rename, value_unit="short", desc=""):
    return {"type": "table", "title": title, "description": desc,
            "targets": targets([(expr, "")], instant=True, fmt="table"),
            "fieldConfig": {"defaults": {"unit": value_unit}, "overrides": []},
            "options": {"showHeader": True, "sortBy": [{"displayName": rename.get("Value", "Value"), "desc": True}]},
            "transformations": [{"id": "organize", "options": {
                "excludeByName": {"Time": True, "__name__": True, "job": True, "instance": True, "host": True,
                                  "cluster": True},
                "renameByName": rename}}]}


def state_timeline(title, exprs):
    mappings = [{"type": "value", "options": {"0": {"text": "down", "color": "red"}, "1": {"text": "up", "color": "green"}}}]
    return {"type": "state-timeline", "title": title, "targets": targets(exprs),
            "fieldConfig": {"defaults": {"mappings": mappings, "custom": {"fillOpacity": 80}}, "overrides": []},
            "options": {"showValue": "never", "mergeValues": True, "rowHeight": 0.8, "legend": {"showLegend": False}}}


NAV = [("zisk-live", "🔴 LIVE"), ("zisk-overview", "Overview"), ("zisk-proofs", "Executions & proofs"),
       ("zisk-recovery", "Cancellations & recovery"), ("zisk-resources", "Worker resources"),
       ("zisk-gpus", "GPUs"), ("zisk-host", "Host")]


def nav_links():
    return [{"title": t, "type": "link", "url": f"/d/{uid}", "icon": "dashboard", "keepTime": True,
             "includeVars": True, "targetBlank": False, "tooltip": "", "asDropdown": False, "tags": []}
            for uid, t in NAV]


def cluster_variable():
    """The Cluster selector: the `cluster` label of the coordinators' targets (All when they have none)."""
    return {"type": "query", "name": "cluster", "label": "Cluster", "datasource": DS,
            "query": {"query": "label_values(coordinator_info, cluster)", "refId": "cluster"},
            "definition": "label_values(coordinator_info, cluster)", "refresh": 2, "includeAll": True,
            "allValue": ".*", "multi": False, "current": {"text": "All", "value": "$__all"}}


def identity_variables():
    """Hidden: the selected coordinator's configured name, environment and version (coordinator_info), for titles."""
    out = []
    for name in ("name", "environment", "version"):
        q = f"label_values(coordinator_info{{{C}}}, {name})"
        out.append({"type": "query", "name": f"coordinator_{name}", "datasource": DS, "hide": 2,
                    "query": {"query": q, "refId": name}, "definition": q, "refresh": 2})
    return out


def dashboard(uid, title, layout, desc):
    return {"uid": uid, "title": title, "description": desc, "tags": ["zisk"], "schemaVersion": 39,
            "time": {"from": "now-1h", "to": "now"}, "refresh": "10s", "timezone": "utc", "links": nav_links(),
            "editable": True, "panels": layout.panels,
            "templating": {"list": [cluster_variable()] + identity_variables()},
            "annotations": {"list": []}}


def identity_row(layout, what):
    """The dashboard's first row: which coordinator it shows."""
    layout.row(f"{what} · ${{coordinator_name}} · ${{coordinator_environment}} · v${{coordinator_version}}")


# How a job ended, the same colour everywhere: by series name (succeeded, cancelled, failed, failed: …).
OUTCOME_COLORS = [("^succeeded$|.*(success|finished|done).*", "green"), (".*cancel.*", "dark-blue"), (".*fail.*", "red")]


def outcome_overrides():
    return [{"matcher": {"id": "byRegexp", "options": rx},
             "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": color}}]}
            for rx, color in OUTCOME_COLORS]


def short_outcomes(expr):
    """`outcome` as the dashboards print it: succeeded, cancelled or failed."""
    return (f'label_replace(label_replace({expr}, "outcome", "succeeded", "outcome", "success"), '
            f'"outcome", "failed", "outcome", "failure")')


def by_outcome(window, reasons=False):
    """Jobs that ended in `window`, summed by outcome: succeeded, cancelled and failed, or with `reasons`
    each failure as "failed: <reason class>" (timeout, worker, other)."""
    counter = f"increase(coordinator_jobs_total{{{C}}}[{window}])"
    if not reasons:
        return short_outcomes(f"sum by (outcome) ({counter})")
    inner = short_outcomes(f"sum by (outcome, reason) ({counter})")
    return f'sum by (outcome) (label_replace({inner}, "outcome", "failed: $1", "reason", "(.+)"))'


def resources():
    l = Layout()
    identity_row(l, "Worker memory")
    m = f"coordinator_worker_cgroup_memory_bytes{{{C}"
    l.add(stat("Worker peak memory", [(f'{m},kind="peak"}}', "worker {{worker_id}}")], "bytes",
               "cgroup memory.peak since the worker's service started, as the worker reports"), 6, 4)
    l.add(stat("Worker memory now", [(f'{m},kind="current"}}', "worker {{worker_id}}")], "bytes"), 6, 4)
    l.add(stat("ZisK shared memory now", [(f"sum(coordinator_worker_shm_bytes{{{C}}})", "allocated")], "bytes",
               "Allocated pages of every ZISK_* segment in /dev/shm on the workers' hosts"), 6, 4)
    l.add(stat("Stale shared memory", [(f'sum(coordinator_worker_shm_bytes{{{C},owner_alive="false"}}) or vector(0)',
                                        "owner gone")], "bytes",
               "ZISK_* segments whose owner process no longer exists: held until the next worker start", red_above=1), 6, 4)
    l.add(timeseries("Worker cgroup memory", [(f'{m},kind=~"current|peak"}}', "worker {{worker_id}} {{kind}}")],
                     "bytes"), 12, 9)
    l.add(timeseries("Worker memory by type (memory.stat)", [(f'{m},kind=~"anon|file|shmem"}}',
                                                               "worker {{worker_id}} {{kind}}")], "bytes", stack=True), 12, 9)
    l.row("Shared memory (/dev/shm)")
    l.add(timeseries("ZisK shared memory by segment kind",
                     [(f"sum by (kind) (coordinator_worker_shm_bytes{{{C}}})", "{{kind}}")], "bytes", stack=True), 12, 9)
    l.add(timeseries("ZisK shared memory segments",
                     [(f"sum by (kind, owner_alive) (coordinator_worker_shm_segments{{{C}}})",
                       "{{kind}} (owner alive: {{owner_alive}})")]), 12, 9)
    l.row("Processes")
    p = f"coordinator_worker_process_memory_bytes{{{C}"
    l.add(timeseries("RSS by process", [(f'sum by (role) ({p},kind="rss"}})', "{{role}}")],
                     "bytes", stack=True, desc="rankN = a zisk-worker MPI rank; asm-<svc>-rankN = its ASM service"), 12, 10)
    l.add(bargauge("RSS now: anonymous and shared, by process",
                   [(f'{p},kind=~"anon|shmem"}}', "{{role}} {{kind}}")], "bytes"), 12, 10)
    l.row("CPU and host")
    l.add(timeseries("CPU cores in use per unit",
                     [(f"rate(coordinator_worker_cgroup_cpu_seconds_total{{{C}}}[1m])", "worker {{worker_id}}"),
                      (f"rate(process_cpu_seconds_total{{{C}}}[1m])", "coordinator")], "none"), 12, 8)
    l.add(timeseries("Host memory", [(f"node_memory_MemTotal_bytes{{{H}}} - node_memory_MemAvailable_bytes{{{H}}}", "{{host}} used"),
                                     (f"node_memory_Shmem_bytes{{{H}}}", "{{host}} shared (all tmpfs and shm)"),
                                     (f"node_memory_MemAvailable_bytes{{{H}}}", "{{host}} available")], "bytes"), 12, 8)
    d = dashboard("zisk-resources", "ZisK — Worker resources", l,
                  "Memory, shared memory, processes and CPU of the workers (as they report) and the coordinator")
    d["templating"]["list"].append(host_variable())
    return d


def recovery():
    l = Layout()
    identity_row(l, "Totals since the coordinator started")
    rec = f"coordinator_worker_recoveries_total{{{C}"
    l.add(stat("Recoveries done", [(f'sum({rec},result="done"}}) or vector(0)', "done")]), 4, 4)
    l.add(stat("Recovery timeouts", [(f'sum({rec},result="expired"}}) or vector(0)', "timeout")], red_above=1,
               desc="Parked workers that never reported their recovery and were unregistered"), 4, 4)
    l.add(stat("Aborts (status 75)", [(f"sum(coordinator_worker_exits_total{{{C}}}) or vector(0)", "aborts")], red_above=1,
               desc="Worker runs that ended themselves (a panic or a failed recovery aborts every rank), as each worker "
                    "reports when it registers again"), 4, 4)
    l.add(stat("Worker restarts", [(f'sum(increase(coordinator_worker_registrations_total{{{C},reconnect="false"}}[$__range]))',
                                    "restarts")], red_above=1, decimals=0,
               desc="Fresh registrations (a new worker process) in the selected range"), 4, 4)
    l.add(stat("Worker disconnects", [(f"sum(coordinator_worker_disconnects_total{{{C}}}) or vector(0)", "disconnects")]), 4, 4)
    l.add(stat("Last recovery", [(f"max(coordinator_last_worker_recovery_seconds{{{C}}})", "seconds")], "s", decimals=2), 4, 4)
    l.row("Availability")
    l.add(state_timeline("Units up", [(f"up{{{C}}}", "coordinator"),
                                      (f"coordinator_worker_state{{{C}}} > bool 0", "worker {{worker_id}}")]), 24, 6)
    s = f"coordinator_worker_start_to_seconds{{{C}"
    l.add(stat("Worker start → registration", [(f'{s},milestone="registration"}}', "worker {{worker_id}}")], "s",
               "After the last worker start", decimals=1), 6, 5)
    l.add(stat("Worker start → Ready", [(f'{s},milestone="ready"}}', "worker {{worker_id}}")], "s",
               "After the last worker start: registration plus its first program set up", decimals=1), 6, 5)
    p = timeseries("Job outcomes (per 5 min)", [(by_outcome("5m", reasons=True), "{{outcome}}")], bars=True)
    p["fieldConfig"]["overrides"] = outcome_overrides()
    l.add(p, 12, 5)
    l.row("Cancellations")
    l.add(timeseries("Recovery duration (each cancel)", [(f"max(coordinator_last_worker_recovery_seconds{{{C}}})", "last recovery")],
                     "s", "From the coordinator parking a worker after a cancelled or failed job to its recovery"), 12, 8)
    l.add(bargauge("Recoveries by duration (cumulative, ≤ le seconds)",
                   [(f"sum by (le) (coordinator_worker_recovery_seconds_bucket{{{C}}})", "≤ {{le}} s")]), 12, 8)
    l.add(timeseries("Recoveries (per 5 min)", [(f"sum by (result) (increase(coordinator_worker_recoveries_total{{{C}}}[5m]))",
                                                 "{{result}}")], bars=True), 12, 7)
    l.add(timeseries("Aborts by reason (per 5 min)", [(f"sum by (reason) (increase(coordinator_worker_exits_total{{{C}}}[5m]))",
                                                       "{{reason}}")], bars=True), 12, 7)
    l.add(table("Panics by source location", f"coordinator_worker_panics_total{{{C}}}",
                {"location": "Location", "Value": "Panics"}), 12, 6)
    l.add(table("Worker exits by reason", f"coordinator_worker_exits_total{{{C}}}",
                {"reason": "Reason", "Value": "Exits"}), 12, 6)
    return dashboard("zisk-recovery", "ZisK — Cancellations & recovery", l,
                     "Recoveries of parked workers, aborts, restarts and the time a worker takes to be back")


def proofs():
    l = Layout()
    identity_row(l, "Jobs since the coordinator started")
    jobs = f"coordinator_jobs_total{{{C}"
    l.add(stat("Proofs succeeded", [(f'sum({jobs},kind="prove",outcome="success"}}) or vector(0)', "proofs")]), 4, 4)
    l.add(stat("Executions succeeded", [(f'sum({jobs},kind="execute",outcome="success"}}) or vector(0)', "executions")]), 4, 4)
    p = stat("Cancelled", [(f'sum({jobs},outcome="cancelled"}}) or vector(0)', "cancelled")])
    p["fieldConfig"]["defaults"]["thresholds"]["steps"] = [{"color": NO_VALUE_COLOR, "value": None},
                                                          {"color": "dark-blue", "value": ANY_VALUE}]
    l.add(p, 4, 4)
    l.add(stat("Failed", [(f'sum({jobs},outcome="failure"}}) or vector(0)', "failed")], red_above=1), 4, 4)
    l.add(stat("Steps proved", [(f'sum(coordinator_job_steps_total{{{C},kind="prove"}})', "steps")], "short"), 4, 4)
    l.add(stat("Last proof", [(f'max(coordinator_last_job_mhz{{{C},kind="prove"}})', "MHz (steps/s ÷ 1e6)")], "none",
               decimals=1), 4, 4)
    l.row("Throughput and durations")
    steps = f"coordinator_job_steps_total{{{C}"
    l.add(timeseries("Proving throughput (MHz proved, 5 min average)",
                     [(f'sum(rate({steps},kind="prove"}}[5m])) / 1e6', "proved"),
                      (f'sum(rate({steps},kind="execute"}}[5m])) / 1e6', "executed only")], "none"), 12, 8)
    l.add(timeseries("Last proof: duration by phase",
                     [(f"coordinator_last_proof_phase_seconds{{{C}}}", "phase {{phase}}")], "s", stack=True,
                     desc="1 contributions (execution and witnesses), 2 prove, 3 aggregation"), 12, 8)
    ph = "coordinator_proof_phase_seconds"
    l.add(bargauge("Mean phase duration: warm vs first proof after a restart (cold)",
                   [(f"sum by (phase, cold) ({ph}_sum{{{C}}}) / sum by (phase, cold) ({ph}_count{{{C}}})",
                     "phase {{phase}}, cold={{cold}}")], "s", decimals=2), 12, 8)
    l.add(timeseries("Last job duration and size",
                     [(f"coordinator_last_job_duration_seconds{{{C}}}", "{{kind}} seconds"),
                      (f"coordinator_last_job_instances{{{C}}}", "{{kind}} instances")]), 12, 8)
    l.add(timeseries("Assembly execution speed (last job)", [(f"coordinator_last_asm_mhz{{{C}}}", "{{kind}}")], "none",
                     "MHz the worker's ASM executor reached"), 12, 8)
    l.add(bargauge("AIR instances in the last execution", [(f"coordinator_last_execution_air_instances{{{C}}}", "{{air}}")]), 12, 8)
    l.row("Inside a proof")
    last = lambda name, kind="prove": f'max({name}{{{C},kind="{kind}"}})'
    l.add(bargauge("Where phase 1 goes (last proof)",
                   [(last("coordinator_last_dispatch_delay_seconds"), "dispatch to the worker"),
                    (last("coordinator_last_asm_seconds"), "ASM execution"),
                    (f'max(coordinator_last_executor_seconds{{{C},kind="prove",part="count_and_plan"}})', "count and plan"),
                    (f'max(coordinator_last_executor_seconds{{{C},kind="prove",part="count_and_plan_mo"}})',
                     "count and plan (memory)"),
                    (f'max(coordinator_last_executor_seconds{{{C},kind="prove",part="execution"}})', "executor"),
                    (last("coordinator_last_witness_seconds"), "witness")], "s",
                   "The last proof's phase 1 on its last worker to report: how long its task took to reach it, the ASM "
                   "emulator, the executor's parts and the witnesses. The parts overlap: they are not a sum",
                   decimals=3), 12, 8)
    d = "coordinator_dispatch_delay_seconds_bucket"
    l.add(timeseries("Dispatch delay (coordinator → worker)",
                     [(f"histogram_quantile(0.5, sum by (le) (rate({d}{{{C}}}[5m])))", "p50"),
                      (f"histogram_quantile(0.95, sum by (le) (rate({d}{{{C}}}[5m])))", "p95")], "s",
                     "From the coordinator starting a phase to each worker receiving its task, over 5 minutes"), 12, 8)
    l.add(bargauge("Cost per type (last proof)",
                   [(f'coordinator_last_job_cost{{{C},kind="prove"}}', "{{type}}")], "short",
                   "The last successful proof's cost by state machine type"), 12, 8)
    p = timeseries("Cost mix over time (proofs)",
                   [(f'sum by (type) (rate(coordinator_job_cost_total{{{C},kind="prove"}}[5m]))', "{{type}}")],
                   "short", desc="Each type's share of the proved cost, over 5 minutes")
    p["fieldConfig"]["defaults"]["custom"]["stacking"] = {"mode": "percent", "group": "A"}
    p["fieldConfig"]["defaults"]["custom"]["fillOpacity"] = 60
    l.add(p, 12, 8)
    l.add(scatter("Proof time vs input size", "coordinator_proof_input_bytes", "Input", "decbytes"), 12, 9)
    l.add(scatter("Proof time vs steps", "coordinator_proof_steps", "Steps", "short"), 12, 9)
    l.row("Workers")
    l.add(bargauge("Phases 1 and 2 per worker (last proof)",
                   [(f"coordinator_last_proof_worker_phase_seconds{{{C}}}", "worker {{worker_id}} · phase {{phase}}")], "s",
                   "Each worker's time from the phase's start to its result, in the last proof", decimals=2), 12, 8)
    sp = "coordinator_proof_worker_spread_seconds"
    l.add(timeseries("Slowest minus fastest worker (mean over 5 min)",
                     [(f"sum by (phase) (rate({sp}_sum{{{C}}}[5m])) / sum by (phase) (rate({sp}_count{{{C}}}[5m]))",
                       "phase {{phase}}")], "s",
                     "In proofs on several workers: how much later the slowest worker finished each phase than the fastest"),
          12, 8)
    l.row("Programs and setups")
    l.add(table("Programs registered (ELF in the coordinator's cache)",
                f"coordinator_program_registered_timestamp_seconds{{{C}}} * 1000",
                {"hash_id": "hash_id", "bytes": "Bytes", "Value": "Registered at"}, "dateTimeAsIso"), 12, 7)
    l.add(table("Last setup duration per program", f"coordinator_last_setup_seconds{{{C}}}",
                {"hash_id": "hash_id", "Value": "Seconds"}, "s"), 8, 7)
    p = stat("Setups", [(f"sum by (outcome) (coordinator_setups_total{{{C}}})", "{{outcome}}")])
    p["fieldConfig"]["overrides"] = outcome_overrides()
    p["options"]["colorMode"] = "value"
    l.add(p, 4, 7)
    return dashboard("zisk-proofs", "ZisK — Executions & proofs", l,
                     "Jobs, throughput, phase durations, what a proof spends its time on, workers, programs and setups")


def once(expr):
    """Only each sample's own evaluation: a timed series' sample (one per proof, at its end) at the step it falls in."""
    return f"{expr} and ((time() - timestamp({expr})) * 1000 < $__interval_ms)"


def scatter(title, x_metric, x_name, x_unit):
    """Each successful proof as a point: its total time against `x_metric`, sampled at the same time."""
    total = f'coordinator_proof_seconds{{{C},phase="total"}}'
    return {"type": "xychart", "title": title,
            "description": f"One point per successful proof of the selected range (the first after a worker restart left out)",
            "targets": targets([(once(f"{x_metric}{{{C}}}"), x_name), (once(total), "Proof time")]),
            "transformations": [{"id": "joinByField", "options": {"byField": "Time", "mode": "inner"}}],
            "fieldConfig": {"defaults": {"custom": {"show": "points", "pointSize": {"fixed": 5}}},
                            "overrides": [{"matcher": {"id": "byName", "options": x_name},
                                           "properties": [{"id": "unit", "value": x_unit}]},
                                          {"matcher": {"id": "byName", "options": "Proof time"},
                                           "properties": [{"id": "unit", "value": "s"}]}]},
            "options": {"seriesMapping": "manual", "series": [{"x": x_name, "y": "Proof time", "name": "proofs"}],
                        "legend": {"showLegend": False}, "tooltip": {"mode": "single"}}}


STATES = {"0": ("disconnected", "red"), "1": ("idle", "green"), "2": ("setup", "purple"),
          "3": ("computing", "blue"), "4": ("recovering", "orange"), "5": ("error", "dark-red")}
STATE_MAPPINGS = [{"type": "value", "options": {k: {"text": t, "color": c, "index": int(k)} for k, (t, c) in STATES.items()}}]


def colored_table(title, expr, columns, color_column, color_rules, desc="", numbers=()):
    """An instant table of one series per row: label columns in order, newest first. `numbers` are
    (label, unit, decimals) to show as numbers in a unit rather than as text."""
    order = {name: i for i, (name, _) in enumerate(columns)}
    rename = dict(columns)
    exclude = {"Time": True, "__name__": True, "job": True, "instance": True, "host": True, "cluster": True,
               "worker_id": True, "seq": True}
    mappings = [{"type": "regex", "options": {"pattern": pat, "result": {"color": col}}} for pat, col in color_rules]
    overrides = [{"matcher": {"id": "byName", "options": rename[color_column]},
                  "properties": [{"id": "mappings", "value": mappings},
                                 {"id": "custom.cellOptions", "value": {"type": "color-text"}}]}]
    overrides += [{"matcher": {"id": "byName", "options": rename[label]},
                   "properties": [{"id": "unit", "value": unit}, {"id": "decimals", "value": decimals}]}
                  for label, unit, decimals in numbers]
    transformations = [{"id": "sortBy", "options": {"sort": [{"field": "Value", "desc": True}]}}]
    if numbers:
        transformations.append({"id": "convertFieldType", "options": {"conversions": [
            {"targetField": label, "destinationType": "number"} for label, _, _ in numbers]}})
    transformations.append({"id": "organize", "options": {"excludeByName": {**exclude, "Value": True},
                                                          "indexByName": order, "renameByName": rename}})
    return {"type": "table", "title": title, "description": desc,
            "targets": targets([(expr, "")], instant=True, fmt="table"),
            "transformations": transformations,
            "fieldConfig": {"defaults": {"custom": {"align": "left", "cellOptions": {"type": "auto"}}},
                            "overrides": overrides},
            "options": {"showHeader": True, "cellHeight": "sm", "footer": {"show": False}}}


def relabel(expr, label, mapping):
    """`expr` with `label`'s values renamed as `mapping` says."""
    for value, text in mapping:
        expr = f'label_replace({expr}, "{label}", "{text}", "{label}", "{value}")'
    return expr


def job_now():
    """The jobs in flight with a `short` label as Running now shows it: "a35d5b89 · ▶ 2/3 prove" while one runs,
    "a35d5b89 · ✔ finished" or "a35d5b89 · ✖ failed 06:13:05" for the one that just ended."""
    x = f"coordinator_job_now_seconds{{{C}}}"
    x = relabel(f'label_replace({x}, "step", "$1", "phase", "(.*)")', "step",
                [("starting", "▶ starting"), ("contributions", "▶ 1/3 contributions"), ("prove", "▶ 2/3 prove"),
                 ("aggregation", "▶ 3/3 aggregation"), ("execution", "▶ execution")])
    for status, text in (("success", "✔ finished"), ("cancelled", "✖ cancelled"), ("failure", "✖ failed")):
        x = f'label_replace({x}, "step", "{text}", "status", "{status}")'
    # A job that did not succeed also says when it ended.
    x = f'label_replace({x}, "tail", " $1", "ended_at", "(.+)")'
    x = f'label_replace({x}, "tail", "", "status", "success")'
    x = f'label_join({x}, "short", " · ", "job_id", "step")'
    return f'label_join({x}, "short", "", "short", "tail")'


def live():
    l = Layout()
    l.row("Cluster — as the coordinator sees it (valid wherever the workers run)")
    state = f"coordinator_worker_state{{{C}}}"
    l.add(stat("Workers connected", [(f"count({state} > 0) or vector(0)", "connected")]), 3, 5)
    l.add(stat("Workers by state", [(f"count({state} == 1) or vector(0)", "idle"),
                                    (f"count({state} == 3) or vector(0)", "computing"),
                                    (f"count({state} == 2) or vector(0)", "setup"),
                                    (f"count({state} == 4) or vector(0)", "recovering")]), 6, 5)
    p = stat("Each worker", [(state, "worker {{worker_id}}")])
    p["fieldConfig"]["defaults"]["mappings"] = STATE_MAPPINGS
    p["fieldConfig"]["defaults"]["thresholds"]["steps"][0]["color"] = "transparent"
    p["options"]["colorMode"] = "background"
    p["options"]["graphMode"] = "none"
    l.add(p, 4, 5)
    p = stat("Running now", [(job_now(), "{{short}}")], "s", decimals=2,
             desc="The jobs the coordinator is running now and for how long each has been running; once they end, "
                  "for 2 minutes or until the next starts, how the last one ended and its real duration")
    p["fieldConfig"]["defaults"]["noValue"] = "nothing running"
    p["fieldConfig"]["defaults"]["thresholds"]["steps"][0]["color"] = "transparent"
    p["options"]["colorMode"] = "background"
    p["fieldConfig"]["overrides"] = [
        {"matcher": {"id": "byRegexp", "options": rx},
         "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": color}}]}
        for rx, color in ((".*▶.*", "blue"), (".*finished.*", "green"), (".*cancelled.*", "dark-blue"), (".*failed.*", "red"))]
    l.add(p, 7, 5)
    p = stat("API clients now", [(f"coordinator_api_connections{{{C}}} > 0", "{{ip}}")],
             desc="Open connections to the coordinator's API, by client address")
    p["fieldConfig"]["defaults"]["noValue"] = "none"
    p["options"]["graphMode"] = "none"
    l.add(p, 4, 5)
    p = stat("Data sources", [(f"up{{{C}}}", "coordinator"), (f'up{{job="node",{H}}}', "host {{host}}"),
                              (f'up{{job="nvidia_gpu_exporter",{H}}}', "GPUs {{host}}")],
             desc="The coordinator feeds almost all of this dashboard: if it is DOWN, empty panels do not mean idle")
    p["fieldConfig"]["defaults"]["mappings"] = [{"type": "value", "options": {"1": {"text": "UP", "color": "green"},
                                                                            "0": {"text": "DOWN", "color": "red"}}}]
    p["fieldConfig"]["defaults"]["noValue"] = "DOWN"
    p["fieldConfig"]["defaults"]["thresholds"]["steps"][0]["color"] = "red"
    p["options"]["colorMode"] = "background"
    p["options"]["graphMode"] = "none"
    l.add(p, 4, 4)
    # One query each, so that they read succeeded, cancelled, failed rather than in the label's order.
    ended = [(f'round(sum(increase(coordinator_jobs_total{{{C},outcome="{outcome}"}}[$__range])) or vector(0))', name)
             for outcome, name in (("success", "succeeded"), ("cancelled", "cancelled"), ("failure", "failed"))]
    p = stat("Jobs in the selected range", ended, desc="Jobs that ended in the dashboard's time range, by outcome")
    p["options"]["colorMode"] = "value"
    p["fieldConfig"]["overrides"] = outcome_overrides()
    l.add(p, 6, 4)
    l.add(stat("Programs registered", [(f"coordinator_registered_programs_total{{{C}}}", "programs")]), 3, 4)
    l.add(stat("Compute capacity", [(f'coordinator_compute_units{{{C},kind="total"}}', "CU")],
               desc="Compute units of the connected workers"), 3, 4)
    # Each successful proof is one sample at the time it ended (coordinator_proof_seconds), so statistics over
    # $__range are exact per proof. "@ end()" pins them to the selected range's end: a constant over the panel,
    # i.e. a horizontal line, the same value the box shows.
    phases = [("1", "1 contributions"), ("2", "2 prove"), ("3", "3 aggregation")]
    DUR = relabel(f"coordinator_proof_seconds{{{C}}}", "phase", phases)
    TOTAL = f'coordinator_proof_seconds{{{C},phase="total"}}'
    BAD = relabel(f"coordinator_job_unsuccessful_seconds{{{C}}}", "outcome", [("failure", "failed")])
    p = timeseries("Proof duration per job (successful proofs; failed and cancelled as points)",
                   [(once(DUR), "{{phase}}"),
                    (once(BAD), "{{outcome}} (time to the end)"),
                    (f"avg_over_time({TOTAL}[$__range] @ end())", "mean (total)"),
                    (f"quantile_over_time(0.999, {TOTAL}[$__range] @ end())", "P99.9 (total)")], "s",
                   desc="One point per successful proof: each phase and the total; the dashed lines are the mean "
                        "and P99.9 of the total over the selected time range")
    p["fieldConfig"]["defaults"]["custom"].update({"showPoints": "always", "pointSize": 4, "spanNulls": True})
    # One line under the chart instead of a table of one row per series.
    p["options"]["legend"] = {"showLegend": True, "displayMode": "list", "placement": "bottom", "calcs": []}
    colors = {"1 contributions": "blue", "2 prove": "purple", "3 aggregation": "yellow", "total": "green",
              "mean (total)": "orange", "P99.9 (total)": "red"}
    # Failed and cancelled jobs: big points, no line, outside the statistics.
    bad_points = [{"id": "custom.drawStyle", "value": "points"}, {"id": "custom.pointSize", "value": 11},
                  {"id": "custom.showPoints", "value": "always"}]
    extra_overrides = [
        {"matcher": {"id": "byName", "options": "failed (time to the end)"},
         "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": "red"}}] + bad_points},
        {"matcher": {"id": "byName", "options": "cancelled (time to the end)"},
         "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": "dark-blue"}}] + bad_points}]
    p["fieldConfig"]["overrides"] = [
        {"matcher": {"id": "byName", "options": name},
         "properties": [{"id": "color", "value": {"mode": "fixed", "fixedColor": color}}]
         + ([{"id": "custom.lineStyle", "value": {"fill": "dash", "dash": [10, 6]}},
             {"id": "custom.showPoints", "value": "never"}, {"id": "custom.lineWidth", "value": 2}]
            if name.endswith("(total)") else [])
         + ([{"id": "custom.lineWidth", "value": 2}] if name == "total" else [])}
        for name, color in colors.items()] + extra_overrides
    l.add(p, 16, 12)
    base_y = p["gridPos"]["y"]
    # Beside the chart, a 2 x 3 grid of small boxes, one statistic each: a single box with
    # six values lets Grafana squeeze and mix them.
    grid = [(f"avg_over_time({TOTAL}[$__range])", "Mean"),
            (f"quantile_over_time(0.5, {TOTAL}[$__range])", "Median"),
            (f"min_over_time({TOTAL}[$__range])", "Min"),
            (f"max_over_time({TOTAL}[$__range])", "Max"),
            (f"stddev_over_time({TOTAL}[$__range])", "Std dev"),
            (f"quantile_over_time(0.999, {TOTAL}[$__range])", "P99.9")]
    for i, (expr, name) in enumerate(grid):
        b = stat(name, [(expr, name)], "s", decimals=3,
                 desc="Total proof time of the successful proofs in the selected time range (the first proof "
                      "after a worker restart is left out)")
        b["fieldConfig"]["defaults"]["noValue"] = "–"
        b["options"]["textMode"] = "value"
        b["options"]["colorMode"] = "none"
        b["id"] = l._id()
        b["gridPos"] = {"x": 16 + 4 * (i % 2), "y": base_y + 4 * (i // 2), "w": 4, "h": 4}
        b["datasource"] = DS
        l.panels.append(b)
    l.x = 24
    p = state_timeline("Workers over time", [(state, "worker {{worker_id}}")])
    p["fieldConfig"]["defaults"]["mappings"] = STATE_MAPPINGS
    p["options"]["legend"] = {"showLegend": True, "displayMode": "list", "placement": "bottom"}
    l.add(p, 24, 6)
    # Left: the last jobs, and under them the failed ones with their error messages;
    # right: the events, as tall as both.
    in_range = lambda sel: f"{sel} > (time() - $__range_s) * 1000"
    recent = relabel(f"coordinator_recent_job{{{C}}}", "outcome", [("failure", "failed")])
    jobs = colored_table("Last 20 jobs", in_range(recent),
                         [("finished", "Finished (UTC)"), ("job_id", "Job"), ("kind", "Kind"), ("outcome", "Result"),
                          ("duration_seconds", "Duration"), ("phases", "Phases 1+2+3 (s)"), ("steps", "Steps"),
                          ("instances", "Instances"), ("mhz", "MHz")],
                         "outcome", [("success", "green"), ("cancelled", "dark-blue"), ("fail.*", "red")],
                         numbers=[("duration_seconds", "s", 2), ("steps", "locale", 0)])
    l.add(jobs, 14, 11)
    top = jobs["gridPos"]["y"]
    failures = colored_table("Failed proofs: error messages", in_range(f"coordinator_job_failure_info{{{C}}}"),
                             [("at", "At (UTC)"), ("job_id", "Job"), ("reason", "Reason (coordinator)"),
                              ("cause", "Cause (worker)"), ("input_bytes", "Input"), ("steps", "Steps")],
                             "cause", [("/./", "red")],
                             desc="Every failed job: the coordinator's reason, the worker's error it quotes, the job's "
                                  "input size and the steps its execution ran",
                             numbers=[("input_bytes", "decbytes", 2), ("steps", "short", 0)])
    failures["id"] = l._id()
    failures["gridPos"] = {"x": 0, "y": top + 11, "w": 14, "h": 9}
    failures["datasource"] = DS
    l.panels.append(failures)
    events = colored_table("Last 30 events", in_range(f"coordinator_recent_event{{{C}}}"),
                           [("at", "At (UTC)"), ("kind", "Event"), ("detail", "Detail")],
                           "kind", [("/(fail|abort|panic|timed out|expired|exited|disconnected)/i", "red"),
                                    ("/cancel/i", "dark-blue"),
                                    ("/(done|registered|reconnected|started)/i", "green")])
    events["id"] = l._id()
    events["gridPos"] = {"x": 14, "y": top, "w": 10, "h": 20}
    events["datasource"] = DS
    l.panels.append(events)
    l.x, l.row_h = 24, 20
    l.add(timeseries("gRPC requests by method", [(f"sum by (method) (rate(coordinator_requests_total{{{C}}}[1m]))", "{{method}}")],
                     "reqps"), 8, 8)
    p = timeseries("Jobs per minute", [(f"round({by_outcome('1m')})", "{{outcome}}")], bars=True, stack=True)
    p["fieldConfig"]["overrides"] = outcome_overrides()
    l.add(p, 8, 8)
    l.add(timeseries("Coordinator process", [(f"rate(process_cpu_seconds_total{{{C}}}[1m])", "CPU cores"),
                                             (f"process_resident_memory_bytes{{{C}}} / 2^30", "memory GiB")], "none"), 8, 8)
    # One row per host, repeated by $host.
    l.newline()
    l.panels.append({"type": "row", "title": "Host $host", "id": l._id(), "collapsed": False, "repeat": "host",
                     "gridPos": {"x": 0, "y": l.y, "w": 24, "h": 1}, "panels": []})
    l.y += 1
    p = stat("Runs here", [(f'count by (roles) (up{{job="node",{H}}})', "{{roles}}")])
    p["options"]["textMode"] = "name"
    p["options"]["graphMode"] = "none"
    l.add(p, 4, 4)
    l.add(stat("CPU busy", [(f'100 * (1 - avg(rate(node_cpu_seconds_total{{{H},mode="idle"}}[30s])))', "busy")], "percent", decimals=1), 4, 4)
    l.add(stat("Load (1 min)", [(f"node_load1{{{H}}}", "load")], "none", decimals=1), 4, 4)
    l.add(stat("RAM used", [(f"100 * (1 - node_memory_MemAvailable_bytes{{{H}}} / node_memory_MemTotal_bytes{{{H}}})", "used")], "percent", decimals=1), 4, 4)
    l.add(stat("GPU busy (mean)", [(f"100 * avg(nvidia_smi_utilization_gpu_ratio{{{H}}})", "busy")], "percent", decimals=0), 4, 4)
    l.add(stat("GPU memory used", [(f"100 * sum(nvidia_smi_memory_used_bytes{{{H}}}) / sum(nvidia_smi_memory_total_bytes{{{H}}})", "used")], "percent", decimals=0), 4, 4)
    l.add(timeseries("CPU", [(f'100 * (1 - avg(rate(node_cpu_seconds_total{{{H},mode="idle"}}[30s])))', "busy %"),
                             (f"node_load1{{{H}}}", "load 1m")], "none"), 12, 8)
    gpu = f"* on (uuid) group_left (index) nvidia_smi_gpu_info{{{H}}}"
    l.add(timeseries("GPU busy, per GPU", [(f"100 * nvidia_smi_utilization_gpu_ratio{{{H}}} {gpu}", "GPU {{index}}")], "percent"), 12, 8)
    l.add(timeseries("Memory", [(f'sum(coordinator_worker_cgroup_memory_bytes{{{C},kind="current"}})', "workers' cgroups"),
                                (f"sum(coordinator_worker_shm_bytes{{{C}}})", "ZisK shared memory"),
                                (f"node_memory_MemTotal_bytes{{{H}}} - node_memory_MemAvailable_bytes{{{H}}}", "host used")], "bytes"), 8, 8)
    l.add(timeseries("GPU memory, per GPU", [(f"nvidia_smi_memory_used_bytes{{{H}}} {gpu}", "GPU {{index}} memory")], "bytes"), 8, 8)
    l.add(timeseries("Network", [(f'sum(rate(node_network_receive_bytes_total{{{H},device!~"lo|veth.*|docker.*|br-.*"}}[30s]))', "in"),
                                 (f'sum(rate(node_network_transmit_bytes_total{{{H},device!~"lo|veth.*|docker.*|br-.*"}}[30s]))', "out")],
                     "Bps"), 8, 8)
    for p in l.panels:
        if p["type"] == "stat":
            for t in p["targets"]:
                t["instant"], t["range"] = True, False
            p["options"]["graphMode"] = "none"
    d = dashboard("zisk-live", "ZisK — Live", l, "What the cluster is doing now: workers, the job in flight, the last jobs and events, and each host's load")
    d["refresh"] = "1s"
    d["templating"]["list"].append(host_variable())
    d["annotations"] = {"list": job_annotations()}
    return d


def host_variable():
    return {"type": "query", "name": "host", "label": "Host", "datasource": DS,
            "query": {"query": 'label_values(up{job="node",cluster=~"$cluster"}, host)', "refId": "host"},
            "definition": 'label_values(up{job="node",cluster=~"$cluster"}, host)', "refresh": 2,
            "includeAll": True, "multi": True, "current": {"text": "All", "value": "$__all"}}


def job_annotations():
    """The job and worker events every time chart marks: worker restarts, cancellations and failures."""
    ann = lambda name, expr, color, title: {"name": name, "datasource": DS, "enable": True, "hide": True, "expr": expr,
                                             "iconColor": color, "step": "2s", "titleFormat": title, "useValueForTime": False}

    # Grafana evaluates annotations at a step of its own for the range ("step" is only its minimum: 2 min
    # at 3 h, 15 min at 24 h), so a few seconds' window only caught the events just before a step. Each
    # one compares a counter with its value a step earlier, its last value of the minute so that a failed
    # scrape's stale marker does not hide it.
    def rose(x):
        return f"((last_over_time({x}[1m]) - last_over_time({x}[1m] offset $__interval)) > 0)"

    fresh_registrations = f'coordinator_worker_registrations_total{{{C},reconnect="false"}}'
    cancelled = f'coordinator_jobs_total{{{C},outcome="cancelled"}}'
    failed_jobs = f'coordinator_jobs_total{{{C},outcome="failure"}}'
    failed = f"sum by (reason) ({rose(failed_jobs)})"
    # A worker that fails a job restarts within the same step; the later in the list is drawn on top.
    return [
        ann("Worker restarted", f"sum({rose(fresh_registrations)})", "purple", "worker restarted"),
        ann("Job cancelled", f"sum({rose(cancelled)})", "dark-blue", "job cancelled"),
        ann("Job failed", f'label_replace(label_replace({failed}, "outcome", "failed", "", ""), "outcome", "failed: $1", "reason", "(.+)")',
            "red", "job {{outcome}}")]


# The proof-time histogram's bins. Prometheus records each proof's bin as it ends (`proof_time_rules`)
# and the histogram only adds the records up: binning the proofs at query time took longer than the
# Live dashboard's 1 s refresh from 6 h of range on, so the panel never drew.
PROOF_BINS_PER_S = 4  # 250 ms, about the Freedman-Diaconis width of an hour of proofs
PROOF_LAST_BIN = 30 * PROOF_BINS_PER_S  # the last bin's index: 30 s and above
PROOF_RECORD_LAG = 10  # s from a proof's end to its record, for its sample to be scraped first
PROOF_ONCE = "coordinator:proof_seconds_total:once"  # each proof's total time, once
PROOF_BIN = "coordinator:proof_seconds_total:bin"  # each proof as a 1 on its bin's series (label `bin`)


def proof_bin_index(x):  # the index of the bin a duration falls in
    return f"clamp_max(floor(({x}) * {PROOF_BINS_PER_S}), {PROOF_LAST_BIN})"


def proof_time_rules():
    """The Prometheus recording rules (YAML) behind the proof-time histogram, evaluated every second.

    They read the same samples as "Proof time per job": one per successful proof at its end, the
    first after a worker restart left out. The evaluation at t records the proof that ended in
    (t - 11 s, t - 10 s], so each one exactly once (two ending in the same second would count as one)."""
    total = 'coordinator_proof_seconds{phase="total"}'
    lagged = f"{total} offset {PROOF_RECORD_LAG}s"
    once_ = f"{lagged} and ((time() - {PROOF_RECORD_LAG} - timestamp({lagged})) < 1)"
    rules = [(PROOF_ONCE, once_), (PROOF_BIN, f'count_values without () ("bin", {proof_bin_index(once_)})')]
    lines = ["# Generated by distributed/deploy/grafana/generator/zisk_dashboards.py: change it and run it again.",
             "groups:", "  - name: zisk-proof-time", "    interval: 1s", "    rules:"]
    for record, expr in rules:
        lines += [f"      - record: {record}", f"        expr: {json.dumps(expr)}"]
    return "\n".join(lines) + "\n"


PROOF_MAX_BARS = 16  # what the histogram's height fits with its 22 px text, about 24 px a bar
PROOF_WIDTHS = (1, 2, 4, 8)  # the bar widths Auto steps through, in 250 ms bins: 250 ms, 500 ms, 1 s, 2 s
# (every proof of 30 s and above shares one bin, so 2 s bars never come to more than 16)


def proof_time_bins():
    """PromQL for the proofs of the selected range per bar of total proof time, from the records of
    `proof_time_rules`: the counts (`rows`, label `g`, the bar's number), the same with each bar's name
    (`named`, its start: "4.00" is 4.00 up to the next bar; the last, "≥ 10.00", holds every slower proof),
    and the counts with their shares as text (`labelled`).

    The bars span the range's fastest to slowest proof, empty ones included. Their width is the "Bin"
    selector's, or with Auto the narrowest of PROOF_WIDTHS that needs at most PROOF_MAX_BARS bars. The
    "Up to" selector puts every proof from that time on (rounded up to a whole bar) in the last bar; with
    All that is 30 s, as in the records. A proof joins the range PROOF_RECORD_LAG seconds after it ends.

    The 250 ms bins are labels, which PromQL cannot compute with: `index` holds each bin's number as its
    value, `count_values` turns the numbers computed from it back into labels."""
    once_ = f"{PROOF_ONCE}{{{C}}}"
    fine = f"sum by (bin) (sum_over_time({PROOF_BIN}{{{C}}}[$__range]))"
    index = "(" + " or ".join(f'label_replace(vector({b}), "bin", "{b}", "", "")' for b in range(PROOF_LAST_BIN + 1)) + ")"
    bars_index = f'max by (g) (label_replace({index}, "g", "$1", "bin", "(.*)"))'  # the bars' numbers, label g
    # In 250 ms bins: the selectors give 0 for Auto and for All; the fastest and the slowest proof.
    up_to = f"({PROOF_LAST_BIN} * ($proof_up_to == bool 0) + $proof_up_to)"
    lo = f"floor(min(min_over_time({once_}[$__range])) * {PROOF_BINS_PER_S})"
    hi = f"clamp_max(floor(max(max_over_time({once_}[$__range])) * {PROOF_BINS_PER_S}), {PROOF_LAST_BIN})"

    def top(k):  # the last bar's first bin with bars k bins wide: "Up to", rounded up to a whole bar
        return f"(scalar(ceil(vector({up_to} / {k}))) * {k})"

    def bars(k):  # how many bars k bins wide the range needs, the last one included
        return f"scalar(floor(clamp_max({hi}, {top(k)}) / {k}) - floor({lo} / {k}) + 1)"

    auto = str(PROOF_WIDTHS[-1])
    for w in reversed(PROOF_WIDTHS[:-1]):  # w, unless it needs more than PROOF_MAX_BARS bars: then the next
        auto = f"({w} + ({bars(w)} > bool {PROOF_MAX_BARS}) * ({auto} - {w}))"
    k = f"($proof_bin + ($proof_bin == bool 0) * {auto})"
    last = f"({top(k)} / {k})"  # the last bar's number

    bar_of_bin = f'count_values by (bin) ("g", clamp_max(floor({index} / {k}), {last}))'
    counts = f"sum by (g) ({fine} * on (bin) group_left (g) {bar_of_bin})"
    span = (f"(({bars_index} >= scalar(floor({lo} / {k}))) <= scalar(floor(clamp_max({hi}, {top(k)}) / {k})))")
    rows = f"({counts} or ({span} * 0))"

    # Each bar's start in seconds as its name, negative for the last bar, which becomes "≥ …".
    start = f"({bars_index} * {k} / {PROOF_BINS_PER_S} * (1 - 2 * ({bars_index} >= bool {last})))"
    names = f'count_values by (g) ("range", {start})'
    names = f'label_replace({names}, "range", "$1.00", "range", "(-?[0-9]+)")'  # "4" → "4.00"
    names = f'label_replace({names}, "pad", "0", "range", "-?[0-9]+[.][0-9]")'  # "4.5" → "4.50"
    names = f'label_replace(label_join({names}, "range", "", "range", "pad"), "pad", "", "", "")'
    names = f'label_replace({names}, "range", "≥ $1", "range", "-(.*)")'
    named = f"({rows} * on (g) group_left (range) {names})"

    def labelled():  # per bar, its count as text and what the bar shows for it: "1032 (67.1%)"
        share = f'count_values by (g) ("pct", round(100 * {rows} / scalar(sum({fine})), 0.1))'
        out = f'(count_values by (g) ("cnt", {rows}) * on(g) group_left(pct) {share})'
        out = f'label_join({out}, "txt", " (", "cnt", "pct")'
        return f'label_replace({out}, "txt", "$1%)", "txt", "(.+)")'

    return rows, named, labelled


def proof_time_histogram():
    """The bins as horizontal bars, as the terminal's histogram: the bins down the y axis (fastest at the top),
    the proofs along x with their count and share at the end of each bar.
    (Grafana's histogram panel only draws vertically.)"""
    rows, named, labelled = proof_time_bins()
    return {"type": "barchart", "title": "Proof time histogram · ${range_text}", "datasource": DS,
            "description": "Successful proofs of the selected range by their total time (the first proof after a "
                           "worker restart left out, as in the chart above), from the fastest to the slowest, with "
                           "each bar's count and share. Each bar is named by its start: 4.00 is 4.00 up to the next "
                           "bar. Bin sets the bars' width; Auto takes 250 ms, or 500 ms, 1 s or 2 s when the range "
                           "needs more than 16 bars. Up to puts every slower proof in the last bar (≥); All stops at "
                           "30 s. The statistics beside are of the same proofs. A proof shows here 10 s after it ends",
            # A: the counts with the bins' names. B: per count, the text its bar shows; it becomes value
            # mappings of the counts (a count always has the same share, so a mapping cannot mix two bins).
            # C: the bars' axis maximum, with room after the longest bar for its text.
            "targets": targets([(named, ""), (labelled(), ""), (f"max({rows}) * 1.3", "")],
                               instant=True, fmt="table"),
            "transformations": [
                {"id": "configFromData", "options": {
                    "configRefId": "B", "applyTo": {"id": "byName", "options": "Value #A"},
                    "mappings": [{"fieldName": "cnt", "handlerKey": "mappings.value"},
                                 {"fieldName": "txt", "handlerKey": "mappings.text"},
                                 {"fieldName": "Time", "handlerKey": "__ignore"},
                                 {"fieldName": "g", "handlerKey": "__ignore"},
                                 {"fieldName": "pct", "handlerKey": "__ignore"},
                                 {"fieldName": "Value #B", "handlerKey": "__ignore"}]}},
                {"id": "configFromData", "options": {
                    "configRefId": "C", "applyTo": {"id": "byName", "options": "Value #A"},
                    "mappings": [{"fieldName": "Value #C", "handlerKey": "max"},
                                 {"fieldName": "Time", "handlerKey": "__ignore"}]}},
                {"id": "convertFieldType", "options": {"conversions": [{"targetField": "g", "destinationType": "number"}]}},
                {"id": "sortBy", "options": {"sort": [{"field": "g", "desc": False}]}},
                {"id": "organize", "options": {
                    "includeByName": {"range": True, "Value #A": True},
                    "renameByName": {"range": "Time (s)", "Value #A": "Proofs"},
                    "indexByName": {"range": 0, "Value #A": 1}}}],
            "fieldConfig": {"defaults": {"unit": "none", "decimals": 0, "min": 0,
                                         "color": {"mode": "fixed", "fixedColor": "green"},
                                         "custom": {"fillOpacity": 85, "gradientMode": "none", "lineWidth": 1,
                                                    "axisPlacement": "auto", "axisSoftMin": 0}},
                            # The counts are on the bars; the value mappings would also rename the axis' ticks.
                            "overrides": [{"matcher": {"id": "byName", "options": "Proofs"},
                                           "properties": [{"id": "custom.axisPlacement", "value": "hidden"}]},
                                          # As written: as a number, "4.00" would be shown as "4".
                                          {"matcher": {"id": "byName", "options": "Time (s)"},
                                           "properties": [{"id": "unit", "value": "string"}]}]},
            "options": {"orientation": "horizontal", "xField": "Time (s)", "showValue": "always",
                        "stacking": "none", "groupWidth": 0.9, "barWidth": 0.95, "barRadius": 0,
                        "xTickLabelRotation": 0, "xTickLabelSpacing": 0, "fullHighlight": False,
                        "legend": {"showLegend": False, "displayMode": "list", "placement": "bottom"},
                        "tooltip": {"mode": "single", "sort": "none"}, "text": {"valueSize": 22}}}


def live2():
    """The Live dashboard: live()'s panels with the top reorganised (connection and services, then jobs,
    then proof time). It replaced the original layout, which live() still builds as its source."""
    base = live()
    panels = {p["title"]: copy.deepcopy(p) for p in base["panels"]}
    out = []
    next_id = [1]

    def put(p, x, y, w, h):
        p = copy.deepcopy(p)
        p["id"] = next_id[0]
        next_id[0] += 1
        p["gridPos"] = {"x": x, "y": y, "w": w, "h": h}
        out.append(p)
        return p

    def row(title, y):
        put({"type": "row", "title": title, "collapsed": False, "panels": []}, 0, y, 24, 1)

    # Grafana sizes a stat's names from the box height (15 %): 12 px in the 4-row boxes, ~17 px in the 5-row
    # ones. Fixed sizes keep the names the same size side by side, and Last job's and Jobs' values the same in both.
    NAME_PX, VALUE_PX = 12, 40

    def sized(p, value=False):
        p["options"]["text"] = {"titleSize": NAME_PX, **({"valueSize": VALUE_PX} if value else {})}
        return p

    # One section: the connection and services; the jobs stacked left of the proof-time chart (the stack as
    # wide as two statistics, as tall as the chart); the histogram with the statistics; then the rest of Live.
    row("Cluster · ${coordinator_host} · ${coordinator_name} · ${coordinator_environment} · v${coordinator_version}", 0)
    sized(put(panels["Data sources"], 0, 1, 4, 4))
    put(panels["API clients now"], 4, 1, 4, 4)
    put(panels["Workers connected"], 8, 1, 3, 4)
    w = put(panels["Workers by state"], 11, 1, 6, 4)
    w["options"].update({"orientation": "vertical", "textMode": "value_and_name"})  # side by side, one line
    put(panels["Programs registered"], 17, 1, 3, 4)
    put(panels["Compute capacity"], 20, 1, 4, 4)
    sized(put(panels["Each worker"], 0, 5, 3, 4))
    rn = sized(put(panels["Running now"], 3, 5, 5, 4))
    rn["options"]["wideLayout"] = False  # the job's line on top, the seconds under it
    # The units go in the names: Grafana breaks a value from its unit ("126.0" / "MHz") when the box is narrow,
    # and the taller box's name then sits higher than the others. Bare numbers stay on one line, as in Jobs.
    last = stat("Last job", [(f'max(coordinator_last_job_duration_seconds{{{C},kind="prove"}})', "Duration (s)"),
                               (f'max(coordinator_last_asm_mhz{{{C},kind="prove"}})', "Emulation (MHz)"),
                               (f'max(coordinator_last_job_steps{{{C},kind="prove"}}) / 1e6', "Steps (M)")], "none",
                desc="The last successful proof job: its total duration, the ASM emulator's speed in its phase 1, its steps")
    for t in last["targets"]:
        t["instant"], t["range"] = True, False
    last["datasource"] = DS
    last["options"].update({"textMode": "value_and_name", "graphMode": "none", "colorMode": "none", "justifyMode": "auto"})
    last["fieldConfig"]["overrides"] = [
        {"matcher": {"id": "byName", "options": name}, "properties": [{"id": "decimals", "value": decimals}]}
        for name, decimals in (("Duration (s)", 2), ("Emulation (MHz)", 0), ("Steps (M)", 0))]
    last["options"]["wideLayout"] = False
    sized(put(last, 0, 9, 8, 4), value=True)
    jobs = sized(put(panels["Jobs in the selected range"], 0, 13, 8, 4), value=True)
    jobs["options"]["wideLayout"] = False  # name above value, as in Last job, also with a single outcome
    jobs["title"] = "Jobs · ${range_text}"
    chart = put(panels["Proof duration per job (successful proofs; failed and cancelled as points)"], 8, 5, 16, 12)
    chart["title"] = "Proof time per job · ${range_text}"
    for i, name in enumerate(["Mean", "Median", "Min", "Max", "Std dev", "P99.9"]):
        put(panels[name], 16 + 4 * (i % 2), 17 + 4 * (i // 2), 4, 4)
    put(proof_time_histogram(), 0, 17, 16, 12)
    # The rest of Live, unchanged, below.
    used = {"Data sources", "API clients now", "Workers connected", "Workers by state", "Programs registered",
            "Compute capacity", "Each worker", "Running now", "Jobs in the selected range",
            "Proof duration per job (successful proofs; failed and cancelled as points)",
            "Mean", "Median", "Min", "Max", "Std dev", "P99.9",
            "Cluster — as the coordinator sees it (valid wherever the workers run)"}
    rest = [p for p in base["panels"] if p["title"] not in used]
    top = min(p["gridPos"]["y"] for p in rest)
    for p in rest:
        g = p["gridPos"]
        put(p, g["x"], g["y"] - top + 29, g["w"], g["h"])
    d = copy.deepcopy(base)
    # The coordinator's host, for the row title: hidden, from the coordinator target's labels.
    q = f"label_values(up{{{C}}}, host)"
    d["templating"]["list"].append({
        "type": "query", "name": "coordinator_host", "label": "Coordinator", "datasource": DS, "hide": 2,
        "query": {"query": q, "refId": "coordinator_host"}, "definition": q, "refresh": 1})
    # The proof-time histogram's selectors (proof_time_bins), valued in 250 ms bins; 0 is Auto and All.
    widths = [("Auto", 0)] + [(f"{w * 250} ms" if w * 250 < 1000 else f"{w * 250 // 1000} s", w) for w in PROOF_WIDTHS]
    up_to = [("All", 0)] + [(f"{s} s", s * PROOF_BINS_PER_S) for s in (10, 15, 20)]
    for name, label, choices in (("proof_bin", "Bin", widths), ("proof_up_to", "Up to", up_to)):
        d["templating"]["list"].append({
            "type": "custom", "name": name, "label": label, "hide": 0, "multi": False, "includeAll": False,
            "query": ", ".join(f"{text} : {value}" for text, value in choices),
            "options": [{"text": text, "value": str(value), "selected": i == 0} for i, (text, value) in enumerate(choices)],
            "current": {"text": choices[0][0], "value": str(choices[0][1])}})
    # The time picker's range in words, for titles: Grafana only prints a range's from/to times, but a variable
    # query sees the picker's range as typed (${__url_time_range}, e.g. "from=now-6h&to=now"), which
    # label_replace turns into "last 6 hours". An absolute range (after a zoom) keeps its from–to times.
    words = [("([0-9]+)s", "last $1 seconds"), ("([0-9]+)m", "last $1 minutes"), ("([0-9]+)h", "last $1 hours"),
             ("([0-9]+)d", "last $1 days"), ("([0-9]+)w", "last $1 weeks"), ("([0-9]+)M", "last $1 months"),
             ("([0-9]+)y", "last $1 years"), ("1m", "last minute"), ("1h", "last hour"), ("1d", "last day"),
             ("1w", "last week"), ("1M", "last month"), ("1y", "last year")]
    q = ('label_replace(label_replace(vector(1), "range_text", "${__from:date:HH:mm}–${__to:date:HH:mm}", "", ""), '
         '"range_url", "${__url_time_range}", "", "")')
    for rx, text in words:  # later entries win: "1h" says "last hour", not "last 1 hours"
        q = f'label_replace({q}, "range_text", "{text}", "range_url", "from=now-{rx}&to=now")'
    d["templating"]["list"].append({
        "type": "query", "name": "range_text", "label": "Range", "datasource": DS, "hide": 2,
        "query": {"query": f"query_result({q})", "refId": "range_text"}, "definition": f"query_result({q})",
        "regex": '/range_text="([^"]*)"/', "refresh": 2})
    d.update({"uid": "zisk-live", "title": "ZisK — Live", "panels": out,
              "description": "What the cluster is doing now: connection and services, jobs, proof time, then the details"})
    return d


def gpus():
    """Every GPU of each host side by side: what each one does now, then each metric over time with a line
    per GPU."""

    def per_gpu(expr):  # labelled with each GPU's number (index), which also orders the series
        return f"({expr}) * on (uuid) group_left (index) nvidia_smi_gpu_info{{{H}}}"

    l = Layout()
    l.panels.append({"type": "row", "title": "Host $host", "id": l._id(), "collapsed": False, "repeat": "host",
                     "gridPos": {"x": 0, "y": 0, "w": 24, "h": 1}, "panels": []})
    l.y = 1

    # (column, expr, unit, decimals): one instant query each, merged into a row per GPU.
    now = [("Busy", f"100 * nvidia_smi_utilization_gpu_ratio{{{H}}}", "percent", 0),
           ("Memory", f"nvidia_smi_memory_used_bytes{{{H}}}", "bytes", 1),
           ("Memory %", f"100 * nvidia_smi_memory_used_bytes{{{H}}} / nvidia_smi_memory_total_bytes{{{H}}}", "percent", 0),
           ("Power", f"nvidia_smi_power_draw_watts{{{H}}}", "watt", 0),
           ("Power limit", f"nvidia_smi_enforced_power_limit_watts{{{H}}}", "watt", 0),
           ("Power capped", f"nvidia_smi_clocks_throttle_reasons_sw_power_cap{{{H}}}", "none", 0),
           ("Temperature", f"nvidia_smi_temperature_gpu{{{H}}}", "celsius", 0),
           ("SM clock", f"nvidia_smi_clocks_current_sm_clock_hz{{{H}}}", "hertz", 2),
           ("P-state", f"nvidia_smi_pstate{{{H}}}", "prefix:P", 0)]
    value = {f"Value #{chr(65 + i)}": name for i, (name, *_) in enumerate(now)}
    gauge = {"type": "gauge", "mode": "basic", "valueDisplayMode": "text"}
    overrides = [{"matcher": {"id": "byName", "options": name},
                  "properties": [{"id": "unit", "value": unit}, {"id": "decimals", "value": decimals}]}
                 for name, _, unit, decimals in now]
    for name in ("Busy", "Memory %"):
        overrides.append({"matcher": {"id": "byName", "options": name},
                          "properties": [{"id": "custom.cellOptions", "value": gauge}, {"id": "min", "value": 0},
                                         {"id": "max", "value": 100}, {"id": "color", "value": {"mode": "continuous-GrYlRd"}}]})
    overrides.append({"matcher": {"id": "byName", "options": "Power capped"},
                      "properties": [{"id": "mappings", "value": [{"type": "value", "options": {
                          "0": {"text": "no", "color": "text"}, "1": {"text": "capped", "color": "orange"}}}]},
                                     {"id": "custom.cellOptions", "value": {"type": "color-text"}}]})
    l.add({"type": "table", "title": "Each GPU now",
           "description": "nvidia-smi's last reading of each GPU (every 5 s). Power capped: the driver is holding "
                          "the clocks down to stay within the power limit",
           "targets": targets([(per_gpu(expr), "") for _, expr, _, _ in now], instant=True, fmt="table"),
           "transformations": [
               {"id": "merge", "options": {}},
               {"id": "convertFieldType", "options": {"conversions": [{"targetField": "index", "destinationType": "number"}]}},
               {"id": "sortBy", "options": {"sort": [{"field": "index", "desc": False}]}},
               {"id": "organize", "options": {
                   "includeByName": {"index": True, **{v: True for v in value}},
                   "renameByName": {"index": "GPU", **value},
                   "indexByName": {"index": 0, **{v: i + 1 for i, v in enumerate(value)}}}}],
           "fieldConfig": {"defaults": {"custom": {"align": "auto"}}, "overrides": overrides},
           "options": {"showHeader": True, "cellHeight": "sm"}}, 24, 10)

    def chart(title, expr, unit, desc, max_=None):
        p = timeseries(title, [(per_gpu(expr), "GPU {{index}}")], unit, desc=desc)
        p["fieldConfig"]["defaults"].update({"min": 0, **({"max": max_} if max_ is not None else {})})
        p["options"]["legend"].update({"calcs": ["mean", "max", "lastNotNull"], "placement": "right"})
        return p

    l.add(chart("Busy", f"100 * nvidia_smi_utilization_gpu_ratio{{{H}}}", "percent",
                "Share of the last sample period in which a kernel ran on the GPU", 100), 12, 8)
    l.add(chart("Memory used", f"nvidia_smi_memory_used_bytes{{{H}}}", "bytes", "Device memory in use"), 12, 8)
    power = chart("Power", f"nvidia_smi_power_draw_watts{{{H}}}", "watt", "Power draw; the dashed line is the enforced limit")
    power["targets"].append({"refId": "B", "expr": f"max(nvidia_smi_enforced_power_limit_watts{{{H}}})",
                             "legendFormat": "limit", "datasource": DS})
    power["fieldConfig"]["overrides"].append({"matcher": {"id": "byName", "options": "limit"}, "properties": [
        {"id": "custom.lineStyle", "value": {"fill": "dash", "dash": [10, 10]}}, {"id": "custom.fillOpacity", "value": 0},
        {"id": "color", "value": {"mode": "fixed", "fixedColor": "red"}}]})
    l.add(power, 12, 8)
    l.add(chart("Temperature", f"nvidia_smi_temperature_gpu{{{H}}}", "celsius", "GPU core temperature"), 12, 8)
    l.add(chart("SM clock", f"nvidia_smi_clocks_current_sm_clock_hz{{{H}}}", "hertz",
                "Streaming multiprocessor clock; it drops while the GPU is power capped or idle"), 12, 8)
    l.add(chart("Memory controller busy", f"100 * nvidia_smi_utilization_memory_ratio{{{H}}}", "percent",
                "Share of the last sample period in which device memory was read or written", 100), 12, 8)

    def timeline(title, expr, mappings, desc):
        return {"type": "state-timeline", "title": title, "description": desc,
                "targets": targets([(per_gpu(expr), "GPU {{index}}")]),
                "fieldConfig": {"defaults": {"mappings": [{"type": "value", "options": {
                    v: {**m, "index": i} for i, (v, m) in enumerate(mappings.items())}}],
                                             "custom": {"fillOpacity": 80}},
                                "overrides": []},
                "options": {"showValue": "never", "mergeValues": True, "rowHeight": 0.8, "legend": {"showLegend": True}}}

    l.add(timeline("Power capped", f"nvidia_smi_clocks_throttle_reasons_sw_power_cap{{{H}}}",
                   {"0": {"text": "no", "color": "transparent"}, "1": {"text": "capped", "color": "orange"}},
                   "When the driver held each GPU's clocks down to stay within its power limit"), 24, 6)
    pstates = {str(n): {"text": f"P{n}", "color": c} for n, c in
               ((0, "dark-green"), (1, "green"), (2, "green"), (3, "yellow"), (4, "yellow"), (5, "yellow"),
                (6, "orange"), (7, "orange"), (8, "gray"), (12, "gray"))}
    pstates["8"]["text"] = "P8 · idle"
    l.add(timeline("P-state", f"nvidia_smi_pstate{{{H}}}", pstates,
                   "Each GPU's performance state: P0 the highest, P8 idle"), 24, 6)

    d = dashboard("zisk-gpus", "ZisK — GPUs", l, "Every GPU of each host side by side: now, and each metric over time")
    d["refresh"] = "5s"  # nvidia-smi is read every 5 s
    q = 'label_values(nvidia_smi_gpu_info{cluster=~"$cluster"}, host)'
    d["templating"]["list"].append({"type": "query", "name": "host", "label": "Host", "datasource": DS,
                                    "query": {"query": q, "refId": "host"}, "definition": q, "refresh": 2,
                                    "includeAll": True, "multi": True, "current": {"text": "All", "value": "$__all"}})
    d["annotations"] = {"list": job_annotations()}
    return d


NEEDS_CPU_INFO = "needs node_exporter's --collector.cpu.info"
NEEDS_NUMA = "needs node_exporter's --collector.meminfo_numa"


def host():
    """One host, from node_exporter: what a worker's host is short of, and what broke on it."""
    h = H.replace("=~", "=")  # one host at a time

    def node(metric, more=""):
        return f"{metric}{{{h}{',' + more if more else ''}}}"

    IDLE, BUSY, FAILED = 'mode="idle"', 'mode!="idle"', 'state="failed"'
    cpus = f'count({node("node_cpu_seconds_total", IDLE)})'
    fs = 'fstype!~"tmpfs|devtmpfs|overlay|squashfs|nsfs|ramfs|fuse.*",mountpoint!~"/(run|boot/efi)(/.*)?"'
    net = 'device!~"lo|veth.*|docker.*|br-.*|virbr.*|cali.*|flannel.*|cni.*"'
    disk = 'device!~"loop.*|ram.*|zram.*|dm-.*|sr.*"'

    def right_legend(p):
        p["options"]["legend"]["placement"] = "right"
        return p

    def plain_line(p, name):  # a reference series: a dashed line, not stacked, not filled
        p["fieldConfig"]["overrides"].append({"matcher": {"id": "byName", "options": name}, "properties": [
            {"id": "custom.stacking", "value": {"mode": "none", "group": "A"}}, {"id": "custom.fillOpacity", "value": 0},
            {"id": "custom.lineStyle", "value": {"fill": "dash", "dash": [10, 10]}},
            {"id": "color", "value": {"mode": "fixed", "fixedColor": "text"}}]})
        return p

    l = Layout()
    l.row("Host $host · now")
    l.add(stat("Uptime", [(f'time() - {node("node_boot_time_seconds")}', "up")], "dtdurations"), 4, 4)
    l.add(stat("CPU busy", [(f'100 * (1 - avg(rate({node("node_cpu_seconds_total", IDLE)}[1m])))', "busy")],
               "percent", decimals=0), 4, 4)
    l.add(stat("Memory used", [(f'100 * (1 - {node("node_memory_MemAvailable_bytes")} / {node("node_memory_MemTotal_bytes")})',
                                "used")], "percent", "Memory not available to new allocations without swapping",
               decimals=0, red_above=90), 4, 4)
    l.add(stat("Shared memory", [(node("node_memory_Shmem_bytes"), "shm and tmpfs")], "bytes",
               "Memory in shm segments and tmpfs, the workers' /dev/shm segments among them"), 4, 4)
    l.add(stat("Fullest filesystem", [(f'max(100 * (1 - {node("node_filesystem_avail_bytes", fs)} / '
                                       f'{node("node_filesystem_size_bytes", fs)}))', "used")],
               "percent", "The disk-backed filesystem nearest to full", decimals=0, red_above=90), 4, 4)
    l.add(stat("Swap used", [(f'{node("node_memory_SwapTotal_bytes")} - {node("node_memory_SwapFree_bytes")}', "used")],
               "bytes", red_above=1), 4, 4)
    l.add(stat("OOM kills", [(f'increase({node("node_vmstat_oom_kill")}[$__range])', "in the range")], "none",
               "Processes the kernel killed for want of memory, over the selected range", decimals=0, red_above=0.5), 4, 4)
    l.add(stat("Memory errors (ECC)", [(f'sum({node("node_edac_correctable_errors_total")})', "corrected"),
                                       (f'sum({node("node_edac_uncorrectable_errors_total")})', "uncorrected")],
               "none", "Errors the memory controllers reported since boot (EDAC)", decimals=0, red_above=0.5), 4, 4)
    l.add(stat("Hottest sensor", [(f'max({node("node_hwmon_temp_celsius")})', "max")], "celsius",
               "The hottest of the host's hwmon temperature sensors (CPUs, board, disks); GPUs are on the GPUs dashboard",
               decimals=0, red_above=90), 4, 4)
    l.add(stat("Failed units", [(f'sum({node("node_systemd_unit_state", FAILED)})', "failed")], "none",
               "systemd units in the failed state (needs node_exporter's --collector.systemd)",
               decimals=0, red_above=0.5), 4, 4)
    clock = stat("Clock", [(node("node_timex_sync_status"), "sync")], "none",
                 "Whether the kernel's clock is synchronised (NTP): the cluster's logs and metrics compare times across hosts")
    clock["fieldConfig"]["defaults"]["mappings"] = [{"type": "value", "options": {
        "0": {"text": "not synced", "color": "red"}, "1": {"text": "synced", "color": "green"}}}]
    l.add(clock, 4, 4)
    l.add(stat("Logical CPUs", [(cpus, "CPUs")], "none", decimals=0), 4, 4)

    l.row("CPU")
    modes = timeseries("CPU busy by mode", [(f'100 * sum by (mode) (rate({node("node_cpu_seconds_total", BUSY)}[1m])) '
                                             f'/ scalar({cpus})', "{{mode}}")], "percent", stack=True,
                       desc="Share of all the host's CPUs; iowait is idle time waiting on disk")
    modes["fieldConfig"]["defaults"].update({"min": 0, "max": 100})
    l.add(right_legend(modes), 12, 9)
    per_cpu = f'100 * (1 - rate({node("node_cpu_seconds_total", IDLE)}[1m]))'
    # Zero-padded, the CPUs sort by number: Prometheus orders a range query's series by their labels.
    per_cpu = f'label_replace(label_replace({per_cpu}, "cpu", "0$1", "cpu", "([0-9]{{2}})"), "cpu", "00$1", "cpu", "([0-9])")'
    l.add({"type": "heatmap", "title": "Busy per CPU",
           "description": "One row per logical CPU, from CPU 0 at the bottom. On a host with several sockets, "
                          "`lscpu` lists each NUMA node's CPUs, and the Busy per socket chart adds them up",
           "targets": targets([(per_cpu, "{{cpu}}")]),
           "fieldConfig": {"defaults": {"custom": {"scaleDistribution": {"type": "linear"},
                                                   "hideFrom": {"legend": False, "tooltip": False, "viz": False}}},
                           "overrides": []},
           "options": {"calculate": False, "cellGap": 0, "rowsFrame": {"layout": "unknown"},
                       "color": {"mode": "opacity", "fill": "orange", "scale": "linear", "steps": 64,
                                 "min": 0, "max": 100, "reverse": False, "exponent": 0.5},
                       "yAxis": {"axisPlacement": "left", "reverse": False},
                       "cellValues": {"unit": "percent", "decimals": 0},
                       "tooltip": {"mode": "single", "yHistogram": False, "showColorScale": True},
                       "legend": {"show": True}, "filterValues": {"le": -1},
                       "exemplars": {"color": "rgba(255,0,255,0.7)"}}}, 12, 9)
    socket = timeseries("Busy per socket", [(
        f'100 * sum by (package) (rate({node("node_cpu_seconds_total", BUSY)}[1m]) '
        f'* on (cpu) group_left (package) {node("node_cpu_info")}) / count by (package) ({node("node_cpu_info")})',
        "socket {{package}}")], "percent",
        desc="Needs node_exporter's --collector.cpu.info, which labels each CPU with its socket")
    socket["fieldConfig"]["defaults"].update({"min": 0, "max": 100, "noValue": NEEDS_CPU_INFO})
    l.add(socket, 8, 8)
    l.add(timeseries("Waiting for a resource (pressure)", [
        (f'100 * rate({node("node_pressure_cpu_waiting_seconds_total")}[1m])', "CPU: some waiting"),
        (f'100 * rate({node("node_pressure_memory_waiting_seconds_total")}[1m])', "memory: some waiting"),
        (f'100 * rate({node("node_pressure_memory_stalled_seconds_total")}[1m])', "memory: all stalled"),
        (f'100 * rate({node("node_pressure_io_waiting_seconds_total")}[1m])', "I/O: some waiting"),
        (f'100 * rate({node("node_pressure_io_stalled_seconds_total")}[1m])', "I/O: all stalled")], "percent",
        desc="Linux pressure stall information: share of the time some (or all) runnable tasks waited for the CPU, "
             "memory or I/O"), 8, 8)
    l.add(plain_line(timeseries("Load", [(node("node_load1"), "1 min"), (node("node_load15"), "15 min"),
                                         (node("node_procs_running"), "running"),
                                         (node("node_procs_blocked"), "blocked on I/O"), (cpus, "logical CPUs")], "none",
                                desc="Runnable (and uninterruptible) tasks; above the CPU count, tasks queue for a CPU"),
                     "logical CPUs"), 8, 8)

    l.row("Memory")
    mem = timeseries("Memory", [
        (node("node_memory_AnonPages_bytes"), "processes (anonymous)"),
        (node("node_memory_Shmem_bytes"), "shared (shm, tmpfs)"),
        (f'{node("node_memory_Cached_bytes")} - {node("node_memory_Shmem_bytes")}', "page cache"),
        (f'{node("node_memory_Buffers_bytes")}', "buffers"),
        (node("node_memory_Slab_bytes"), "kernel slab"),
        (node("node_memory_MemFree_bytes"), "free"),
        (node("node_memory_MemTotal_bytes"), "total")], "bytes", stack=True,
        desc="Where the host's memory goes. Shared memory includes the workers' /dev/shm segments")
    mem["fieldConfig"]["defaults"]["min"] = 0
    l.add(right_legend(plain_line(mem, "total")), 12, 9)
    numa = timeseries("Memory per NUMA node", [
        (node("node_memory_numa_MemUsed"), "node {{node}} used"),
        (node("node_memory_numa_MemTotal"), "node {{node}} total")], "bytes",
        desc="Needs node_exporter's --collector.meminfo_numa. A node that fills up sends its allocations to "
             "the other socket's memory")
    numa["fieldConfig"]["defaults"]["noValue"] = NEEDS_NUMA
    l.add(numa, 12, 9)
    off = timeseries("Allocations off their NUMA node", [
        (f'rate({node("node_memory_numa_numa_miss_total")}[1m]) * 4096', "node {{node}}: placed here, meant for another"),
        (f'rate({node("node_memory_numa_other_node_total")}[1m]) * 4096', "node {{node}}: placed here, for a task on another")],
        "Bps", desc="Needs node_exporter's --collector.meminfo_numa. Pages (4 KiB) a task on one socket got from "
                    "the other socket's memory")
    off["fieldConfig"]["defaults"]["noValue"] = NEEDS_NUMA
    l.add(off, 12, 8)
    l.add(timeseries("Paging and swapping", [
        (f'rate({node("node_vmstat_pgmajfault")}[1m])', "major faults (read from disk)"),
        (f'rate({node("node_vmstat_pswpin")}[1m])', "swapped in"),
        (f'rate({node("node_vmstat_pswpout")}[1m])', "swapped out")], "ops",
        desc="Per second. Major faults while proving mean the working set does not fit in memory"), 12, 8)

    l.row("Disks")
    used = f'100 * (1 - {node("node_filesystem_avail_bytes", fs)} / {node("node_filesystem_size_bytes", fs)})'
    # `+ 0` drops the metric name, as the Used query's arithmetic does, so the three merge into a row each.
    now = [("Used", used, "percent"), ("Available", f'{node("node_filesystem_avail_bytes", fs)} + 0', "bytes"),
           ("Size", f'{node("node_filesystem_size_bytes", fs)} + 0', "bytes")]
    value = {f"Value #{chr(65 + i)}": name for i, (name, *_) in enumerate(now)}
    overrides = [{"matcher": {"id": "byName", "options": name}, "properties": [{"id": "unit", "value": unit}]}
                 for name, _, unit in now]
    overrides.append({"matcher": {"id": "byName", "options": "Used"}, "properties": [
        {"id": "custom.cellOptions", "value": {"type": "gauge", "mode": "basic", "valueDisplayMode": "text"}},
        {"id": "min", "value": 0}, {"id": "max", "value": 100}, {"id": "decimals", "value": 0},
        {"id": "thresholds", "value": {"mode": "absolute", "steps": [{"color": "green", "value": None},
                                                                     {"color": "orange", "value": 80},
                                                                     {"color": "red", "value": 90}]}},
        {"id": "color", "value": {"mode": "thresholds"}}]})
    l.add({"type": "table", "title": "Filesystems", "description": "Disk-backed filesystems",
           "targets": targets([(expr, "") for _, expr, _ in now], instant=True, fmt="table"),
           "transformations": [
               {"id": "merge", "options": {}},
               {"id": "organize", "options": {
                   "includeByName": {"mountpoint": True, "device": True, "fstype": True, **{v: True for v in value}},
                   "renameByName": {"mountpoint": "Mount point", "device": "Device", "fstype": "Type", **value},
                   "indexByName": {"mountpoint": 0, "device": 1, "fstype": 2,
                                   **{v: i + 3 for i, v in enumerate(value)}}}}],
           # Grafana's default 150 px a column scrolls the last ones out of a third of the screen.
           "fieldConfig": {"defaults": {"custom": {"align": "auto", "minWidth": 60}}, "overrides": overrides},
           "options": {"showHeader": True, "cellHeight": "sm", "sortBy": [{"displayName": "Used", "desc": True}]}}, 10, 9)
    l.add(timeseries("Disk throughput", [
        (f'rate({node("node_disk_read_bytes_total", disk)}[1m])', "{{device}} read"),
        (f'rate({node("node_disk_written_bytes_total", disk)}[1m])', "{{device}} written")], "Bps"), 7, 9)
    busy = timeseries("Disk busy", [(f'100 * rate({node("node_disk_io_time_seconds_total", disk)}[1m])', "{{device}}")],
                      "percent", desc="Share of the time each device had I/O in flight")
    busy["fieldConfig"]["defaults"].update({"min": 0, "max": 100})
    l.add(busy, 7, 9)

    l.row("Network")
    l.add(timeseries("Network throughput", [
        (f'8 * rate({node("node_network_receive_bytes_total", net)}[1m])', "{{device}} in"),
        (f'8 * rate({node("node_network_transmit_bytes_total", net)}[1m])', "{{device}} out")], "bps"), 8, 8)
    l.add(timeseries("Network errors and drops", [
        (f'rate({node("node_network_receive_errs_total", net)}[1m])', "{{device}} in: errors"),
        (f'rate({node("node_network_transmit_errs_total", net)}[1m])', "{{device}} out: errors"),
        (f'rate({node("node_network_receive_drop_total", net)}[1m])', "{{device}} in: dropped"),
        (f'rate({node("node_network_transmit_drop_total", net)}[1m])', "{{device}} out: dropped")], "pps"), 8, 8)
    l.add(timeseries("TCP retransmissions", [
        (f'100 * rate({node("node_netstat_Tcp_RetransSegs")}[1m]) / rate({node("node_netstat_Tcp_OutSegs")}[1m])',
         "retransmitted")], "percent",
        desc="Share of the TCP segments sent again: the coordinator's and the workers' connections run over TCP"), 8, 8)

    l.row("Hardware")
    temps = right_legend(timeseries("Temperatures", [(node("node_hwmon_temp_celsius"), "{{chip}} {{sensor}}")], "celsius"))
    temps["fieldConfig"]["defaults"]["custom"]["fillOpacity"] = 0
    l.add(temps, 12, 8)
    failed = table("Failed units", f'{node("node_systemd_unit_state", FAILED)} == 1', {"name": "Unit"},
                   desc="systemd units in the failed state (needs node_exporter's --collector.systemd)")
    failed["transformations"] = [{"id": "organize", "options": {"includeByName": {"name": True},
                                                               "renameByName": {"name": "Unit"}}}]
    failed["options"]["sortBy"] = [{"displayName": "Unit", "desc": False}]
    failed["fieldConfig"]["defaults"]["noValue"] = "none"
    l.add(failed, 6, 8)
    l.add(timeseries("Clock offset", [(node("node_timex_offset_seconds"), "offset")], "s",
                     desc="The kernel clock's offset from its NTP source"), 6, 8)

    d = dashboard("zisk-host", "ZisK — Host", l,
                  "One host, from node_exporter: CPU, memory, NUMA, disks, network and hardware")
    v = host_variable()
    v.update({"includeAll": False, "multi": False, "current": {}})
    d["templating"]["list"].append(v)
    d["annotations"] = {"list": job_annotations()}
    return d


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    if len(sys.argv) == 3:
        out, rules = sys.argv[1:]
    elif len(sys.argv) == 1:
        out = os.path.join(here, "..", "dashboards")
        rules = os.path.join(here, "..", "..", "prometheus", "rules", "zisk-proof-time.yml")
    else:
        sys.exit(__doc__)
    for d in (resources(), recovery(), proofs(), live2(), gpus(), host()):
        with open(os.path.join(out, f"{d['uid']}.json"), "w") as f:
            json.dump(d, f, indent=1, ensure_ascii=False)
            f.write("\n")
        print(d["uid"], len([p for p in d["panels"] if p["type"] != "row"]), "panels")
    with open(rules, "w") as f:
        f.write(proof_time_rules())
    print(os.path.relpath(rules), "rules")
