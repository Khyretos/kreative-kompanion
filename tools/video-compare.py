#!/usr/bin/env python3
"""Compare video clips side by side with labels and optional still sheet."""

import argparse
import os
import subprocess
import sys


def clean_label(text):
    """Keep only letters, digits, space, `.`, `_` and `-`, collapse spaces."""
    s = "".join(c for c in text if c.isalnum() or c in " ._-")
    return " ".join(s.split())


def parse_item(arg):
    """Parse item into (path, label). Split on last '=' if present."""
    if "=" in arg:
        path, label = arg.rsplit("=", 1)
    else:
        path = arg
        name = os.path.basename(path)
        _, ext = os.path.splitext(name)
        label = name[: -len(ext)]
    return path, label


def filter_graph(labels, w, h):
    """Build ffmpeg filter graph for side-by-side videos with labels."""
    lab = ":x=8:y=8:fontsize=18:fontcolor=white:box=1:boxcolor=black@0.6"
    parts = [
        f"[{i}:v]scale={w}:{h},setsar=1,drawtext=text='{clean_label(l)}'{lab}[v{i}]"
        for i, l in enumerate(labels)
    ]
    return (
        ";".join(parts)
        + ";"
        + "".join(f"[v{i}]" for i in range(len(labels)))
        + f"hstack=inputs={len(labels)}"
    )


def build_cmd(files, labels, out, w, h):
    """Build the ffmpeg command list for comparing videos."""
    cmd = ["ffmpeg", "-v", "error", "-y"]
    for f in files:
        cmd.extend(["-i", f])
    cmd.extend(
        [
            "-filter_complex",
            filter_graph(labels, w, h),
            "-c:v",
            "libx264",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            out,
        ]
    )
    return cmd


def frame_count(path):
    """Return number of frames in a video file."""
    result = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-count_packets",
            "-show_entries",
            "stream=nb_read_packets",
            "-of",
            "csv=p=0",
            path,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return int(result.stdout.strip())


def sheet_cmd(video, out, n):
    """Build ffmpeg command to create a still sheet of first, middle, last frames."""
    m = n // 2
    last = n - 1
    select = f"select='eq(n\\,0)+eq(n\\,{m})+eq(n\\,{last})'"
    return [
        "ffmpeg",
        "-v",
        "error",
        "-y",
        "-i",
        video,
        "-vf",
        f"{select},tile=1x3",
        "-frames:v",
        "1",
        out,
    ]


def main():
    parser = argparse.ArgumentParser(description="Compare video clips side by side.")
    parser.add_argument("out", help="Output video file")
    parser.add_argument("items", nargs="+", help="Video items (path[=label])")
    parser.add_argument("--size", default="416x240", help="Output size (widthxheight)")
    parser.add_argument("--sheet", default=None, help="Output PNG sheet file")

    args = parser.parse_args()

    if len(args.items) < 2:
        print("error: give at least two videos", file=sys.stderr)
        sys.exit(1)

    width, height = map(int, args.size.split("x"))

    parsed = []
    for item in args.items:
        path, label = parse_item(item)
        if not os.path.exists(path):
            print(f"error: {path}: not found", file=sys.stderr)
            sys.exit(1)
        parsed.append((path, label))

    files = [p[0] for p in parsed]
    labels = [p[1] for p in parsed]

    cmd = build_cmd(files, labels, args.out, width, height)
    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        print(f"error: ffmpeg: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(1)

    print(args.out)

    if args.sheet:
        n = frame_count(args.out)
        sheet_cmd_list = sheet_cmd(args.out, args.sheet, n)
        result = subprocess.run(
            sheet_cmd_list, capture_output=True, text=True, check=False
        )
        if result.returncode != 0:
            print(f"error: ffmpeg: {result.stderr.strip()}", file=sys.stderr)
            sys.exit(1)
        print(args.sheet)


if __name__ == "__main__":
    main()
