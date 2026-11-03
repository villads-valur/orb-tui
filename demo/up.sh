#!/bin/sh
# Starts the demo containers used for the README screenshots.
# They run in their own Docker-in-Docker daemon so your real containers stay out of the shot.
# Afterwards run orb-tui with: DOCKER_HOST=tcp://127.0.0.1:23750 orb-tui
set -e

cd "$(dirname "$0")"

if ! docker inspect orb-tui-demo-dockerd >/dev/null 2>&1; then
  docker run -d --privileged --name orb-tui-demo-dockerd \
    -e DOCKER_TLS_CERTDIR= -p 127.0.0.1:23750:2375 docker:27-dind >/dev/null
fi

until docker exec orb-tui-demo-dockerd docker info >/dev/null 2>&1; do :; done

demo() {
  docker exec -w /demo orb-tui-demo-dockerd "$@"
}

# Compose runs inside the daemon container so the working-dir labels show /demo, not a local path
demo mkdir -p /demo
docker cp . orb-tui-demo-dockerd:/demo
demo docker compose -f gringotts.compose.yaml up -d
demo docker compose -f owlery.compose.yaml up -d
demo docker compose -f daily-prophet.compose.yaml up -d
demo docker compose -f daily-prophet.compose.yaml stop

# One container outside Compose, to show how ungrouped containers look
if ! demo docker inspect marauders-map >/dev/null 2>&1; then
  demo docker run -d --name marauders-map -p 7777:80 nginx:alpine >/dev/null
fi
