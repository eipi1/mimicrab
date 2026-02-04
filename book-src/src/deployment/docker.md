# Docker Deployment

Mimicrab can be easily containerized using Docker.

## Using the provided Dockerfile

A multi-stage `Dockerfile` is included in the repository for building a small, efficient image.

### 1. Build the Image

```bash
docker build -t ghcr.io/eipi1/mimicrab:latest .
```

### 2. Run the Container

```bash
docker run -d -p 3000:3000 --name mimicrab ghcr.io/eipi1/mimicrab:latest
```

## Persistence

In Docker, expectations are saved to `expectations.json` by default. To persist mocks across container restarts, mount a volume:

```bash
docker run -d \
  -p 3000:3000 \
  -v $(pwd)/expectations.json:/app/expectations.json \
  ghcr.io/eipi1/mimicrab:latest
```
