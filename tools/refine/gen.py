"""STU-R2: writes the five refine workflows (graph.json [+ graph-face.json] + workflow.toml).
Usage: python3 tools/refine/gen.py studio/workflows"""

import copy
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(__file__))
from fmt import dump  # noqa: E402

ROOT = sys.argv[1]
CK = "novaAnimeXL_ilV170.safetensors"
POS = "masterpiece, best quality, amazing quality, very aesthetic, absurdres, {rating}, adult, mature, {value}"
NEG = "worst quality, bad quality, low quality, lowres, blurry, jpeg artifacts, watermark, signature, text, logo, bad anatomy, bad hands, extra fingers, missing fingers, fused fingers, extra limbs, deformed, disfigured, mutated, child, loli, shota, underage, young child, kid, toddler, teen, minor, childlike body, {rating_neg}{value}"


def L(n, i=0):
    return [str(n), i]


def base():
    return {
        "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": CK}},
        "3": {
            "class_type": "CLIPSetLastLayer",
            "inputs": {"clip": L(1, 1), "stop_at_clip_layer": -2},
        },
        "4": {
            "class_type": "CLIPTextEncode",
            "inputs": {"clip": L(3), "text": "masterpiece"},
        },
        "5": {
            "class_type": "CLIPTextEncode",
            "inputs": {"clip": L(3), "text": "worst quality"},
        },
        "10": {"class_type": "LoadImage", "inputs": {"image": "source.png"}},
    }


def ks(model, pos, neg, latent, denoise, steps=24):
    return {
        "class_type": "KSampler",
        "inputs": {
            "model": model,
            "positive": pos,
            "negative": neg,
            "latent_image": latent,
            "seed": 1,
            "steps": steps,
            "cfg": 5.5,
            "sampler_name": "euler_ancestral",
            "scheduler": "normal",
            "denoise": denoise,
        },
    }


def tiled(kind, src):
    i = {
        "vae": L(1, 2),
        "tile_size": 512,
        "overlap": 64,
        "temporal_size": 64,
        "temporal_overlap": 8,
    }
    i["pixels" if kind == "VAEEncodeTiled" else "samples"] = src
    return {"class_type": kind, "inputs": i}


def save(img, name):
    return {
        "class_type": "SaveImage",
        "inputs": {"images": img, "filename_prefix": f"kompanion/{name}"},
    }


def face_fix(img, model, seed_from=None):
    return {
        "40": {
            "class_type": "UltralyticsDetectorProvider",
            "inputs": {"model_name": "bbox/face_yolov8m.pt"},
        },
        "41": {
            "class_type": "SAMLoader",
            "inputs": {"model_name": "sam_vit_b_01ec64.pth", "device_mode": "AUTO"},
        },
        "42": {
            "class_type": "FaceDetailer",
            "inputs": {
                "image": img,
                "model": model,
                "clip": L(3),
                "vae": L(1, 2),
                "positive": L(4),
                "negative": L(5),
                "guide_size": 512,
                "guide_size_for": True,
                "max_size": 1024,
                "seed": 2,
                "steps": 20,
                "cfg": 5.5,
                "sampler_name": "euler_ancestral",
                "scheduler": "normal",
                "denoise": 0.35,
                "feather": 5,
                "noise_mask": True,
                "force_inpaint": True,
                "bbox_threshold": 0.5,
                "bbox_dilation": 10,
                "bbox_crop_factor": 3,
                "sam_detection_hint": "center-1",
                "sam_dilation": 0,
                "sam_threshold": 0.93,
                "sam_bbox_expansion": 0,
                "sam_mask_hint_threshold": 0.7,
                "sam_mask_hint_use_negative": "False",
                "drop_size": 10,
                "bbox_detector": L(40),
                "sam_model_opt": L(41),
                "wildcard": "",
                "cycle": 1,
                "tiled_encode": True,
                "tiled_decode": True,
            },
        },
    }


def upscaler(img):
    return {
        "50": {
            "class_type": "UpscaleModelLoader",
            "inputs": {"model_name": "4x-AnimeSharp.pth"},
        },
        "51": {
            "class_type": "ImageUpscaleWithModel",
            "inputs": {"upscale_model": L(50), "image": img},
        },
    }


def back_to_source(img):
    return {
        "52": {"class_type": "GetImageSize", "inputs": {"image": L(10)}},
        "53": {
            "class_type": "ImageScale",
            "inputs": {
                "image": img,
                "upscale_method": "lanczos",
                "width": L(52, 0),
                "height": L(52, 1),
                "crop": "disabled",
            },
        },
    }


M = {  # model licences as in the other workflows
    "ck": ("novaAnimeXL_ilV170.safetensors", "Fair AI Public License 1.0-SD"),
    "face": ("bbox/face_yolov8m.pt", "AGPL-3.0"),
    "sam": ("sam_vit_b_01ec64.pth", "Apache-2.0"),
    "up": (
        "4x-AnimeSharp.pth",
        "CC BY-NC-SA 4.0 (non-commercial; already used by the Studio upscale)",
    ),
    "cn": ("xinsir-union-sdxl-promax.safetensors", "Apache-2.0"),
    "ipf": ("ip-adapter-plus-face_sdxl_vit-h.safetensors", "Apache-2.0"),
    "ip": ("ip-adapter-plus_sdxl_vit-h.safetensors", "Apache-2.0"),
    "clipv": ("CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors", "MIT"),
    "bg": ("birefnet.safetensors", "MIT (BiRefNet)"),
}
RATING = """[[param]]
name = "rating"
type = "choice"
default = "general"
choices = ["general", "sensitive", "questionable", "explicit"]
adult = ["questionable", "explicit"]
words = { rating_neg = { general = "nsfw, nude, explicit, " } }
"""
CHIBI_RATING = """# chibi proportions: never the adult ratings, nudity always left out
[[param]]
name = "rating"
type = "choice"
default = "general"
choices = ["general", "sensitive"]
words = { rating_neg = { general = "nsfw, nude, explicit, ", sensitive = "nsfw, nude, explicit, " } }
"""


def p_str(name, node, template, default=""):
    return f'[[param]]\nname = "{name}"\ntype = "string"\ndefault = "{default}"\nnode = "{node}"\ninput = "text"\ntemplate = "{template}"\n'


def p_num(name, kind, node, inp, default, lo, hi, comment=""):
    c = f"# {comment}\n" if comment else ""
    return f'{c}[[param]]\nname = "{name}"\ntype = "{kind}"\ndefault = {default}\nmin = {lo}\nmax = {hi}\nnode = "{node}"\ninput = "{inp}"\n'


def p_seed(node, also=()):
    a = ", ".join(f'{{ node = "{n}", input = "seed", add = {k} }}' for n, k in also)
    return (
        f'[[param]]\nname = "seed"\ntype = "seed"\ndefault = -1\nnode = "{node}"\ninput = "seed"\n'
        + (f"also = [{a}]\n" if a else "")
    )


def write(name, head, graph, params, models, face=None, inputs='source = "10"'):
    d = os.path.join(ROOT, name)
    os.makedirs(d, exist_ok=True)
    dump(graph, os.path.join(d, "graph.json"))
    t = head + "\n[inputs]\n" + inputs + "\n"
    if face:
        g2 = copy.deepcopy(graph)
        face(g2)
        dump(g2, os.path.join(d, "graph-face.json"))
        t += '\n# A face photo makes the new part look like that person (IP-Adapter plus-face, nodes 30-34).\n[face]\ngraph = "graph-face.json"\nimage = "30"\nweight = "34"\n'
    t += (
        "\n"
        + "\n".join(params)
        + "\n"
        + "\n".join(
            f'[[model]]\nfile = "{M[m][0]}"\nlicence = "{M[m][1]}"\n' for m in models
        )
    )
    Path(d, "workflow.toml").write_text(t)


def head(title, desc, label, hint, order, vram, outputs='["20"]', comment=""):
    return (
        f"# STU-R2: a refine step for Kreative Studio: {comment}\n"
        f'title = "{title}"\ndescription = "{desc}"\ngraph = "graph.json"\noutputs = {outputs}\nmachines = ["soucouyant"]\nvram_mb = {vram}\nram_mb = 12000\n\n'
        f'[studio]\nlabel = "{label}"\nhint = "{hint}"\nsizes = ["square"]\norder = {order}\nrefine = true\n'
    )


# 1. 4K for an artist: no diffusion (a detail pass changed faces and eye colours), only 4x-AnimeSharp:
# the long side first to half the target (1920), 4x, then lanczos to 3840 (or `size`).
g = {
    "10": {"class_type": "LoadImage", "inputs": {"image": "source.png"}},
    "11": {
        "class_type": "ImageScaleToMaxDimension",
        "inputs": {"image": L(10), "upscale_method": "lanczos", "largest_size": 1920},
    },
    **upscaler(L(11)),
    "54": {
        "class_type": "ImageScaleToMaxDimension",
        "inputs": {"image": L(51), "upscale_method": "lanczos", "largest_size": 3840},
    },
    "20": save(L(54), "refine-4k"),
}
write(
    "refine-4k",
    head(
        "Refine: 4K",
        "Makes a picture 4K (3840 px on the long side) with 4x-AnimeSharp, nothing redrawn, to hand to an artist.",
        "4K for an artist",
        "Same picture, 3840 px, crisp lines",
        101,
        3000,
        comment="4K upscale (4x-AnimeSharp only, long side 3840).",
    ),
    g,
    [
        p_num(
            "size",
            "int",
            "54",
            "largest_size",
            3840,
            2048,
            4096,
            "Pixels on the long side.",
        )
    ],
    ["up"],
)

# 2. Recolour / restyle: img2img at 2 MP held by the anime lineart (ControlNet union), face fix, back to the source size.
g = base()
g.update(
    {
        "11": {
            "class_type": "ImageScaleToTotalPixels",
            "inputs": {
                "image": L(10),
                "upscale_method": "lanczos",
                "megapixels": 2.0,
                "resolution_steps": 8,
            },
        },
        "12": {
            "class_type": "AnimeLineArtPreprocessor",
            "inputs": {"image": L(11), "resolution": 1280},
        },
        "13": {
            "class_type": "ControlNetLoader",
            "inputs": {"control_net_name": "xinsir-union-sdxl-promax.safetensors"},
        },
        "14": {
            "class_type": "SetUnionControlNetType",
            "inputs": {
                "control_net": L(13),
                "type": "canny/lineart/anime_lineart/mlsd",
            },
        },
        "15": {
            "class_type": "ControlNetApplyAdvanced",
            "inputs": {
                "positive": L(4),
                "negative": L(5),
                "control_net": L(14),
                "image": L(12),
                "strength": 0.75,
                "start_percent": 0.0,
                "end_percent": 1.0,
                "vae": L(1, 2),
            },
        },
        "16": tiled("VAEEncodeTiled", L(11)),
        "7": ks(L(1), L(15, 0), L(15, 1), L(16), 1.0, 28),
        "17": tiled("VAEDecodeTiled", L(7)),
        **face_fix(L(17), L(1)),
        **upscaler(L(42)),
        **back_to_source(L(51)),
        "20": save(L(53), "refine-change"),
    }
)
write(
    "refine-change",
    head(
        "Refine: change",
        "Recolours or restyles a picture while its lines stay: say what changes (black and gold armour, dark skin).",
        "Recolour or change",
        "Keeps the lines, changes colours and details",
        102,
        11000,
        comment="recolour/restyle held by the lineart.",
    ),
    g,
    [
        p_str("prompt", "4", POS, "black and gold armour"),
        p_str("negative", "5", NEG),
        p_seed("7", [("42", 1)]),
        p_num(
            "strength",
            "float",
            "7",
            "denoise",
            1.0,
            0.2,
            1.0,
            "How much may change: a white suit stayed white at 0.65 and 0.85 (the start picture's colours win); 1.0 redraws on the lines.",
        ),
        p_num(
            "keep",
            "float",
            "15",
            "strength",
            0.75,
            0.0,
            1.0,
            "How closely the lines are kept.",
        ),
        RATING,
    ],
    ["ck", "cn", "face", "sam", "up"],
)

# 3. Repaint a painted area (head swap, a closed helmet): cropped, inpainted at 1024 and pasted back (Impact DetailerForEach).
g = base()
g.update(
    {
        "11": {
            "class_type": "LoadImageMask",
            "inputs": {"image": "mask.png", "channel": "red"},
        },
        "12": {
            "class_type": "GrowMask",
            "inputs": {"mask": L(11), "expand": 12, "tapered_corners": True},
        },
        "13": {
            "class_type": "MaskToSEGS",
            "inputs": {
                "mask": L(12),
                "combined": True,
                "crop_factor": 1.6,
                "bbox_fill": False,
                "drop_size": 10,
                "contour_fill": True,
            },
        },
        "14": {
            "class_type": "DetailerForEach",
            "inputs": {
                "image": L(10),
                "segs": L(13),
                "model": L(1),
                "clip": L(3),
                "vae": L(1, 2),
                "guide_size": 1024,
                "guide_size_for": True,
                "max_size": 1536,
                "seed": 1,
                "steps": 28,
                "cfg": 5.5,
                "sampler_name": "euler_ancestral",
                "scheduler": "normal",
                "positive": L(4),
                "negative": L(5),
                "denoise": 0.9,
                "feather": 8,
                "noise_mask": True,
                "force_inpaint": True,
                "wildcard": "",
                "cycle": 1,
                "noise_mask_feather": 20,
                "tiled_encode": True,
                "tiled_decode": True,
            },
        },
        "20": save(L(14), "refine-repaint"),
    }
)


def add_face(g2):
    g2.update(
        {
            "30": {"class_type": "LoadImage", "inputs": {"image": "face.png"}},
            "31": {
                "class_type": "PrepImageForClipVision",
                "inputs": {
                    "image": L(30),
                    "interpolation": "LANCZOS",
                    "crop_position": "center",
                    "sharpening": 0,
                },
            },
            "32": {
                "class_type": "CLIPVisionLoader",
                "inputs": {"clip_name": "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors"},
            },
            "33": {
                "class_type": "IPAdapterModelLoader",
                "inputs": {
                    "ipadapter_file": "ip-adapter-plus-face_sdxl_vit-h.safetensors"
                },
            },
            "34": {
                "class_type": "IPAdapterAdvanced",
                "inputs": {
                    "model": L(1),
                    "ipadapter": L(33),
                    "image": L(31),
                    "clip_vision": L(32),
                    "weight": 0.85,
                    "weight_type": "linear",
                    "combine_embeds": "concat",
                    "start_at": 0,
                    "end_at": 0.85,
                    "embeds_scaling": "V only",
                },
            },
        }
    )
    g2["14"]["inputs"]["model"] = L(34)


write(
    "refine-repaint",
    head(
        "Refine: repaint",
        "Repaints the area you paint over and keeps the rest pixel for pixel: a new head, a closed helmet, another weapon.",
        "Repaint an area",
        "Paint over a part, say what goes there",
        103,
        9000,
        comment="repaint the painted area (head swap, closed helmet).",
    ),
    g,
    [
        p_str("prompt", "4", POS, "closed dragon helmet"),
        p_str("negative", "5", NEG),
        p_seed("14"),
        p_num(
            "strength",
            "float",
            "14",
            "denoise",
            0.9,
            0.3,
            1.0,
            "1 paints the area anew, lower keeps its shapes.",
        ),
        RATING,
    ],
    ["ck", "ipf", "clipv"],
    face=add_face,
    inputs='source = "10"\nmask = "11"',
)

# 4. Chibi: a new picture from the source's look (IP-Adapter plus) with chibi proportions.
g = base()
g.update(
    {
        "11": {
            "class_type": "PrepImageForClipVision",
            "inputs": {
                "image": L(10),
                "interpolation": "LANCZOS",
                "crop_position": "pad",
                "sharpening": 0,
            },
        },
        "12": {
            "class_type": "CLIPVisionLoader",
            "inputs": {"clip_name": "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors"},
        },
        "13": {
            "class_type": "IPAdapterModelLoader",
            "inputs": {"ipadapter_file": "ip-adapter-plus_sdxl_vit-h.safetensors"},
        },
        "14": {
            "class_type": "IPAdapterAdvanced",
            "inputs": {
                "model": L(1),
                "ipadapter": L(13),
                "image": L(11),
                "clip_vision": L(12),
                "weight": 0.6,
                "weight_type": "style transfer",
                "combine_embeds": "concat",
                "start_at": 0,
                "end_at": 0.8,
                "embeds_scaling": "V only",
            },
        },
        "6": {
            "class_type": "EmptyLatentImage",
            "inputs": {"width": 832, "height": 1216, "batch_size": 1},
        },
        "7": ks(L(14), L(4), L(5), L(6), 1.0, 28),
        "17": tiled("VAEDecodeTiled", L(7)),
        **upscaler(L(17)),
        "54": {
            "class_type": "ImageScale",
            "inputs": {
                "image": L(51),
                "upscale_method": "lanczos",
                "width": 1664,
                "height": 2432,
                "crop": "disabled",
            },
        },
        "20": save(L(54), "refine-chibi"),
    }
)
write(
    "refine-chibi",
    head(
        "Refine: chibi",
        "Draws the character again as a chibi (big head, small body) in the same colours and outfit.",
        "Chibi version",
        "Same outfit and colours, chibi proportions",
        104,
        9000,
        comment="chibi version (IP-Adapter plus, style transfer).",
    ),
    g,
    [
        p_str(
            "prompt",
            "4",
            "masterpiece, best quality, amazing quality, very aesthetic, {rating}, adult, chibi, super deformed, big head, small body, cute, full body, standing, simple background, white background, {value}",
        ),
        p_str(
            "negative",
            "5",
            NEG.replace(
                "child, loli, shota, underage, young child, kid, toddler, teen, minor, childlike body, ",
                "loli, shota, underage, toddler, minor, ",
            ),
        ),
        p_seed("7"),
        p_num(
            "likeness",
            "float",
            "14",
            "weight",
            0.6,
            0.0,
            1.0,
            "How much of the source's look carries over.",
        ),
        CHIBI_RATING,
    ],
    ["ck", "ip", "clipv", "up"],
)

# 5. Parts for an avatar: cut out (BiRefNet), a head layer (the face box grown by `head`) and the body without it.
g = {
    "10": {"class_type": "LoadImage", "inputs": {"image": "source.png"}},
    "11": {
        "class_type": "ImageScaleToMaxDimension",
        "inputs": {"image": L(10), "upscale_method": "lanczos", "largest_size": 2048},
    },
    "12": {
        "class_type": "LoadBackgroundRemovalModel",
        "inputs": {"bg_removal_name": "birefnet.safetensors"},
    },
    "13": {
        "class_type": "RemoveBackground",
        "inputs": {"bg_removal_model": L(12), "image": L(11)},
    },
    "14": {
        "class_type": "UltralyticsDetectorProvider",
        "inputs": {"model_name": "bbox/face_yolov8m.pt"},
    },
    "15": {
        "class_type": "BboxDetectorSEGS",
        "inputs": {
            "bbox_detector": L(14),
            "image": L(11),
            "threshold": 0.4,
            "dilation": 60,
            "crop_factor": 3.0,
            "drop_size": 10,
            "labels": "all",
        },
    },
    "16": {"class_type": "SegsToCombinedMask", "inputs": {"segs": L(15)}},
    "17": {
        "class_type": "MaskComposite",
        "inputs": {
            "destination": L(13),
            "source": L(16),
            "x": 0,
            "y": 0,
            "operation": "multiply",
        },
    },
    "18": {
        "class_type": "MaskComposite",
        "inputs": {
            "destination": L(13),
            "source": L(16),
            "x": 0,
            "y": 0,
            "operation": "subtract",
        },
    },
}
for n, m in {"20": "13", "21": "17", "22": "18"}.items():
    g[f"3{n[1]}"] = {"class_type": "InvertMask", "inputs": {"mask": L(m)}}
    g[f"4{n[1]}"] = {
        "class_type": "JoinImageWithAlpha",
        "inputs": {"image": L(11), "alpha": L(f"3{n[1]}")},
    }
    g[n] = save(L(f"4{n[1]}"), "refine-parts")
write(
    "refine-parts",
    head(
        "Refine: parts",
        "Splits a character into layers for an avatar: the whole cut-out, the head and the body without the head, as transparent PNGs.",
        "Split into parts",
        "Cut-out, head and body as transparent layers",
        105,
        4000,
        outputs='["20", "21", "22"]',
        comment="avatar layers (BiRefNet cut-out, head, body).",
    ),
    g,
    [
        p_num(
            "head",
            "int",
            "15",
            "dilation",
            60,
            0,
            400,
            "Pixels the face box grows by to take in hair and helmet (the picture is 2048 px on the long side).",
        )
    ],
    ["face", "bg"],
)

HELMET = """# STU-C3: helmet on and off. Kompanion's make turns helmet_desc into two runs with one seed (helmet "on" and
# "off", the same pair id); the words keep the face fix on the helmet when it is on.
[[param]]
name = "helmet"
type = "choice"
default = "none"
node = ""
choices = ["none", "on", "off"]
words = { helmet_tags = { on = "helmet, full helmet covering the head, face hidden, ", off = "no helmet, face visible, bare head, short hair, " }, helmet_neg = { on = "visible face, bare head, ", off = "helmet, headwear, visor, mask, hood, " }, helmet_face = { none = "detailed face, ", on = "helmet, visor, face hidden, ", off = "detailed face, " } }

[[param]]
name = "helmet_desc"
type = "string"
default = ""
node = ""
input = ""

[[param]]
name = "pair"
type = "string"
default = ""
node = ""
input = ""
"""

# 6. STU-C2: a headshot or a cowboy shot of a finished OC sheet's character. The sheet (scaled to 3024x1512) gives two IP-Adapter references: the front cell (outfit, colours, accessories) and the front head (face, hair, moustache); same prompt and seed as the sheet, only the shot tags change.
M["style"] = (
    "ILLUST_STYLE_MarvelRivels_ownwaifu.safetensors",
    "unknown (Civitai, check the model page)",
)

g = base()  # gives nodes 1, 3, 4, 5, 10
g["3"]["inputs"]["clip"] = L(45, 1)
g.update(
    {
        "45": {
            "class_type": "LoraLoader",
            "inputs": {
                "model": L(1),
                "clip": L(1, 1),
                "lora_name": "ILLUST_STYLE_MarvelRivels_ownwaifu.safetensors",
                "strength_model": 0.8,
                "strength_clip": 0.8,
            },
        },
        "16": {
            "class_type": "CLIPTextEncode",
            "inputs": {"clip": L(3), "text": "detailed face"},
        },
        "18": {
            "class_type": "ConditioningConcat",
            "inputs": {"conditioning_to": L(4), "conditioning_from": L(16)},
        },
        "11": {
            "class_type": "ImageScale",
            "inputs": {
                "image": L(10),
                "upscale_method": "lanczos",
                "width": 3024,
                "height": 1512,
                "crop": "disabled",
            },
        },
        "12": {
            "class_type": "ImageCrop",
            "inputs": {"image": L(11), "width": 756, "height": 1512, "x": 0, "y": 0},
        },
        "13": {
            "class_type": "ImageCrop",
            "inputs": {"image": L(11), "width": 320, "height": 320, "x": 208, "y": 80},
        },
        "30": {
            "class_type": "CLIPVisionLoader",
            "inputs": {"clip_name": "CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors"},
        },
        "31": {
            "class_type": "PrepImageForClipVision",
            "inputs": {
                "image": L(12),
                "interpolation": "LANCZOS",
                "crop_position": "pad",
                "sharpening": 0,
            },
        },
        "32": {
            "class_type": "PrepImageForClipVision",
            "inputs": {
                "image": L(13),
                "interpolation": "LANCZOS",
                "crop_position": "center",
                "sharpening": 0,
            },
        },
        "33": {
            "class_type": "IPAdapterModelLoader",
            "inputs": {"ipadapter_file": "ip-adapter-plus_sdxl_vit-h.safetensors"},
        },
        "34": {
            "class_type": "IPAdapterAdvanced",
            "inputs": {
                "model": L(45),
                "ipadapter": L(33),
                "image": L(37),
                "clip_vision": L(30),
                "weight": 0.7,
                "weight_type": "linear",
                "combine_embeds": "concat",
                "start_at": 0,
                "end_at": 0.8,
                "embeds_scaling": "V only",
            },
        },
        "37": {
            "class_type": "ImageBatch",
            "inputs": {"image1": L(31), "image2": L(32)},
        },
        "6": {
            "class_type": "EmptyLatentImage",
            "inputs": {"width": 1024, "height": 1280, "batch_size": 1},
        },
        "7": ks(L(34), L(18), L(5), L(6), 1.0, steps=28),
        "17": tiled("VAEDecodeTiled", L(7)),
        **face_fix(L(17), L(34)),
        **upscaler(L(42)),
        "54": {
            "class_type": "ImageScaleBy",
            "inputs": {"image": L(51), "upscale_method": "lanczos", "scale_by": 0.5},
        },
        "20": save(L(54), "oc-shots"),
    }
)
g["42"]["inputs"]["positive"] = L(16)

hd = (
    "# STU-C2: a headshot or a cowboy shot of a finished OC sheet's character (the sheet is the source picture).\n"
    'title = "OC shots"\n'
    'description = "A headshot (neck up) or a cowboy shot (thighs up) of the character on a finished OC sheet, with the same face, hair, outfit and colours."\n'
    'graph = "graph.json"\noutputs = ["20"]\nmachines = ["soucouyant"]\nvram_mb = 10000\nram_mb = 12000\n\n'
    '[studio]\nlabel = "OC shots"\nhint = "Headshot or cowboy shot of a sheet\'s character"\nsizes = ["tall", "square"]\norder = 106\nrefine = true\n'
    "# headshot 1024x1280, cowboy shot 1216x1216 (the 4x model and a 0.5 scale give twice that)\n"
    "px = { tall = [1024, 1280], square = [1216, 1216] }\n"
)

params = [
    p_str(
        "prompt",
        "4",
        "masterpiece, best quality, amazing quality, very aesthetic, absurdres, {rating}, adult, mature, solo, {shot_tags}, {bg}, {helmet_tags}{value}",
    ),
    p_str(
        "negative",
        "5",
        "worst quality, bad quality, low quality, lowres, blurry, jpeg artifacts, watermark, signature, text, logo, bad anatomy, bad hands, extra fingers, missing fingers, fused fingers, extra limbs, deformed, disfigured, mutated, multiple views, character sheet, reference sheet, turnaround, child, loli, shota, underage, young child, kid, toddler, teen, minor, childlike body, {helmet_neg}{shot_neg}{bg_neg}{rating_neg}{value}",
    ),
    p_str(
        "face_prompt",
        "16",
        "masterpiece, best quality, amazing quality, very aesthetic, {rating}, adult, mature, {helmet_face}{value}",
    )
    + 'fallback = "prompt"\n',
    """# STU-C2: which shot; the size comes with it (headshot tall, cowboy square, sent by Kreative Studio).
[[param]]
name = "shot"
type = "choice"
default = "headshot"
choices = ["headshot", "cowboy"]
words = { shot_tags = { headshot = "portrait, close-up, head and shoulders, upper body, looking at viewer", cowboy = "cowboy shot, from the thighs up, standing, looking at viewer" }, shot_neg = { headshot = "full body, feet, legs, ", cowboy = "full body, feet, close-up, " } }
""",
    """[[param]]
name = "background"
type = "choice"
default = "white"
choices = ["white", "starry"]
words = { bg = { white = "simple background, white background", starry = "purple starry night sky background, stars, deep purple to pink gradient sky" }, bg_neg = { white = "gradient background, starry sky, ", starry = "white background, simple background, " } }
""",
    p_num("width", "int", "6", "width", 1024, 1024, 1216),
    p_num("height", "int", "6", "height", 1280, 1216, 1280),
    p_seed("7", [("42", 2)]),
    p_num(
        "likeness",
        "float",
        "34",
        "weight",
        0.7,
        0.0,
        1.0,
        "How much of the sheet's look (outfit, colours, face) carries over.",
    ),
    HELMET,
    RATING,
]

write(
    "oc-shots",
    hd,
    g,
    params,
    ["ck", "style", "ip", "clipv", "face", "sam", "up"],
)
