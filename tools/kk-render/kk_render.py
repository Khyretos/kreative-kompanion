#!/usr/bin/env python3
"""kk-render: the only way Kompanion render jobs run on soucouyant (Kees's daily driver, RX 9070 XT 16 GB).

Kees 2026-10-19 13:47Z, after three desktop crashes from ComfyUI renders: soucouyant runs only Kompanion's ComfyUI
work, one job at a time, capped, with a host-side timeout and a guard that refuses work while the desktop needs the GPU.

What happened before (why the old caps did not help):
  03:14Z  MV-Adapter, no cap, 15.5 GB card total -> Hyprland aborted.
  04:38Z  MV-Adapter + SDXL in one ComfyUI, 13.9 GB -> desktop hung.
  13:45Z  q20-sdxl: 0.55 torch cap and a 13 GB watcher, but PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True (which Kees
          turned off on 2026-10-11 because ROCm never gave that memory back), --reserve-vram 7.5 forcing constant partial
          loads/offloads, and gpu-guard watching the wrong port with 1-minute heartbeats. The first render came out all
          black (NaN) and the batch went on; the next render ended in a GPU page fault in Hyprland's context, a gfx ring
          reset, and Hyprland aborting (it cannot survive any GPU reset). So the line to hold is "no GPU fault at all",
          not only "VRAM under X".

So every job here: one at a time (flock), preflight refuses while CI/games/other GPU users are active, a fresh container
with a 0.50 torch cap, no expandable segments, bf16 VAE, RAM cap, a 4 Hz watchdog that docker-kills on VRAM, heat or
any new amdgpu kernel fault, a hard per-job timeout, a black/flat-image check that stops the batch, and the container is
always stopped at the end.

Usage:  kk_render.py check                      preflight only, prints why it would refuse
        kk_render.py run GRAPH.json [GRAPH2.json ...] [--timeout 300] [--outdir DIR]
GRAPH is a ComfyUI API-format prompt (as /prompt takes). Outputs land in ~/kk-render/out/<subfolder>.
"""
import argparse, fcntl, json, os, subprocess, sys, threading, time, urllib.request, urllib.error

DEV = "/sys/class/drm/card1/device"
PORT = 18190
NAME = "kk-render"
HOME = os.path.expanduser("~")
WORK = os.path.join(HOME, "kk-render")
LOCK = f"/run/user/{os.getuid()}/kk-render.lock"
MIB = 1 << 20

# Limits. Desktop alone sits at 1.2-2.4 GB.
KILL_VRAM_MIB = 12 * 1024       # card total; leaves 4 GB for Hyprland, the browser and the Claude app
START_MAX_VRAM_MIB = 3584       # something else holds the GPU if more than this is in use before we start
START_MAX_BUSY = 30             # % gpu_busy averaged over 5 s before start: desktop/game is using the GPU
KILL_JUNCTION_C = 95
KILL_MEM_C = 96
MAX_LOAD1 = 6.0
MIN_FREE_RAM_GIB = 8
GAMES = ("Funkin", "steam_app", "gamescope", "wine64-preloader", "wineserver", ".exe", "retroarch", "dolphin-emu")
# Kees 2026-10-24: the Studio's ComfyUI may stay up, and render jobs must not be refused because of it. kk-render
# waits until its queue is empty, stops it for the job (two ComfyUIs on the card is how the desktop died), then starts
# it again.
STUDIO = "comfyui-rocm"
STUDIO_PORT = 8188
STUDIO_IDLE_WAIT_S = 1800

M = os.path.join(HOME, "comfy-models-local")
# One model family per container (04:38Z: MV-Adapter and SDXL in one ComfyUI kept the MV memory and hit 13.9 GB).
PROFILES = {
    # name: (image, torch VRAM fraction, --reserve-vram GB so ComfyUI plans inside the cap, VAE flag)
    "sdxl": ("comfyui-rocm:local", 0.50, "8.5", "--bf16-vae"),
    # TRELLIS.2: the fp16 and bf16 VAE decodes give NaN on gfx1201, so fp32 (3d-01 notes).
    "trellis": ("comfyui-rocm:local", 0.60, "6.8", "--fp32-vae"),
    # the production setup: no global VAE flag; studio/comfy-nodes/kk_trellis.py loads the TRELLIS VAEs in fp32 itself
    "trellis-nodes": ("comfyui-rocm:local", 0.60, "6.8", ""),
    # MV-Adapter SD2.1 (STU-SHEET3D; Kees chose SD21 on 2026-10-19): 4 views at 512 measured 6.9 GB card total.
    "mv": ("comfyui-mv:q19", 0.50, "8.5", "--bf16-vae"),
}
KK_NODES = [(f"{HOME}/.local/lib/kk-render/kk_trellis.py", "/app/custom_nodes/kk_trellis.py", "ro")]
MV_MOUNTS = [
    (f"{M}/mvadapter", "/mvm/mvadapter", "ro"),
    (f"{HOME}/kk-mv/diffusers-cache", "/mvcache", "rw"),
]
MV_ENV = ["-e", "MV_HF_DIR=/mvcache", "-e", "MV_OFFLOAD=0", "-e", "HF_HOME=/mvcache/hf", "-e", "HF_HUB_OFFLINE=1"]
TRELLIS_MOUNTS = [
    (f"{M}/diffusion_models", "/app/models/diffusion_models", "ro"),
    (f"{M}/vae/trellis_2_shape_vae_bf16.safetensors", "/app/models/vae/trellis_2_shape_vae_bf16.safetensors", "ro"),
    (f"{M}/vae/trellis_2_texture_vae_bf16.safetensors", "/app/models/vae/trellis_2_texture_vae_bf16.safetensors", "ro"),
    (f"{M}/clip_vision", "/app/models/clip_vision", "ro"),
    (f"{M}/background_removal/birefnet.safetensors", "/app/models/background_removal/birefnet.safetensors", "ro"),
    (f"{HOME}/.local/lib/kk-render/kk_dump.py", "/app/custom_nodes/kk_dump.py", "ro"),
]
MOUNTS = [
    (f"{WORK}/out", "/media-out", "rw"),
    (f"{WORK}/in", "/app/input", "rw"),
    (f"{HOME}/Server-Docker/Services/ai/comfyui/user", "/app/user", "rw"),
    (f"{HOME}/.local/lib/kk-render/vram_cap.py", "/app/custom_nodes/kk_vram_cap.py", "ro"),
]
SDXL_MOUNTS = [
    (f"{M}/checkpoints/novaAnimeXL_ilV170.safetensors", "/app/models/checkpoints/novaAnimeXL_ilV170.safetensors", "ro"),
    (f"{M}/loras", "/app/models/loras", "ro"),
    (f"{M}/controlnet/xinsir-union-sdxl-promax.safetensors", "/app/models/controlnet/xinsir-union-sdxl-promax.safetensors", "ro"),
    (f"{M}/ipadapter/ip-adapter-plus-face_sdxl_vit-h.safetensors", "/app/models/ipadapter/ip-adapter-plus-face_sdxl_vit-h.safetensors", "ro"),
    (f"{M}/clip_vision/CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors", "/app/models/clip_vision/CLIP-ViT-H-14-laion2B-s32B-b79K.safetensors", "ro"),
    (f"{M}/upscale_models/4x-AnimeSharp.pth", "/app/models/upscale_models/4x-AnimeSharp.pth", "ro"),
    (f"{M}/ultralytics", "/app/models/ultralytics", "ro"),
    (f"{M}/sams", "/app/models/sams", "ro"),
    (f"{M}/background_removal/birefnet.safetensors", "/app/models/background_removal/birefnet.safetensors", "ro"),
]


def log(ev, **kw):
    print(json.dumps({"t": time.strftime("%H:%M:%SZ", time.gmtime()), "event": ev, **kw}), flush=True)


def rd(p):
    with open(p) as f:
        return f.read().strip()


def hwmon():
    base = os.path.join(DEV, "hwmon")
    return os.path.join(base, os.listdir(base)[0])


def sample():
    h = hwmon()
    temps = {}
    for i in (1, 2, 3):
        try:
            temps[rd(f"{h}/temp{i}_label")] = int(rd(f"{h}/temp{i}_input")) // 1000
        except OSError:
            pass
    return {"vram_mib": int(rd(f"{DEV}/mem_info_vram_used")) // MIB, "busy": int(rd(f"{DEV}/gpu_busy_percent")),
            "junction_c": temps.get("junction", 0), "mem_c": temps.get("mem", 0)}


def running_containers():
    out = subprocess.run(["docker", "ps", "--format", "{{.Names}}"], capture_output=True, text=True, timeout=20).stdout
    return [n for n in out.split() if n]


def gpu_free_ci(name):
    """Kees 2026-10-25 ("Allow"): a Forgejo CI job container that has no GPU device may run next to a render.
    Anything else (tests, other ComfyUIs, CI with /dev/kfd or /dev/dri) still stops it."""
    if not name.startswith("FORGEJO-ACTIONS-TASK-"):
        return False
    r = subprocess.run(["docker", "inspect", "-f", "{{json .HostConfig.Devices}} {{json .HostConfig.Binds}}", name],
                       capture_output=True, text=True, timeout=20)
    if r.returncode != 0:
        return True  # gone already
    return "/dev/kfd" not in r.stdout and "/dev/dri" not in r.stdout


def blocking_containers(skip):
    return [n for n in running_containers() if n not in skip and not gpu_free_ci(n)]


def preflight(gpu=True):
    """Return a list of reasons to refuse; empty means the GPU is free for one job.
    gpu=False skips the VRAM/busy/heat checks (used while the Studio ComfyUI still holds the card)."""
    why = []
    others = blocking_containers((NAME, STUDIO))
    if others:
        why.append("other containers running (CI or tests): " + ", ".join(others))
    games = []
    for pid in os.listdir("/proc"):
        if pid.isdigit():
            try:
                cmd = open(f"/proc/{pid}/cmdline", "rb").read().replace(b"\0", b" ").decode(errors="replace")
            except OSError:
                continue
            if any(g in cmd for g in GAMES) and "kk_render" not in cmd:
                games.append(cmd[:60])
    if games:
        why.append("a game/GPU app is running: " + "; ".join(games[:3]))
    if os.getloadavg()[0] > MAX_LOAD1:
        why.append("load %.1f > %.0f" % (os.getloadavg()[0], MAX_LOAD1))
    avail = int([l for l in open("/proc/meminfo") if l.startswith("MemAvailable")][0].split()[1]) // (1 << 20)
    if avail < MIN_FREE_RAM_GIB:
        why.append(f"only {avail} GiB RAM available")
    if not gpu:
        return why
    busy = []
    for _ in range(5):
        busy.append(sample()["busy"])
        time.sleep(1)
    s = sample()
    if s["vram_mib"] > START_MAX_VRAM_MIB:
        why.append(f"{s['vram_mib']} MiB VRAM already in use (> {START_MAX_VRAM_MIB})")
    if sum(busy) / len(busy) > START_MAX_BUSY:
        why.append("desktop is using the GPU (%d%% busy)" % (sum(busy) / len(busy)))
    if s["junction_c"] > 70:
        why.append(f"GPU still hot ({s['junction_c']} C)")
    return why


class Watchdog(threading.Thread):
    """4 Hz: docker-kill the container on VRAM, heat, or any new amdgpu fault in the kernel log."""

    def __init__(self):
        super().__init__(daemon=True)
        self.stop_ev = threading.Event()
        self.tripped = None
        self.peak = {"vram_mib": 0, "junction_c": 0, "mem_c": 0}
        self.kmsg = subprocess.Popen(["journalctl", "-k", "-f", "-n", "0", "-o", "cat"], stdout=subprocess.PIPE, text=True)
        threading.Thread(target=self._kernel, daemon=True).start()

    def _kernel(self):
        for line in self.kmsg.stdout:
            l = line.lower()
            if "amdgpu" in l and any(k in l for k in ("page fault", "timeout", "reset", "wedged", "memory access fault")):
                self.trip("kernel: " + line.strip()[:200])

    def trip(self, why):
        if self.tripped is None:
            self.tripped = why
            subprocess.run(["docker", "kill", NAME], capture_output=True, timeout=20)
            log("KILLED", why=why)

    def run(self):
        n = 0
        while not self.stop_ev.is_set():
            n += 1
            if n % 20 == 0:  # every ~5 s: a CI job or test container that starts mid-render ends the render
                others = blocking_containers((NAME,))
                if others:
                    self.trip("another container started: " + ", ".join(others))
            s = sample()
            for k in self.peak:
                self.peak[k] = max(self.peak[k], s[k])
            if s["vram_mib"] > KILL_VRAM_MIB:
                self.trip(f"VRAM {s['vram_mib']} MiB > {KILL_VRAM_MIB}")
            elif s["junction_c"] >= KILL_JUNCTION_C or s["mem_c"] >= KILL_MEM_C:
                self.trip(f"heat junction {s['junction_c']} C mem {s['mem_c']} C")
            time.sleep(0.25)

    def stop(self):
        self.stop_ev.set()
        self.kmsg.terminate()


def studio_busy():
    """True while the Studio ComfyUI has a prompt running or queued."""
    try:
        q = json.load(urllib.request.urlopen(f"http://127.0.0.1:{STUDIO_PORT}/queue", timeout=5))
    except (urllib.error.URLError, OSError, ValueError):
        return False
    return bool(q.get("queue_running") or q.get("queue_pending"))


def pause_studio():
    """Stop the Studio ComfyUI once it is idle. Returns True if it was running (so it gets started again)."""
    if STUDIO not in running_containers():
        return False
    t0 = time.time()
    while studio_busy():
        if time.time() - t0 > STUDIO_IDLE_WAIT_S:
            raise RuntimeError(f"{STUDIO} stayed busy for {STUDIO_IDLE_WAIT_S} s")
        log("WAIT", why=f"{STUDIO} is rendering; waiting for its queue to empty")
        time.sleep(15)
    subprocess.run(["docker", "stop", "-t", "20", STUDIO], capture_output=True, timeout=90)
    log("PAUSED", container=STUDIO)
    t0 = time.time()
    while sample()["vram_mib"] > START_MAX_VRAM_MIB and time.time() - t0 < 30:
        time.sleep(1)
    return True


def resume_studio():
    subprocess.run(["docker", "start", STUDIO], capture_output=True, timeout=90)
    log("RESUMED", container=STUDIO)


# ComfyUI flags a run may add (memory planning and attention only; nothing that lifts a cap)
COMFY_FLAGS = ("--cpu", "--lowvram", "--novram", "--disable-smart-memory", "--use-split-cross-attention", "--use-quad-cross-attention", "--use-pytorch-cross-attention")


def start_container(profile, extra=()):
    image, frac, reserve, vae = PROFILES[profile]
    for d in ("out", "in"):
        os.makedirs(os.path.join(WORK, d), exist_ok=True)
    subprocess.run(["docker", "rm", "-f", NAME], capture_output=True, timeout=60)
    cmd = ["docker", "run", "-d", "--name", NAME, "--network", "host", "--device", "/dev/kfd", "--device", "/dev/dri",
           "--group-add", "988", "--group-add", "video", "--security-opt", "seccomp=unconfined",
           "--memory", "12g", "--memory-swap", "14g", "--cpus", "8", "--pids-limit", "2048",
           "-e", "TZ=Europe/Amsterdam", "-e", "HSA_OVERRIDE_GFX_VERSION=12.0.1",
           "-e", f"KK_VRAM_FRACTION={frac}", "-e", "PYTORCH_CUDA_ALLOC_CONF=expandable_segments:False", "-e", "PYTORCH_HIP_ALLOC_CONF=expandable_segments:False",
           "-v", "kk-render-cache:/home/comfy/.cache"]
    family = {"trellis": TRELLIS_MOUNTS, "trellis-nodes": TRELLIS_MOUNTS + KK_NODES, "mv": MV_MOUNTS}.get(profile, SDXL_MOUNTS)
    if profile == "mv":
        cmd += MV_ENV
    for src, dst, mode in MOUNTS + family:
        if os.path.exists(src):
            cmd += ["-v", f"{src}:{dst}:{mode}"]
    cmd += [image, "python", "main.py", "--listen", "127.0.0.1", "--port", str(PORT), "--disable-auto-launch",
            "--output-directory", "/media-out", "--fast-disk", "--disable-pinned-memory", "--reserve-vram", reserve, *([vae] if vae else []), "--preview-method", "none", *extra]
    subprocess.run(cmd, check=True, capture_output=True, timeout=120)
    t0 = time.time()
    while time.time() - t0 < 180:
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{PORT}/system_stats", timeout=3)
            return
        except (urllib.error.URLError, OSError):
            if NAME not in running_containers():
                raise RuntimeError("container exited during start")
            time.sleep(3)
    raise RuntimeError("ComfyUI did not answer within 180 s")


def stop_container():
    subprocess.run(["docker", "stop", "-t", "5", NAME], capture_output=True, timeout=60)


def image_ok(path):
    """False for all-black / flat images (NaN output, the 13:45Z warning sign)."""
    try:
        from PIL import Image
    except ImportError:
        return True
    im = Image.open(path).convert("L").resize((64, 64))
    lo, hi = im.getextrema()
    return hi - lo > 8


def run_job(graph_path, timeout, wd):
    g = json.load(open(graph_path))
    g = g.get("prompt", g)
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/prompt", data=json.dumps({"prompt": g}).encode(),
                                 headers={"Content-Type": "application/json"})
    try:
        pid = json.load(urllib.request.urlopen(req, timeout=30))["prompt_id"]
    except urllib.error.HTTPError as e:
        return {"ok": False, "why": "refused by ComfyUI: " + e.read().decode()[:1500]}
    t0 = time.time()
    while True:
        if wd.tripped:
            return {"ok": False, "why": "watchdog: " + wd.tripped}
        if time.time() - t0 > timeout:
            stop_container()
            return {"ok": False, "why": f"timeout after {timeout} s"}
        try:
            h = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/history/{pid}", timeout=20)).get(pid, {})
        except (urllib.error.URLError, OSError) as e:
            return {"ok": False, "why": f"ComfyUI gone: {e}"}
        st = h.get("status", {}).get("status_str")
        if st in ("success", "error"):
            break
        time.sleep(1)
    secs = round(time.time() - t0)
    if st != "success":
        msgs = [m for m in h["status"].get("messages", []) if m[0] in ("execution_error", "execution_interrupted")]
        return {"ok": False, "secs": secs, "why": json.dumps([{k: m[1].get(k) for k in ("node_type", "exception_message")} for m in msgs])[:1500]}
    files = [os.path.join(WORK, "out", im.get("subfolder", ""), im["filename"])
             for node in h["outputs"].values() for items in node.values() if isinstance(items, list)
             for im in items if isinstance(im, dict) and "filename" in im and im.get("type", "output") == "output"]
    bad = [f for f in files if f.endswith(".png") and not image_ok(f)]
    if bad:
        return {"ok": False, "secs": secs, "files": files, "why": "black/flat output (sign of GPU trouble): " + ", ".join(bad)}
    return {"ok": True, "secs": secs, "files": files}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=("check", "run"))
    ap.add_argument("graphs", nargs="*")
    ap.add_argument("--timeout", type=int, default=300, help="per job, seconds")
    ap.add_argument("--profile", choices=sorted(PROFILES), default="sdxl")
    ap.add_argument("--comfy", action="append", default=[], choices=COMFY_FLAGS, help="extra ComfyUI flag (repeatable)")
    a = ap.parse_args()
    lockf = open(LOCK, "w")
    try:
        fcntl.flock(lockf, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        sys.exit("REFUSED: another kk-render job is running")
    studio_up = STUDIO in running_containers()
    why = preflight(gpu=not studio_up)
    if why:
        log("REFUSED", why=why)
        sys.exit(2)
    if a.cmd == "check":
        log("CLEAR", sample=sample(), studio=("will be paused for the job" if studio_up else "not running"))
        return
    paused = False
    try:
        paused = pause_studio()
    except Exception as e:
        log("REFUSED", why=[str(e)[:300]])
        sys.exit(2)
    if paused:
        why = preflight()
        if why:
            resume_studio()
            log("REFUSED", why=why)
            sys.exit(2)
    wd = Watchdog()
    wd.start()
    results, rc = [], 0
    try:
        start_container(a.profile, a.comfy)
        log("UP", sample=sample(), comfy=a.comfy)
        for gpath in a.graphs:
            r = run_job(gpath, a.timeout, wd)
            r["graph"] = gpath
            log("JOB", **r)
            results.append(r)
            if not r["ok"]:
                rc = 1
                log("STOPPED_BATCH", why="a job failed; not retrying, not running the rest")
                break
    except Exception as e:
        log("ERROR", why=str(e)[:500])
        rc = 1
    finally:
        stop_container()
        wd.stop()
        if paused:
            resume_studio()
        log("DONE", peak=wd.peak, after=sample(), tripped=wd.tripped)
    sys.exit(rc)


if __name__ == "__main__":
    main()
