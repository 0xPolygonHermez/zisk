# Local Prometheus template. Entrypoint renders one scrape target per
# coord listed in ZISK_COORDS (format: id1=host:port,id2=host:port,...).
# Bearer token is shared with Grafana via /etc/prometheus/zisk-scrape-token.

global:
  scrape_interval: 5s
  evaluation_interval: 5s

scrape_configs:
${COORD_SCRAPE_CONFIGS}
