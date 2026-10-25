# STU-P1: rigged 3D-01 characters with four clips (2026-10-25)

`tools/rig.py` (Blender 4.3, CPU, blender-mcp image, 6 GB cap): imports the four clips exported by the pose editor
(`tools/clips.js`: idle, walk from PE2, run, attack, written with lib/anim-api.js render 'glb'), keeps one armature
(the pose editor's 17-bone body skeleton) with each clip as an NLA track, scales the 3D-01 character to the rig, and
weights it by bone heat on a voxel-remeshed copy (TRELLIS meshes are not manifold: heat on the mesh failed for 3 of 5;
the copy needed 1/100 of the height as voxel size for goblin and dragon), copied to the real mesh by nearest face.
Exports one .glb with the skin and the four animations (about 8 MB, on soucouyant ~/kk-render/out/p1/).

All five characters: 96-100 % of vertices weighted, 4 animations each. Known faults in `out/rig-all.png`: in the
attack and run side views the feet smear along the ground on some characters, and held weapons bend with the arm
or leg nearest to them (no separate weapon bone); the run clip lifts the whole body.
