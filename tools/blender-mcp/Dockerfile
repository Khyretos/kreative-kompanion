# BLD-02: Blender MCP in a container. Official Blender build (Debian's has no OpenImageDenoise): CPU Cycles + denoising, no GPU needed.
FROM debian:trixie-slim
ARG BLENDER_VERSION=4.3.2
RUN apt-get update && apt-get install -y --no-install-recommends python3 curl xz-utils ca-certificates \
        libxi6 libxxf86vm1 libxfixes3 libxrender1 libxkbcommon0 libsm6 libgl1 libegl1 libgomp1 \
    && curl -fsSL "https://download.blender.org/release/Blender${BLENDER_VERSION%.*}/blender-${BLENDER_VERSION}-linux-x64.tar.xz" \
        | tar -xJ -C /opt && mv /opt/blender-${BLENDER_VERSION}-linux-x64 /opt/blender \
    && apt-get purge -y curl xz-utils && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*
COPY server.py scene.py /app/
WORKDIR /app
ENV BLENDER_CMD=/opt/blender/blender BLENDER_MCP_BIND=0.0.0.0:8000 BLENDER_TIMEOUT=300 BLENDER_ENGINE=CYCLES BLENDER_SAMPLES=128 HOME=/tmp
USER 1000:1000
CMD ["python3", "/app/server.py"]
