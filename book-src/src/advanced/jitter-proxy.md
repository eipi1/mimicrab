# Jitter & Proxying

Mimicrab provides advanced network simulation and transparent proxying capabilities to help you test edge cases and integrate with existing services.

## Jitter (Network Simulation)

Jitter allows you to simulate unreliable network conditions for a specific mock. This is essential for testing how your application handles slow responses or intermittent failures.

### Configuration Options

- **Delay (ms)**: Adds a fixed latency to the response.
- **Random Delay (ms)**: Adds a random delay up to the specified maximum.
- **Failure Rate (0.0 - 1.0)**: The probability that a request will fail with a 500 error instead of returning the mock response. For example, `0.1` means a 10% failure rate.

### Usage in UI

1. Expand **Advanced Options** in the mock form.
2. Toggle **Enable Jitter**.
3. Configure your desired latency and failure rate.

---

## Proxying

Proxying allows Mimicrab to act as a transparent intermediary. If a request doesn't match a mock condition, or if a specific mock is configured to proxy, Mimicrab can forward the request to an upstream server and return that server's response.

### Configuration Options

- **Upstream URL**: The base URL of the service you want to proxy to (e.g., `https://api.production.com`).
- **Follow Redirects**: Whether Mimicrab should follow 3xx redirects from the upstream.

### Usage in UI

1. Expand **Advanced Options** in the mock form.
2. Toggle **Enable Proxying**.
3. Enter the **Upstream URL**.

> [!NOTE]
> Jitter, Proxying, and Lua Scripting are mutually exclusive for a single mock to ensure predictable behavior.
