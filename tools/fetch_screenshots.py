"""Downloads KaTeX's Chrome screenshots for the cases in cases.json.

Usage: python3 -I tools/fetch_screenshots.py cases.json out-dir [commit]

The default commit matches the KaTeX test data the scores were taken with.
"""
import json
import sys
import urllib.request
from pathlib import Path

cases_path, out = Path(sys.argv[1]), Path(sys.argv[2])
commit = sys.argv[3] if len(sys.argv) > 3 else "8000fbd4e29cca2c0cb1e88b745af3ed2f436b66"
out.mkdir(parents=True, exist_ok=True)
base = f"https://raw.githubusercontent.com/KaTeX/KaTeX/{commit}/test/screenshotter/images"
for case in json.loads(cases_path.read_text()):
    target = out / f"{case['name']}-chrome.png"
    if not target.exists():
        with urllib.request.urlopen(f"{base}/{case['name']}-chrome.png") as response:
            target.write_bytes(response.read())
print(f"Screenshots in {out}")
