#!/bin/sh
# Integration gate (KG-09): server tests in a rust container on the runner's Docker (DOCKER_HOST).
# The job image has no cargo; this mirrors the `server` job in build.yml (same musl image) and
# fetches the commit inside the container, because a dind host cannot mount the job's workspace.
# Needs TOKEN, REPO_URL and SHA in the environment (the workflow sets them).
set -eu
: "${TOKEN:?}" "${REPO_URL:?}" "${SHA:?}"
docker run -i --rm \
    -e TOKEN -e REPO_URL -e SHA -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" \
    -e CFLAGS="-Du_int8_t=uint8_t -Du_int16_t=uint16_t -Du_int64_t=uint64_t" \
    rust:1-alpine sh -eu <<'BUILD'
apk add --no-cache -q git musl-dev
mkdir /src && cd /src && git init -q .
git -c http.extraHeader="Authorization: token $TOKEN" fetch -q --depth 1 "$REPO_URL" "$SHA"
git checkout -q FETCH_HEAD
(cd machine-stats && cargo test)
(cd server && cargo test --locked)
BUILD
