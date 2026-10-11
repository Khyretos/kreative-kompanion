"""GPU guard for ComfyUI on soucouyant. Monitors VRAM, junction and memory temperatures.
Only uses ComfyUI's /queue clear and /interrupt endpoints to cancel work when limits are exceeded.
Standard library only: urllib.request, json, argparse, time, os, dataclasses."""

import argparse
import dataclasses
import json
import os
import time
import urllib.request

MIB = 1024 * 1024


@dataclasses.dataclass
class Limits:
    min_free_mib: int = 1024
    low_secs: float = 6
    junction_c: int = 100
    mem_c: int = 100
    hot_secs: float = 4
    cooldown_secs: float = 30


def read_sample(device_dir: str) -> dict:
    """Read VRAM usage and temperatures from sysfs under device_dir."""
    total_path = os.path.join(device_dir, "mem_info_vram_total")
    used_path = os.path.join(device_dir, "mem_info_vram_used")

    with open(total_path) as f:
        total_bytes = int(f.read().strip())
    with open(used_path) as f:
        used_bytes = int(f.read().strip())

    total_mib = total_bytes // MIB
    used_mib = used_bytes // MIB
    free_mib = (total_bytes - used_bytes) // MIB

    hwmon_dirs = [
        d
        for d in os.listdir(os.path.join(device_dir, "hwmon"))
        if os.path.isdir(os.path.join(device_dir, "hwmon", d))
    ]
    if not hwmon_dirs:
        raise RuntimeError("No hwmon directory found")

    # Find the single hwmon dir by matching tempN_label files
    hwmon_path = None
    for d in hwmon_dirs:
        label_file = os.path.join(device_dir, "hwmon", d, "temp1_label")
        input_file = os.path.join(device_dir, "hwmon", d, "temp1_input")
        if os.path.exists(label_file) and os.path.exists(input_file):
            hwmon_path = os.path.join(device_dir, "hwmon", d)
            break

    if hwmon_path is None:
        raise RuntimeError("No valid hwmon directory with temp sensors found")

    temps = {}
    for fname in os.listdir(hwmon_path):
        if fname.startswith("temp") and fname.endswith("_label"):
            label_file = os.path.join(hwmon_path, fname)
            input_file = label_file.replace("_label", "_input")
            if not os.path.exists(input_file):
                continue
            with open(label_file) as f:
                label = f.read().strip()
            with open(input_file) as f:
                value = int(f.read().strip()) // 1000
            temps[label] = value

    edge_c = temps.get("edge", 0)
    junction_c = temps.get("junction", 0)
    mem_c = temps.get("mem", 0)

    power_path = os.path.join(hwmon_path, "power1_average")
    power_w = None
    if os.path.exists(power_path):
        with open(power_path) as f:
            power_w = int(f.read().strip()) // 1000000

    return {
        "total_mib": total_mib,
        "used_mib": used_mib,
        "free_mib": free_mib,
        "edge_c": edge_c,
        "junction_c": junction_c,
        "mem_c": mem_c,
        "power_w": power_w,
    }


def is_busy(queue) -> bool:
    """Check if ComfyUI queue has running jobs."""
    if not isinstance(queue, dict):
        return False
    return len(queue.get("queue_running", [])) > 0


def _post(url: str, body: str):
    req = urllib.request.Request(
        url, data=body.encode(), headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=5):
        pass


def cancel(base_url: str, post=None):
    """Clear ComfyUI's pending queue, then interrupt the running prompt. Each call is tried on its own."""
    post = post or _post
    base = base_url.rstrip("/")
    for url, body in (
        (f"{base}/queue", json.dumps({"clear": True})),
        (f"{base}/interrupt", "{}"),
    ):
        try:
            post(url, body)
        except Exception:
            pass


class Guard:
    """Decides when to cancel: a limit has to hold for its whole window while ComfyUI is busy."""

    def __init__(self, limits: Limits):
        self.limits = limits
        self.since = {}  # check name -> time the limit was first crossed
        self.quiet_until = None

    def _held(self, name: str, crossed: bool, now: float, secs: float):
        if not crossed:
            self.since.pop(name, None)
            return None
        start = self.since.setdefault(name, now)
        return now - start if now - start >= secs else None

    def step(self, sample: dict, now: float, busy: bool):
        if not busy:
            self.since.clear()
            return None
        lim = self.limits
        checks = (
            (
                "vram",
                sample["free_mib"] < lim.min_free_mib,
                lim.low_secs,
                lambda t: f"VRAM free {sample['free_mib']} MiB < {lim.min_free_mib} MiB for {t:.0f} s",
            ),
            (
                "junction",
                sample["junction_c"] >= lim.junction_c,
                lim.hot_secs,
                lambda t: f"junction {sample['junction_c']} C >= {lim.junction_c} C for {t:.0f} s",
            ),
            (
                "mem",
                sample["mem_c"] >= lim.mem_c,
                lim.hot_secs,
                lambda t: f"mem {sample['mem_c']} C >= {lim.mem_c} C for {t:.0f} s",
            ),
        )
        reason = None
        for name, crossed, secs, say in checks:
            held = self._held(name, crossed, now, secs)
            if held is not None and reason is None:
                reason = say(held)
        if self.quiet_until is not None and now < self.quiet_until:
            return None
        if reason:
            self.since.clear()
            self.quiet_until = now + lim.cooldown_secs
        return reason


def main():
    parser = argparse.ArgumentParser(description="GPU guard for ComfyUI")
    parser.add_argument("--url", default="http://127.0.0.1:8188", help="ComfyUI URL")
    parser.add_argument(
        "--device", default="/sys/class/drm/card1/device", help="Device dir"
    )
    parser.add_argument("--interval", type=float, default=2.0, help="Polling interval")
    parser.add_argument("--min-free-mib", type=int, default=1024, help="Min free VRAM")
    parser.add_argument(
        "--low-secs", type=float, default=6, help="Low VRAM threshold seconds"
    )
    parser.add_argument(
        "--junction-c", type=int, default=100, help="Junction temp limit"
    )
    parser.add_argument("--mem-c", type=int, default=100, help="Memory temp limit")
    parser.add_argument("--once", action="store_true", help="Print one sample and exit")

    args = parser.parse_args()

    limits = Limits(
        min_free_mib=args.min_free_mib,
        low_secs=args.low_secs,
        junction_c=args.junction_c,
        mem_c=args.mem_c,
    )

    guard = Guard(limits)

    if args.once:
        try:
            sample = read_sample(args.device)
            print(json.dumps(sample))
        except Exception as e:
            print(json.dumps({"error": str(e)}))
        return

    base_url = args.url
    beat = 0.0
    print(json.dumps({"event": "start", **dataclasses.asdict(limits)}), flush=True)
    while True:
        try:
            sample = read_sample(args.device)
        except Exception as e:
            print(json.dumps({"event": "error", "reason": str(e)}), flush=True)
            time.sleep(args.interval)
            continue

        try:
            req = urllib.request.Request(f"{base_url}/queue")
            with urllib.request.urlopen(req, timeout=5) as resp:
                queue = json.loads(resp.read().decode())
        except Exception:
            queue = None

        reason = guard.step(sample, time.monotonic(), is_busy(queue))

        if reason:
            line = {"event": "cancel", "reason": reason, **sample}
            print(json.dumps(line), flush=True)
            cancel(base_url)
            time.sleep(args.interval)
            continue

        if time.monotonic() - beat >= 60:
            beat = time.monotonic()
            print(
                json.dumps({"event": "heartbeat", "busy": is_busy(queue), **sample}),
                flush=True,
            )

        time.sleep(args.interval)


if __name__ == "__main__":
    main()
