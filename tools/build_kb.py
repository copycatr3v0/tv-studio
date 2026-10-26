#!/usr/bin/env python3
"""Build crates/tvs-kb/data/kb.json from a CleanYourTV checkout (MIT, (c) 2026 Gathin).

    python3 tools/build_kb.py <path to cleanyourtv checkout>

For every package named in examples/*/debloat.txt it records how many tested TVs disabled it and how
many kept it on purpose (a commented-out line, or a "Keep:" note), the section header it sat under
(mapped to a category), and a few of the contributors' notes. It also imports the privacy catalogue.
This is evidence for the UI and the recommender; it never overrides the hard safety rules.
Dev-only: end users never run this.
"""
import ast
import json
import re
import sys
from pathlib import Path

PKG = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)+)\b")
FULL_PKG = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$")
NOT_PKGS = re.compile(r"(\.(sh|ps1|py|txt|md|json|xml|apk|jpg|png)$)|^(tools|docs|examples|launcher)\.|^\d|^e\.g$|^i\.e$")

# Order matters: first match wins.
CATEGORIES = [
    ("keep", r"\bkeep\b|do not disable|don't disable|left enabled|kept"),
    ("launcher", r"stock home|launcher|home screen"),
    ("telemetry", r"telemetry|analytic|tracking|tracker|metrics|log upload|feedback|diagnos|report|\bacr\b|viewing"),
    ("ads_content", r"\bads?\b|promo|content|recommend|sponsor|patchwall|rows|waterfall|fast channel|advert"),
    ("retail_demo", r"retail|demo|hotel|store mode|shop mode"),
    ("factory", r"factory|test tool|service menu"),
    ("voice", r"voice|assistant|alexa|hotword"),
    ("casting", r"cast|miracast|screen ?shar|mirroring|dlna"),
    ("updater", r"update|ota|firmware"),
    ("streaming", r"streaming|video apps|netflix|youtube"),
    ("preinstalled", r"preload|pre-?install|bundled|games?|apps not used|not needed|optional|extras|background"),
]
CAT_RE = [(c, re.compile(p, re.I)) for c, p in CATEGORIES]


def category_of(header):
    for c, r in CAT_RE:
        if r.search(header or ""):
            return c
    return "other"


def is_pkg(tok):
    return bool(FULL_PKG.match(tok)) and not NOT_PKGS.search(tok) and tok.count(".") >= 1 and len(tok) <= 120


def parse_list(text):
    """Yield (package, action, header, note). action is 'disable' or 'keep'."""
    header = ""
    in_header_block = True  # the top-of-file comment block is metadata, not entries
    for raw in text.splitlines():
        line = raw.rstrip()
        s = line.strip()
        if not s:
            in_header_block = False
            continue
        if s.startswith("#"):
            body = s.lstrip("#").strip()
            first = body.split()[0] if body.split() else ""
            if is_pkg(first) and not in_header_block:
                note = body[len(first):].lstrip(" #").strip()
                yield first, "keep", header, note
                continue
            if re.match(r"(?i)keep\b|do not disable|kept", body):
                for tok in PKG.findall(body):
                    if is_pkg(tok) and "*" not in tok:
                        yield tok, "keep", body[:80], ""
                if body.endswith(":") or not PKG.search(body):
                    header = body
                continue
            if not in_header_block and body and not is_pkg(first):
                header = body
            continue
        in_header_block = False
        name, _, note = s.partition("#")
        name = name.strip().split()[0] if name.strip() else ""
        if is_pkg(name):
            yield name, "disable", header, note.strip()


def privacy_catalogue(path):
    src = path.read_text(encoding="utf-8")
    m = re.search(r"^CATALOGUE = (\{.*?^\})", src, re.S | re.M)
    if not m:
        return {}
    return ast.literal_eval(m.group(1))


def main(root):
    root = Path(root)
    kb = {}
    lists = sorted(root.glob("examples/*/debloat.txt"))
    for f in lists:
        device = f.parent.name
        seen = set()
        for pkg, action, header, note in parse_list(f.read_text(encoding="utf-8", errors="replace")):
            key = (pkg, action)
            if key in seen:
                continue
            seen.add(key)
            e = kb.setdefault(pkg, {"disabled_in": 0, "kept_in": 0, "categories": {}, "notes": [], "devices": []})
            if action == "disable":
                e["disabled_in"] += 1
                cat = category_of(header)
                if cat != "keep":
                    e["categories"][cat] = e["categories"].get(cat, 0) + 1
            else:
                e["kept_in"] += 1
            if note and len(e["notes"]) < 3 and note not in e["notes"]:
                e["notes"].append(note[:220])
            if len(e["devices"]) < 6 and device not in e["devices"]:
                e["devices"].append(device)
    for pkg, (cat, note) in privacy_catalogue(root / "tools" / "privacy.py").items():
        e = kb.setdefault(pkg, {"disabled_in": 0, "kept_in": 0, "categories": {}, "notes": [], "devices": []})
        e["privacy"] = {"category": cat, "note": note}
    out = {
        "source": "CleanYourTV examples/ and tools/privacy.py, MIT (c) 2026 Gathin",
        "lists": len(lists),
        "packages": {
            k: {
                "disabled_in": v["disabled_in"],
                "kept_in": v["kept_in"],
                "category": max(v["categories"], key=v["categories"].get) if v["categories"] else ("privacy" if "privacy" in v else "other"),
                "notes": v["notes"],
                "devices": v["devices"],
                **({"privacy": v["privacy"]} if "privacy" in v else {}),
            }
            for k, v in sorted(kb.items())
        },
    }
    dest = Path(__file__).resolve().parent.parent / "crates" / "tvs-kb" / "data" / "kb.json"
    dest.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    cats = {}
    for v in out["packages"].values():
        cats[v["category"]] = cats.get(v["category"], 0) + 1
    print(f"{len(lists)} lists, {len(out['packages'])} packages -> {dest} ({dest.stat().st_size // 1024} KB)")
    print(sorted(cats.items(), key=lambda x: -x[1]))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
