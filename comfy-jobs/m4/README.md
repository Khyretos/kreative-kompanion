# STU-M4 test: motion to images (2026-10-25)

A MoMask clip ("a person jumps forward over an obstacle", 88 frames) through the Studio's `toOpenPose`
(lib/motion-api.js), 8 evenly spaced frames drawn as OpenPose PNGs, then SDXL + union ControlNet (openpose 1.0) with
one goblin prompt and one seed per frame (`jump.json`, 69 s for 8 frames, 10.2 GB card). The figure follows the pose
in every frame (`out/jump-strip.png`). Faults: some frames come out from behind (OpenPose has no depth: the first
try, with the face drawn at HumanML3D's low head joint, gave backs in all 8; moving the face up fixed 2 of 8), and the
outfit and hair drift between frames. Next: the OC sheet's IP-Adapter for the character, and the pose editor's depth
render of the same frames for front/back.
