# Configuration Reference

This document is the detailed companion for the quick-start configuration section in README.

## Load order

bluetty loads configuration in this order:

1. path from BLUETTY_CONFIG environment variable
2. ./bluetty.conf
3. /etc/bluetty/bluetty.conf
4. built-in defaults

## Example files

- full example: bluetty.conf.example
- minimal example: bluetty.conf.minimal

## Sections

### bluez

- ProfilePath: object path used for Profile1 registration
- Uuid: profile UUID, default SPP UUID
- Name: profile display name
- RequireAuthentication: bluez profile option
- RequireAuthorization: bluez profile option

Pairing, trust management, and authorization prompts are delegated to the native BlueZ stack tools (for example `bluetoothctl`).

### session

- Mode: none, getty, exec
- SubcommandTemplate: shell-like command template to start the session child
- HupToTermDelay: milliseconds to wait between SIGHUP and SIGTERM when stopping a managed child
- ProcessGroupTermTimeout: milliseconds to wait for session shutdown before escalating to SIGKILL
- MaxSessions: maximum active sessions, 0 means unlimited

Session mode controls process session behavior:

- none mode: BlueTTY invokes SubcommandTemplate as a notify hook and does not manage that process lifecycle
- getty mode: BlueTTY starts child with setsid behavior enabled
- exec mode: BlueTTY starts child without setsid

When running in managed modes (`getty`/`exec`), BlueTTY sends signals to the child process group first and falls back to the child PID when group lookup fails.
Template placeholders for SubcommandTemplate:

- {tty}
- {addr}
- {name}
- {host}

## Child reaping

bluetty now uses a single child reaping path based on pidfd-capable runtime behavior and does not expose child-reap mode configuration.

## Suggested profiles

### minimal getty profile

- session.Mode=getty
- keep default SubcommandTemplate for agetty

### stricter profile

- set RequireAuthorization=true when bluez policy requires it

### custom bridge process profile

- session.Mode=exec
- set session.SubcommandTemplate, for example `/usr/local/bin/my-shell-bridge --tty /dev/{tty} --device {addr}`

### external notify profile

- session.Mode=none
- set session.SubcommandTemplate, for example `/usr/local/bin/bluetty-notify --tty /dev/{tty} --addr {addr}`

## Validation checklist

- verify bluez and dbus are available
- verify command binaries used in session.SubcommandTemplate exist
- verify object paths are valid dbus paths
- verify mode values use supported enums only
- if RequireAuthorization=true, verify target devices satisfy your BlueZ authorization policy
