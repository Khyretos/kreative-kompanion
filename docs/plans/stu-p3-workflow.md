# STU-P3 part 2: the Banner workflow (queue 24, 2026-10-25)

Before: banners came from the command-line composer only (`tools/promo/compose.py`, part 1) with pictures made by hand.

Now: a Studio type "Banner". One run paints a background scene and a subject with SDXL, cuts the subject out with
BiRefNet, and the `KKPromoBanner` node (`studio/comfy-nodes/kk_promo.py`, which loads `tools/promo/compose.py`
mounted beside it as `kk_promo_compose.py`) lays out background, subject, the Kreative Kompas logo, headline and
tagline with the WCAG AA text check, and saves banner.png (shown in the Studio), banner.ora (layers for Krita and
GIMP) and banner.svg (live text). Params: prompt (scene), subject, headline, tagline, format (repo-banner, social,
steam-header, steam-main, discord), side, seed.

Checked: node-only run on soucouyant (2 s, social format, .ora with 7 layers incl. scrim); the pictures of the
kk-engine and Kompanion banners (comfy-jobs/p3 on khyretos/pe3-renders) came from the same graph, split in two jobs
under kk-render's 0.50 cap (BiRefNet next to SDXL needs about 9.3 GB, which the production ComfyUI has).

Deploy: mount `studio/comfy-nodes/kk_promo.py` and `tools/promo/compose.py` (as `kk_promo_compose.py`) into
soucouyant's ComfyUI custom_nodes (~/.config/comfyui-rocm/compose.override.yml), restart only that container.
