# STU-SHEET3D: OC sheet v3 from a picture, with MV-Adapter as an on-demand app (queue 24, 2026-10-25)

Before: the v3 sheet (oc-sheet-v3) drew all views in one SDXL pass from pose templates: the side views faced the
same way, the head grid repeated one head and the helmet column lost the helmet (Kees's reviews of 2026-10-19).

Now: `oc-sheet-mv`, a refine step from one character picture.
- `[pre]` (new in workflow.toml, `server/src/studio/pre.rs`): MV-Adapter SD2.1 (Apache-2.0) makes front, right, back
  and left views on its own ComfyUI app ("mv", port 8192, container `kompanion-mv` on soucouyant, created stopped by
  `~/.config/comfyui-rocm/mv-app.compose.yml`, runner `[gpu_apps] mv`). Kompanion stops the Studio ComfyUI through
  the runner, starts the app, runs the stage, stops it and starts ComfyUI again (also when the stage fails): two
  ComfyUIs on the card killed the desktop on 2026-10-19. Measured: 16 s, 8.2 GB card.
- Main graph (`graph.json`, made by comfy-jobs/sheet3d/mkgraph2.py on khyretos/pe3-renders): each view is cropped to
  its figure (`studio/comfy-nodes/kk_sheet.py`, KKFigureCrop, mounted into soucouyant's ComfyUI), refined with SDXL
  img2img into a 640x1280 body cell; the head is refined bare and with the helmet prompt; one 3200x1280 sheet with
  black cell lines (`lines.png`). Measured: about 111 s, 9.1 GB card.
- Params: prompt (node 4), helmet_look (node 6, empty = bare head in both columns), negative, rating.

Tests: `cargo test studio::` (registry test for oc-sheet-mv, pre.rs unit tests). The run from the Studio UI waits for
Kees's look at the result.
