#!/bin/sh
# STU-A1: quality numbers per audio file (choppy, quiet or clipping sound).
# Usage: tools/audio-metrics.sh FILE... | DIR   (a directory: its wav/ogg/mp3/flac files)
# Columns: duration s, peak and RMS dBFS, integrated loudness, astats flat factor (> 0 means
# clipped runs), click = largest sample-to-sample jump in dBFS, gaps = silences (< -50 dB,
# > 30 ms) inside the clip (leading and trailing silence excluded). PASS: no gaps and a peak
# between -6 and -1 dB (the STU-A1 target).
set -eu
[ $# -gt 0 ] || {
    echo "usage: $0 FILE... | DIR" >&2
    exit 2
}
if [ $# -eq 1 ] && [ -d "$1" ]; then
    d=$1
    shift
    for f in "$d"/*.wav "$d"/*.ogg "$d"/*.mp3 "$d"/*.flac; do
        [ -e "$f" ] && set -- "$@" "$f"
    done
fi
printf 'file\tdur\tpeak\trms\tlufs\tflat\tclick\tgaps\tverdict\n'
for f in "$@"; do
    dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")
    ffmpeg -hide_banner -nostats -i "$f" \
        -af "aformat=sample_fmts=s16,astats=metadata=0,silencedetect=n=-50dB:d=0.03,loudnorm=print_format=summary" \
        -f null - 2>&1 | awk -v f="${f##*/}" -v d="$dur" '
        /Overall/ { o = 1 }
        o && /Peak level dB:/ { p = $NF }
        o && /RMS level dB:/ { r = $NF }
        o && /Flat factor:/ { fl = $NF }
        o && /Max difference:/ { md = $NF }
        /Input Integrated:/ { l = $(NF - 1) }
        /silence_start:/ { s = $NF; open = 1 }
        /silence_end:/ && open { e = $5; open = 0; if (s > 0.05 && e < d - 0.05) g++ }
        END {
            c = md > 0 ? sprintf("%.1f", 20 * log(md / 32768) / log(10)) : "-inf"
            v = (g + 0 == 0 && p >= -6 && p <= -1) ? "PASS" : "FAIL"
            printf "%s\tdur=%.2f\tpeak=%.1f\trms=%.1f\tlufs=%s\tflat=%.1f\tclick=%s\tgaps=%d\t%s\n", f, d, p, r, l, fl, c, g, v
        }'
done
