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


def _load(name):
    return torch.load(os.path.join("/media-out/dump", name + ".pt"), map_location="cpu", weights_only=False)


class KKLoadLatent:
    """Replays a dumped latent (to rerun one stage, e.g. the texture decode on the CPU)."""
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"name": ("STRING", {"default": "latent"})}}
    RETURN_TYPES = ("LATENT",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, name):
        return (_load(name),)


class KKLoadSubdivides:
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"name": ("STRING", {"default": "subs"})}}
    RETURN_TYPES = ("SHAPE_SUBDIVIDES",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, name):
        from comfy.ldm.trellis2.vae import SparseTensor
        return ([SparseTensor(feats=d["feats"], coords=d["coords"], shape=d["shape"], scale=d["scale"]) for d in _load(name)],)


class KKVaeDecodeTextureCPU:
    """VaeDecodeTextureTrellis with the texture VAE on the CPU in fp32. On the RX 9070 XT (gfx1201) the GPU decode
    returns colours of about +-200 with one half of the model twice as noisy (the 3D-01 colour split); the same
    latent decoded on the CPU gives 0..1 on both halves (2026-10-24, about 33 s for 1.5 M voxels)."""
    @classmethod
    def INPUT_TYPES(cls):
        return {"required": {"samples": ("LATENT",), "vae": ("VAE",), "shape_subdivides": ("SHAPE_SUBDIVIDES",)}}
    RETURN_TYPES = ("VOXEL",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, samples, vae, shape_subdivides):
        import comfy.model_management as mm
        from comfy_extras.nodes_trellis2 import VaeDecodeTextureTrellis
        cpu = torch.device("cpu")
        model = vae.first_stage_model
        old = (mm.get_torch_device, vae.device, vae.vae_dtype, next(model.parameters()).device, next(model.parameters()).dtype)
        mm.get_torch_device = lambda: cpu
        vae.device, vae.vae_dtype = cpu, torch.float32
        model.to(cpu, torch.float32)
        try:
            out = VaeDecodeTextureTrellis.execute(samples, vae, shape_subdivides)
        finally:
            mm.get_torch_device, vae.device, vae.vae_dtype = old[0], old[1], old[2]
            model.to(old[3], old[4])
        voxel = out[0] if isinstance(out, tuple) else out.result[0]
        return (voxel,)


NODE_CLASS_MAPPINGS = {"KKDumpLatent": KKDumpLatent, "KKDumpVoxel": KKDumpVoxel, "KKDumpSubdivides": KKDumpSubdivides,
                       "KKLoadLatent": KKLoadLatent, "KKLoadSubdivides": KKLoadSubdivides,
                       "KKVaeDecodeTextureCPU": KKVaeDecodeTextureCPU}
