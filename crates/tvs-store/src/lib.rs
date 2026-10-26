//! Local persistence (SQLite). Schema versions are tracked with `PRAGMA user_version`; migrations
//! only ever append. Baseline snapshots are write-once per device: the first one wins, so a retry
//! after a half-applied deployment can never record a broken state as "original".

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database schema version {found} is newer than this app supports ({supported})")]
    TooNew { found: i64, supported: i64 },
}
pub type Result<T> = std::result::Result<T, StoreError>;

const MIGRATIONS: &[&str] = &[
    // v1
    r#"
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE devices (
        fingerprint TEXT PRIMARY KEY,
        serial TEXT NOT NULL,
        brand TEXT, model TEXT, sdk INTEGER,
        first_seen TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        last_seen  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
    );
    CREATE TABLE snapshots (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        fingerprint TEXT NOT NULL REFERENCES devices(fingerprint),
        kind TEXT NOT NULL CHECK (kind IN ('baseline','pre_deploy','manual')),
        created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        body TEXT NOT NULL
    );
    CREATE UNIQUE INDEX one_baseline_per_device ON snapshots(fingerprint) WHERE kind = 'baseline';
    CREATE TABLE profiles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        version INTEGER NOT NULL,
        created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        body TEXT NOT NULL,
        UNIQUE(name, version)
    );
    CREATE TABLE deployments (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        fingerprint TEXT NOT NULL REFERENCES devices(fingerprint),
        profile_id INTEGER REFERENCES profiles(id),
        started TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        finished TEXT,
        status TEXT NOT NULL DEFAULT 'running'
    );
    CREATE TABLE journal (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        deployment_id INTEGER NOT NULL REFERENCES deployments(id),
        step_id TEXT NOT NULL,
        phase TEXT NOT NULL CHECK (phase IN ('begin','done','failed','skipped','verified')),
        at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        detail TEXT
    );
    "#,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiMode { #[default] Beginner, Advanced }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub ui_mode: UiMode,
    /// Custom adb path; empty uses the bundled one.
    pub adb_path: String,
    pub default_dry_run: bool,
    /// Packages per verified batch during debloat (clamped to 1..=10).
    pub batch_size: u8,
    pub check_updates: bool,
    pub reduced_motion_ui: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { ui_mode: UiMode::Beginner, adb_path: String::new(), default_dry_run: true, batch_size: 5, check_updates: false, reduced_motion_ui: false }
    }
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.batch_size = self.batch_size.clamp(1, 10);
        self.adb_path = self.adb_path.trim().to_string();
        self
    }
}

pub struct Store { conn: Connection }

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        Self::init(Connection::open(path)?)
    }
    pub fn open_in_memory() -> Result<Self> { Self::init(Connection::open_in_memory()?) }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let mut s = Self { conn };
        s.migrate()?;
        Ok(s)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<()> {
        let current = self.schema_version()?;
        let supported = MIGRATIONS.len() as i64;
        if current > supported { return Err(StoreError::TooNew { found: current, supported }); }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn settings(&self) -> Result<AppSettings> {
        let raw: Option<String> = self.conn
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |r| r.get(0))
            .optional()?;
        Ok(match raw { Some(s) => serde_json::from_str::<AppSettings>(&s)?.normalized(), None => AppSettings::default() })
    }

    pub fn save_settings(&self, s: &AppSettings) -> Result<AppSettings> {
        let s = s.clone().normalized();
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES('app', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![serde_json::to_string(&s)?],
        )?;
        Ok(s)
    }

    pub fn upsert_device(&self, fingerprint: &str, serial: &str, brand: Option<&str>, model: Option<&str>, sdk: Option<i64>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO devices(fingerprint, serial, brand, model, sdk) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(fingerprint) DO UPDATE SET serial=excluded.serial, brand=excluded.brand, model=excluded.model,
               sdk=excluded.sdk, last_seen=strftime('%Y-%m-%dT%H:%M:%SZ','now')",
            params![fingerprint, serial, brand, model, sdk],
        )?;
        Ok(())
    }

    /// Store the baseline for a device if none exists. Returns `true` if written, `false` if one was already kept.
    pub fn save_baseline_once(&self, fingerprint: &str, body: &serde_json::Value) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO snapshots(fingerprint, kind, body) VALUES(?1, 'baseline', ?2)",
            params![fingerprint, body.to_string()],
        )?;
        Ok(n == 1)
    }

    pub fn baseline(&self, fingerprint: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self.conn
            .query_row("SELECT body FROM snapshots WHERE fingerprint=?1 AND kind='baseline'", params![fingerprint], |r| r.get(0))
            .optional()?;
        Ok(match raw { Some(s) => Some(serde_json::from_str(&s)?), None => None })
    }

    /// Save a new version of a named profile; returns the version number.
    pub fn save_profile(&self, name: &str, body: &serde_json::Value) -> Result<i64> {
        let next: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM profiles WHERE name = ?1", params![name], |r| r.get(0))?;
        self.conn.execute("INSERT INTO profiles(name, version, body) VALUES(?1,?2,?3)", params![name, next, body.to_string()])?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrates_and_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub").join("studio.db");
        {
            let s = Store::open(&p).unwrap();
            assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len() as i64);
            s.save_settings(&AppSettings { ui_mode: UiMode::Advanced, ..Default::default() }).unwrap();
        }
        let s = Store::open(&p).unwrap();
        assert_eq!(s.settings().unwrap().ui_mode, UiMode::Advanced);
    }

    #[test]
    fn rejects_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("studio.db");
        { let c = Connection::open(&p).unwrap(); c.pragma_update(None, "user_version", 99).unwrap(); }
        assert!(matches!(Store::open(&p), Err(StoreError::TooNew { found: 99, .. })));
    }

    #[test]
    fn settings_default_and_clamp() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.settings().unwrap(), AppSettings::default());
        let saved = s.save_settings(&AppSettings { batch_size: 50, adb_path: "  C:\\adb.exe ".into(), ..Default::default() }).unwrap();
        assert_eq!(saved.batch_size, 10);
        assert_eq!(saved.adb_path, "C:\\adb.exe");
        assert_eq!(s.settings().unwrap(), saved);
    }

    #[test]
    fn settings_tolerate_missing_fields() {
        let s = Store::open_in_memory().unwrap();
        s.conn.execute("INSERT INTO settings(key,value) VALUES('app', '{\"ui_mode\":\"advanced\"}')", []).unwrap();
        let got = s.settings().unwrap();
        assert_eq!(got.ui_mode, UiMode::Advanced);
        assert_eq!(got.batch_size, 5);
    }

    #[test]
    fn baseline_first_wins() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_device("google/tv/G09:12/STT2", "192.168.1.50:5555", Some("TCL"), Some("G09"), Some(31)).unwrap();
        assert!(s.save_baseline_once("google/tv/G09:12/STT2", &json!({"home": "stock"})).unwrap());
        assert!(!s.save_baseline_once("google/tv/G09:12/STT2", &json!({"home": "broken"})).unwrap());
        assert_eq!(s.baseline("google/tv/G09:12/STT2").unwrap(), Some(json!({"home": "stock"})));
    }

    #[test]
    fn baseline_requires_known_device() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.save_baseline_once("unknown", &json!({})).is_err());
    }

    #[test]
    fn profile_versions_increment() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.save_profile("Living room", &json!({"theme": "amoled"})).unwrap(), 1);
        assert_eq!(s.save_profile("Living room", &json!({"theme": "aurora"})).unwrap(), 2);
        assert_eq!(s.save_profile("Bedroom", &json!({})).unwrap(), 1);
    }
}
