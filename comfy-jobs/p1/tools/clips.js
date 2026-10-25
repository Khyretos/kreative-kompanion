// STU-P1: four clips for rigged 3D-01 characters, in the PE2 clip format (walk from clips.js; idle, run, attack new)
const C = require('/app/public/js/pose/clips.js');
const P = (root, joints) => ({ version: 1, root: { pos: [0, 0, root] }, joints });
const idleA = P(0.95, { spine: { bend: 2 }, chest: { bend: 1 }, l_shoulder: { side: -8 }, r_shoulder: { side: -8 }, l_elbow: { bend: 8 }, r_elbow: { bend: 8 } });
const idleB = P(0.945, { spine: { bend: 4 }, chest: { bend: 3 }, neck: { bend: -2 }, l_shoulder: { side: -10 }, r_shoulder: { side: -10 }, l_elbow: { bend: 12 }, r_elbow: { bend: 12 } });
const runContact = P(0.86, { spine: { bend: 12, twist: 6 }, chest: { twist: -12 }, l_hip: { bend: 45 }, l_knee: { bend: -15 }, l_ankle: { bend: -10 },
  r_hip: { bend: -30 }, r_knee: { bend: -40 }, r_ankle: { bend: 20 }, l_shoulder: { bend: -40 }, r_shoulder: { bend: 40 }, l_elbow: { bend: 80 }, r_elbow: { bend: 80 } });
const runFlight = P(0.98, { spine: { bend: 12 }, l_hip: { bend: 10 }, l_knee: { bend: -60 }, r_hip: { bend: 30 }, r_knee: { bend: -90 }, r_ankle: { bend: 15 },
  l_shoulder: { bend: 0 }, r_shoulder: { bend: 0 }, l_elbow: { bend: 80 }, r_elbow: { bend: 80 } });
const atkReady = P(0.92, { spine: { twist: -15 }, chest: { twist: -10 }, l_hip: { bend: 15 }, r_hip: { bend: -10 }, l_knee: { bend: -15 }, r_knee: { bend: -10 },
  r_shoulder: { bend: -60, side: 30 }, r_elbow: { bend: 90 }, l_shoulder: { bend: 30 }, l_elbow: { bend: 60 } });
const atkSwing = P(0.9, { spine: { twist: 20, bend: 10 }, chest: { twist: 15 }, l_hip: { bend: 25 }, r_hip: { bend: -15 }, l_knee: { bend: -20 }, r_knee: { bend: -10 },
  r_shoulder: { bend: 70, side: 10 }, r_elbow: { bend: 10 }, l_shoulder: { bend: -10 }, l_elbow: { bend: 70 } });
const clips = {
  idle: { version: 1, fps: 24, length: 2, loop: true, keys: [{ t: 0, pose: idleA, ease: 'ease-in-out' }, { t: 1, pose: idleB, ease: 'ease-in-out' }] },
  walk: C.CLIPS.walk,
  run: { version: 1, fps: 24, length: 0.6, loop: true, keys: [{ t: 0, pose: runContact, ease: 'linear' }, { t: 0.15, pose: runFlight, ease: 'linear' },
    { t: 0.3, pose: C.mirrored(runContact), ease: 'linear' }, { t: 0.45, pose: C.mirrored(runFlight), ease: 'linear' }] },
  attack: { version: 1, fps: 24, length: 1, loop: false, keys: [{ t: 0, pose: idleA, ease: 'ease-in' }, { t: 0.35, pose: atkReady, ease: 'ease-in' },
    { t: 0.55, pose: atkSwing, ease: 'ease-out' }, { t: 1, pose: idleA, ease: 'ease-in-out' }] },
};
const A = require('/app/lib/anim-api.js');
require('fs').writeFileSync('/out/clips.json', JSON.stringify(clips));
console.log(Object.keys(A));
for (const [n, c] of Object.entries(clips)) {
  A.checkClip(c);
  const out = A.render(c, 'glb', {});
  require('fs').writeFileSync(`/out/clip-${n}${out.ext}`, out.body);
  console.log(n, out.ext, out.body.length);
}
