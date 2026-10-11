#!/usr/bin/env python3
"""Video metrics: colour drift, flicker and detail loss."""

import argparse
import json
import math
import subprocess
import sys


def parse_metadata(text):
    """Parse ffmpeg metadata output into a list of dicts."""
    if not text:
        return []
    frames = []
    current = {}
    for line in text.splitlines():
        if line.startswith("frame:"):
            if current:
                frames.append(current)
            current = {}
        elif "lavfi.signalstats." in line and "=" in line:
            parts = line.split("lavfi.signalstats.", 1)
            if len(parts) == 2:
                key, val = parts[1].split("=", 1)
                try:
                    current[key] = float(val)
                except ValueError:
                    pass
    if current:
        frames.append(current)
    return frames


def stats(path, vf):
    """Run ffmpeg to extract signalstats or edgedetect+signalstats."""
    cmd = [
        "ffmpeg",
        "-v",
        "error",
        "-i",
        path,
        "-vf",
        f"{vf},metadata=print:file=-",
        "-f",
        "null",
        "-",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        err = result.stderr.strip() or "ffmpeg failed"
        raise RuntimeError(err)
    return parse_metadata(result.stdout)


def summarize(frames, edges):
    """Summarize frames into drift, cast, detail and flicker."""
    n = len(frames)
    w = min(8, max(1, n // 4))

    def mean(xs):
        return sum(xs) / len(xs) if xs else 0.0

    def ht(xs):
        h = mean(xs[:w])
        t = mean(xs[-w:])
        return h, t

    def pct(xs):
        h, t = ht(xs)
        return 0.0 if h == 0 else (t - h) / h * 100.0

    ys = [f["YAVG"] for f in frames]
    sats = [f["SATAVG"] for f in frames]
    con = [f["YHIGH"] - f["YLOW"] for f in frames]
    us = [f["UAVG"] for f in frames]
    vs = [f["VAVG"] for f in frames]

    drift_y = pct(ys)
    drift_sat = pct(sats)
    drift_con = pct(con)

    head_u, tail_u = ht(us)
    head_v, tail_v = ht(vs)
    cast = math.hypot(tail_u - head_u, tail_v - head_v)

    detail = pct(edges)

    flicker = 0.0
    if n >= 3:
        diffs = []
        for i in range(1, n - 1):
            expected = (ys[i - 1] + ys[i + 1]) / 2
            diffs.append(abs(ys[i] - expected))
        flicker = mean(diffs)

    return {
        "frames": n,
        "drift_y": round(drift_y, 3),
        "drift_sat": round(drift_sat, 3),
        "drift_con": round(drift_con, 3),
        "cast": round(cast, 3),
        "detail": round(detail, 3),
        "flicker": round(flicker, 3),
    }


def measure(path):
    """Measure a video file."""
    frames = stats(path, "signalstats")
    if not frames:
        raise RuntimeError("no frames")
    edges = stats(path, "edgedetect,signalstats")
    edges_vals = [e["YAVG"] for e in edges]
    return summarize(frames, edges_vals)


def main():
    parser = argparse.ArgumentParser(description="Measure video metrics")
    parser.add_argument("--json", action="store_true", help="Output JSON per file")
    parser.add_argument("files", nargs="+", help="Input video files")
    args = parser.parse_args()

    failures = []
    for path in args.files:
        try:
            result = measure(path)
        except FileNotFoundError as e:
            failures.append((path, str(e)))
            continue
        except RuntimeError as e:
            failures.append((path, str(e)))
            continue

        if args.json:
            print(json.dumps({"file": path, **result}))
        else:
            r = result
            print(
                f"{path} "
                f"frames={r['frames']} "
                f"y={r['drift_y']:+.1f}% "
                f"sat={r['drift_sat']:+.1f}% "
                f"con={r['drift_con']:+.1f}% "
                f"cast={r['cast']:.1f} "
                f"detail={r['detail']:+.1f}% "
                f"flicker={r['flicker']:.2f}"
            )

    if failures:
        for path, msg in failures:
            print(f"error: {path}: {msg}", file=sys.stderr)
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
