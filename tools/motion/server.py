"""STU-M2: text to motion as a small CPU service (MoMask, MIT; weights trained on HumanML3D, a research dataset).

POST /motion {"prompt": str, "seconds": 0 (the model picks) or 1-9.8, "seed": int}
  -> {"fps": 20, "frames": N, "joints": [[[x, y, z] x 22] x N] (HumanML3D order, metres, y up), "bvh": "..."}
GET /health -> {"ok": true}
The models load once at start (about 15 s) and one request runs at a time (a few seconds on 6 cores).
Run inside the kk-motion image: python /srv/server.py --serve 0.0.0.0:8000
"""

import argparse
import json
import os
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

MOMASK = os.environ.get("MOMASK_DIR", "/momask")
FPS = 20
MAX_FRAMES = 196  # HumanML3D clips are at most 9.8 s
JOINT_NAMES = ["pelvis", "l_hip", "r_hip", "spine1", "l_knee", "r_knee", "spine2", "l_ankle", "r_ankle", "spine3", "l_foot",
               "r_foot", "neck", "l_collar", "r_collar", "head", "l_shoulder", "r_shoulder", "l_elbow", "r_elbow", "l_wrist",
               "r_wrist"]


def check_request(body):
    """Validate a request body; returns (prompt, frames or 0, seed) or raises ValueError with a message for the user."""
    if not isinstance(body, dict):
        raise ValueError("send a JSON object")
    prompt = body.get("prompt")
    if not isinstance(prompt, str) or not prompt.strip():
        raise ValueError("prompt must be a non-empty string")
    if len(prompt) > 300:
        raise ValueError("prompt is at most 300 characters")
    seconds = body.get("seconds", 0)
    if not isinstance(seconds, (int, float)) or isinstance(seconds, bool) or not (seconds == 0 or 1 <= seconds <= MAX_FRAMES / FPS):
        raise ValueError("seconds is 0 (let the model choose) or 1 to 9.8")
    seed = body.get("seed", 1)
    if not isinstance(seed, int) or isinstance(seed, bool) or not (0 <= seed < 2**31):
        raise ValueError("seed is a whole number from 0")
    frames = 0 if seconds == 0 else max(4, min(MAX_FRAMES, int(round(seconds * FPS)) // 4 * 4))
    return prompt.strip(), frames, seed


class Generator:
    """MoMask's gen_t2m.py setup, loaded once (paths and options as in the repo's README)."""

    def __init__(self):
        sys.path.insert(0, MOMASK)
        os.chdir(MOMASK)
        sys.argv = ["gen_t2m.py", "--gpu_id", "-1", "--dataset_name", "t2m", "--name",
                    "t2m_nlayer8_nhead6_ld384_ff1024_cdp0.1_rvq6ns", "--res_name",
                    "tres_nlayer8_ld384_ff1024_rvq6ns_cdp0.2_sw", "--cond_scale", "4", "--time_steps", "10"]
        import numpy as np
        import torch
        import gen_t2m as g
        from options.eval_option import EvalT2MOptions
        from utils.get_opt import get_opt
        from os.path import join as pjoin

        self.np, self.torch, self.g = np, torch, g
        opt = EvalT2MOptions().parse()
        opt.device = torch.device("cpu")
        root = pjoin(opt.checkpoints_dir, opt.dataset_name, opt.name)
        model_opt = get_opt(pjoin(root, "opt.txt"), device=opt.device)
        vq_opt = get_opt(pjoin(opt.checkpoints_dir, opt.dataset_name, model_opt.vq_name, "opt.txt"), device=opt.device)
        vq_opt.dim_pose = 263
        self.vq, vq_opt = g.load_vq_model(vq_opt)
        model_opt.num_tokens, model_opt.num_quantizers, model_opt.code_dim = vq_opt.nb_code, vq_opt.num_quantizers, vq_opt.code_dim
        res_opt = get_opt(pjoin(opt.checkpoints_dir, opt.dataset_name, opt.res_name, "opt.txt"), device=opt.device)
        g.opt = opt  # the loaders read the module-level opt
        self.res = g.load_res_model(res_opt, vq_opt, opt)
        self.t2m = g.load_trans_model(model_opt, opt, "latest.tar")
        self.length = g.load_len_estimator(model_opt)
        for m in (self.vq, self.res, self.t2m, self.length):
            m.eval()
        meta = pjoin(opt.checkpoints_dir, opt.dataset_name, model_opt.vq_name, "meta")
        self.mean, self.std = np.load(pjoin(meta, "mean.npy")), np.load(pjoin(meta, "std.npy"))
        self.opt = opt
        self.lock = threading.Lock()

    def __call__(self, prompt, frames, seed):
        np, torch, g = self.np, self.torch, self.g
        from utils.fixseed import fixseed
        from utils.motion_process import recover_from_ric
        from visualization.joints2bvh import Joint2BVHConvertor
        import torch.nn.functional as F
        from torch.distributions.categorical import Categorical

        with self.lock, torch.no_grad():
            fixseed(seed)
            if frames == 0:
                probs = F.softmax(self.length(self.t2m.encode_text([prompt])), dim=-1)
                tokens = Categorical(probs).sample()
            else:
                tokens = torch.LongTensor([frames // 4])
            o = self.opt
            ids = self.t2m.generate([prompt], tokens, timesteps=o.time_steps, cond_scale=o.cond_scale,
                                    temperature=o.temperature, topk_filter_thres=o.topkr, gsample=o.gumbel_sample)
            ids = self.res.generate(ids, [prompt], tokens, temperature=1, cond_scale=5)
            data = self.vq.forward_decoder(ids).detach().cpu().numpy() * self.std + self.mean
            n = int(tokens[0]) * 4
            joint = recover_from_ric(torch.from_numpy(data[0][:n]).float(), 22).numpy()
            with tempfile.TemporaryDirectory() as d:
                path = os.path.join(d, "m.bvh")
                _, ik_joint = Joint2BVHConvertor().convert(joint, filename=path, iterations=100)
                bvh = open(path, encoding="utf-8").read()
        joints = [[[round(float(v), 4) for v in p] for p in fr] for fr in ik_joint]
        return {"fps": FPS, "frames": len(joints), "joint_names": JOINT_NAMES, "joints": joints, "bvh": bvh}


class Handler(BaseHTTPRequestHandler):
    generate = None

    def log_message(self, *a):
        pass

    def send_json(self, code, data):
        body = json.dumps(data).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self.send_json(200, {"ok": True}) if self.path == "/health" else self.send_json(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/motion":
            self.send_json(404, {"error": "not found"})
            return
        try:
            n = int(self.headers.get("Content-Length", 0))
            if n == 0 or n > 10000:
                raise ValueError("send a small JSON body")
            prompt, frames, seed = check_request(json.loads(self.rfile.read(n)))
        except (ValueError, json.JSONDecodeError) as e:
            self.send_json(400, {"error": str(e)})
            return
        try:
            self.send_json(200, Handler.generate(prompt, frames, seed))
        except (RuntimeError, OSError) as e:
            self.send_json(500, {"error": str(e)})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--serve", default="0.0.0.0:8000")
    a = ap.parse_args()
    Handler.generate = Generator()
    host, port = a.serve.rsplit(":", 1)
    print(f"Serving on {a.serve}", flush=True)
    HTTPServer((host, int(port)), Handler).serve_forever()


if __name__ == "__main__":
    main()
