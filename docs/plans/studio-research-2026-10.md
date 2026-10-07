# Studio research (STU-R1, 2026-10-07)

What can Kompanion make locally with ComfyUI on our 16 GB GPUs (RX 9070 XT on soucouyant, A770 on kireserver)? Verdicts: GO, LIKELY GO, PLAN or NOT YET. Short version: landscapes and animation work today, posing is one model download away, promo art is a plan, and 3D and rigging are not yet possible on our AMD GPU.

## Summary

| Feature | Workflow and models | Licence | Tested on | Time | Verdict |
|---|---|---|---|---|---|
| Landscapes | Z-Image Turbo Q8 preset | Apache-2.0 | RX 9070 | 49 s | GO |
| Animation | Wan2.2 TI2V 5B Q8 | Apache-2.0 | RX 9070 | 577 s for 2 s of video | GO (slow) |
| 3D from an image | TRELLIS.2 (ComfyUI built-in), int8 weights + DINOv3 encoder | MIT; DINOv3 licence (warning) | RX 9070 | 56 s (shape, broken) | NOT YET |
| Auto-rigging | UniRig (ComfyUI-UniRig nodes) or Blender Rigify | MIT | not tested | - | NOT YET |
| Posing | DWPose preprocessor + xinsir OpenPose SDXL ControlNet | Apache-2.0 | not tested | - | LIKELY GO |
| Promo art | LayerDiffuse layers + .ora/SVG text in Kompanion | CreativeML OpenRAIL-M (warning) | not tested | - | PLAN |

## Details

### Landscapes and environments
The Landscape preset using Z-Image Turbo Q8 generated a wide landscape in 49 seconds on the RX 9070 XT. This workflow is ready to use. Panoramas and tileable textures are not yet possible because they require specific LoRAs and nodes that are missing.

### Animation
Wan2.2 TI2V 5B Q8 created two seconds of video in 577 seconds on the RX 9070 XT. The process works but is slow. Kompanion currently lacks a start image input for its video type, though ComfyUI supports it directly.

### 3D model from one image
TRELLIS.2 (Microsoft, MIT) is built into ComfyUI 0.38; its image encoder DINOv3 has Meta's own licence (commercial use allowed, not OSI: a licence warning). Test on the RX 9070 XT with the int8 weights (5.3 GB) and a Character image: the shape stage finished in 56 s and saved a 79 MB GLB with 2.6 M vertices and 4.2 M triangles, but every vertex position was NaN, and the texture stage stopped with "x must be finite". The bf16 weights (10.3 GB) did not finish loading within the 15 minutes the studio stays on, so that test was stopped (and the file removed). Next: try the A770 (XPU), ROCm flags that upcast attention, or Pixal3D. Hunyuan3D 2.x cannot be used: its licence does not apply in the European Union, the UK or South Korea.

### Auto-rigging
UniRig has a ComfyUI node pack but was not installed or tested. It requires a working mesh first. The practical path is to use Blender Rigify via the Blender MCP task BLD-01.

### Posing a character with a controller
DWPose and OpenPose preprocessors are installed (comfyui_controlnet_aux). Missing is an SDXL OpenPose ControlNet: xinsir/controlnet-openpose-sdxl-1.0 (Apache-2.0, about 2.5 GB) fits the novaAnimeXL OC presets. Kompanion would send a stick-figure image to control the pose once this node is added.

### Editable promotional art
LayerDiffuse creates subjects with transparent backgrounds under CreativeML OpenRAIL-M. Kompanion will stack these with generated backgrounds and text into layered .ora files plus SVG text.

## What was installed
Files were added to `~/comfy-models-local` on soucouyant: `trellis_2_int8_convrot.safetensors`, VAEs, `dino_v3_vit_l`, `birefnet`. These are mounted in `~/.config/comfyui-rocm/compose.override.yml` (backup `.bak-20261007`). Empty files with the same names exist in kireserver's `Services/ai/comfyui/models` as mount points over NFS; the A770 ComfyUI lists them but cannot load them. The test GLB: `/media/Generated/images/kompanion/stu-r1/trellis2-shape_00001_.glb`.

## Next tasks
Map findings to M6-S03 / STU-P1 for 3D and rigging, STU-P2 for posing, STU-P3 for promo art, and BLD-01 for Blender integration.
