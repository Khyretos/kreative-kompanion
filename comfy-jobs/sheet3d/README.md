# STU-SHEET3D on soucouyant (2026-10-25)

OC sheet v3 (front, right, left, back body cells + a 2 x 4 head grid with a helmet column, black cell lines) built
in four stages, each its own kk-render container so MV-Adapter and SDXL never share a ComfyUI:

1. `mv-*.json` (profile `mv`): MV-Adapter SD2.1 i2mv, 4 views at 512 from one reference picture. 13-15 s, 8.2 GB card.
2. `tools/prep.py` (kireserver, CPU): crop each view to its figure and pad to 1:2.
3. `refine-*.json` (profile `sdxl`): each view 4x-AnimeSharp up, then SDXL img2img at 640 x 1280, denoise 0.3, with
   the character and view in the prompt. 74-84 s per character, 10.0 GB card.
4. `tools/compose.py` + `tools/heads.py`: body cells from the refined views, head cells cropped from the same views;
   `helmet-*.json` (profile `sdxl`) makes the helmet column from the head crops (img2img at 1024, denoise 0.6;
   the knight needed 0.75 and the front 0.85 because her silver hair read as a helmet already); `tools/helm.py`
   pastes them in. 41-43 s per character.

The prompt must describe the reference: a "horned helmet" prompt on the knight reference gave doubled figures in the
side views. Results: `out/sheet-knight.png`, `out/sheet-goblin.png` (3200 x 1280).
