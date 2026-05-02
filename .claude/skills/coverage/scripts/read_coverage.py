#!/usr/bin/env python3

import json
import subprocess
import sys
from pathlib import Path

cov_file = "coverage.json"
if len(sys.argv) > 1:
    cov_file = sys.argv[1]

with open(cov_file) as f:
    data = json.load(f)

d = data["data"][0]

proj = Path(__file__)
while not (proj / "Cargo.toml").exists():
    print(proj)
    if proj.name == "/" or proj.name == "":
        raise ValueError("Didn't find src/ parent")
    proj = proj.parent

proj = f"{proj}/src/"
# proj = "/var/mnt/mercury/projects/exergon/src/"

# ── Totals ────────────────────────────────────────────────────────────────────
t = d["totals"]
print("=== TOTALS ===")
print(
    f"  Lines:     {t['lines']['covered']}/{t['lines']['count']}  ({t['lines']['percent']:.1f}%)"
)
print(
    f"  Functions: {t['functions']['covered']}/{t['functions']['count']}  ({t['functions']['percent']:.1f}%)"
)
print()

# ── Per-file table ─────────────────────────────────────────────────────────────
files = []
for f in d["files"]:
    if not f["filename"].startswith(proj):
        continue
    name = f["filename"].replace(proj, "")
    s = f["summary"]
    files.append(
        (
            s["lines"]["percent"],
            s["lines"]["covered"],
            s["lines"]["count"],
            s["functions"]["covered"],
            s["functions"]["count"],
            name,
        )
    )

files.sort()
print("=== PER FILE (sorted by line coverage) ===")
for pct, lc, lt, fc, ft, name in files:
    bar = "█" * int(pct / 5) + "░" * (20 - int(pct / 5))
    print(f"  {pct:5.1f}%  {bar}  {lc:3}/{lt:<3} lines  {fc}/{ft} fn  {name}")
print()


# ── Uncovered project functions (demangled where possible) ────────────────────
def demangle_batch(syms):
    try:
        result = subprocess.run(
            ["rustfilt"],
            input="\n".join(syms),
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.splitlines()
    except (FileNotFoundError, subprocess.CalledProcessError):
        return syms


uncov = []
for fn in d["functions"]:
    fnames = fn.get("filenames", [])
    if not fnames or not fnames[0].startswith(proj):
        continue
    if fn["count"] == 0:
        rel = fnames[0].replace(proj, "")
        uncov.append((rel, fn["name"]))

uncov.sort()
demangled = demangle_batch([sym for _, sym in uncov])
print(f"=== UNCOVERED PROJECT FUNCTIONS ({len(uncov)}) ===")
for (fpath, _), sym in zip(uncov, demangled):
    print(f"  {fpath}: {sym}")
