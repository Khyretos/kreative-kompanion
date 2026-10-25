# STU-M1: MoMask (MIT) on the CPU for the text-to-motion research run
FROM python:3.10-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ffmpeg && rm -rf /var/lib/apt/lists/* \
 && pip install --no-cache-dir --extra-index-url https://download.pytorch.org/whl/cpu torch==2.4.1+cpu torchvision==0.19.1+cpu \
 && pip install --no-cache-dir numpy==1.23.5 scipy "matplotlib==3.7.5" einops==0.6.1 ftfy regex tqdm "vector-quantize-pytorch==1.6.30" git+https://github.com/openai/CLIP.git@dcba3cb2e2827b402d2701e7e1c7d9fed8a20ef1
WORKDIR /w
