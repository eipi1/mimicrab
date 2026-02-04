# Installation

Mimicrab can be installed in several ways.

## Building from Source

To build Mimicrab from source, you need to have Rust installed.

```bash
git clone https://github.com/eipi1/mimicrab.git
cd mimicrab
cargo build --release
```

The binary will be available at `target/release/mimicrab`.

## Running with Docker

You can run Mimicrab using the official Docker image.

```bash
docker run -p 3000:3000 ghcr.io/eipi1/mimicrab:latest
```
