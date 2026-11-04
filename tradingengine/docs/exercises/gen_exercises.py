#!/usr/bin/env python3
# Regenerates exercises/ from solutions/: every block between a `//>>` line and
# a `//<<` line becomes `todo!()`. Edit the solution, then run this.
import pathlib, re

root = pathlib.Path(__file__).parent
for sol in sorted((root / "solutions").glob("ex*.rs")):
    out, skipping = [], False
    for line in sol.read_text().splitlines():
        stripped = line.strip()
        if stripped == "//>>":
            indent = line[: len(line) - len(line.lstrip())]
            out.append(f"{indent}todo!()")
            skipping = True
        elif stripped == "//<<":
            skipping = False
        elif not skipping:
            out.append(line)
    text = "\n".join(out) + "\n"
    text = text.replace("#![allow(unused)]\n", "#![allow(unused)]\n// TODO: replace every todo!() until `./check.sh` passes this file. Hints: HINTS.md\n", 1)
    (root / "exercises" / sol.name).write_text(text)
    print("wrote exercises/" + sol.name)
