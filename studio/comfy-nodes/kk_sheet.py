# Kompanion ComfyUI node for the OC sheet v3 (STU-SHEET3D, 2026-10-25). Install: custom_nodes (read-only mount).
# KKFigureCrop: a figure on a flat background (an MV-Adapter view) -> the figure cropped and padded to a fixed cell, and
# a square around its head (the top part of the figure, centred on that band's pixels) for the head grid.
import numpy as np
import torch


def _figure(a, tol):
    """Pixels that differ from the corner colour by more than tol (sum of RGB in 0..255)."""
    bg = np.median(
        np.concatenate(
            [
                a[:4, :4].reshape(-1, 3),
                a[:4, -4:].reshape(-1, 3),
                a[-4:, :4].reshape(-1, 3),
                a[-4:, -4:].reshape(-1, 3),
            ]
        ),
        axis=0,
    )
    return np.abs(a - bg).sum(2) > tol, bg


def _fit(a, bg, w, h):
    """Scale a to fit inside w x h (aspect kept) and centre it on a background-coloured canvas."""
    from PIL import Image

    s = min(w / a.shape[1], h / a.shape[0])
    im = Image.fromarray(a.astype(np.uint8)).resize(
        (max(1, round(a.shape[1] * s)), max(1, round(a.shape[0] * s))), Image.LANCZOS
    )
    out = Image.new("RGB", (w, h), tuple(int(v) for v in bg))
    out.paste(im, ((w - im.width) // 2, (h - im.height) // 2))
    return np.asarray(out).astype(np.float32) / 255.0


class KKFigureCrop:
    @classmethod
    def INPUT_TYPES(cls):
        return {
            "required": {
                "image": ("IMAGE",),
                "width": ("INT", {"default": 640, "min": 64, "max": 4096}),
                "height": ("INT", {"default": 1280, "min": 64, "max": 4096}),
                "pad": (
                    "FLOAT",
                    {"default": 0.04, "min": 0.0, "max": 0.5, "step": 0.01},
                ),
                "head_frac": (
                    "FLOAT",
                    {"default": 0.22, "min": 0.05, "max": 0.6, "step": 0.01},
                ),
                "head_size": ("INT", {"default": 512, "min": 64, "max": 2048}),
                "tolerance": ("INT", {"default": 30, "min": 1, "max": 400}),
            }
        }

    RETURN_TYPES = ("IMAGE", "IMAGE")
    RETURN_NAMES = ("body", "head")
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, image, width, height, pad, head_frac, head_size, tolerance):
        bodies, heads = [], []
        for img in image:
            a = (img.cpu().numpy()[..., :3] * 255.0).astype(np.float32)
            m, bg = _figure(a, tolerance)
            ys, xs = np.where(m)
            if len(ys) == 0:
                raise ValueError(
                    "KKFigureCrop: no figure found (the picture is one flat colour)"
                )
            y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
            p = int(round((y1 - y0) * pad))
            H, W = a.shape[:2]
            body = a[max(0, y0 - p) : min(H, y1 + p), max(0, x0 - p) : min(W, x1 + p)]
            bodies.append(_fit(body, bg, width, height))
            hh = max(8, int(round((y1 - y0) * head_frac)))
            band = m[y0 : y0 + hh]
            bx = np.where(band.any(0))[0]
            cx = int((bx.min() + bx.max()) // 2) if len(bx) else (x0 + x1) // 2
            side = int(hh * 1.15)
            hx0, hy0 = max(0, cx - side // 2), max(0, y0 - side // 12)
            head = a[hy0 : min(H, hy0 + side), hx0 : min(W, hx0 + side)]
            heads.append(_fit(head, bg, head_size, head_size))
        return (torch.from_numpy(np.stack(bodies)), torch.from_numpy(np.stack(heads)))


NODE_CLASS_MAPPINGS = {"KKFigureCrop": KKFigureCrop}
NODE_DISPLAY_NAME_MAPPINGS = {"KKFigureCrop": "Figure crop (body cell and head)"}
