# STU-M2: the text-to-motion service (MoMask, MIT, pinned) on the CPU. Checkpoints (HumanML3D models, about 190 MB) and
# the CLIP ViT-B/32 cache are mounted, not baked in: see docker-compose.yml (profile motion) and docs/plans/stu-m2.md.
FROM python:3.10-slim
RUN apt-get update && apt-get install -y --no-install-recommends git && rm -rf /var/lib/apt/lists/* \
 && pip install --no-cache-dir --extra-index-url https://download.pytorch.org/whl/cpu torch==2.4.1+cpu \
 && pip install --no-cache-dir numpy==1.23.5 scipy matplotlib==3.7.5 einops==0.6.1 ftfy regex tqdm \
      vector-quantize-pytorch==1.6.30 git+https://github.com/openai/CLIP.git@dcba3cb2e2827b402d2701e7e1c7d9fed8a20ef1 \
 && git clone https://github.com/EricGuo5513/momask-codes.git /momask && git -C /momask checkout 94a6636 \
 && useradd -m -u 10001 motion
COPY momask-numpy-matplotlib.patch /tmp/
RUN git -C /momask apply /tmp/momask-numpy-matplotlib.patch && chown -R 10001 /momask
COPY server.py /srv/server.py
USER 10001
ENV HOME=/home/motion MOMASK_DIR=/momask
EXPOSE 8000
HEALTHCHECK --start-period=120s CMD ["python", "-c", "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8000/health', timeout=3)"]
CMD ["python", "/srv/server.py", "--serve", "0.0.0.0:8000"]
