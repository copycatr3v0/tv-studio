//! Validator for the free-canvas home layout (`layout.json`). Mirrors `src/theme/layout.ts`; the app
//! validates in the browser for feedback and here again before anything is sent to a TV.
//! Coordinates are percentages of the screen.

use serde::Serialize;
use serde_json::Value;

const KINDS: &[&str] = &["apps", "recent", "watchnext", "clock", "date", "greeting", "text", "cpu", "memory", "network", "calendar", "visualizer", "shape", "bluetooth", "nowplaying", "weather"];

/// The looks (`variant`) each kind accepts; kinds not listed take none.
fn variants(kind: &str) -> &'static [&'static str] {
    match kind {
        "weather" => &["compact", "full", "forecast"],
        "clock" => &["digital", "stacked", "analog"],
        "date" => &["weekday_date", "weekday", "full", "short", "daymonth"],
        "cpu" | "memory" | "network" => &["text", "bar", "ring", "graph"],
        "visualizer" => &["bars", "mirror", "dots"],
        "shape" => &["rect", "circle", "line", "ring"],
        _ => &[],
    }
}
const FONTS: &[&str] = &["sans-serif", "sans-serif-light", "sans-serif-thin", "sans-serif-condensed", "sans-serif-medium", "sans-serif-black", "serif", "monospace", "cursive", "casual"];
const MOTIONS: &[&str] = &["none", "float", "pulse", "slide", "fade", "drift", "parallax"];
const SHAPES: &[&str] = &["rounded", "square", "circle"];
const ALIGN: &[&str] = &["start", "center", "end"];
const BG: &[&str] = &["image", "video", "color", "gradient", "scene"];
/// Ambient scenes the launcher draws, and the parts of the day they can be fixed to.
pub const SCENES: &[&str] = &["rain", "snow", "aurora", "fireflies", "space", "sakura", "embers", "underwater"];
pub const SCENE_TIMES: &[&str] = &["auto", "sunrise", "day", "sunset", "night"];
/// Tile looks for scroll mode: focus style, app-open animation, tile style and icon shape.
pub const FOCUS: &[&str] = &["pop", "lift", "glow", "border", "bounce", "tilt", "zoom", "parallax"];
pub const LAUNCH: &[&str] = &["default", "zoom", "reveal", "fade", "slide", "grow", "none"];
pub const TILES: &[&str] = &["banner", "card", "icon", "poster", "tvos"];
pub const ICON_SHAPES: &[&str] = &["rounded", "circle", "squircle", "square", "leaf"];
pub const MAX_ITEMS: usize = 40;

/// A font file the person added: a plain ttf/otf name, no folders.
pub fn valid_font_name(s: &str) -> bool {
    let ok_ext = s.ends_with(".ttf") || s.ends_with(".otf");
    ok_ext && s.len() <= 64 && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// `file:<name>.ttf` in a layout's font setting; returns the file name.
pub fn font_file(font: &str) -> Option<&str> {
    font.strip_prefix("file:").filter(|n| valid_font_name(n))
}

fn is_package(s: &str) -> bool {
    let mut parts = s.split('.');
    let first_ok = parts.next().is_some_and(|p| p.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
    let rest: Vec<_> = parts.collect();
    first_ok && !rest.is_empty() && rest.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')) && s.len() <= 200
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LayoutIssue { pub path: String, pub message: String }

fn bad(out: &mut Vec<LayoutIssue>, path: &str, message: &str) { out.push(LayoutIssue { path: path.into(), message: message.into() }) }

fn num(v: &Value, path: &str, lo: f64, hi: f64, out: &mut Vec<LayoutIssue>) -> f64 {
    match v.as_f64() {
        Some(n) if n.is_finite() && n >= lo && n <= hi => n,
        _ => { bad(out, path, &format!("must be a number from {lo} to {hi}")); f64::NAN }
    }
}

fn one_of(v: &Value, path: &str, set: &[&str], out: &mut Vec<LayoutIssue>) {
    if !v.as_str().is_some_and(|s| set.contains(&s)) { bad(out, path, "not an allowed value"); }
}

fn hex(v: &Value, path: &str, out: &mut Vec<LayoutIssue>) {
    let ok = v.as_str().is_some_and(|s| {
        let b = s.as_bytes();
        (b.len() == 7 || b.len() == 9) && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
    });
    if !ok { bad(out, path, "must be a colour like #aabbcc"); }
}

/// Where the scroll-mode rows get their apps: hand-picked, recent, or the category apps declare on Google Play.
pub const ROW_SOURCES: &[&str] = &["favorites", "recent", "games", "video", "music", "news", "social", "tools"];

/// Whether a layout shows the Now Playing widget (it needs the TV's notification-listener switch).
pub fn uses_now_playing(l: &Value) -> bool {
    l.get("items").and_then(Value::as_array).is_some_and(|a| a.iter().any(|i| i.get("kind").and_then(Value::as_str) == Some("nowplaying") && i.get("enabled").and_then(Value::as_bool) == Some(true)))
}

/// A profile id: the file profiles/<id>.json on the TV.
pub fn valid_profile_id(s: &str) -> bool { (1..=16).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') }

/// The profile ids a main layout lists.
pub fn profile_ids(l: &Value) -> Vec<String> {
    l.get("profiles").and_then(Value::as_array).map(|a| a.iter().filter_map(|p| p.get("id").and_then(Value::as_str).map(str::to_string)).collect()).unwrap_or_default()
}

/// A picture for one app, under `tiles/`: plain name, JPG/PNG/WebP.
pub fn valid_tile_image(s: &str) -> bool {
    s.rsplit_once('.').is_some_and(|(stem, ext)| matches!(ext, "jpg" | "png" | "webp") && (1..=60).contains(&stem.len())
        && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

fn text(v: Option<&Value>, path: &str, max: usize, out: &mut Vec<LayoutIssue>) {
    if !v.and_then(Value::as_str).is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= max) { bad(out, path, &format!("must be text of 1 to {max} characters")); }
}

fn pkg_list(v: Option<&Value>, path: &str, max: usize, out: &mut Vec<LayoutIssue>) {
    match v {
        None => {}
        Some(Value::Array(a)) if a.len() <= max => {
            if !a.iter().all(|p| p.as_str().is_some_and(is_package)) { bad(out, path, "must be package names"); }
        }
        Some(_) => bad(out, path, &format!("must be a list of at most {max} package names")),
    }
}

/// Rows, folders, hidden apps and per-app looks for scroll mode.
fn apps_section(o: &serde_json::Map<String, Value>, out: &mut Vec<LayoutIssue>) {
    if let Some(rows) = o.get("rows") {
        match rows.as_array() {
            Some(a) if a.len() <= 8 => for (i, r) in a.iter().enumerate() {
                text(r.get("title"), &format!("rows[{i}].title"), 40, out);
                one_of(r.get("source").unwrap_or(&Value::Null), &format!("rows[{i}].source"), ROW_SOURCES, out);
                pkg_list(r.get("packages"), &format!("rows[{i}].packages"), 24, out);
                if let Some(m) = r.get("max") { num(m, &format!("rows[{i}].max"), 1.0, 24.0, out); }
            },
            _ => bad(out, "rows", "at most 8 rows"),
        }
    }
    if let Some(f) = o.get("folders") {
        match f.as_array() {
            Some(a) if a.len() <= 8 => for (i, x) in a.iter().enumerate() {
                text(x.get("name"), &format!("folders[{i}].name"), 30, out);
                pkg_list(x.get("packages"), &format!("folders[{i}].packages"), 24, out);
            },
            _ => bad(out, "folders", "at most 8 folders"),
        }
    }
    pkg_list(o.get("hidden"), "hidden", 100, out);
    pkg_list(o.get("locked"), "locked", 50, out);
    if let Some(w) = o.get("weather") {
        match w.as_object() {
            Some(m) => {
                text(m.get("city"), "weather.city", 60, out);
                if let Some(u) = m.get("units") { one_of(u, "weather.units", &["c", "f"], out); }
                if m.get("consent").is_some_and(|v| !v.is_boolean()) { bad(out, "weather.consent", "must be true or false"); }
            }
            None => bad(out, "weather", "must be an object"),
        }
    }
    match o.get("openOnStart") { None | Some(Value::Null) => {}, Some(v) => if !v.as_str().is_some_and(is_package) { bad(out, "openOnStart", "must be a package name"); } }
    if let Some(n) = o.get("profileName") { text(Some(n), "profileName", 20, out); }
    if let Some(c) = o.get("profileColor") { hex(c, "profileColor", out); }
    if let Some(p) = o.get("profiles") {
        match p.as_array() {
            Some(a) if a.len() <= 3 => {
                let mut ids = std::collections::BTreeSet::new();
                for (i, x) in a.iter().enumerate() {
                    let id = x.get("id").and_then(Value::as_str).unwrap_or("");
                    if !valid_profile_id(id) || !ids.insert(id.to_string()) { bad(out, &format!("profiles[{i}].id"), "1-16 lower-case letters, digits or -, unique"); }
                    text(x.get("name"), &format!("profiles[{i}].name"), 20, out);
                    if let Some(c) = x.get("color") { hex(c, &format!("profiles[{i}].color"), out); }
                    if x.get("pin").is_some_and(|v| !v.is_boolean()) { bad(out, &format!("profiles[{i}].pin"), "must be true or false"); }
                }
            }
            _ => bad(out, "profiles", "at most 3 more profiles (4 with the main one)"),
        }
    }
    if let Some(apps) = o.get("apps") {
        match apps.as_object() {
            Some(m) if m.len() <= 60 => for (pkg, look) in m {
                if !is_package(pkg) { bad(out, &format!("apps.{pkg}"), "not a package name"); continue; }
                if let Some(n) = look.get("name").filter(|v| !v.is_null()) { text(Some(n), &format!("apps.{pkg}.name"), 40, out); }
                if let Some(c) = look.get("color").filter(|v| !v.is_null()) { hex(c, &format!("apps.{pkg}.color"), out); }
                if let Some(img) = look.get("image").filter(|v| !v.is_null()) {
                    if !img.as_str().is_some_and(valid_tile_image) { bad(out, &format!("apps.{pkg}.image"), "not a safe picture name"); }
                }
            },
            _ => bad(out, "apps", "at most 60 app looks"),
        }
    }
}

/// Video containers the TV's own player opens (H.264/H.265/VP8/VP9 inside). AVI, WMV and FLV are not among them.
pub const VIDEO_EXTS: [&str; 7] = ["mp4", "m4v", "webm", "mkv", "mov", "3gp", "ts"];

/// A video file name the launcher may open: plain name, no folders, a container in `VIDEO_EXTS`.
pub fn valid_video_name(s: &str) -> bool {
    let ok_ext = s.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && VIDEO_EXTS.contains(&ext));
    ok_ext && s.len() <= 64 && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub fn validate(layout: &Value) -> Vec<LayoutIssue> {
    let mut out = Vec::new();
    let Some(o) = layout.as_object() else { bad(&mut out, "", "layout must be an object"); return out };
    if o.get("version").and_then(Value::as_i64) != Some(1) { bad(&mut out, "version", "must be 1"); }
    if !o.get("enabled").is_some_and(Value::is_boolean) { bad(&mut out, "enabled", "must be true or false"); }
    // "classic" (the default): the launcher's own Home keeps the apps; "canvas": the canvas replaces it.
    if let Some(h) = o.get("home") { one_of(h, "home", &["classic", "canvas", "scroll", "tvos"], &mut out); }
    if o.get("screensaver").is_some_and(|v| !v.is_boolean()) { bad(&mut out, "screensaver", "must be true or false"); }
    if o.get("heavyOnLowRam").is_some_and(|v| !v.is_boolean()) { bad(&mut out, "heavyOnLowRam", "must be true or false"); }
    for (key, set) in [("focus", FOCUS), ("launch", LAUNCH), ("tiles", TILES), ("iconShape", ICON_SHAPES)] {
        if let Some(v) = o.get(key) { one_of(v, key, set, &mut out); }
    }
    match o.get("accent") { None | Some(Value::Null) => {}, Some(v) => hex(v, "accent", &mut out) }
    apps_section(o, &mut out);

    match o.get("background").and_then(Value::as_object) {
        None => bad(&mut out, "background", "missing"),
        Some(b) => {
            one_of(b.get("kind").unwrap_or(&Value::Null), "background.kind", BG, &mut out);
            hex(b.get("color").unwrap_or(&Value::Null), "background.color", &mut out);
            hex(b.get("color2").unwrap_or(&Value::Null), "background.color2", &mut out);
            num(b.get("dim").unwrap_or(&Value::Null), "background.dim", 0.0, 0.9, &mut out);
            num(b.get("blur").unwrap_or(&Value::Null), "background.blur", 0.0, 24.0, &mut out);
            if let Some(v) = b.get("scene") { one_of(v, "background.scene", SCENES, &mut out); }
            if let Some(v) = b.get("sceneTime") { one_of(v, "background.sceneTime", SCENE_TIMES, &mut out); }
            if b.get("followWeather").is_some_and(|v| !v.is_boolean()) { bad(&mut out, "background.followWeather", "must be true or false"); }
            match b.get("video") {
                Some(Value::Null) | None => {}
                Some(v) => {
                    let ok = v.as_str().is_some_and(|s| {
                        // A bundled sample id, or a video the person added (a plain file name).
                        (!s.is_empty() && s.len() <= 32 && s.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')) || valid_video_name(s)
                    });
                    if !ok { bad(&mut out, "background.video", "not a safe video name"); }
                }
            }
        }
    }

    let Some(items) = o.get("items").and_then(Value::as_array) else { bad(&mut out, "items", "must be a list"); return out };
    if items.len() > MAX_ITEMS { bad(&mut out, "items", "too many items"); }
    let mut seen: Vec<&str> = Vec::new();
    for (i, it) in items.iter().enumerate().take(MAX_ITEMS) {
        let p = |f: &str| format!("items[{i}].{f}");
        let Some(it) = it.as_object() else { bad(&mut out, &format!("items[{i}]"), "must be an object"); continue };
        match it.get("id").and_then(Value::as_str) {
            Some(id) if !id.is_empty() && id.len() <= 32 && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') => {
                if seen.contains(&id) { bad(&mut out, &p("id"), "duplicate id"); }
                seen.push(id);
            }
            _ => bad(&mut out, &p("id"), "must be lowercase letters, digits and dashes"),
        }
        one_of(it.get("kind").unwrap_or(&Value::Null), &p("kind"), KINDS, &mut out);
        if !it.get("enabled").is_some_and(Value::is_boolean) { bad(&mut out, &p("enabled"), "must be true or false"); }
        let x = num(it.get("x").unwrap_or(&Value::Null), &p("x"), 0.0, 100.0, &mut out);
        let y = num(it.get("y").unwrap_or(&Value::Null), &p("y"), 0.0, 100.0, &mut out);
        let w = num(it.get("w").unwrap_or(&Value::Null), &p("w"), 3.0, 100.0, &mut out);
        let h = num(it.get("h").unwrap_or(&Value::Null), &p("h"), 3.0, 100.0, &mut out);
        if x + w > 100.001 || y + h > 100.001 { bad(&mut out, &p("x"), "runs off the screen"); }
        if let Some(t) = it.get("text") {
            if !t.as_str().is_some_and(|s| s.chars().count() <= 80 && !s.chars().any(char::is_control)) { bad(&mut out, &p("text"), "at most 80 characters, no control characters"); }
        }
        match it.get("style").and_then(Value::as_object) {
            None => bad(&mut out, &p("style"), "missing"),
            Some(s) => {
                let font = s.get("font").and_then(Value::as_str).unwrap_or("");
                if !FONTS.contains(&font) && font_file(font).is_none() { bad(&mut out, &p("style.font"), "not an allowed font"); }
                for flag in ["upper", "shadow", "italic"] {
                    if let Some(v) = s.get(flag) { if !v.is_boolean() { bad(&mut out, &p(&format!("style.{flag}")), "must be true or false"); } }
                }
                if let Some(v) = s.get("spacing") { num(v, &p("style.spacing"), 0.0, 1.0, &mut out); }
                num(s.get("size").unwrap_or(&Value::Null), &p("style.size"), 8.0, 400.0, &mut out);
                hex(s.get("color").unwrap_or(&Value::Null), &p("style.color"), &mut out);
                num(s.get("opacity").unwrap_or(&Value::Null), &p("style.opacity"), 0.05, 1.0, &mut out);
                num(s.get("radius").unwrap_or(&Value::Null), &p("style.radius"), 0.0, 80.0, &mut out);
                one_of(s.get("align").unwrap_or(&Value::Null), &p("style.align"), ALIGN, &mut out);
                match s.get("fill") { Some(Value::Null) | None => {}, Some(f) => hex(f, &p("style.fill"), &mut out) }
            }
        }
        if let Some(ic) = it.get("icon") {
            match ic.as_object() {
                None => bad(&mut out, &p("icon"), "must be an object"),
                Some(ic) => {
                    num(ic.get("size").unwrap_or(&Value::Null), &p("icon.size"), 24.0, 256.0, &mut out);
                    one_of(ic.get("shape").unwrap_or(&Value::Null), &p("icon.shape"), SHAPES, &mut out);
                    num(ic.get("columns").unwrap_or(&Value::Null), &p("icon.columns"), 1.0, 12.0, &mut out);
                    if !ic.get("labels").is_some_and(Value::is_boolean) { bad(&mut out, &p("icon.labels"), "must be true or false"); }
                }
            }
        }
        match it.get("motion").and_then(Value::as_object) {
            None => bad(&mut out, &p("motion"), "missing"),
            Some(m) => {
                one_of(m.get("type").unwrap_or(&Value::Null), &p("motion.type"), MOTIONS, &mut out);
                num(m.get("speed").unwrap_or(&Value::Null), &p("motion.speed"), 0.25, 4.0, &mut out);
                num(m.get("amount").unwrap_or(&Value::Null), &p("motion.amount"), 0.0, 100.0, &mut out);
            }
        }
        if let Some(v) = it.get("variant") {
            let kind = it.get("kind").and_then(Value::as_str).unwrap_or("");
            if !v.as_str().is_some_and(|v| variants(kind).contains(&v)) { bad(&mut out, &p("variant"), "not a look this item has"); }
        }
        if let Some(op) = it.get("options").and_then(Value::as_object) {
            if let Some(m) = op.get("max") { num(m, &p("options.max"), 1.0, 20.0, &mut out); }
            if let Some(m) = op.get("bars") { num(m, &p("options.bars"), 8.0, 96.0, &mut out); }
            if let Some(m) = op.get("stroke") { num(m, &p("options.stroke"), 0.0, 40.0, &mut out); }
            if let Some(pk) = op.get("packages") {
                let ok = pk.as_array().is_some_and(|a| a.len() <= 24 && a.iter().all(|x| x.as_str().is_some_and(is_package)));
                if !ok { bad(&mut out, &p("options.packages"), "at most 24 valid package names"); }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn good() -> Value {
        json!({"version":1,"enabled":true,
          "background":{"kind":"video","color":"#0b1020","color2":"#27325c","video":"rain","dim":0.3,"blur":0},
          "items":[{"id":"clock-1","kind":"clock","enabled":true,"x":72,"y":5,"w":24,"h":12,
            "style":{"font":"sans-serif-light","size":72,"color":"#ffffff","opacity":1,"radius":0,"fill":null,"align":"end"},
            "motion":{"type":"float","speed":1,"amount":20},"options":{"seconds":false}}]})
    }

    #[test] fn a_good_layout_passes() { assert_eq!(validate(&good()), vec![]); }

    #[test] fn weather_is_checked() {
        let mut l = good();
        l["weather"] = json!({"city": "Ongole", "units": "c", "consent": true});
        l["background"]["followWeather"] = json!(true);
        l["items"][0]["kind"] = json!("weather");
        l["items"][0]["variant"] = json!("forecast");
        assert_eq!(validate(&l), vec![], "{:?}", validate(&l));
        l["weather"]["units"] = json!("k");
        l["items"][0]["variant"] = json!("radar");
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"weather.units".to_string()) && paths.iter().any(|p| p.ends_with("variant")), "{paths:?}");
    }

    #[test] fn status_widgets_and_open_on_start() {
        let mut l = good();
        l["items"][0]["kind"] = json!("nowplaying");
        l["items"][0].as_object_mut().unwrap().remove("options");
        l["openOnStart"] = json!("com.netflix.ninja");
        assert_eq!(validate(&l), vec![], "{:?}", validate(&l));
        assert!(uses_now_playing(&l));
        l["items"][0]["enabled"] = json!(false);
        assert!(!uses_now_playing(&l));
        l["openOnStart"] = json!("no way");
        assert!(validate(&l).iter().any(|i| i.path == "openOnStart"));
    }

    #[test] fn locked_apps_and_profiles_are_checked() {
        let mut l = good();
        l["locked"] = json!(["com.netflix.ninja"]);
        l["profileName"] = json!("Dad");
        l["profiles"] = json!([{"id": "kids", "name": "Kids", "color": "#ffaa00"}, {"id": "mum", "name": "Mum", "pin": true}]);
        assert_eq!(validate(&l), vec![]);
        assert_eq!(profile_ids(&l), vec!["kids", "mum"]);
        l["profiles"][1]["id"] = json!("kids");
        l["locked"] = json!(["not a pkg"]);
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"profiles[1].id".to_string()) && paths.contains(&"locked".to_string()), "{paths:?}");
        l["profiles"] = json!([{"id":"a","name":"A"},{"id":"b","name":"B"},{"id":"c","name":"C"},{"id":"d","name":"D"}]);
        assert!(validate(&l).iter().any(|i| i.path == "profiles"));
    }

    #[test] fn rows_folders_hidden_and_app_looks_are_checked() {
        let mut l = good();
        l["rows"] = json!([{"title": "Favourites", "source": "favorites", "packages": ["com.netflix.ninja"]}, {"title": "Games", "source": "games", "max": 8}]);
        l["folders"] = json!([{"name": "Kids", "packages": ["com.a.b"]}]);
        l["hidden"] = json!(["com.sony.dtv.promos"]);
        l["apps"] = json!({"com.netflix.ninja": {"name": "Films", "color": "#e50914", "image": "netflix.jpg"}});
        assert_eq!(validate(&l), vec![]);
        l["rows"][1]["source"] = json!("ads");
        l["hidden"] = json!(["rm -rf"]);
        l["apps"]["com.netflix.ninja"]["image"] = json!("../x.jpg");
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        for p in ["rows[1].source", "hidden", "apps.com.netflix.ninja.image"] { assert!(paths.contains(&p.to_string()), "{p}: {paths:?}"); }
        assert!(valid_tile_image("a-b_1.png") && !valid_tile_image("x.gif") && !valid_tile_image(".jpg"));
    }

    #[test] fn tile_looks_are_checked() {
        let mut l = good();
        l["focus"] = json!("bounce"); l["launch"] = json!("reveal"); l["tiles"] = json!("icon"); l["iconShape"] = json!("leaf"); l["accent"] = json!("#ffaa33");
        assert_eq!(validate(&l), vec![]);
        l["focus"] = json!("spin"); l["accent"] = json!("orange");
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"focus".to_string()) && paths.contains(&"accent".to_string()), "{paths:?}");
    }

    #[test] fn scenes_are_checked() {
        let mut l = good();
        l["background"]["kind"] = json!("scene");
        l["background"]["scene"] = json!("aurora");
        l["background"]["sceneTime"] = json!("night");
        l["heavyOnLowRam"] = json!(true);
        assert_eq!(validate(&l), vec![]);
        l["background"]["scene"] = json!("lava");
        l["heavyOnLowRam"] = json!("yes");
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"background.scene".to_string()) && paths.contains(&"heavyOnLowRam".to_string()), "{paths:?}");
    }

    #[test] fn home_mode_is_classic_or_canvas() {
        let mut l = good();
        for ok in ["classic", "canvas", "scroll", "tvos"] { l["home"] = json!(ok); assert_eq!(validate(&l), vec![]); }
        l["home"] = json!("tv");
        assert!(validate(&l).iter().any(|i| i.path == "home"));
    }

    #[test] fn bad_values_are_named() {
        let mut l = good();
        l["items"][0]["style"]["font"] = json!("Comic Sans");
        l["items"][0]["x"] = json!(90);
        l["background"]["video"] = json!("../../etc/passwd");
        let paths: Vec<_> = validate(&l).into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"items[0].style.font".to_string()));
        assert!(paths.contains(&"items[0].x".to_string()), "off-screen: {paths:?}");
        assert!(paths.contains(&"background.video".to_string()));
    }

    #[test] fn duplicate_ids_and_too_many_items_are_refused() {
        let mut l = good();
        let it = l["items"][0].clone();
        l["items"].as_array_mut().unwrap().push(it);
        assert!(validate(&l).iter().any(|i| i.message == "duplicate id"));
        let it = good()["items"][0].clone();
        l["items"] = Value::Array(vec![it; MAX_ITEMS + 1]);
        assert!(validate(&l).iter().any(|i| i.path == "items"));
    }

    #[test] fn nan_and_wrong_types_are_refused() {
        let mut l = good();
        l["items"][0]["w"] = json!("wide");
        l["items"][0]["motion"]["speed"] = json!(99);
        l["enabled"] = json!("yes");
        let n = validate(&l).len();
        assert!(n >= 3, "{n}");
        assert!(!validate(&json!([1, 2])).is_empty());
    }

    #[test] fn new_looks_fonts_and_options_are_checked() {
        let mut l = good();
        l["items"][0]["variant"] = json!("analog");
        l["items"][0]["style"]["font"] = json!("file:Montserrat-Light.ttf");
        l["items"][0]["style"]["spacing"] = json!(0.3);
        l["items"][0]["style"]["upper"] = json!(true);
        l["items"][0]["options"] = json!({"packages": ["com.netflix.ninja"], "bars": 48, "stroke": 2});
        assert_eq!(validate(&l), vec![]);
        for (path, v) in [("variant", json!("ring")), ("variant", json!(3))] { let mut b = l.clone(); b["items"][0][path] = v; assert!(!validate(&b).is_empty(), "{path}"); }
        let mut b = l.clone(); b["items"][0]["style"]["font"] = json!("file:../x.ttf"); assert!(!validate(&b).is_empty());
        let mut b = l.clone(); b["items"][0]["style"]["font"] = json!("file:x.exe"); assert!(!validate(&b).is_empty());
        let mut b = l.clone(); b["items"][0]["style"]["spacing"] = json!(5); assert!(!validate(&b).is_empty());
        let mut b = l.clone(); b["items"][0]["options"]["packages"] = json!(["not a package"]); assert!(!validate(&b).is_empty());
        let mut b = l.clone(); b["items"][0]["options"]["bars"] = json!(500); assert!(!validate(&b).is_empty());
        for k in ["calendar", "visualizer", "shape"] { let mut b = good(); b["items"][0]["kind"] = json!(k); assert_eq!(validate(&b), vec![], "{k}"); }
    }

    #[test] fn font_names_are_plain_files_only() {
        for ok in ["Montserrat-Light.ttf", "a_b.otf"] { assert!(valid_font_name(ok), "{ok}"); }
        for no in ["../x.ttf", "a/b.ttf", "x.woff", ".h.ttf", "a b.ttf", ""] { assert!(!valid_font_name(no), "{no}"); }
        assert_eq!(font_file("file:a.ttf"), Some("a.ttf"));
        assert_eq!(font_file("sans-serif"), None);
    }

    #[test] fn video_names_are_plain_files_only() {
        for ok in ["loop.mp4", "my_rain-2.webm", "clip.mkv", "phone.mov", "a.m4v", "b.3gp", "c.ts"] { assert!(valid_video_name(ok), "{ok}"); }
        for no in ["../x.mp4", "a/b.mp4", "x.avi", "x.wmv", ".hidden.mp4", ".mp4", "a b.mp4", "x.mp4;rm", ""] { assert!(!valid_video_name(no), "{no}"); }
    }
}
