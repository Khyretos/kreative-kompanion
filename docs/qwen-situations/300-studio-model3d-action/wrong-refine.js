// STU-R2: the Refine form -> a Kompanion refine run (type, prompt, params, rating); refuses what a step cannot take.
// STU-C2: a headshot or a cowboy shot of an OC sheet's character; the sheet gives what the form leaves out.
const ACTIONS = {
  '4k': {
    type: 'refine-4k',
    prompt: 'same picture',
    params: { size: [2048, 4096, 'int'] },
    mask: false,
    face: false,
    rating: false
  },
  change: {
    type: 'refine-change',
    prompt: null,
    params: { strength: [0.2, 1, 'float'], keep: [0, 1, 'float'] },
    mask: false,
    face: false,
    rating: true
  },
  repaint: {
    type: 'refine-repaint',
    prompt: null,
    params: { strength: [0.3, 1, 'float'] },
    mask: true,
    face: true,
    rating: true
  },
  chibi: {
    type: 'refine-chibi',
    prompt: 'chibi version',
    params: { likeness: [0, 1, 'float'] },
    mask: false,
    face: false,
    rating: true
  },
  parts: {
    type: 'refine-parts',
    prompt: 'parts',
    params: { head: [0, 400, 'int'] },
    mask: false,
    face: false,
    rating: false
  },
  headshot: {
    type: 'oc-shots',
    prompt: null,
    params: { likeness: [0, 1, 'float'] },
    mask: false,
    face: false,
    rating: true,
    shot: 'headshot',
    size: 'tall'
  },
  cowboy: {
    type: 'oc-shots',
    prompt: null,
    params: { likeness: [0, 1, 'float'] },
    mask: false,
    face: false,
    rating: true,
    shot: 'cowboy',
    size: 'square'
  },
  model3d: {
    type: 'refine-3d',
    prompt: '3d model',
    params: {},
    mask: false,
    face: false,
    rating: false,
    size: 'square'
  }
};

function refineRequest(body, { hasMask, hasFace, sheet }) {
  const a = Object.prototype.hasOwnProperty.call(ACTIONS, body.action) ? ACTIONS[body.action] : null;
  if (!a) throw new Error('Pick what to do with the picture.');

  let prompt = String(body.prompt ?? '').trim();
  if (prompt.length > 2000) throw new Error('Keep the description under 2000 characters.');
  if (!prompt) {
    if (a.prompt === null) {
      if (a.shot) {
        const sheetPrompt = String(sheet?.prompt ?? '').trim();
        if (!sheetPrompt) throw new Error('Say who is on the sheet.');
        prompt = sheetPrompt;
      } else {
        throw new Error('Say what should change.');
      }
    } else {
      prompt = a.prompt;
    }
  }

  if (a.mask && !hasMask) throw new Error('Paint the area to change.');
  if (!a.mask && hasMask) throw new Error('Only Repaint takes a painted area.');
  if (!a.face && hasFace) throw new Error('Only Repaint takes a face photo.');

  const params = {};
  // STU-C2: shot, face prompt and background first; the loop below adds likeness
  if (a.shot) {
    const facePrompt = String(body.facePrompt ?? '').trim();
    if (facePrompt.length > 500) throw new Error('Keep the face description under 500 characters.');
    params.shot = a.shot;
    params.face_prompt = facePrompt || String(sheet?.params?.face_prompt ?? '');
    params.background = ['white', 'starry'].includes(body.background) ? body.background :
      (['white', 'starry'].includes(sheet?.params?.background) ? sheet.params.background : 'white');
  }
  for (const [key, [lo, hi, kind]] of Object.entries(a.params)) {
    const raw = body[key];
    if (raw === undefined || raw === null || raw === '') continue;
    const n = Number(raw);
    if (!Number.isFinite(n) || n < lo || n > hi || (kind === 'int' && !Number.isInteger(n))) {
      throw new Error(`${key} must be between ${lo} and ${hi}.`);
    }
    params[key] = n;
  }

  let rating;
  if (a.rating) {
    if (a.shot) {
      rating = ['general', 'sensitive', 'questionable', 'explicit'].includes(body.rating) ? body.rating :
        (sheet?.rating && ['general', 'sensitive', 'questionable', 'explicit'].includes(sheet.rating)) ? sheet.rating : 'general';
    } else {
      rating = ['general', 'sensitive', 'questionable', 'explicit'].includes(body.rating) ? body.rating : 'general';
      if (body.action === 'chibi' && !['general', 'sensitive'].includes(rating)) {
        throw new Error('Chibi comes in general or sensitive only.');
      }
    }
  }

  let seed;
  if (a.shot) {
    if ((body.seed === undefined || body.seed === null || body.seed === '') && Number.isInteger(sheet?.seed)) seed = sheet.seed;
  }

  return { type: a.type, prompt, params, rating, size: a.size || 'square', seed };
}

module.exports = { refineRequest, ACTIONS };
