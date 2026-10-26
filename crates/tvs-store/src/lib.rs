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
    // v2: deployments remember their profile and the snapshot taken right before them.
    r#"
    ALTER TABLE deployments ADD COLUMN profile TEXT;
    ALTER TABLE deployments ADD COLUMN snapshot_id INTEGER REFERENCES snapshots(id);
    "#,
    // v3: benchmark runs.
    r#"
    CREATE TABLE bench_runs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        fingerprint TEXT NOT NULL REFERENCES devices(fingerprint),
        label TEXT NOT NULL,
        created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        body TEXT NOT NULL
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
    /// Run against a simulated TV instead of adb: nothing real is changed.
    pub demo_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { ui_mode: UiMode::Beginner, adb_path: String::new(), default_dry_run: true, batch_size: 5, check_updates: false, reduced_motion_ui: false, demo_mode: false }
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

    pub fn save_snapshot(&self, fingerprint: &str, kind: &str, body: &serde_json::Value) -> Result<i64> {
        self.conn.execute("INSERT INTO snapshots(fingerprint, kind, body) VALUES(?1, ?2, ?3)", params![fingerprint, kind, body.to_string()])?;
        Ok(self.conn.last_insert_rowid())
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeploymentRow {
    pub id: i64,
    pub fingerprint: String,
    pub started: String,
    pub finished: Option<String>,
    pub status: String,
    pub profile: Option<serde_json::Value>,
    pub snapshot_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JournalRow { pub step_id: String, pub phase: String, pub at: String, pub detail: Option<String> }

type RawRow = (i64, String, String, Option<String>, String, Option<String>, Option<i64>);

impl Store {
    pub fn start_deployment(&self, fingerprint: &str, profile: &serde_json::Value, snapshot_id: Option<i64>) -> Result<i64> {
        self.conn.execute("INSERT INTO deployments(fingerprint, profile, snapshot_id) VALUES(?1, ?2, ?3)",
            params![fingerprint, profile.to_string(), snapshot_id])?;
        Ok(self.conn.last_insert_rowid())
    }
    pub fn set_deployment_snapshot(&self, id: i64, snapshot_id: i64) -> Result<()> {
        self.conn.execute("UPDATE deployments SET snapshot_id=?2 WHERE id=?1", params![id, snapshot_id])?;
        Ok(())
    }
    pub fn journal(&self, deployment_id: i64, step_id: &str, phase: &str, detail: Option<&str>) -> Result<()> {
        self.conn.execute("INSERT INTO journal(deployment_id, step_id, phase, detail) VALUES(?1,?2,?3,?4)",
            params![deployment_id, step_id, phase, detail])?;
        Ok(())
    }
    pub fn finish_deployment(&self, id: i64, status: &str) -> Result<()> {
        self.conn.execute("UPDATE deployments SET status=?2, finished=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1", params![id, status])?;
        Ok(())
    }
    fn row(r: &rusqlite::Row) -> rusqlite::Result<RawRow> {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
    }
    fn to_row(t: RawRow) -> Result<DeploymentRow> {
        Ok(DeploymentRow { id: t.0, fingerprint: t.1, started: t.2, finished: t.3, status: t.4,
            profile: t.5.map(|p| serde_json::from_str(&p)).transpose()?, snapshot_id: t.6 })
    }
    const COLS: &'static str = "id, fingerprint, started, finished, status, profile, snapshot_id";
    /// A deployment that never finished (app closed, crash, power cut) for this TV.
    pub fn interrupted_deployment(&self, fingerprint: &str) -> Result<Option<DeploymentRow>> {
        let t = self.conn.query_row(&format!("SELECT {} FROM deployments WHERE fingerprint=?1 AND status='running' ORDER BY id DESC LIMIT 1", Self::COLS),
            params![fingerprint], Self::row).optional()?;
        t.map(Self::to_row).transpose()
    }
    pub fn deployments(&self, fingerprint: &str, limit: u32) -> Result<Vec<DeploymentRow>> {
        let mut st = self.conn.prepare(&format!("SELECT {} FROM deployments WHERE fingerprint=?1 ORDER BY id DESC LIMIT ?2", Self::COLS))?;
        let rows = st.query_map(params![fingerprint, limit], Self::row)?.collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter().map(Self::to_row).collect()
    }
    pub fn deployment(&self, id: i64) -> Result<Option<DeploymentRow>> {
        let t = self.conn.query_row(&format!("SELECT {} FROM deployments WHERE id=?1", Self::COLS), params![id], Self::row).optional()?;
        t.map(Self::to_row).transpose()
    }
    pub fn journal_rows(&self, deployment_id: i64) -> Result<Vec<JournalRow>> {
        let mut st = self.conn.prepare("SELECT step_id, phase, at, detail FROM journal WHERE deployment_id=?1 ORDER BY id")?;
        let rows = st.query_map(params![deployment_id], |r| Ok(JournalRow { step_id: r.get(0)?, phase: r.get(1)?, at: r.get(2)?, detail: r.get(3)? }))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn snapshot(&self, id: i64) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self.conn.query_row("SELECT body FROM snapshots WHERE id=?1", params![id], |r| r.get(0)).optional()?;
        Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BenchMeta { pub id: i64, pub label: String, pub created: String }

impl Store {
    pub fn save_bench(&self, fingerprint: &str, label: &str, body: &serde_json::Value) -> Result<i64> {
        self.conn.execute("INSERT INTO bench_runs(fingerprint, label, body) VALUES(?1,?2,?3)", params![fingerprint, label, body.to_string()])?;
        Ok(self.conn.last_insert_rowid())
    }
    pub fn list_bench(&self, fingerprint: &str) -> Result<Vec<BenchMeta>> {
        let mut st = self.conn.prepare("SELECT id, label, created FROM bench_runs WHERE fingerprint=?1 ORDER BY id DESC")?;
        let rows = st.query_map(params![fingerprint], |r| Ok(BenchMeta { id: r.get(0)?, label: r.get(1)?, created: r.get(2)? }))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    /// (fingerprint, body) of one run.
    pub fn get_bench(&self, id: i64) -> Result<Option<(String, serde_json::Value)>> {
        let row: Option<(String, String)> = self.conn.query_row("SELECT fingerprint, body FROM bench_runs WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
        Ok(match row { Some((f, b)) => Some((f, serde_json::from_str(&b)?)), None => None })
    }
    /// The newest deployment that changed something and finished (fully or partly).
    pub fn last_applied_deployment(&self, fingerprint: &str) -> Result<Option<DeploymentRow>> {
        Ok(self.deployments(fingerprint, 50)?.into_iter().find(|d| matches!(d.status.as_str(), "completed" | "partial" | "stopped") && d.profile.is_some()))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProfileMeta { pub name: String, pub version: i64, pub created: String }

impl Store {
    /// Every saved version whose name starts with `prefix`, newest first.
    pub fn list_profiles(&self, prefix: &str) -> Result<Vec<ProfileMeta>> {
        let mut st = self.conn.prepare(
            "SELECT name, version, created FROM profiles WHERE substr(name, 1, length(?1)) = ?1 ORDER BY id DESC")?;
        let rows = st.query_map(params![prefix], |r| Ok(ProfileMeta { name: r.get(0)?, version: r.get(1)?, created: r.get(2)? }))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_profile(&self, name: &str, version: i64) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self.conn
            .query_row("SELECT body FROM profiles WHERE name=?1 AND version=?2", params![name, version], |r| r.get(0))
            .optional()?;
        Ok(match raw { Some(s) => Some(serde_json::from_str(&s)?), None => None })
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
    fn pre_deploy_snapshots_accumulate_and_kind_is_checked() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_device("d", "x", None, None, None).unwrap();
        assert!(s.save_snapshot("d", "pre_deploy", &json!({})).is_ok());
        assert!(s.save_snapshot("d", "pre_deploy", &json!({})).is_ok());
        assert!(s.save_snapshot("d", "bogus", &json!({})).is_err());
        // A second baseline through this path is refused by the unique index too.
        s.save_baseline_once("d", &json!({"a": 1})).unwrap();
        assert!(s.save_snapshot("d", "baseline", &json!({"a": 2})).is_err());
    }

    #[test]
    fn baseline_requires_known_device() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.save_baseline_once("unknown", &json!({})).is_err());
    }

    #[test]
    fn deployments_and_journal() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_device("d", "x", None, None, None).unwrap();
        let snap = s.save_snapshot("d", "pre_deploy", &json!({"home": "stock"})).unwrap();
        let id = s.start_deployment("d", &json!({"name": "p"}), None).unwrap();
        s.set_deployment_snapshot(id, snap).unwrap();
        s.journal(id, "identify", "begin", None).unwrap();
        s.journal(id, "identify", "done", Some("ok")).unwrap();
        let r = s.interrupted_deployment("d").unwrap().unwrap();
        assert_eq!(r.id, id);
        assert_eq!(r.snapshot_id, Some(snap));
        assert_eq!(r.profile, Some(json!({"name": "p"})));
        assert_eq!(s.journal_rows(id).unwrap().len(), 2);
        s.finish_deployment(id, "completed").unwrap();
        assert!(s.interrupted_deployment("d").unwrap().is_none());
        assert_eq!(s.deployments("d", 10).unwrap()[0].status, "completed");
        assert_eq!(s.snapshot(snap).unwrap(), Some(json!({"home": "stock"})));
    }

    #[test]
    fn v1_database_upgrades_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("studio.db");
        {
            let c = Connection::open(&p).unwrap();
            c.execute_batch(MIGRATIONS[0]).unwrap();
            c.pragma_update(None, "user_version", 1).unwrap();
            c.execute("INSERT INTO devices(fingerprint, serial) VALUES('d','x')", []).unwrap();
            c.execute("INSERT INTO deployments(fingerprint) VALUES('d')", []).unwrap();
        }
        let s = Store::open(&p).unwrap();
        assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len() as i64);
        let r = s.interrupted_deployment("d").unwrap().unwrap();
        assert_eq!(r.profile, None);
    }

    #[test]
    fn bench_runs_and_last_applied() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_device("d", "x", None, None, None).unwrap();
        let a = s.save_bench("d", "before", &json!({"k": 1})).unwrap();
        let b = s.save_bench("d", "after", &json!({"k": 2})).unwrap();
        assert_eq!(s.list_bench("d").unwrap().iter().map(|m| m.id).collect::<Vec<_>>(), vec![b, a]);
        assert_eq!(s.get_bench(a).unwrap(), Some(("d".to_string(), json!({"k": 1}))));
        let d1 = s.start_deployment("d", &json!({"p": 1}), None).unwrap();
        s.finish_deployment(d1, "completed").unwrap();
        let d2 = s.start_deployment("d", &json!({"p": 2}), None).unwrap();
        s.finish_deployment(d2, "abandoned").unwrap();
        assert_eq!(s.last_applied_deployment("d").unwrap().unwrap().id, d1);
    }

    #[test]
    fn list_and_get_profiles() {
        let s = Store::open_in_memory().unwrap();
        s.save_profile("theme:Living room", &json!({"a": 1})).unwrap();
        s.save_profile("theme:Living room", &json!({"a": 2})).unwrap();
        s.save_profile("deploy:x", &json!({})).unwrap();
        let l = s.list_profiles("theme:").unwrap();
        assert_eq!(l.iter().map(|p| p.version).collect::<Vec<_>>(), vec![2, 1]);
        assert_eq!(s.get_profile("theme:Living room", 1).unwrap(), Some(json!({"a": 1})));
        assert_eq!(s.get_profile("theme:Living room", 9).unwrap(), None);
        // A prefix with LIKE wildcards matches literally.
        assert!(s.list_profiles("theme%").unwrap().is_empty());
    }

    #[test]
    fn profile_versions_increment() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.save_profile("Living room", &json!({"theme": "amoled"})).unwrap(), 1);
        assert_eq!(s.save_profile("Living room", &json!({"theme": "aurora"})).unwrap(), 2);
        assert_eq!(s.save_profile("Bedroom", &json!({})).unwrap(), 1);
    }
}
