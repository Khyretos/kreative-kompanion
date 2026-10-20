"""Tools/anatomy/dwpose.py (STU-PE3: DWPose on the CPU, generated image -> OpenPose keypoints).

This module provides a CPU-only implementation of DWpose for detecting people in an image
and converting the result to OpenPose format. It uses YOLOX for detection and a custom
ONNX model for pose estimation. No PyTorch is used; only numpy, cv2, and onnxruntime.
"""

import os
from typing import Any

import numpy as np


DET_SIZE = (640, 640)
POSE_SIZE = (288, 384)
MEAN = np.array([123.675, 116.28, 103.53])
STD = np.array([58.395, 57.12, 57.375])
SPLIT = 2.0


def det_preprocess(
    img: np.ndarray, size: tuple[int, int] = DET_SIZE
) -> tuple[np.ndarray, float]:
    """Preprocess image for detection.

    Args:
        img: HxWx3 uint8 (BGR).
        size: Target size (width, height).

    Returns:
        Padded and normalized tensor (1, 3, H, W) and the resize ratio.
    """
    H, W = img.shape[:2]
    ratio = min(size[0] / H, size[1] / W)
    padded = np.full((size[0], size[1], 3), 114, dtype=np.uint8)
    import cv2

    resized = cv2.resize(
        img, (int(W * ratio), int(H * ratio)), interpolation=cv2.INTER_LINEAR
    )
    padded[: int(H * ratio), : int(W * ratio)] = resized
    return (padded.transpose(2, 0, 1)[None].astype(np.float32), ratio)


def det_grid_decode(out: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    """YOLOX grid decode: cell offsets and strides 8, 16, 32 (cells row-major, the strides one after the other)."""
    grids, strides = [], []
    for s in (8, 16, 32):
        rows, cols = size[0] // s, size[1] // s
        yv, xv = np.meshgrid(np.arange(rows), np.arange(cols), indexing="ij")
        grids.append(np.stack((xv, yv), -1).reshape(-1, 2))
        strides.append(np.full((rows * cols, 1), s))
    grid = np.concatenate(grids)[None].astype(np.float32)
    stride = np.concatenate(strides)[None].astype(np.float32)
    d = out.astype(np.float32).copy()
    d[..., 0:2] = (out[..., 0:2] + grid) * stride
    d[..., 2:4] = np.exp(out[..., 2:4]) * stride
    return d


def nms(boxes: np.ndarray, scores: np.ndarray, thr: float) -> list[int]:
    """Non-maximum suppression.

    Args:
        boxes: Nx4 xyxy boxes.
        scores: Nx1 scores.
        thr: IoU threshold.

    Returns:
        List of kept indices.
    """
    order = np.argsort(scores)[::-1]
    keep = []
    while len(order) > 0:
        i = order[0]
        keep.append(i)
        x1a, y1a, x2a, y2a = boxes[i]
        area_a = (x2a - x1a) * (y2a - y1a)
        if area_a <= 0:
            break
        order = order[1:]
        new_order = []
        for j in order:
            x1b, y1b, x2b, y2b = boxes[j]
            xi1, yi1 = max(x1a, x1b), max(y1a, y1b)
            xi2, yi2 = min(x2a, x2b), min(y2a, y2b)
            if xi2 <= xi1 or yi2 <= yi1:
                new_order.append(j)
            else:
                inter = (xi2 - xi1) * (yi2 - yi1)
                ua = area_a
                ub = (x2b - x1b) * (y2b - y1b)
                iou_val = inter / (ua + ub - inter) if (ua + ub - inter) > 0 else 0.0
                if iou_val <= thr:
                    new_order.append(j)
        order = new_order
    return keep


def detect_people(
    sess, img: np.ndarray, score_thr: float = 0.3, nms_thr: float = 0.45
) -> np.ndarray:
    """Detect people in the image.

    Args:
        sess: ONNX inference session.
        img: HxWx3 uint8 (BGR).
        score_thr: Minimum confidence score.
        nms_thr: NMS IoU threshold.

    Returns:
        Kept boxes (K, 4) xyxy.
    """
    t, ratio = det_preprocess(img)
    out = sess.run(None, {sess.get_inputs()[0].name: t})[0]
    d = det_grid_decode(out, DET_SIZE)[0]
    scores = d[:, 4] * d[:, 5]
    c, wh = d[:, 0:2], d[:, 2:4]
    boxes = np.concatenate((c - wh / 2, c + wh / 2), axis=1) / ratio
    sel = np.where(scores > score_thr)[0]
    if len(sel) == 0:
        return np.zeros((0, 4), dtype=np.float32)
    kept = nms(boxes[sel], scores[sel], nms_thr)
    return boxes[sel][kept]


def box_to_cs(box: np.ndarray, padding: float = 1.25) -> tuple[np.ndarray, np.ndarray]:
    """Convert box to centre and scale.

    Args:
        box: 1x4 xyxy.
        padding: Padding factor.

    Returns:
        Centre (1, 2) and scale (1, 2).
    """
    c = np.array([(box[0] + box[2]) / 2, (box[1] + box[3]) / 2])
    s = np.array([(box[2] - box[0]) * padding, (box[3] - box[1]) * padding])
    return c, s


def fix_aspect(scale: np.ndarray, aspect: float) -> np.ndarray:
    """Fix aspect ratio to match target.

    Args:
        scale: Current scale (w, h).
        aspect: Target aspect ratio (width/height).

    Returns:
        Fixed scale (w, h).
    """
    w, h = scale
    if w > h * aspect:
        return np.array([w, w / aspect])
    return np.array([h * aspect, h])


def third(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Compute third point by rotating (second - first) 90 degrees around first."""
    return b + np.array([a[1] - b[1], b[0] - a[0]])


def pose_preprocess(
    img: np.ndarray, box: np.ndarray
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Preprocess image for pose estimation.

    Args:
        img: HxWx3 uint8 (BGR).
        box: 1x4 xyxy.

    Returns:
        Normalized input (1, 3, H, W), centre, and scale.
    """
    c, s = box_to_cs(box)
    s = fix_aspect(s, POSE_SIZE[0] / POSE_SIZE[1])
    src = np.zeros((3, 2), dtype=np.float32)
    src[0] = c
    src[1] = c + np.array([0, -s[1] / 2])
    src[2] = third(src[0], src[1])
    dst = np.zeros((3, 2), dtype=np.float32)
    dst[0] = np.array([POSE_SIZE[0] / 2, POSE_SIZE[1] / 2])
    dst[1] = np.array([POSE_SIZE[0] / 2, 0])
    dst[2] = third(dst[0], dst[1])
    import cv2

    M = cv2.getAffineTransform(src, dst)
    crop = cv2.warpAffine(img, M, POSE_SIZE, flags=cv2.INTER_LINEAR)
    x = ((crop - MEAN) / STD).astype(np.float32)
    x = x.transpose(2, 0, 1)[None]
    return x, c, s


def simcc_decode(
    simcc_x: np.ndarray, simcc_y: np.ndarray, split: float = SPLIT
) -> tuple[np.ndarray, np.ndarray]:
    """Decode SimCC output to keypoints.

    Args:
        simcc_x: X coordinates (N, K, Wx).
        simcc_y: Y coordinates (N, K, Wy).
        split: Split factor.

    Returns:
        Keypoints (N, K, 2) and scores (N, K).
    """
    x = np.argmax(simcc_x, axis=-1)
    y = np.argmax(simcc_y, axis=-1)
    vx = simcc_x[np.arange(simcc_x.shape[0]), np.arange(simcc_x.shape[1]), x]
    vy = simcc_y[np.arange(simcc_y.shape[0]), np.arange(simcc_y.shape[1]), y]
    kp = np.stack([x / split, y / split], axis=-1)
    sc = np.minimum(vx, vy)
    return kp, sc


def to_image(
    kp: np.ndarray, input_size: tuple[int, int], center: np.ndarray, scale: np.ndarray
) -> np.ndarray:
    """Transform keypoints back to original image coordinates.

    Args:
        kp: Keypoints (N, K, 2).
        input_size: Pose input size (W, H).
        center: Centre (1, 2).
        scale: Scale (1, 2).

    Returns:
        Transformed keypoints (N, K, 2).
    """
    return kp / np.array(input_size) * scale + center - scale / 2


def estimate(sess, img: np.ndarray, boxes: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Estimate poses for detected people.

    Args:
        sess: ONNX inference session.
        img: HxWx3 uint8 (BGR).
        boxes: Detected boxes (K, 4).

    Returns:
        Keypoints (P, 133, 2) and scores (P, 133).
    """
    if len(boxes) == 0:
        return np.zeros((0, 133, 2)), np.zeros((0, 133))
    all_kp = []
    all_sc = []
    for box in boxes:
        x, c, s = pose_preprocess(img, box)
        sx, sy = sess.run(None, {sess.get_inputs()[0].name: x})[:2]
        kp, sc = simcc_decode(sx, sy)
        kp = to_image(kp, POSE_SIZE, c, s)
        all_kp.append(kp[0])  # one box per call: drop the batch axis
        all_sc.append(sc[0])
    return np.stack(all_kp), np.stack(all_sc)


def to_openpose(
    kp: np.ndarray, sc: np.ndarray, width: int, height: int
) -> dict[str, Any]:
    """Convert keypoints to OpenPose format.

    Args:
        kp: Keypoints (P, 133, 2).
        sc: Scores (P, 133).
        width: Image width.
        height: Image height.

    Returns:
        OpenPose JSON structure.
    """
    body_order = [0, "neck", 6, 8, 10, 5, 7, 9, 12, 14, 16, 11, 13, 15, 2, 1, 4, 3]
    hand_left = list(range(91, 112))
    hand_right = list(range(112, 133))

    def make_person(k: int, s: float) -> dict[str, list[float]]:
        pose = []
        for idx in body_order:
            if isinstance(idx, str):
                if idx == "neck":
                    neck_x = (kp[k, 5, 0] + kp[k, 6, 0]) / 2
                    neck_y = (kp[k, 5, 1] + kp[k, 6, 1]) / 2
                    neck_s = min(sc[k, 5], sc[k, 6])
                    pose.extend([float(neck_x), float(neck_y), float(neck_s)])
                else:
                    pose.extend(
                        [float(kp[k, idx, 0]), float(kp[k, idx, 1]), float(sc[k, idx])]
                    )
            else:
                pose.extend(
                    [float(kp[k, idx, 0]), float(kp[k, idx, 1]), float(sc[k, idx])]
                )

        left_hand = []
        for idx in hand_left:
            left_hand.extend(
                [float(kp[k, idx, 0]), float(kp[k, idx, 1]), float(sc[k, idx])]
            )

        right_hand = []
        for idx in hand_right:
            right_hand.extend(
                [float(kp[k, idx, 0]), float(kp[k, idx, 1]), float(sc[k, idx])]
            )

        return {
            "pose_keypoints_2d": pose,
            "hand_left_keypoints_2d": left_hand,
            "hand_right_keypoints_2d": right_hand,
        }

    people = []
    for k in range(len(kp)):
        people.append(make_person(k, 1.0))

    return {"canvas_width": width, "canvas_height": height, "people": people}


class DWPose:
    """DWpose detector and pose estimator."""

    def __init__(self, models_dir: str) -> None:
        import onnxruntime as ort

        opts = ort.SessionOptions()
        opts.intra_op_num_threads = 4
        self.det = ort.InferenceSession(
            os.path.join(models_dir, "yolox_l.onnx"),
            opts,
            providers=["CPUExecutionProvider"],
        )
        self.pose = ort.InferenceSession(
            os.path.join(models_dir, "dw-ll_ucoco_384.onnx"),
            opts,
            providers=["CPUExecutionProvider"],
        )

    def __call__(self, img: np.ndarray) -> dict[str, Any]:
        boxes = detect_people(self.det, img)
        kp, sc = estimate(self.pose, img, boxes)
        return to_openpose(kp, sc, img.shape[1], img.shape[0])
