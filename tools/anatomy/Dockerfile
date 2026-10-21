# STU-C4: the anatomy check as a small CPU service (POST /check with a PNG, GET /health).
# Models are mounted at /models: face_yolov8m.pt and hand_yolov8s.pt (AGPL-3.0, the same files the OC sheet's
# face and hand fix use). STU-PE3: POST /pose runs DWPose (Apache-2.0) from /pose-models.
FROM python:3.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/* \
    && pip install --no-cache-dir --index-url https://download.pytorch.org/whl/cpu torch torchvision \
    && pip install --no-cache-dir ultralytics onnxruntime \
    && useradd -m -u 10001 anatomy
COPY check.py dwpose.py /app/
USER 10001
ENV ANATOMY_MODELS=/models POSE_MODELS=/pose-models YOLO_CONFIG_DIR=/tmp
EXPOSE 8000
HEALTHCHECK CMD ["python", "-c", "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8000/health', timeout=3)"]
CMD ["python", "/app/check.py", "--serve", "0.0.0.0:8000"]
