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
    """VaeDecodeTextureTrellis with the texture VAE loaded on the CPU in fp32. On the RX 9070 XT (gfx1201) the GPU
    decode returns colours of about +-200 with one half of the model twice as noisy (the 3D-01 colour split); the
    same latent decoded on the CPU gives 0..1 on both halves (2026-10-24, about 33 s for 1.5 M voxels)."""
    @classmethod
    def INPUT_TYPES(cls):
        import folder_paths
        return {"required": {"samples": ("LATENT",), "shape_subdivides": ("SHAPE_SUBDIVIDES",),
                             "vae_name": (folder_paths.get_filename_list("vae"),)}}
    RETURN_TYPES = ("VOXEL",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, samples, shape_subdivides, vae_name):
        import comfy.model_management as mm
        import comfy.sd
        import comfy.utils
        import folder_paths
        from comfy_extras.nodes_trellis2 import VaeDecodeTextureTrellis
        cpu = torch.device("cpu")
        vae = comfy.sd.VAE(sd=comfy.utils.load_torch_file(folder_paths.get_full_path_or_raise("vae", vae_name)), device=cpu, dtype=torch.float32)
        subs = [type(s)(feats=s.feats.to(cpu, torch.float32), coords=s.coords.to(cpu), shape=s.shape, scale=s._scale) for s in shape_subdivides]
        old = mm.get_torch_device
        mm.get_torch_device = lambda: cpu
        try:
            out = VaeDecodeTextureTrellis.execute(samples, vae, subs)
        finally:
            mm.get_torch_device = old
        voxel = out[0] if isinstance(out, tuple) else out.result[0]
        return (voxel,)


NODE_CLASS_MAPPINGS = {"KKDumpLatent": KKDumpLatent, "KKDumpVoxel": KKDumpVoxel, "KKDumpSubdivides": KKDumpSubdivides,
                       "KKLoadLatent": KKLoadLatent, "KKLoadSubdivides": KKLoadSubdivides,
                       "KKVaeDecodeTextureCPU": KKVaeDecodeTextureCPU}
