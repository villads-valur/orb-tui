#!/bin/sh
# Runs orb-tui against the demo daemon started by up.sh.
DOCKER_HOST=tcp://127.0.0.1:23750 exec "$(dirname "$0")/../target/release/orb-tui"
