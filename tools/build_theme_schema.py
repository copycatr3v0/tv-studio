#!/usr/bin/env python3
"""Build schema/theme.v1.json from CleanYourTV's themes/README.md key table (MIT, (c) 2026 Gathin).

    python3 tools/build_theme_schema.py <path to cleanyourtv checkout>

One schema drives three things: the desktop editor's controls, the Rust validator that runs before a
theme is pushed, and the claim of which keys the bundled launcher supports (all keys documented for the
forked version). Dev-only.
"""
import json
import re
import sys
from pathlib import Path

LAUNCHER_VERSION = "3.0-studio.1"

ENUMS = {
    "glow_style": ["radial", "horizon"],
    "glass_style": ["glass", "rim", "halo"],
    "wallpaper_order": ["shuffle", "sequential"],
    "festival": ["", "dussehra", "diwali"],
}
# Keys that are config.xml-only or need the desktop app's own tooling, not a theme.
EXCLUDE = {"settings_fallback_components"}
# Data that the editor passes through unchanged (validated as JSON only).
PASSTHROUGH = {"race_calendar", "wallpaper_moods"}

GROUPS = [
    ("remote", r"keycodes|scancodes|app_keys|key_filter|gamepad|volume_keys|stock_home_redirect|power_on_home|boot_to_live_tv|open_on_start|home_long_press|handle_view_inputs|restore_key_services|settings_key"),
    ("inputs", r"live_tv|source_panel|coocaa|input_action|input_uri|input_sources|show_home_settings|module_cards"),
    ("motion", r"^animations$|reduced_motion|focus_overshoot|advanced_effects"),
    ("background", r"background_color|^wallpaper|sky_intensity|fade_rows|clock_wallpaper"),
    ("focus", r"glass_style|glow_style|app_glow|ambient_glow|cinematic|accent_color"),
    ("dock", r"^dock|fill_dock|single_row"),
    ("apps", r"columns|rows_below|hidden_apps|top_bar_apps|app_library|library|uniform_tiles|always_show_names|tile_menu|reorder|app_labels|app_banners|launch_actions"),
    ("header", r"greeting|date|shamsi|lunar|show_stats|network_pill|wifi|vpn|inputs_pill|icon_only|top_menu|settings_button|header_white|locale"),
    ("extras", r"festival|race_calendar|idle_clock|clock_glow"),
]
ADVANCED_GROUPS = {"remote", "inputs"}


def group_of(key):
    for g, r in GROUPS:
        if re.search(r, key):
            return g
    return "extras"


def label_of(key):
    words = key.replace("_", " ").strip()
    return words[:1].upper() + words[1:]


def parse_type(key, t, desc):
    t = t.strip().strip("`")
    if key in PASSTHROUGH:
        return {"type": "json"}
    if key in ENUMS:
        return {"type": "enum", "options": ENUMS[key]}
    if t == "bool":
        return {"type": "bool"}
    m = re.fullmatch(r"(-?\d+) to (\d+)", t)
    if m:
        return {"type": "int", "min": int(m.group(1)), "max": int(m.group(2))}
    if t == "#RRGGBB":
        return {"type": "color"}
    if t == "list of package names":
        return {"type": "packages"}
    if t == "list of keycodes":
        return {"type": "int_list"}
    if t == "YYYY-MM-DD":
        return {"type": "date"}
    if t.startswith("list"):
        return {"type": "string_list"}
    return {"type": "string"}


def parse_default(spec, desc):
    d = desc
    if spec["type"] == "bool":
        if re.search(r"`?true`? by default|\(`true`\)|true by default|on by default", d, re.I):
            return True
        if re.search(r"`?false`? by default|false\)|off by default|off unless|Off by default|\(`false`|false by default", d, re.I):
            return False
        return None
    if spec["type"] == "int":
        m = re.search(r"\(`?(-?\d+)`? by default\)|(-?\d+) by default|default (-?\d+)|0 by default", d)
        if m:
            return int(next(g for g in m.groups() if g is not None)) if any(m.groups()) else 0
        return None
    if spec["type"] == "color" and "black by default" in d:
        return "#000000"
    if spec["type"] == "enum":
        m = re.search(r"`(\w+)` \(default", d)
        return m.group(1) if m else spec["options"][0]
    return None


def config_defaults(launcher_dir):
    """Built-in values from the launcher's res/values/config.xml: the truth when a key is absent."""
    xml = (launcher_dir / "app" / "src" / "main" / "res" / "values" / "config.xml").read_text(encoding="utf-8")
    out = {}
    for kind, name, val in re.findall(r'<(bool|integer|string|color)\s+name="([a-z_]+)"[^>]*>([^<]*)<', xml):
        v = val.strip()
        out[name] = (v == "true") if kind == "bool" else int(v) if kind == "integer" and re.fullmatch(r"-?\d+", v) else v
    for kind, name in re.findall(r'<(string-array|integer-array)\s+name="([a-z_]+)"', xml):
        out.setdefault(name, [])
    return out


def main(root):
    built_in = config_defaults(Path(__file__).resolve().parent.parent / "launcher")
    text = (Path(root) / "themes" / "README.md").read_text(encoding="utf-8")
    keys = []
    for line in text.splitlines():
        if not re.match(r"^\| `[a-z_]+`", line):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 3:
            continue
        names = re.findall(r"`([a-z_]+)`", cells[0])
        desc = re.sub(r"\s+", " ", "|".join(cells[2:])).strip()
        for key in names:
            if key in EXCLUDE:
                continue
            spec = parse_type(key, cells[1], desc)
            g = group_of(key)
            default = built_in[key] if key in built_in else parse_default(spec, desc)
            if spec["type"] == "json" and default == "":
                default = None
            entry = {"key": key, **spec, "default": default, "group": g,
                     "advanced": g in ADVANCED_GROUPS, "label": label_of(key), "description": desc[:600],
                     "since": LAUNCHER_VERSION}
            keys.append(entry)
    out = {
        "$comment": "Generated by tools/build_theme_schema.py from CleanYourTV themes/README.md (MIT, (c) 2026 Gathin). Do not edit by hand.",
        "version": 1,
        "launcher_version": LAUNCHER_VERSION,
        "keys": keys,
    }
    dest = Path(__file__).resolve().parent.parent / "schema" / "theme.v1.json"
    dest.parent.mkdir(exist_ok=True)
    dest.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    by = {}
    for k in keys:
        by.setdefault(k["group"], 0)
        by[k["group"]] += 1
    print(f"{len(keys)} keys -> {dest}: {by}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
