// KS-01: Kreative Studio's jobs go through Kompanion's Studio service API (placement, GPU
// scheduler, "Studio runs on"). Drafted by the local Coder; Claude fixed videoFrames and the retry count.

function presetType(preset) {
  const map = { oc: 'oc-sheet', portrait: 'vn-portrait', scene: 'vn-scene', video: 'video' };
  return map.hasOwnProperty(preset) ? map[preset] : null;
}

function sizeName(w, h) {
  if (w > h) return 'wide';
  if (w < h) return 'tall';
  return 'square';
}

function videoFrames(seconds) {
  return Math.floor(Math.floor(seconds * 24) / 4) * 4 + 1;
}

// KS-02: Kompanion's own studio types (not the ones Kreative Studio already has as presets), in order.
function typeList(rows) {
  const presets = ['oc-sheet', 'vn-portrait', 'vn-scene', 'video'];
  return rows
    .filter(row => row.studio && !row.error)
    .filter(row => !presets.includes(row.name))
    .filter(row => !row.studio.refine) // STU-R2: refine steps live in the Refine tab
    .sort((a, b) => (a.studio.order ?? 0) - (b.studio.order ?? 0))
    .map((row) => ({
      id: row.name,
      label: row.studio.label,
      hint: row.studio.hint || '',
      sizes: (row.studio.sizes && row.studio.sizes.length) ? row.studio.sizes : ['square'],
      face: Boolean(row.face),
      rating: (row.params || []).some(p => p.name === 'rating'),
      ...(row.problems && row.problems.length ? { warning: row.problems.join('; ') } : {})
    }));
}

// KS-04: licence warnings of the types Kreative Studio has as its own presets (and video), by name.
function presetWarnings(rows) {
  const out = {};
  for (const row of rows) {
    if (['oc-sheet', 'vn-portrait', 'vn-scene', 'video'].includes(row.name) && (row.problems || []).length) out[row.name] = row.problems.join('; ');
  }
  return out;
}

function jobState(run) {
  const state = run.state;
  if (state === 'done') {
    return { status: 'done', finished: true };
  }
  if (state === 'failed') {
    return { status: 'error', finished: true, error: run.error || 'failed' };
  }
  return { status: 'running', finished: false };
}

// KS-03: one library with Kompanion. A local job as Kompanion's import body, and a Kompanion run as a job.

function importPayload(job, root) {
  if (job.status !== 'done' || job.kompanionId || job.fromKompanion) return null;

  let type;
  if (job.kind === 'music' || job.kind === 'sfx') {
    type = job.kind;
  } else {
    const pt = presetType(job.preset);
    if (pt !== null) {
      type = pt;
    } else if (typeof job.preset === 'string' && job.preset.startsWith('k:')) {
      type = job.preset.slice(2);
    } else {
      type = job.preset;
    }
  }

  const files = [];
  if (job.images) {
    for (const i of job.images) {
      files.push([root, 'images', i.subfolder, i.filename].filter(Boolean).join('/'));
    }
  }
  if (job.files) {
    for (const f of job.files) {
      files.push([root, 'audio', f.name].join('/'));
    }
  }

  return {
    id: job.promptId,
    type,
    prompt: job.prompt || '',
    seed: job.seed ?? null,
    seconds: job.seconds ?? null,
    lyrics: job.lyrics ?? null,
    state: 'done',
    created: job.created,
    files
  };
}

function runJob(run, user, email) {
  const back = {
    'oc-sheet': 'oc',
    'vn-portrait': 'portrait',
    'vn-scene': 'scene',
    video: 'video',
    music: 'music',
    sfx: 'sfx'
  };

  let kind;
  if (run.type === 'music' || run.type === 'sfx' || run.type === 'video') {
    kind = run.type;
  } else if (run.type === 'image-to-3d') {
    kind = 'model'; // 3D-01: a .glb plus its preview picture
  } else {
    kind = 'image';
  }

  const preset = back[run.type] || 'k:' + run.type;
  const names = run.names || [];
  const outs = run.files || [];
  const audio = kind === 'music' || kind === 'sfx';

  const name = (n) => names[n] || `${run.id}-${n + 1}`;
  const isGlb = (n) => kind === 'model' && name(n).endsWith('.glb');
  const images = audio ? [] : outs.map((_, n) => n).filter(n => !isGlb(n)).map(n => ({
    filename: name(n),
    subfolder: '',
    kompanion: n
  }));

  const files = kind === 'model' ? outs.map((_, n) => n).filter(isGlb).map(n => ({ name: name(n), mime: 'model/gltf-binary', kompanion: n }))
    : audio ? outs.map((_, n) => {
    const name = names[n] || `${run.id}-${n + 1}`;
    return {
      name,
      mime: name.endsWith('.wav') ? 'audio/wav' : 'audio/ogg',
      kompanion: n
    };
  }) : [];

  return {
    promptId: run.id,
    kompanionId: run.id,
    kind,
    user,
    email,
    preset,
    prompt: run.prompt || '',
    seconds: run.seconds ?? undefined,
    created: Date.parse(run.startedAt) || 0,
    ...jobState(run),
    progress: run.state === 'done' ? 1 : 0,
    images,
    files,
    fromKompanion: true
  };
}
class Kompanion {
  constructor({ url, token, pollMs = 2000 }) {
    this.url = String(url || '').replace(/\/+$/, '');
    this.token = token || '';
    this.pollMs = pollMs;
  }

  get enabled() {
    return Boolean(this.url && this.token);
  }

  async _call(method, path, email, { body, json } = {}) {
    const headers = {
      Authorization: `Bearer ${this.token}`,
      'X-Studio-User': email
    };
    if (json !== undefined) {
      headers['Content-Type'] = 'application/json';
      body = JSON.stringify(json);
    }
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 120000);
    try {
      const r = await fetch(`${this.url}/api/studio/service${path}`, {
        method,
        headers,
        body,
        signal: controller.signal
      });
      clearTimeout(timeoutId);
      if (!r.ok) {
        let msg = `Kompanion answered ${r.status}`;
        try {
          const text = await r.text();
          const err = JSON.parse(text).error;
          if (err) msg = err;
        } catch {
          // ignore parse errors
        }
        throw new Error(msg);
      }
      return r;
    } catch (e) {
      clearTimeout(timeoutId);
      throw e;
    }
  }

  async make(email, { type, prompt, size, count, rating, seed, length, face, faceWeight, negative, params, source, mask }) {
    const fd = new FormData();
    fd.append('type', type);
    fd.append('prompt', prompt);
    fd.append('size', size);
    if (count !== undefined && count !== null) fd.append('count', String(count));
    if (rating !== undefined && rating !== null) fd.append('rating', String(rating));
    if (seed !== undefined && seed !== null) fd.append('seed', String(seed));
    if (length !== undefined && length !== null) fd.append('length', String(length));
    if (negative) fd.append('negative', String(negative)); // STU-N1
    // STU-R2: the type's own settings (face_prompt, background, strength, ...) and a refine step's pictures
    if (params && Object.keys(params).length) fd.append('params', JSON.stringify(params));
    if (source) fd.append('source', new Blob([source.buffer], { type: source.mime }), source.name);
    if (mask) fd.append('mask', new Blob([mask.buffer], { type: mask.mime }), mask.name);
    if (face) {
      fd.append('face', new Blob([face.buffer], { type: face.mime }), face.name);
    }
    if (faceWeight !== undefined && faceWeight !== null) {
      fd.append('face_weight', String(faceWeight ?? 0.85));
    }
    const r = await this._call('POST', '/make', email, { body: fd });
    return await r.json();
  }

  async makeImage(email, opts) {
    return (await this.make(email, opts)).ids[0];
  }

  async workflows(email) {
    const r = await this._call('GET', '/workflows', email);
    return r.json();
  }

  async types(email) {
    return typeList(await this.workflows(email));
  }

  async makeAudio(email, { kind, prompt, lyrics, seconds }) {
    const r = await this._call('POST', '/audio', email, {
      json: { kind, prompt, lyrics, seconds }
    });
    return (await r.json()).ids[0];
  }

  async run(email, id) {
    const r = await this._call('GET', `/runs/${encodeURIComponent(id)}`, email);
    return r.json();
  }

  async file(email, id, n) {
    return this._call('GET', `/runs/${encodeURIComponent(id)}/files/${Number(n)}`, email);
  }

// KS-03: send a finished local job to Kompanion's library; list this user's runs there.
  async importJob(email, payload) {
    const r = await this._call('POST', '/import', email, { json: payload });
    return r.json();
  }

  // STU-D1: delete a finished run (and its files) in Kompanion.
  async deleteRun(email, id) {
    await this._call('DELETE', `/runs/${encodeURIComponent(id)}`, email);
  }

  async mine(email) {
    const r = await this._call('GET', '/mine', email);
    return r.json();
  }
  async follow(email, id, job, notify) {
    let failures = 0;
    while (!job.finished) {
      let run;
      try {
        run = await this.run(email, id);
      } catch (e) {
        failures++;
        if (failures >= 5) {
          job.status = 'error';
          job.error = e.message;
          job.finished = true;
          notify(job);
          return;
        }
        await new Promise(r => setTimeout(r, this.pollMs));
        continue;
      }
      failures = 0;
      const st = jobState(run);
      if (st.status !== job.status) {
        Object.assign(job, st);
        if (st.status === 'done') {
          job.progress = 1;
          job.outputs = Array.isArray(run.outputs) ? run.outputs : [];
          job.gpu = run.gpu;
        }
        notify(job);
      }
      if (job.finished) break;
      await new Promise(r => setTimeout(r, this.pollMs));
    }
  }
}

module.exports = { Kompanion, presetType, sizeName, videoFrames, jobState, typeList, presetWarnings, importPayload, runJob };
