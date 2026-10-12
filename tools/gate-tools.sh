#!/bin/sh
# Integration gate (KG-09): tools tests in a node:22 container on the runner's Docker (DOCKER_HOST).
# The job image has no Pillow, numpy or ffmpeg; it runs the same tests as `python3 -m unittest discover` and
# fetches the commit inside the container, because a dind host cannot mount the job's workspace.
# Needs TOKEN, REPO_URL and SHA in the environment (the workflow sets them).
set -eu
: "${TOKEN:?}" "${REPO_URL:?}" "${SHA:?}"
docker run -i --rm \
    -e TOKEN -e REPO_URL -e SHA node:22 sh -eu <<'TOOLS'
apt-get update -qq
apt-get install -y -qq git ffmpeg python3-pil python3-numpy
mkdir /src && cd /src && git init -q .
git -c http.extraHeader="Authorization: token $TOKEN" fetch -q --depth 1 "$REPO_URL" "$SHA"
git checkout -q FETCH_HEAD
python3 -m unittest discover -s tools -p 'test_*.py'
TOOLS
