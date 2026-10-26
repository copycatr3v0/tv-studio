//! Theme schema v1 (generated from CleanYourTV's documented keys) and validation.
//! The same JSON file drives the desktop editor, so a key the editor offers is a key this validates.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub static SCHEMA_JSON: &str = include_str!("../../../schema/theme.v1.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeySpec {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    pub default: Value,
    pub group: String,
    pub advanced: bool,
    pub label: String,
    pub description: String,
    pub since: String,
}

#[derive(Debug, Deserialize)]
pub struct Schema {
    pub version: u32,
    pub launcher_version: String,
    pub keys: Vec<KeySpec>,
}

pub static SCHEMA: Lazy<Schema> = Lazy::new(|| serde_json::from_str(SCHEMA_JSON).expect("bundled theme schema is valid"));

/// Groups whose keys need the remote-key accessibility helpers or vendor input plumbing. The app does
/// not install or verify those helpers yet, so these keys would look deployed while doing nothing.
pub const NOT_YET_SUPPORTED_GROUPS: &[&str] = &["remote"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueLevel { Error, Warning }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue { pub level: IssueLevel, pub key: String, pub message: String }

static COLOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap());
static DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());
static PKG: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$").unwrap());

const MAX_STRING: usize = 500;
const MAX_LIST: usize = 200;
const MAX_JSON_BYTES: usize = 64 * 1024;

fn clean_string(s: &str) -> bool { s.len() <= MAX_STRING && !s.chars().any(|c| c.is_control()) }

fn check(spec: &KeySpec, v: &Value) -> Result<(), String> {
    match spec.kind.as_str() {
        "bool" => v.as_bool().map(|_| ()).ok_or("must be true or false".into()),
        "int" => {
            let n = v.as_i64().ok_or("must be a whole number")?;
            if spec.min.is_some_and(|m| n < m) || spec.max.is_some_and(|m| n > m) {
                Err(format!("must be between {} and {}", spec.min.unwrap_or(i64::MIN), spec.max.unwrap_or(i64::MAX)))
            } else { Ok(()) }
        }
        "color" => v.as_str().filter(|s| COLOR.is_match(s)).map(|_| ()).ok_or("must be a colour like #1A2B3C".into()),
        "date" => v.as_str().filter(|s| DATE.is_match(s)).map(|_| ()).ok_or("must be a date like 2026-12-31".into()),
        "enum" => {
            let s = v.as_str().ok_or("must be text")?;
            if spec.options.as_ref().is_some_and(|o| o.iter().any(|x| x == s)) { Ok(()) } else { Err(format!("must be one of {:?}", spec.options.clone().unwrap_or_default())) }
        }
        "string" => v.as_str().filter(|s| clean_string(s)).map(|_| ()).ok_or("must be plain text up to 500 characters".into()),
        "packages" => {
            let a = v.as_array().ok_or("must be a list of package names")?;
            if a.len() > MAX_LIST { return Err("too many entries".into()); }
            for x in a { x.as_str().filter(|s| PKG.is_match(s)).ok_or_else(|| format!("{x} is not a package name"))?; }
            Ok(())
        }
        "int_list" => {
            let a = v.as_array().ok_or("must be a list of numbers")?;
            if a.len() > MAX_LIST || a.iter().any(|x| !x.is_i64()) { Err("must be a list of numbers".into()) } else { Ok(()) }
        }
        "string_list" => {
            let a = v.as_array().ok_or("must be a list of text entries")?;
            if a.len() > MAX_LIST || a.iter().any(|x| !x.as_str().is_some_and(clean_string)) { Err("must be a list of plain text entries".into()) } else { Ok(()) }
        }
        "json" => if serde_json::to_vec(v).map(|b| b.len()).unwrap_or(usize::MAX) <= MAX_JSON_BYTES { Ok(()) } else { Err("is too large".into()) },
        other => Err(format!("unknown schema type {other}")),
    }
}

pub fn spec(key: &str) -> Option<&'static KeySpec> { SCHEMA.keys.iter().find(|k| k.key == key) }

/// Validate a theme config for deployment. Errors block the push; nothing is silently changed.
pub fn validate(config: &Value) -> Vec<Issue> {
    let mut issues = vec![];
    let Some(obj) = config.as_object() else {
        return vec![Issue { level: IssueLevel::Error, key: String::new(), message: "A theme must be a JSON object.".into() }];
    };
    for (k, v) in obj {
        match spec(k) {
            None => issues.push(Issue { level: IssueLevel::Error, key: k.clone(), message: "Unknown key: the launcher would ignore it.".into() }),
            Some(s) if NOT_YET_SUPPORTED_GROUPS.contains(&s.group.as_str()) => issues.push(Issue {
                level: IssueLevel::Error, key: k.clone(),
                message: "Remote-button keys need the launcher's accessibility helpers, which this version does not set up yet.".into(),
            }),
            Some(s) => if let Err(m) = check(s, v) { issues.push(Issue { level: IssueLevel::Error, key: k.clone(), message: format!("{} {m}", s.label) }) },
        }
    }
    issues
}

/// Import a CleanYourTV `config.json`: keep valid keys, report every dropped one. Never guesses a value.
pub fn import(config: &Value) -> (Map<String, Value>, Vec<Issue>) {
    let mut kept = Map::new();
    let mut issues = vec![];
    let Some(obj) = config.as_object() else {
        return (kept, vec![Issue { level: IssueLevel::Error, key: String::new(), message: "Not a JSON object.".into() }]);
    };
    for (k, v) in obj {
        let single = Value::Object([(k.clone(), v.clone())].into_iter().collect());
        let errs = validate(&single);
        if errs.is_empty() { kept.insert(k.clone(), v.clone()); } else {
            issues.extend(errs.into_iter().map(|mut e| { e.level = IssueLevel::Warning; e.message = format!("Dropped: {}", e.message); e }));
        }
    }
    (kept, issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schema_loads_with_core_keys() {
        assert_eq!(SCHEMA.version, 1);
        assert!(SCHEMA.keys.len() > 90);
        for k in ["dock_apps", "hidden_apps", "columns", "rows_below_dock", "background_color", "wallpaper", "reduced_motion",
                  "app_library", "uniform_tiles", "show_network_pill", "top_bar_apps", "always_show_names"] {
            assert!(spec(k).is_some(), "{k}");
        }
        assert_eq!(spec("columns").unwrap().max, Some(8));
        assert_eq!(spec("columns").unwrap().default, json!(4));
    }

    #[test]
    fn valid_theme_passes() {
        let t = json!({"dock_apps": ["com.netflix.ninja"], "columns": 5, "background_color": "#070A14", "wallpaper": true,
                       "glass_style": "rim", "wallpaper_dim": 30, "festival_until": "2026-11-10"});
        assert_eq!(validate(&t), vec![]);
    }

    #[test]
    fn bad_values_are_errors() {
        let t = json!({"columns": 12, "background_color": "red", "dock_apps": ["not a pkg"], "glass_style": "neon",
                       "greeting_name": "a\nb", "made_up": 1, "app_keys": ["190:com.x.y"]});
        let keys: Vec<_> = validate(&t).into_iter().map(|i| i.key).collect();
        for k in ["columns", "background_color", "dock_apps", "glass_style", "greeting_name", "made_up", "app_keys"] {
            assert!(keys.contains(&k.to_string()), "{k} should be flagged: {keys:?}");
        }
    }

    #[test]
    fn import_keeps_valid_and_reports_dropped() {
        let (kept, issues) = import(&json!({"columns": 4, "future_key": true, "app_keys": []}));
        assert_eq!(kept.len(), 1);
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().all(|i| i.level == IssueLevel::Warning));
    }

    #[test]
    fn imports_every_cleanyourtv_example_theme_shape() {
        // The upstream default theme, verbatim.
        let t = json!({"dock_apps": ["com.netflix.ninja", "com.amazon.amazonvideo.livingroom", "com.google.android.youtube.tv", "in.startv.hotstar"],
                       "hidden_apps": ["com.mediatek.wwtv.tvcenter", "com.mitv.gallery"], "columns": 4, "rows_below_dock": 0,
                       "reduced_motion": false, "background_color": "#000000", "wallpaper": false});
        assert_eq!(validate(&t), vec![]);
    }
}
