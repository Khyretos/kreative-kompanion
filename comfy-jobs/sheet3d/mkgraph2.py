"""STU-SHEET3D: the OC sheet v3 graph for Kompanion's oc-sheet-v3 workflow (main stage; the MV-Adapter views come from
the workflow's [pre] stage). Node 4 = the character prompt (param), node 6 = the helmet prompt (param, may be empty),
LoadImage 101-104 = the four views in MV-Adapter's order (front, right, back, left), 190 = the cell lines.
Per view: KKFigureCrop (studio/comfy-nodes/kk_sheet.py) -> SDXL img2img body cell; its head square -> bare and helmet
img2img; everything composited onto one 3200x1280 sheet, lines on top. Run: python3 mkgraph2.py > graph.json"""
import json

NEG = "lowres, bad anatomy, bad hands, extra fingers, multiple views, two people, frame, border, text, watermark, blurry"
VIEWS = [("front", 101, "front view", 0, 0), ("right", 102, "right side view, profile", 1, 1), ("back", 103, "back view, from behind", 3, 3), ("left", 104, "left side view, profile", 2, 2)]
enc = lambda text: {"class_type": "CLIPTextEncode", "inputs": {"clip": ["3", 0], "text": text}}
cat = lambda a, b: {"class_type": "ConditioningConcat", "inputs": {"conditioning_to": a, "conditioning_from": b}}
g = {
    "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "novaAnimeXL_ilV170.safetensors"}},
    "3": {"class_type": "CLIPSetLastLayer", "inputs": {"clip": ["1", 1], "stop_at_clip_layer": -2}},
    "4": enc("masterpiece, best quality, a character"),
    "5": enc(NEG),
    "6": enc(""),
    "7": cat(["4", 0], ["6", 0]),
    # one face for every cell: IP-Adapter plus-face on the front view's head (Kees 2026-10-25: "the head in every
    # picture is different" when each cell was refined on its own)
    "30": {"class_type": "CLIPVisionLoader", "inputs": {"clip_name": "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors"}},
    "33": {"class_type": "IPAdapterModelLoader", "inputs": {"ipadapter_file": "ip-adapter-plus-face_sdxl_vit-h.safetensors"}},
    "34": {"class_type": "IPAdapterAdvanced", "inputs": {"model": ["1", 0], "ipadapter": ["33", 0], "image": ["201", 1], "clip_vision": ["30", 0], "weight": 0.6, "weight_type": "linear", "combine_embeds": "concat", "start_at": 0, "end_at": 0.8, "embeds_scaling": "V only"}},
    "180": {"class_type": "EmptyImage", "inputs": {"width": 3200, "height": 1280, "batch_size": 1, "color": 0x808080}},
    "190": {"class_type": "LoadImage", "inputs": {"image": "lines.png"}},
    "191": {"class_type": "InvertMask", "inputs": {"mask": ["190", 1]}},
}
vae = ["1", 2]
canvas = ["180", 0]
tiled = lambda k, src: {"class_type": k, "inputs": {("pixels" if k == "VAEEncodeTiled" else "samples"): src, "vae": vae, "tile_size": 512, "overlap": 64, "temporal_size": 64, "temporal_overlap": 8}}
for i, (name, load, vt, col, row) in enumerate(VIEWS):
    n = lambda k, b=200 + 30 * i: str(b + k)
    g[str(load)] = {"class_type": "LoadImage", "inputs": {"image": f"view-{name}.png"}}
    g[n(1)] = {"class_type": "KKFigureCrop", "inputs": {"image": [str(load), 0], "width": 640, "height": 1280, "pad": 0.04, "head_frac": 0.22, "head_size": 1024, "tolerance": 30}}
    g[n(2)] = enc(f"full body, standing, {vt}, plain grey background")
    g[n(3)] = cat(["4", 0], [n(2), 0])
    g[n(4)] = tiled("VAEEncodeTiled", [n(1), 0])
    g[n(5)] = {"class_type": "KSampler", "inputs": {"model": ["34", 0], "positive": [n(3), 0], "negative": ["5", 0], "latent_image": [n(4), 0], "seed": 300, "steps": 24, "cfg": 5.5, "sampler_name": "euler_ancestral", "scheduler": "normal", "denoise": 0.3}}
    g[n(6)] = tiled("VAEDecodeTiled", [n(5), 0])
    g[n(7)] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": [n(6), 0], "x": col * 640, "y": 0, "resize_source": False}}
    canvas = [n(7), 0]
    g[n(8)] = tiled("VAEEncodeTiled", [n(1), 1])
    g[n(9)] = enc(f"head and shoulders, {vt}, plain grey background")
    for j, (cond_base, den, x) in enumerate(((["4", 0], 0.35, 2560), (["7", 0], 0.55, 2880))):
        c = 10 + 5 * j
        g[n(c)] = cat(cond_base, [n(9), 0])
        g[n(c + 1)] = {"class_type": "KSampler", "inputs": {"model": ["34", 0], "positive": [n(c), 0], "negative": ["5", 0], "latent_image": [n(8), 0], "seed": 500 + 10 * j, "steps": 24, "cfg": 6, "sampler_name": "euler_ancestral", "scheduler": "normal", "denoise": den}}
        g[n(c + 2)] = tiled("VAEDecodeTiled", [n(c + 1), 0])
        g[n(c + 3)] = {"class_type": "ImageScale", "inputs": {"image": [n(c + 2), 0], "upscale_method": "lanczos", "width": 320, "height": 320, "crop": "disabled"}}
        g[n(c + 4)] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": [n(c + 3), 0], "x": x, "y": row * 320, "resize_source": False}}
        canvas = [n(c + 4), 0]
g["192"] = {"class_type": "ImageCompositeMasked", "inputs": {"destination": canvas, "source": ["190", 0], "x": 0, "y": 0, "resize_source": False, "mask": ["191", 0]}}
g["20"] = {"class_type": "SaveImage", "inputs": {"images": ["192", 0], "filename_prefix": "kompanion/oc-sheet-v3"}}
print(json.dumps(g, indent=2))
