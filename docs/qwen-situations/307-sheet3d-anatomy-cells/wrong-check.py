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

CELLS = {
    "v2": [
        ("front", 0.0, 0.0, 0.25, 1.0),
        ("left", 0.25, 0.0, 0.5, 1.0),
        ("back", 0.5, 0.0, 0.75, 1.0),
        ("right", 0.75, 0.0, 1.0, 1.0),
    ],
    "oc-v3": [
        ("front", 0.0, 0.0, 0.2, 1.0),
        ("right", 0.2, 0.0, 0.4, 1.0),
        ("left", 0.4, 0.0, 0.6, 1.0),
        ("back", 0.6, 0.0, 0.8, 1.0),
        ("head-front", 0.8, 0.0, 0.9, 0.25),
        ("helmet-front", 0.9, 0.0, 1.0, 0.25),
        ("head-right", 0.8, 0.25, 0.9, 0.5),
        ("helmet-right", 0.9, 0.25, 1.0, 0.5),
        ("head-left", 0.8, 0.5, 0.9, 0.75),
        ("helmet-left", 0.9, 0.5, 1.0, 0.75),
        ("head-back", 0.8, 0.75, 0.9, 1.0),
        ("helmet-back", 0.9, 0.75, 1.0, 1.0),
    ],
}


def cell_index(box, width, height, cells):
    fx = (box[0] + box[2]) / 2 / width
    fy = (box[1] + box[3]) / 2 / height
    fx = max(0.0, min(0.999999, fx))
    fy = max(0.0, min(0.999999, fy))
    for i, (name, x0, y0, x1, y1) in enumerate(cells):
        if x0 <= fx < x1 and y0 <= fy < y1:
            return i
    return None


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
    cells=None,
):
    if cells is None:
        cells = CELLS["v2"]
    faces = [f for f in faces if f[4] >= face_conf]
    hands = [h for h in hands if h[4] >= hand_conf]
    cells_out = [{"view": c[0], "faces": 0, "hands": 0} for c in cells]
    issues = []
    cell_w = width / 4

    face_boxes = []
    hand_boxes = []
    for box in faces:
        if box[4] >= face_conf:
            face_boxes.append(box)
    for box in hands:
        if box[4] >= hand_conf:
            hand_boxes.append(box)

    for box in face_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            base = name.split("-")[-1]
            if name.startswith("helmet-"):
                continue
            if base in face_views:
                if len(face_boxes) == 0:
                    pass
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf == 0:
                    issues.append(f"{name}: no face (head turned away)")
                elif nf > 1:
                    issues.append(f"{name}: {nf} faces")
            elif base == "back":
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf >= 1:
                    issues.append(f"{name}: face seen (head facing the wrong way)")
            cells_out[idx]["faces"] += 1

    for box in hand_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            if "-" not in name:
                cells_out[idx]["hands"] += 1

    return {
        "ok": not issues,
        "score": len(issues),
        "issues": issues,
        "cells": cells_out,
    }

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


def detect(path, models_dir, cells=None):
    from PIL import Image
    from ultralytics import YOLO

    if cells is None:
        cells = CELLS["v2"]

    def model(name):
        mpath = os.path.join(models_dir, name)
        if mpath not in _MODELS:
            _MODELS[mpath] = YOLO(mpath)
        return _MODELS[mpath]

    img = Image.open(path).convert("RGB")
    w, h = img.size
    faces, hands = [], []
    for name, out in (("face_yolov8m.pt", faces), ("hand_yolov8s.pt", hands)):
        for _, (x0_frac, y0_frac, x1_frac, y1_frac) in enumerate(cells):
            x0 = int(x0_frac * w)
            y0 = int(y0_frac * h)
            x1 = int(x1_frac * w)
            y1 = int(y1_frac * h)
            crop = img.crop((x0, y0, x1, y1))
            r = model(name)(crop, conf=0.2, imgsz=1024, verbose=False)[0]
            for b, c in zip(r.boxes.xyxy.tolist(), r.boxes.conf.tolist()):
                bx, by, bw, bh = b
                out.append([bx + x0, by, bw + x0, bh, float(c)])
    return faces, hands, w, h


def judge_v3_old(
    faces,
    hands,
    width,
    height,
    face_views=FACE_VIEWS,
    face_conf=FACE_CONF,
    hand_conf=HAND_CONF,
    cells=None,
):
    if cells is None:
        cells = CELLS["oc-v3"]
    faces = [f for f in faces if f[4] >= face_conf]
    hands = [h for h in hands if h[4] >= hand_conf]
    cells_out = [{"view": c[0], "faces": 0, "hands": 0} for c in cells]
    issues = []

    face_boxes = []
    hand_boxes = []
    for box in faces:
        if box[4] >= face_conf:
            face_boxes.append(box)
    for box in hands:
        if box[4] >= hand_conf:
            hand_boxes.append(box)

    cell_w = width / 10.0

    for box in face_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            base = name.split("-")[-1]
            if name.startswith("helmet-"):
                continue
            if base in face_views:
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf == 0:
                    issues.append(f"{name}: no face (head turned away)")
                elif nf > 1:
                    issues.append(f"{name}: {nf} faces")
            elif base == "back":
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf >= 1:
                    issues.append(f"{name}: face seen (head facing the wrong way)")
            cells_out[idx]["faces"] += 1

    for box in hand_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            if "-" not in name:
                cells_out[idx]["hands"] += 1

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
        hand_boxes_in_cell = [
            h for h in hands if max(0, min(3, int(((h[0] + h[2]) / 2) // cell_w))) == i
        ]
        if len(hand_boxes_in_cell) < 2:
            continue
        wrist_issue_added = False
        for j in range(len(hand_boxes_in_cell)):
            for k in range(j + 1, len(hand_boxes_in_cell)):
                hb1, hb2 = hand_boxes_in_cell[j], hand_boxes_in_cell[k]
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

    return {
        "ok": not issues,
        "score": len(issues),
        "issues": issues,
        "cells": cells_out,
    }


def judge_v3(
    faces,
    hands,
    width,
    height,
    face_views=FACE_VIEWS,
    face_conf=FACE_CONF,
    hand_conf=HAND_CONF,
    cells=None,
):
    if cells is None:
        cells = CELLS["oc-v3"]
    faces = [f for f in faces if f[4] >= face_conf]
    hands = [h for h in hands if h[4] >= hand_conf]
    cells_out = [{"view": c[0], "faces": 0, "hands": 0} for c in cells]
    issues = []

    face_boxes = []
    hand_boxes = []
    for box in faces:
        if box[4] >= face_conf:
            face_boxes.append(box)
    for box in hands:
        if box[4] >= hand_conf:
            hand_boxes.append(box)

    cell_w = width / 10.0

    for box in face_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            base = name.split("-")[-1]
            if name.startswith("helmet-"):
                continue
            if base in face_views:
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf == 0:
                    issues.append(f"{name}: no face (head turned away)")
                elif nf > 1:
                    issues.append(f"{name}: {nf} faces")
            elif base == "back":
                nf = sum(
                    1 for b in face_boxes if cell_index(b, width, height, cells) == idx
                )
                if nf >= 1:
                    issues.append(f"{name}: face seen (head facing the wrong way)")
            cells_out[idx]["faces"] += 1

    for box in hand_boxes:
        idx = cell_index(box, width, height, cells)
        if idx is not None:
            name = cells[idx][0]
            if "-" not in name:
                cells_out[idx]["hands"] += 1

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
        hand_boxes_in_cell = [
            h for h in hands if max(0, min(3, int(((h[0] + h[2]) / 2) // cell_w))) == i
        ]
        if len(hand_boxes_in_cell) < 2:
            continue
        wrist_issue_added = False
        for j in range(len(hand_boxes_in_cell)):
            for k in range(j + 1, len(hand_boxes_in_cell)):
                hb1, hb2 = hand_boxes_in_cell[j], hand_boxes_in_cell[k]
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

    return {
        "ok": not issues,
        "score": len(issues),
        "issues": issues,
        "cells": cells_out,
    }


def parse_query(path):
    """Parse query string to get face_views and layout."""
    q = path.split("?")[1] if "?" in path else ""
    if not q:
        return FACE_VIEWS, "v2"

    kv = {}
    for item in q.split("&"):
        if "=" in item:
            k, v = item.split("=", 1)
            kv[k] = v

    fv = FACE_VIEWS
    layout = "v2"

    if "face_views" in kv:
        fv = tuple(kv["face_views"].split(","))
    if "layout" in kv:
        layout = kv["layout"]

    if layout not in CELLS:
        raise ValueError(f"unknown layout {layout}")

    return fv, layout


def check(path, models_dir, face_views=FACE_VIEWS, layout="v2"):
    cells = CELLS[layout]
    faces, hands, width, height = detect(path, models_dir, cells=cells)
    result = judge(faces, hands, width, height, face_views=face_views, cells=cells)
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
                fv, layout = parse_query(self.path)
                res = check(tmp_path, self.models_dir, fv, layout)
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
    parser.add_argument("--layout", default="v2", choices=sorted(CELLS))
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
        layout = data.get("layout", args.layout)
        cells = CELLS[layout]
        result = judge(
            data["faces"], data["hands"], data["width"], data["height"], cells=cells
        )
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
