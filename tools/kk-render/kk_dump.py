# kk-render debug nodes (3D-01 colour bug): save a TRELLIS.2 latent or voxel-colour output to /media-out/dump/<name>.pt
# so it can be checked on the CPU. Output-only; they change nothing in the graph.
import os
import torch


def _save(obj, name):
    d = "/media-out/dump"
    os.makedirs(d, exist_ok=True)
    torch.save(obj, os.path.join(d, name + ".pt"))


class KKDumpLatent:
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"samples": ("LATENT",), "name": ("STRING", {"default": "latent"})}}
    RETURN_TYPES = ()
    FUNCTION = "run"
    OUTPUT_NODE = True
    CATEGORY = "kk"

    def run(self, samples, name):
        _save({k: (v.detach().cpu() if torch.is_tensor(v) else v) for k, v in samples.items()}, name)
        return {}


class KKDumpVoxel:
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"voxel": ("VOXEL",), "name": ("STRING", {"default": "voxel"})}}
    RETURN_TYPES = ()
    FUNCTION = "run"
    OUTPUT_NODE = True
    CATEGORY = "kk"

    def run(self, voxel, name):
        out = {}
        for k in dir(voxel):
            if k.startswith("_"):
                continue
            try:
                v = getattr(voxel, k)
            except Exception:
                continue
            if torch.is_tensor(v):
                out[k] = v.detach().cpu()
            elif isinstance(v, (int, float)):
                out[k] = v
        _save(out, name)
        return {}


class KKDumpSubdivides:
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"shape_subdivides": ("SHAPE_SUBDIVIDES",), "name": ("STRING", {"default": "subs"})}}
    RETURN_TYPES = ()
    FUNCTION = "run"
    OUTPUT_NODE = True
    CATEGORY = "kk"

    def run(self, shape_subdivides, name):
        _save([{"feats": s.feats.detach().cpu(), "coords": s.coords.detach().cpu(), "shape": tuple(s.shape),
                "scale": s._scale} for s in shape_subdivides], name)
        return {}


NODE_CLASS_MAPPINGS = {"KKDumpLatent": KKDumpLatent, "KKDumpVoxel": KKDumpVoxel, "KKDumpSubdivides": KKDumpSubdivides}
