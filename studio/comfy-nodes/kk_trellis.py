# Kompanion ComfyUI nodes for TRELLIS.2 on the RX 9070 XT (gfx1201), 3D-01 (2026-10-25).
# Install: put this file in ComfyUI's custom_nodes (Kreative Studio's comfyui-rocm mounts it read-only).
# - KKVAELoaderFP32: a VAE in fp32 on the GPU or the CPU, without starting all of ComfyUI with --fp32-vae (the fp16
#   and bf16 shape decodes give NaN vertices on this card, and a global flag would double every SDXL decode).
# - KKVaeDecodeTextureCPU: the texture decode on the CPU. On the GPU it returns colours of about +-200 with one half
#   of the model twice as noisy (the "half noise" colour split); the same latent on the CPU gives 0..1 on both
#   halves, about 33 s for 1.5 M voxels.
import torch


def _vae(vae_name, device):
    import comfy.sd
    import comfy.utils
    import folder_paths

    sd = comfy.utils.load_torch_file(folder_paths.get_full_path_or_raise("vae", vae_name))
    return comfy.sd.VAE(sd=sd, device=device, dtype=torch.float32)


class KKVAELoaderFP32:
    @classmethod
    def INPUT_TYPES(cls):
        import folder_paths

        return {"required": {"vae_name": (folder_paths.get_filename_list("vae"),), "device": (["gpu", "cpu"],)}}

    RETURN_TYPES = ("VAE",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, vae_name, device):
        import comfy.model_management as mm

        return (_vae(vae_name, mm.get_torch_device() if device == "gpu" else torch.device("cpu")),)


class KKVaeDecodeTextureCPU:
    @classmethod
    def INPUT_TYPES(cls):
        import folder_paths

        return {
            "required": {
                "samples": ("LATENT",),
                "shape_subdivides": ("SHAPE_SUBDIVIDES",),
                "vae_name": (folder_paths.get_filename_list("vae"),),
            }
        }

    RETURN_TYPES = ("VOXEL",)
    FUNCTION = "run"
    CATEGORY = "kk"

    def run(self, samples, shape_subdivides, vae_name):
        import comfy.model_management as mm
        from comfy_extras.nodes_trellis2 import VaeDecodeTextureTrellis

        cpu = torch.device("cpu")
        vae = _vae(vae_name, cpu)
        subs = [
            type(s)(feats=s.feats.to(cpu, torch.float32), coords=s.coords.to(cpu), shape=s.shape, scale=s._scale)
            for s in shape_subdivides
        ]
        old = mm.get_torch_device
        mm.get_torch_device = lambda: cpu  # the node decodes on get_torch_device(), not on the VAE's device
        try:
            out = VaeDecodeTextureTrellis.execute(samples, vae, subs)
        finally:
            mm.get_torch_device = old
        del vae
        return (out[0] if isinstance(out, tuple) else out.result[0],)


NODE_CLASS_MAPPINGS = {"KKVAELoaderFP32": KKVAELoaderFP32, "KKVaeDecodeTextureCPU": KKVaeDecodeTextureCPU}
NODE_DISPLAY_NAME_MAPPINGS = {
    "KKVAELoaderFP32": "VAE Loader (fp32)",
    "KKVaeDecodeTextureCPU": "Trellis2 VAE Decode Texture (CPU)",
}
