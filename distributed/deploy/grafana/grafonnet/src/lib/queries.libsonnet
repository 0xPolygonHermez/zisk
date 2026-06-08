// Dashboard PromQL strings; assumes $coordinator, $program, and $job_id variables exist.

{
  cluster_up: '(max(up{job=~"zisk-coordinator.*|zisk-coord.*"} * on(instance, job) group_left(coordinator_id) max by (instance, job, coordinator_id) (coordinator_info{coordinator_id=~"$coordinator"})) or vector(0)) > bool 0',

  connected_workers: 'sum(coordinator_workers_connected{coordinator_id=~"$coordinator"}) or vector(0)',

  active_jobs: 'sum(coordinator_active_jobs{coordinator_id=~"$coordinator"}) or vector(0)',

  time_since_last_success: 'time() - max(coordinator_last_successful_job_timestamp_seconds{coordinator_id=~"$coordinator"} > 0)',

  throughput_by_kind_outcome: 'sum by (kind, outcome) (rate(coordinator_jobs_total{coordinator_id=~"$coordinator"}[5m])) * 3600',

  failure_rate_5m_by_kind: '(sum by (kind) (rate(coordinator_jobs_total{coordinator_id=~"$coordinator",outcome="failure"}[5m])) or (sum by (kind) (rate(coordinator_jobs_total{coordinator_id=~"$coordinator"}[5m])) * 0)) / clamp_min(sum by (kind) (rate(coordinator_jobs_total{coordinator_id=~"$coordinator"}[5m])), 1e-9)',

  coordinator_inventory: 'up{job=~"zisk-coordinator.*|zisk-coord.*"} * on(instance, job) group_left(coordinator_id, version, environment) max by (instance, job, coordinator_id, version, environment) (coordinator_info{coordinator_id=~"$coordinator"})',

  grpc_request_rate: 'sum by (method, status) (rate(coordinator_requests_total{coordinator_id=~"$coordinator"}[5m]))',

  db_write_queue_depth: 'coordinator_db_write_queue_depth{coordinator_id=~"$coordinator"}',

  db_write_dropped_range: 'sum(increase(coordinator_db_write_dropped_total{coordinator_id=~"$coordinator"}[$__range])) or vector(0)',

  db_query_p95_by_op: 'max by (op) (coordinator_db_query_duration_seconds{coordinator_id=~"$coordinator",status="success",quantile="0.95"})',

  phase_utilization_by_phase: 'sum by (phase) (increase(coordinator_phase_duration_seconds_sum{coordinator_id=~"$coordinator",program=~"$program"}[15m])) / 900',

  duration_p50: 'histogram_quantile(0.50, sum by (le) (rate(coordinator_job_duration_seconds_bucket{coordinator_id=~"$coordinator",program=~"$program"}[$__rate_interval])))',

  duration_p95: 'histogram_quantile(0.95, sum by (le) (rate(coordinator_job_duration_seconds_bucket{coordinator_id=~"$coordinator",program=~"$program"}[$__rate_interval])))',

  duration_p99: 'histogram_quantile(0.99, sum by (le) (rate(coordinator_job_duration_seconds_bucket{coordinator_id=~"$coordinator",program=~"$program"}[$__rate_interval])))',

  duration_p95_by_program: 'histogram_quantile(0.95, sum by (program, le) (increase(coordinator_job_duration_seconds_bucket{coordinator_id=~"$coordinator",program=~"$program"}[15m])))',

  phase_duration_p95_by_program: 'histogram_quantile(0.95, sum by (program, phase, le) (increase(coordinator_phase_duration_seconds_bucket{coordinator_id=~"$coordinator",program=~"$program"}[15m])))',

  steps_per_second_by_program: 'sum by (program) (increase(coordinator_job_executed_steps_total{coordinator_id=~"$coordinator",program=~"$program"}[15m])) / 900',

  worker_pool_by_status: 'sum by (status) (coordinator_workers_by_status{coordinator_id=~"$coordinator"})',

  worker_proof_assignments: 'sum by (worker_id, program) (rate(coordinator_worker_jobs_total{coordinator_id=~"$coordinator",program=~"$program"}[5m]))',

  worker_errors_by_reason: 'sum by (reason) (increase(coordinator_worker_errors_total{coordinator_id=~"$coordinator"}[$__range]))',

  coordinator_request_p95_by_method: 'histogram_quantile(0.95, sum by (method, le) (rate(coordinator_request_duration_seconds_bucket{coordinator_id=~"$coordinator"}[$__rate_interval])))',

  coordinator_restarts_range: 'sum(changes(coordinator_start_time_seconds{coordinator_id=~"$coordinator"}[$__range])) or vector(0)',

  coordinator_scrape_up: 'avg_over_time((up{job=~"zisk-coordinator.*|zisk-coord.*"} * on(instance, job) group_left(coordinator_id) max by (instance, job, coordinator_id) (coordinator_info{coordinator_id=~"$coordinator"}))[$__range:]) * 100',

  coord_up_per_id: 'max by (coordinator_id) (up{job=~"zisk-coordinator.*|zisk-coord.*"} * on(instance, job) group_left(coordinator_id) max by (instance, job, coordinator_id) (coordinator_info{coordinator_id=~"$coordinator"}))',

  worker_heartbeat_lag_per_worker: 'coordinator_worker_heartbeat_lag_seconds{coordinator_id=~"$coordinator"}',

  template_coordinator: 'label_values(coordinator_info, coordinator_id)',
  template_program: 'label_values(coordinator_program_info, program)',

  annot_coord_scrape: 'changes(up{job=~"zisk-coordinator.*|zisk-coord.*"}[1m]) > 0',
  annot_coord_restart: 'changes(coordinator_start_time_seconds{coordinator_id=~"$coordinator"}[1m]) > 0',
  annot_worker_pool: 'changes(coordinator_workers_connected{coordinator_id=~"$coordinator"}[1m]) != 0',
}
