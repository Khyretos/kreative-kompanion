"""STU-SHEET3D: the OC sheet v3 graph (stages 2-4) from four MV-Adapter views: crop each view to its figure, refine it
with SDXL img2img into a 640x1280 body cell, cut the head from the refined cell for the bare and helmet columns, compose
a 3200x1280 sheet with black cell lines. Inputs: LoadImage nodes 101-104 (views front, right, back, left as MV-Adapter
gives them), 190 (lines.png). Params: character prompt (node 4), helmet prompt (node 6)."""
import json, sys

CH = sys.argv[1] if len(sys.argv) > 1 else "1boy, solo, character"
HELM = sys.argv[2] if len(sys.argv) > 2 else ""
NEG = "lowres, bad anatomy, bad hands, extra fingers, multiple views, two people, frame, border, text, watermark, blurry"
VIEWS = [("front", 101, "front view", 0), ("right", 102, "right side view, profile", 1), ("back", 103, "back view, from behind", 3), ("left", 104, "left side view, profile", 2)]
g = {
    "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "novaAnimeXL_ilV170.safetensors"}},
    "3": {"class_type": "CLIPSetLastLayer", "inputs": {"clip": ["1", 1], "stop_at_clip_layer": -2}},
    "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["3", 0], "text": NEG}},
    "180": {"class_type": "EmptyImage", "inputs": {"width": 3200, "height": 1280, "batch_size": 1, "color": 0x808080}},
    "190": {"class_type": "LoadImage", "inputs": {"image": "sheet3d/lines.png"}},
    "191": {"class_type": "InvertMask", "inputs": {"mask": ["190", 1]}},
}
canvas = ["180", 0]
for i, (name, load, vt, col) in enumerate(VIEWS):
    b = 200 + 20 * i
    n = lambda k: str(b + k)
    g[str(load)] = {"class_type": "LoadImage", "inputs": {"image": f"sheet3d/view-{name}.png"}}
    # the figure cropped to the 640x1280 body cell and a 1024 square around its head (studio/comfy-nodes/kk_sheet.py)
    g[n(1)] = {"class_type": "KKFigureCrop", "inputs": {"image": [str(load), 0], "width": 640, "height": 1280, "pad": 0.04, "head_frac": 0.22, "head_size": 1024, "tolerance": 30}}
    g[n(3)] = {"class_type": "VAEEncodeTiled", "inputs": {"pixels": [n(1), 0], "vae": ["1", 2], "tile_size": 512, "overlap": 64, "temporal_size": 64, "temporal_overlap": 8}}
    g[n(4)] = {"class_type": "CLIPTextEncode", "inputs": {"clip": ["3", 0], "text": f"masterpiece, best quality, {CH}, full body, standing, {vt}, plain grey background"}}
    g[n(5)] = {"class_type": "KSampler", "inputs": {"model": ["1", 0], "positive": [n(4), 0], "negative": ["5", 0], "latent_image": [n(3), 0], "seed": 300 + i, "steps": 24, "cfg": 5.5, "sampler_name": "euler_ancestral", "scheduler": "normal", "denoise": 0.35}}
    g[n(6)] = {"class_type": "VAEDecodeTiled", "inputs": {"samples": [n(5), 0], "vae": ["1", 2], "tile_size": 512, "overlap": 64, "temporal_size": 64, "temporal_overlap": 8}}
    g[n(7)] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": [n(6), 0], "x": col * 640, "y": 0, "resize_source": False}}
    canvas = [n(7), 0]
    g[n(10)] = {"class_type": "VAEEncodeTiled", "inputs": {"pixels": [n(1), 1], "vae": ["1", 2], "tile_size": 512, "overlap": 64, "temporal_size": 64, "temporal_overlap": 8}}
    row = {"front": 0, "right": 1, "left": 2, "back": 3}[name]
    for j, (txt, den, x) in enumerate(((f"{CH}, head and shoulders, {vt}, plain grey background", 0.4, 2560),
                                       ((f"{CH}, wearing a {HELM}, head and shoulders, {vt}, plain grey background" if HELM else f"{CH}, head and shoulders, {vt}, plain grey background"), 0.7 if HELM else 0.4, 2880))):
        c = 11 + 4 * j
        g[n(c)] = {"class_type": "CLIPTextEncode", "inputs": {"clip": ["3", 0], "text": "masterpiece, best quality, " + txt}}
        g[n(c + 1)] = {"class_type": "KSampler", "inputs": {"model": ["1", 0], "positive": [n(c), 0], "negative": ["5", 0], "latent_image": [n(10), 0], "seed": 500 + i + 10 * j, "steps": 24, "cfg": 6, "sampler_name": "euler_ancestral", "scheduler": "normal", "denoise": den}}
        g[n(c + 2)] = {"class_type": "VAEDecodeTiled", "inputs": {"samples": [n(c + 1), 0], "vae": ["1", 2], "tile_size": 512, "overlap": 64, "temporal_size": 64, "temporal_overlap": 8}}
        g[n(c + 3)] = {"class_type": "ImageScale", "inputs": {"image": [n(c + 2), 0], "upscale_method": "lanczos", "width": 320, "height": 320, "crop": "disabled"}}
        g[str(900 + 10 * i + j)] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": [n(c + 3), 0], "x": x, "y": row * 320, "resize_source": False}}
        canvas = [str(900 + 10 * i + j), 0]
g["192"] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": ["190", 0], "x": 0, "y": 0, "resize_source": False, "mask": ["191", 0]}}
g["199"] = {"class_type": "SaveImage", "inputs": {"images": ["192", 0], "filename_prefix": "sheet3d/v3-sheet"}}
json.dump(g, sys.stdout, indent=1)
