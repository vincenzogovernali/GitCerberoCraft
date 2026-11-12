//! Language files.
//!
//! Every `*.json` file in a `lang` folder is a language. The file name (without
//! `.json`) is the language code, and the optional key `"language.name"` is the
//! name shown in the selector. Dropping a new file into one of the folders makes
//! the language appear automatically: the folders are re-scanned every couple of
//! seconds while the app runs. Missing keys fall back to English.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::SystemTime;

const EMBEDDED_EN: &str = include_str!("../lang/en.json");
const EMBEDDED_IT: &str = include_str!("../lang/it.json");

#[derive(Clone, Debug)]
pub struct LangInfo {
    pub code: String,
    pub name: String,
    pub path: Option<PathBuf>,
}

pub struct I18n {
    pub languages: Vec<LangInfo>,
    pub current: String,
    strings: HashMap<String, String>,
    fallback: HashMap<String, String>,
    signature: Vec<(PathBuf, SystemTime)>,
    pub last_new: Vec<String>,
}

fn parse(text: &str) -> HashMap<String, String> {
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_default();
    let mut out = HashMap::new();
    flatten("", &v, &mut out);
    out
}

fn flatten(prefix: &str, v: &serde_json::Value, out: &mut HashMap<String, String>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&key, val, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

/// Folders searched for language files, most specific last (later files win).
pub fn lang_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            dirs.push(d.join("lang"));
            // Running from target/debug or target/release inside the project.
            if let Some(p) = d.parent().and_then(|p| p.parent()) {
                dirs.push(p.join("lang"));
            }
        }
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lang"));
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("lang"));
    }
    dirs.push(crate::config::config_dir().join("lang"));
    let mut seen = Vec::new();
    for d in dirs {
        let c = d.canonicalize().unwrap_or(d.clone());
        if !seen.iter().any(|s: &PathBuf| s == &c) {
            seen.push(c);
        }
    }
    seen
}

fn scan_files() -> Vec<(PathBuf, SystemTime)> {
    let mut files = Vec::new();
    for d in lang_dirs() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x.eq_ignore_ascii_case("json")).unwrap_or(false) {
                    let m = e.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
                    files.push((p, m));
                }
            }
        }
    }
    files.sort();
    files
}

pub fn system_language() -> String {
    sys_locale::get_locale()
        .unwrap_or_else(|| "en".into())
        .split(['-', '_'])
        .next()
        .unwrap_or("en")
        .to_lowercase()
}

impl I18n {
    pub fn new(setting: &str) -> Self {
        let mut s = Self {
            languages: vec![],
            current: "en".into(),
            strings: HashMap::new(),
            fallback: parse(EMBEDDED_EN),
            signature: vec![],
            last_new: vec![],
        };
        s.rescan(false);
        s.set_language(setting);
        s
    }

    fn rescan(&mut self, report_new: bool) {
        let files = scan_files();
        let mut map: BTreeMap<String, LangInfo> = BTreeMap::new();
        map.insert("en".into(), LangInfo { code: "en".into(), name: "English".into(), path: None });
        map.insert("it".into(), LangInfo { code: "it".into(), name: "Italiano".into(), path: None });
        for (p, _) in &files {
            let code = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            if code.is_empty() {
                continue;
            }
            let strings = std::fs::read_to_string(p).map(|t| parse(&t)).unwrap_or_default();
            let name = strings.get("language.name").cloned().unwrap_or_else(|| code.clone());
            map.insert(code.clone(), LangInfo { code, name, path: Some(p.clone()) });
        }
        let new_langs: Vec<LangInfo> = map.into_values().collect();
        if report_new {
            for l in &new_langs {
                if !self.languages.iter().any(|o| o.code == l.code) {
                    self.last_new.push(l.name.clone());
                }
            }
        }
        self.languages = new_langs;
        self.signature = files;
        // Reload the English fallback from disk if present (lets people fix texts).
        if let Some(en) = self.languages.iter().find(|l| l.code == "en").and_then(|l| l.path.clone()) {
            if let Ok(t) = std::fs::read_to_string(en) {
                let mut fb = parse(EMBEDDED_EN);
                fb.extend(parse(&t));
                self.fallback = fb;
            }
        }
    }

    /// Re-scan the language folders if a file was added, removed or changed.
    /// Returns true when something changed.
    pub fn poll(&mut self, setting: &str) -> bool {
        let files = scan_files();
        if files != self.signature {
            self.rescan(true);
            self.set_language(setting);
            return true;
        }
        false
    }

    pub fn resolve(&self, setting: &str) -> String {
        if setting == "auto" || setting.is_empty() {
            let sys = system_language();
            if self.languages.iter().any(|l| l.code == sys) {
                return sys;
            }
            return "en".into();
        }
        if self.languages.iter().any(|l| l.code == setting) {
            setting.to_string()
        } else {
            "en".into()
        }
    }

    pub fn set_language(&mut self, setting: &str) {
        let code = self.resolve(setting);
        let info = self.languages.iter().find(|l| l.code == code).cloned();
        let mut strings = match code.as_str() {
            "it" => parse(EMBEDDED_IT),
            "en" => parse(EMBEDDED_EN),
            _ => HashMap::new(),
        };
        if let Some(p) = info.and_then(|i| i.path) {
            if let Ok(t) = std::fs::read_to_string(p) {
                strings.extend(parse(&t));
            }
        }
        self.strings = strings;
        self.current = code;
    }

    pub fn get(&self, key: &str) -> String {
        self.strings
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn missing_keys(&self, code: &str) -> Vec<String> {
        let path = self.languages.iter().find(|l| l.code == code).and_then(|l| l.path.clone());
        let strings = match path {
            Some(p) => std::fs::read_to_string(p).map(|t| parse(&t)).unwrap_or_default(),
            None => HashMap::new(),
        };
        let mut v: Vec<String> = self.fallback.keys().filter(|k| !strings.contains_key(*k)).cloned().collect();
        v.sort();
        v
    }
}

static GLOBAL: OnceLock<RwLock<I18n>> = OnceLock::new();

pub fn init(setting: &str) {
    let _ = GLOBAL.set(RwLock::new(I18n::new(setting)));
}

pub fn global() -> &'static RwLock<I18n> {
    GLOBAL.get_or_init(|| RwLock::new(I18n::new("auto")))
}

/// Translate a key.
pub fn t(key: &str) -> String {
    global().read().map(|g| g.get(key)).unwrap_or_else(|_| key.to_string())
}

/// Translate a key and replace `{0}`, `{1}`, ... with the arguments.
pub fn tf(key: &str, args: &[&str]) -> String {
    let mut s = t(key);
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

pub fn user_lang_dir() -> PathBuf {
    crate::config::config_dir().join("lang")
}

pub fn project_lang_dir() -> Option<PathBuf> {
    lang_dirs().into_iter().find(|d| d.join("en.json").exists())
}

pub fn is_json(p: &Path) -> bool {
    p.extension().map(|x| x == "json").unwrap_or(false)
}
