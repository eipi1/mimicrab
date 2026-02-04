# Kubernetes Deployment

Mimicrab is designed to run seamlessly in Kubernetes, with built-in support for distributed state management.

## State Management

In Kubernetes, Mimicrab uses a **ConfigMap** to store its expectations. This allows multiple Mimicrab pods to share the same state.

- **Initialization**: On startup, Mimicrab loads expectations from the configured ConfigMap.
- **Auto-Sync**: Mimicrab watches the ConfigMap for changes and automatically refreshes its local state when the ConfigMap is updated (e.g., via the Management API in a different pod).

## Environment Variables

- `KUBERNETES_SERVICE_HOST`: Automatically set by K8s; enables K8s mode.
- `CONFIG_MAP_NAME`: Name of the ConfigMap to use (default: `mimicrab-config`).
- `KUBERNETES_NAMESPACE`: Namespace of the ConfigMap (default: `default`).

## Example Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mimicrab
spec:
  replicas: 2
  selector:
    matchLabels:
      app: mimicrab
  template:
    metadata:
      labels:
        app: mimicrab
    spec:
      containers:
      - name: mimicrab
        image: ghcr.io/eipi1/mimicrab:latest
        ports:
        - containerPort: 3000
        env:
        - name: CONFIG_MAP_NAME
          value: "mimicrab-mocks"
```
