# Docker Guide

## Build image

```bash
docker build -t bluetty:latest .
```

## Run container

```bash
docker run --rm -it \
  --network host \
  --privileged \
  -e RUST_LOG=info \
  -e BLUETTY_CONFIG=/etc/bluetty/bluetty.conf \
  -v $(pwd)/bluetty.conf.minimal:/etc/bluetty/bluetty.conf:ro \
  -v /run/dbus:/run/dbus \
  -v /var/run/dbus:/var/run/dbus \
  -v /var/lib/bluetooth:/var/lib/bluetooth \
  bluetty:latest
```

## Compose example

This repository no longer maintains a compose sample file.
Use the run command above as the reference baseline and adapt it to your deployment stack.
