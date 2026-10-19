# BANNER-HOST unit b1: docs/branding/blur.py blurs one box of a PNG in place and leaves the rest untouched
F=docs/branding/blur.py
test -e $F || { echo "error: $F: not written"; exit 1; }
T=$(mktemp -d)
python3 - "$T/a.png" <<'PY'
import sys
from PIL import Image, ImageDraw
im = Image.new("RGB", (200, 100), "black")
d = ImageDraw.Draw(im)
for x in range(0, 200, 4):
    d.line((x, 0, x, 99), fill="white")
im.save(sys.argv[1])
PY
cp $T/a.png $T/orig.png
timeout 60 python3 -I $F $T/a.png 50 20 150 80 2>&1 | sed "s|^|error: $F: |"
python3 - "$T/orig.png" "$T/a.png" "$F" <<'PY'
import sys
from PIL import Image, ImageChops
a, b, f = Image.open(sys.argv[1]).convert("RGB"), Image.open(sys.argv[2]).convert("RGB"), sys.argv[3]
if a.size != b.size:
    sys.exit("error: %s: the size changed" % f)
inside = ImageChops.difference(a.crop((60, 30, 140, 70)), b.crop((60, 30, 140, 70))).getbbox()
outside = [ImageChops.difference(a.crop(r), b.crop(r)).getbbox() for r in ((0, 0, 50, 100), (150, 0, 200, 100), (0, 0, 200, 20), (0, 80, 200, 100))]
if inside is None:
    sys.exit("error: %s: the box was not blurred" % f)
if any(o is not None for o in outside):
    sys.exit("error: %s: pixels outside the box changed" % f)
px = b.crop((60, 30, 140, 70)).convert("L").getextrema()
if px[1] - px[0] > 160:
    sys.exit("error: %s: the box is still sharp (contrast %d), use a stronger blur" % (f, px[1] - px[0]))
PY
r=$?; rm -rf $T; [ $r -eq 0 ] || exit 1
echo "ok: $F"
