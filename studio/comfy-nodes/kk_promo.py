# Kompanion ComfyUI node for promotional banners (STU-P3, 2026-10-25). Install: this file and tools/promo/compose.py
# (as kk_promo_compose.py) side by side in ComfyUI's custom_nodes, read-only.
# KKPromoBanner: background, cut-out subject and logo pictures plus headline and tagline -> banner.png, banner.ora
# (layers for Krita and GIMP) and banner.svg (live text) in the output folder, with the WCAG AA text check.
import importlib.util
import json
import os

import numpy as np
from PIL import Image

_spec = importlib.util.spec_from_file_location(
    "kk_promo_compose", os.path.join(os.path.dirname(__file__), "kk_promo_compose.py")
)
pc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pc)


def _pil(image, mask=None):
    """A ComfyUI IMAGE (1 x H x W x C, 0..1) as a PIL RGBA picture; a MASK (ComfyUI: 1 = masked) becomes its alpha."""
    a = (image[0].cpu().numpy() * 255).clip(0, 255).astype(np.uint8)
    im = Image.fromarray(a[..., :4] if a.shape[-1] == 4 else a[..., :3])
    im = im.convert("RGBA")
    if mask is not None:
        m = ((1.0 - mask[0].cpu().numpy()) * 255).clip(0, 255).astype(np.uint8)
        im.putalpha(Image.fromarray(m).resize(im.size))
    return im


class KKPromoBanner:
    @classmethod
    def INPUT_TYPES(cls):
        return {
            "required": {
                "background": ("IMAGE",),
                "headline": ("STRING", {"default": "kk-engine"}),
                "tagline": ("STRING", {"default": ""}),
                "size": (sorted(pc.SIZES),),
                "side": (["right", "left"],),
                "filename_prefix": ("STRING", {"default": "kompanion/banner"}),
            },
            "optional": {
                "subject": ("IMAGE",),
                "subject_mask": ("MASK",),
                "logo": ("IMAGE",),
                "logo_mask": ("MASK",),
            },
        }

    RETURN_TYPES = ()
    FUNCTION = "run"
    OUTPUT_NODE = True
    CATEGORY = "kk"

    def run(
        self,
        background,
        headline,
        tagline,
        size,
        side,
        filename_prefix,
        subject=None,
        subject_mask=None,
        logo=None,
        logo_mask=None,
    ):
        import folder_paths

        spec = {
            "size": size,
            "side": side,
            "headline": headline,
            "tagline": tagline,
            "background": _pil(background),
        }
        if subject is not None:
            spec["subject"] = _pil(subject, subject_mask)
        if logo is not None:
            spec["logo"] = _pil(logo, logo_mask)
        b = pc.compose(spec)
        w, h = pc.SIZES[size]
        out_dir, stem, _, subfolder, _ = folder_paths.get_save_image_path(
            filename_prefix, folder_paths.get_output_directory(), w, h
        )
        n = 1
        while os.path.exists(os.path.join(out_dir, f"{stem}_{n:05}_.png")):
            n += 1
        name = f"{stem}_{n:05}_"
        paths = pc.write_all(b, out_dir, name)
        os.replace(
            os.path.join(out_dir, "check.json"),
            os.path.join(out_dir, name + ".check.json"),
        )
        # the PNG first: it is the picture the Studio shows; the .ora and .svg are the files to edit
        files = [os.path.basename(p) for p in (paths[2], paths[0], paths[1])]
        print("KKPromoBanner", json.dumps(b["check"]))
        return {
            "ui": {
                "images": [
                    {"filename": f, "subfolder": subfolder, "type": "output"}
                    for f in files
                ]
            }
        }


NODE_CLASS_MAPPINGS = {"KKPromoBanner": KKPromoBanner}
NODE_DISPLAY_NAME_MAPPINGS = {"KKPromoBanner": "Banner (layers, SVG, PNG)"}
