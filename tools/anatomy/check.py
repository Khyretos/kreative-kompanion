import argparse
import json
import os
import sys
import tempfile
from http.server import BaseHTTPRequestHandler, HTTPServer

VIEWS = ("front", "left", "back", "right")
FACE_VIEWS = ("front", "left", "right")
# Measured on nine STU-C1 sheets: real faces 0.77-0.90, the back of a head 0.58-0.64; real hands 0.59-0.87,
# a sleeve cuff read as a third hand 0.46-0.50.
FACE_CONF = 0.7
HAND_CONF = 0.55
WRIST_IOU = 0.3
_MODELS = {}


def iou(a, b):
    x1a, y1a, x2a, y2a = a[:4]
    x1b, y1b, x2b, y2b = b[:4]
    xi1, yi1 = max(x1a, x1b), max(y1a, y1b)
    xi2, yi2 = min(x2a, x2b), min(y2a, y2b)
    if xi2 <= xi1 or yi2 <= yi1:
        return 0.0
    inter = (xi2 - xi1) * (yi2 - yi1)
    ua = (x2a - x1a) * (y2a - y1a)
    ub = (x2b - x1b) * (y2b - y1b)
    return inter / (ua + ub - inter) if (ua + ub - inter) > 0 else 0.0


def judge(
    faces,
    hands,
    width,
    height,
    face_views=FACE_VIEWS,
    face_conf=FACE_CONF,
    hand_conf=HAND_CONF,
):
    faces = [f for f in faces if f[4] >= face_conf]
    hands = [h for h in hands if h[4] >= hand_conf]
    cell_w = width / 4
    cells = [{"view": v, "faces": 0, "hands": 0} for v in VIEWS]
    issues = []

    for box in faces:
        cx = (box[0] + box[2]) / 2
        cell_idx = max(0, min(3, int(cx // cell_w)))
        cells[cell_idx]["faces"] += 1

    for box in hands:
        cx = (box[0] + box[2]) / 2
        cell_idx = max(0, min(3, int(cx // cell_w)))
        cells[cell_idx]["hands"] += 1

    for i, cell in enumerate(cells):
        view = cell["view"]
        nf = cell["faces"]
        nh = cell["hands"]

        if view in face_views and nf == 0:
            issues.append(f"{view}: no face (head turned away)")
        if view in face_views and nf > 1:
            issues.append(f"{view}: {nf} faces")
        if view == "back" and nf >= 1:
            issues.append("back: face seen (head facing the wrong way)")
        if nh > 2:
            issues.append(f"{view}: {nh} hands")

        # Use the same clamped cell index as the counting loop
        hand_boxes = [
            h for h in hands if max(0, min(3, int(((h[0] + h[2]) / 2) // cell_w))) == i
        ]
        if len(hand_boxes) < 2:
            continue
        wrist_issue_added = False
        for j in range(len(hand_boxes)):
            for k in range(j + 1, len(hand_boxes)):
                hb1, hb2 = hand_boxes[j], hand_boxes[k]
                if iou(hb1, hb2) > WRIST_IOU:
                    if not wrist_issue_added:
                        issues.append(f"{view}: two hands at one wrist")
                        wrist_issue_added = True
                    break
                w1 = hb1[2] - hb1[0]
                w2 = hb2[2] - hb2[0]
                mean_w = (w1 + w2) / 2
                c1 = (hb1[0] + hb1[2]) / 2
                c2 = (hb2[0] + hb2[2]) / 2
                if abs(c1 - c2) < mean_w / 2:
                    if not wrist_issue_added:
                        issues.append(f"{view}: two hands at one wrist")
                        wrist_issue_added = True
                    break

    return {"ok": not issues, "score": len(issues), "issues": issues, "cells": cells}


def detect(path, models_dir):
    from PIL import Image
    from ultralytics import YOLO

    def model(name):
        mpath = os.path.join(models_dir, name)
        if mpath not in _MODELS:
            _MODELS[mpath] = YOLO(mpath)
        return _MODELS[mpath]

    img = Image.open(path).convert("RGB")
    w, h = img.size
    cw = w / 4
    faces, hands = [], []
    for i in range(4):
        crop = img.crop((int(i * cw), 0, int((i + 1) * cw), h))
        x0 = int(i * cw)
        for name, out in (("face_yolov8m.pt", faces), ("hand_yolov8s.pt", hands)):
            r = model(name)(crop, conf=0.2, imgsz=1024, verbose=False)[0]
            for b, c in zip(r.boxes.xyxy.tolist(), r.boxes.conf.tolist()):
                out.append([b[0] + x0, b[1], b[2] + x0, b[3], float(c)])
    return faces, hands, w, h


def check(path, models_dir, face_views=FACE_VIEWS):
    faces, hands, width, height = detect(path, models_dir)
    result = judge(faces, hands, width, height, face_views=face_views)
    result["file"] = path
    return result


class AnatomyHandler(BaseHTTPRequestHandler):
    models_dir = None

    def log_message(self, format, *args):
        pass

    def send_json(self, code, data):
        body = json.dumps(data).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/health":
            self.send_json(200, {"ok": True})
        else:
            self.send_json(404, {"error": "not found"})

    def do_POST(self):
        try:
            clen = int(self.headers.get("Content-Length", 0))
            if clen == 0:
                self.send_json(400, {"error": "no body"})
                return
            body = self.rfile.read(clen)
            with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
                tmp_path = f.name
                f.write(body)
            try:
                q = self.path.split("?")[1] if "?" in self.path else ""
                fv = FACE_VIEWS
                if q:
                    kv = dict(v.split("=") for v in q.split("&"))
                    if "face_views" in kv:
                        fv = tuple(kv["face_views"].split(","))

                res = check(tmp_path, self.models_dir, fv)
                del res["file"]
                self.send_json(200, res)
            finally:
                os.unlink(tmp_path)
        except (OSError, ValueError, RuntimeError, ImportError) as e:
            self.send_json(500, {"error": str(e)})


def serve(addr, models_dir):
    host, port = addr.split(":")
    AnatomyHandler.models_dir = models_dir
    server = HTTPServer((host, int(port)), AnatomyHandler)
    print(f"Serving on {addr}")
    server.serve_forever()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("images", nargs="*")
    parser.add_argument("--models", default=os.environ.get("ANATOMY_MODELS", "/models"))
    parser.add_argument("--face-views", default="front,left,right")
    parser.add_argument("--serve", metavar="HOST:PORT")
    parser.add_argument("--judge-json", default=None)
    args = parser.parse_args()

    if args.serve:
        serve(args.serve, args.models)
        return

    if args.judge_json:
        if args.judge_json == "-":
            data = json.load(sys.stdin)
        else:
            with open(args.judge_json) as f:
                data = json.load(f)
        result = judge(data["faces"], data["hands"], data["width"], data["height"])
        print(json.dumps(result))
        sys.exit(0)

    ok_any = False
    failed = False
    for path in args.images:
        try:
            res = check(path, args.models, tuple(args.face_views.split(",")))
            print(json.dumps(res))
            if res["ok"]:
                ok_any = True
            else:
                failed = True
        except (OSError, ValueError, RuntimeError, ImportError) as e:
            print(json.dumps({"ok": False, "error": str(e)}))
            failed = True
    sys.exit(1 if failed else (0 if ok_any else 1))


if __name__ == "__main__":
    main()
