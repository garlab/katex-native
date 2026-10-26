"""Compares native renders with KaTeX's reference screenshots.

Usage: python3 -I tools/compare.py cases.json shots-dir ours-dir out-dir

`shots-dir` holds KaTeX's `<name>-chrome.png` screenshots (see
tools/fetch_screenshots.py) and `ours-dir` the PNGs from `examples/corpus.rs`.

Both images are cropped to their ink (pixels darker than mid-gray). The score
is the smaller of: the share of KaTeX's ink within 2 px of ours, and the
share of ours within 2 px of KaTeX's, at the best offset within ±3 px.
`out-dir` receives a sheet per case, KaTeX above and native below, and
scores.json. Needs Pillow.
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageFilter

cases_path, shots, ours_dir, out = map(Path, sys.argv[1:5])
out.mkdir(parents=True, exist_ok=True)
# Their screenshots include text or styles around the math, or test errors.
CONTEXT = {"DisplayMode", "NegativeSpace", "SizingBaseline", "VerticalSpacing", "SvgReset", "UnsupportedCmds"}


def ink(path):
    gray = Image.open(path).convert("L")
    mask = gray.point(lambda p: 255 if p < 128 else 0).convert("1")
    box = mask.getbbox()
    return (mask.crop(box), gray.crop(box)) if box else (None, None)


def count(mask):
    return mask.histogram()[255]


def score(ref, ours):
    ref_near = ref.filter(ImageFilter.MaxFilter(5))
    w, h = max(ref.width, ours.width) + 8, max(ref.height, ours.height) + 8
    best = (0.0, 0, 0)
    base_ref = Image.new("1", (w, h))
    base_ref.paste(ref, (4, 4))
    base_near = Image.new("1", (w, h))
    base_near.paste(ref_near, (4, 4))
    ref_total = count(base_ref)
    for dx in range(-3, 4):
        for dy in range(-3, 4):
            placed = Image.new("1", (w, h))
            placed.paste(ours, (4 + dx, 4 + dy))
            placed_near = placed.filter(ImageFilter.MaxFilter(5))
            recall = count(ImageChops.logical_and(base_ref, placed_near)) / max(ref_total, 1)
            precision = count(ImageChops.logical_and(placed, base_near)) / max(count(placed), 1)
            value = min(recall, precision)
            if value > best[0]:
                best = (value, dx, dy)
    return best


cases = json.loads(cases_path.read_text())
rows = []
for case in cases:
    name = case["name"]
    stem = "".join(c for c in name if c.isalnum() or c in "-_")
    ours_path = ours_dir / f"{stem}.png"
    if not ours_path.exists():
        rows.append({"name": name, "score": None, "note": "no native render"})
        continue
    ref, ref_gray = ink(shots / f"{name}-chrome.png")
    mine, mine_gray = ink(ours_path)
    if ref is None or mine is None:
        rows.append({"name": name, "score": None, "note": "no ink"})
        continue
    value, dx, dy = score(ref, mine)
    rows.append({
        "name": name,
        "score": round(value, 4),
        "size_ratio": [round(mine.width / ref.width, 3), round(mine.height / ref.height, 3)],
        "context": name in CONTEXT,
    })
    sheet = Image.new("L", (max(ref.width, mine.width) + 20, ref.height + mine.height + 50), 255)
    sheet.paste(ref_gray, (10, 10))
    sheet.paste(Image.new("L", (sheet.width, 4), 200), (0, ref.height + 22))
    sheet.paste(mine_gray, (10, ref.height + 38))
    sheet.save(out / f"{stem}.png")

scored = [r for r in rows if r["score"] is not None and not r["context"]]
scored.sort(key=lambda r: r["score"])
for r in scored:
    print(f"{r['score']:.3f}  {r['size_ratio']}  {r['name']}")
for r in rows:
    if r["score"] is None or r.get("context"):
        print("--", r["name"], r.get("note", f"context case, score {r['score']}"))
values = [r["score"] for r in scored]
for threshold in (0.99, 0.97, 0.95, 0.9):
    print(f">= {threshold}: {sum(v >= threshold for v in values)}/{len(values)}")
(out / "scores.json").write_text(json.dumps(rows, indent=2))
