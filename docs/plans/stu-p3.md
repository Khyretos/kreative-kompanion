# STU-P3: promotional banners a designer can edit (queue 24, 2026-10-19), part 1: the composer

`tools/promo/compose.py` turns a spec (size preset, background picture or colour, optional transparent subject,
logo, headline, tagline, side) into `banner.ora` (OpenRaster: background, scrim, subject, logo, headline and
tagline as separate layers; Krita, GIMP), `banner.svg` (picture layers as images, headline and tagline as live
`<text>` in Inter; Inkscape, Penpot), `banner.png` and `check.json`. Sizes: repo-banner 1280x640, social 1200x630,
steam-header 460x215, steam-main 616x353, discord 960x540.

Readability (CLAUDE.md core rule): text is white or night, whichever has the better worst-case contrast against the
light and dark ends (5th and 95th luminance percentile) of what lies under it; below 4.5:1 a night scrim at 60%
goes behind the text and the check runs again. Text shrinks to fit its half of the banner.
`python3 tools/promo/compose.py spec.json --out DIR [--font Inter.ttf]` exits 2 when the check fails.

Tests: `python3 tools/test_promo_compose.py` (10). Coder: one unit, 3 attempts, none of the asked functions
defined; Claude wrote the file (lessons 420-421).

## Still to do (needs GPU runs)

Workflow and Studio preset: background and subject generated separately (subject made transparent with BiRefNet,
MIT, already on soucouyant), then this composer; landscape, 360 skybox and tileable ground presets.
