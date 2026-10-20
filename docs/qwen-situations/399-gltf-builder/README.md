# Lesson 399

Lesson 399 (STU-PE2 pe2-gltf, Coder): 386-line draft with two binary builders (one unused), typed-array element counts used as byte lengths, accessor type FLOAT/UNSIGNED_SHORT instead of VEC3/VEC4/MAT4, invented S.byName[p].indexInNames and quat.x (quat.js returns arrays), json.skin instead of skins, root translation converted twice, loop end time = second-to-last frame, rotation accessor indices counted from 0. Two patch rounds fixed nothing. Claude rewrote toGLTF (about 100 lines); the module wrapper, load() and helpers were kept. Job too big for one draft: split builder, nodes/skin and animation.

Rule: A file-format builder of 100+ lines is three units: the binary builder, the node/skin part and the animation part, each with its own test.

Files: job.json (the pipeline job; its check needs the Kreative Studio tree at /tmp/kk-pe2/studio and the kreative-studio:local image), `wrong-*` (the Coder's output), `after-repair-*` (after the repair rounds), `corrected-*` (what was kept; a corrected test where the mistake was Claude's). Code files carry a .txt suffix so the lint gate skips them.
