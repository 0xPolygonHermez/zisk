// Named threshold sets. Change SLO once, every panel inherits.

local c = import 'lib/colors.libsonnet';

{
  up_down: [
    { color: c.critical, value: null },
    { color: c.healthy, value: 1 },
  ],

  worker_count: [
    { color: c.critical, value: null },
    { color: c.healthy, value: 1 },
  ],

  observed_count: [{ color: c.healthy, value: null }],
  observed_duration: [{ color: c.healthy, value: null }],

  failure_rate_pct: [
    { color: c.healthy, value: null },
    { color: c.warning, value: 0.01 },
    { color: c.critical, value: 0.05 },
  ],

  db_latency_s: [
    { color: c.healthy, value: null },
    { color: c.warning, value: 0.05 },
    { color: c.critical, value: 0.25 },
  ],
}
