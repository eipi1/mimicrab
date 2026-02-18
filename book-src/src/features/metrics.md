# Prometheus Metrics

Mimicrab exports Prometheus-compatible metrics to help you monitor the health and performance of your mock server.

## Metrics Endpoint

Metrics are available at the following admin endpoint:

```http
GET /_admin/metrics
```

## Available Metrics

### Mock Performance
- `mimicrab_requests_total`: Total number of requests handled.
  - Labels: `matched` (true/false), `path`.
- `mimicrab_request_duration_seconds`: Histogram of request latencies (including Lua execution and Proxying).
  - Labels: `path`.

### Process Metrics (Linux only)
Mimicrab also exports standard process metrics including:
- `process_cpu_seconds_total`
- `process_resident_memory_bytes`
- `process_virtual_memory_bytes`
- `process_open_fds`
- `process_max_fds`

## Scraping with Prometheus

Add the following job to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mimicrab'
    metrics_path: '/_admin/metrics'
    static_configs:
      - targets: ['localhost:3000']
```
