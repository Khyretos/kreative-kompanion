// STU-R2: Refine: take one picture (a library result or an upload), pick a step, and make a new
// picture from it. Every result has its own Refine button, so a picture can be refined again and again.
(() => {
    const STEPS = {
        '4k': { label: '4K for an artist', short: '4K', hint: 'Same picture at 3840 px on the long side, crisp lines, nothing redrawn.', prompt: null,
            amount: { name: 'size', label: 'Long side', min: 2048, max: 4096, step: 256, value: 3840, unit: ' px' } },
        change: { label: 'Recolour or change', short: 'Recolour', hint: 'The lines and pose stay, the picture is painted again: say the new colours and also what must stay (starry sky, moustache).', prompt: 'What should change', placeholder: 'black and gold armour, dark skin, red cape',
            amount: { name: 'strength', label: 'How much changes', min: 0.2, max: 1, step: 0.05, value: 1 }, rating: true },
        repaint: { label: 'Repaint an area', short: 'Repaint', hint: 'Paint over the part to replace (a head, a helmet), then say what goes there. The rest stays exactly as it is.', prompt: 'What goes in the painted area', placeholder: 'closed black dragon helmet with horns', mask: true, face: true,
            amount: { name: 'strength', label: 'How new', min: 0.3, max: 1, step: 0.05, value: 0.9 }, rating: true },
        chibi: { label: 'Chibi version', short: 'Chibi', hint: 'The same outfit and colours with chibi proportions. Works best on one view.', prompt: 'Describe the character (optional)', placeholder: '1boy, moustache, white suit, purple tie, headphones',
            amount: { name: 'likeness', label: 'Likeness', min: 0, max: 1, step: 0.05, value: 0.6 }, rating: 'chibi' },
        parts: { label: 'Split into parts', short: 'Parts', hint: 'Transparent layers for an avatar: the whole cut-out, the head and the body. Works best on one view.', prompt: null,
            amount: { name: 'head', label: 'Head margin', min: 0, max: 400, step: 10, value: 60, unit: ' px' } },
        // STU-C2: a shot of a sheet's character; for your own sheets the prompt, seed and face details come from the sheet
        headshot: { label: 'Headshot', short: 'Headshot', hint: 'Neck up, with the same face, hair and outfit as the sheet. Works on your OC sheets; for an upload, describe the character.', prompt: 'Describe the character (optional for your sheets)', placeholder: '1boy, moustache, white suit, purple tie, headphones',
            amount: { name: 'likeness', label: 'Outfit likeness', min: 0, max: 1, step: 0.05, value: 0.6 }, rating: true },
        cowboy: { label: 'Cowboy shot', short: 'Cowboy', hint: 'From the thighs up, with the same face, hair and outfit as the sheet. Works on your OC sheets; for an upload, describe the character.', prompt: 'Describe the character (optional for your sheets)', placeholder: '1boy, moustache, white suit, purple tie, headphones',
            amount: { name: 'likeness', label: 'Outfit likeness', min: 0, max: 1, step: 0.05, value: 0.6 }, rating: true },
        // 3D-01: a textured .glb of the character, for Blender and Kompanion's 3D viewer
        model3d: { label: '3D model', short: '3D model', hint: 'A 3D model (.glb) of the character, coloured from the picture, for Blender and the 3D viewer. Works best on one full-body front view.', prompt: null }
    };
    const ICONS = {
        '4k': '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
        change: '<path d="M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.6 0-1.4-1.3-1.6-1.3-2.9 0-1 .8-1.5 1.8-1.5H17a4 4 0 0 0 4-4c0-4.4-4-8-9-8z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7" r="1"/><circle cx="15" cy="7.5" r="1"/>',
        repaint: '<path d="M18 3l3 3-9 9-4 1 1-4z"/><path d="M4 21c2 0 3-1 3-3"/>',
        chibi: '<circle cx="12" cy="9" r="6"/><path d="M8 21v-2a4 4 0 0 1 8 0v2"/>',
        parts: '<rect x="8" y="2" width="8" height="7" rx="2"/><rect x="6" y="11" width="12" height="11" rx="2"/>',
        headshot: '<circle cx="12" cy="9" r="4"/><path d="M5 21c0-4 3-7 7-7s7 3 7 7"/>',
        cowboy: '<circle cx="12" cy="5" r="2.5"/><path d="M8 21v-9a4 4 0 0 1 8 0v9M8 14h8"/>',
        model3d: '<path d="M12 2l9 5v10l-9 5-9-5V7z"/><path d="M12 12l9-5M12 12v10M12 12L3 7"/>'
    };
    window.refineLabel = (step) => (STEPS[step] ? STEPS[step].short : 'Refined'); // short: it sits in a card badge

    const $ = (id) => document.getElementById(id);
    const stage = $('refine-stage'), img = $('refine-img'), canvas = $('refine-mask');
    const ctx = canvas.getContext('2d');
    let step = 'change';
    let source = null; // { job, file, sub } or { upload: File }
    let painted = false;
    let erase = false;

    // steps
    const grid = $('refine-steps');
    for (const [id, s] of Object.entries(STEPS)) {
        const b = document.createElement('button');
        b.type = 'button';
        b.className = 'step-btn';
        b.dataset.step = id;
        b.setAttribute('role', 'radio');
        b.innerHTML = `<span class="preset-icon" aria-hidden="true"><svg viewBox="0 0 24 24">${ICONS[id]}</svg></span><span class="preset-label">${s.short}</span>`;
        b.title = s.label;
        b.addEventListener('click', () => pick(id));
        grid.appendChild(b);
    }

    function pick(id) {
        step = id;
        const s = STEPS[id];
        for (const b of grid.children) {
            b.classList.toggle('active', b.dataset.step === id);
            b.setAttribute('aria-checked', String(b.dataset.step === id));
        }
        $('refine-hint').textContent = s.hint;
        $('refine-prompt-group').hidden = !s.prompt;
        $('refine-prompt-label').textContent = s.prompt || '';
        $('refine-prompt').placeholder = s.placeholder || '';
        $('refine-face-group').hidden = !s.face;
        $('refine-paint-tools').hidden = !s.mask;
        stage.classList.toggle('painting', Boolean(s.mask));
        $('refine-rating-group').hidden = !s.rating;
        const r = $('refine-rating'), keep = r.value;
        const ratings = ['general', 'sensitive'].concat(s.rating === true && window.studioAdult ? ['questionable', 'explicit'] : []);
        r.innerHTML = ratings.map(v => `<option value="${v}">${v[0].toUpperCase() + v.slice(1)}</option>`).join('');
        r.value = ratings.includes(keep) ? keep : 'general';
        const a = s.amount;
        if (!a) {
            $('refine-amount-group').hidden = true;
            return;
        }
        $('refine-amount-label').textContent = a.label;
        Object.assign($('refine-amount'), { min: a.min, max: a.max, step: a.step });
        $('refine-amount').value = a.value;
        showAmount();
        sync();
    }
    function showAmount() {
        if (STEPS[step].amount === undefined) return;
        const a = STEPS[step].amount;
        $('refine-amount-val').textContent = `${$('refine-amount').value}${a.unit || ''}`;
    }
    $('refine-amount').addEventListener('input', showAmount);

    function sync() {
        const s = STEPS[step];
        let why = '';
        if (!source) why = 'Pick a picture first.';
        else if (s.mask && !painted) why = 'Paint over the area to change.';
        $('generate-btn-refine').disabled = Boolean(why);
        $('generate-btn-refine').title = why;
    }

    // the picture
    function show(url, name) {
        $('refine-empty').hidden = true;
        stage.hidden = false;
        $('refine-source-tools').hidden = false;
        $('refine-source-name').textContent = name;
        img.onload = () => {
            canvas.width = img.naturalWidth;
            canvas.height = img.naturalHeight;
            clearMask();
        };
        img.src = url;
    }
    function useLibrary(id, file, sub) {
        source = { job: id, file, sub };
        const card = document.querySelector(`.job-card[data-prompt-id="${CSS.escape(id)}"]`);
        const prompt = card?.querySelector('.job-prompt')?.title || '';
        if (prompt && !$('refine-prompt').value) $('refine-prompt').value = prompt; // start from what made it
        show(`/api/image?promptId=${encodeURIComponent(id)}&filename=${encodeURIComponent(file)}&subfolder=${encodeURIComponent(sub || '')}`,
            card?.querySelector('.kind-badge')?.textContent || 'From your library');
    }
    $('refine-upload').addEventListener('change', (e) => {
        const f = e.target.files[0];
        if (!f) return;
        source = { upload: f };
        show(URL.createObjectURL(f), f.name);
    });
    $('refine-clear').addEventListener('click', () => {
        source = null;
        stage.hidden = true;
        $('refine-source-tools').hidden = true;
        $('refine-empty').hidden = false;
        $('refine-upload').value = '';
        sync();
    });
    document.addEventListener('click', (e) => {
        const b = e.target.closest('[data-act="refine"]');
        if (!b) return;
        useLibrary(b.dataset.id, b.dataset.file, b.dataset.sub);
        switchTab('refine');
    });

    // painting the area (orange, half see-through; sent as a white-on-black mask)
    function clearMask() {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        painted = false;
        sync();
    }
    $('refine-mask-clear').addEventListener('click', clearMask);
    $('refine-erase').addEventListener('click', (e) => {
        erase = !erase;
        e.currentTarget.setAttribute('aria-pressed', String(erase));
    });
    let last = null;
    function at(ev) {
        const r = canvas.getBoundingClientRect();
        return { x: (ev.clientX - r.left) * canvas.width / r.width, y: (ev.clientY - r.top) * canvas.height / r.height, k: canvas.width / r.width };
    }
    function dab(p) {
        ctx.globalCompositeOperation = erase ? 'destination-out' : 'source-over';
        ctx.strokeStyle = ctx.fillStyle = 'rgba(243, 148, 31, 1)';
        ctx.lineCap = ctx.lineJoin = 'round';
        ctx.lineWidth = $('refine-brush').value * p.k;
        ctx.beginPath();
        if (last) { ctx.moveTo(last.x, last.y); ctx.lineTo(p.x, p.y); ctx.stroke(); } else { ctx.arc(p.x, p.y, ctx.lineWidth / 2, 0, Math.PI * 2); ctx.fill(); }
        last = p;
        if (!erase) painted = true;
    }
    canvas.addEventListener('pointerdown', (ev) => {
        if (!STEPS[step].mask) return;
        canvas.setPointerCapture(ev.pointerId);
        last = null;
        dab(at(ev));
        ev.preventDefault();
    });
    canvas.addEventListener('pointermove', (ev) => { if (last) dab(at(ev)); });
    const end = () => { if (last) { last = null; sync(); } };
    canvas.addEventListener('pointerup', end);
    canvas.addEventListener('pointercancel', end);

    function maskBlob() {
        const out = document.createElement('canvas');
        out.width = canvas.width;
        out.height = canvas.height;
        const o = out.getContext('2d');
        o.drawImage(canvas, 0, 0);
        o.globalCompositeOperation = 'source-in';
        o.fillStyle = '#fff';
        o.fillRect(0, 0, out.width, out.height);
        o.globalCompositeOperation = 'destination-over';
        o.fillStyle = '#000';
        o.fillRect(0, 0, out.width, out.height);
        return new Promise((ok) => out.toBlob(ok, 'image/png'));
    }

    // send
    $('generate-form-refine').addEventListener('submit', async (e) => {
        e.preventDefault();
        const s = STEPS[step];
        const btn = $('generate-btn-refine'), err = $('error-msg-refine');
        err.textContent = '';
        const fd = new FormData();
        fd.append('action', step);
        if (s.prompt) fd.append('prompt', $('refine-prompt').value.trim());
        if (s.amount) fd.append(s.amount.name, $('refine-amount').value);
        if (s.rating) fd.append('rating', $('refine-rating').value);
        if ($('refine-seed').value) fd.append('seed', $('refine-seed').value);
        if (source.upload) fd.append('source', source.upload);
        else { fd.append('sourceJob', source.job); fd.append('sourceFile', source.file); }
        if (s.mask) fd.append('mask', await maskBlob(), 'mask.png');
        const face = $('refine-face').files[0];
        if (s.face && face) fd.append('face', face);
        btn.disabled = true;
        btn.textContent = 'Sending…';
        try {
            const res = await fetch('/api/refine', { method: 'POST', body: fd });
            const body = await res.json().catch(() => ({}));
            if (!res.ok) throw new Error(body.error || 'Refine failed');
            btn.textContent = 'Started. It appears in your library';
            setTimeout(() => { btn.textContent = 'Refine'; sync(); }, 2500);
        } catch (x) {
            err.textContent = x.message;
            btn.textContent = 'Refine';
            sync();
        }
    });

    pick('change');
})();
