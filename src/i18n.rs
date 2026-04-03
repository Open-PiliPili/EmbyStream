//! JSON-backed UI strings (`locales/en.json`, `locales/zh.json`).
//! Missing keys in `zh` fall back to `en`; if still missing, the key string is returned.

use std::cell::Cell;
use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde_json::Value;

use crate::cli::UiLang;

static EN_MAP: Lazy<HashMap<String, String>> = Lazy::new(load_en);
static ZH_MAP: Lazy<HashMap<String, String>> = Lazy::new(load_zh);

fn load_en() -> HashMap<String, String> {
    parse_locale_map(include_str!("../locales/en.json"), "locales/en.json")
}

fn load_zh() -> HashMap<String, String> {
    parse_locale_map(include_str!("../locales/zh.json"), "locales/zh.json")
}

fn parse_locale_map(raw: &str, source: &str) -> HashMap<String, String> {
    let parsed: Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to parse {source}: {error}");
            return HashMap::new();
        }
    };

    let Some(object) = parsed.as_object() else {
        eprintln!("Failed to load {source}: locale root must be a JSON object");
        return HashMap::new();
    };

    object
        .iter()
        .map(|(k, v)| {
            let s = match v {
                Value::String(s) => s.clone(),
                _ => v.to_string(),
            };
            (k.clone(), s)
        })
        .collect()
}

fn get_map(lang: UiLang) -> &'static HashMap<String, String> {
    match lang {
        UiLang::En => &EN_MAP,
        UiLang::Zh => &ZH_MAP,
    }
}

/// Resolve `key` for an explicit language (e.g. `--help` localization in `cli_lang`).
pub fn lookup(lang: UiLang, key: &str) -> String {
    if let Some(s) = get_map(lang).get(key) {
        return s.clone();
    }
    if lang == UiLang::Zh {
        if let Some(s) = EN_MAP.get(key) {
            return s.clone();
        }
    }
    key.to_string()
}

/// Replace `{name}` placeholders in a looked-up string.
pub fn lookup_fmt(lang: UiLang, key: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = lookup(lang, key);
    for (k, v) in pairs {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

thread_local! {
    static UI_LANG: Cell<UiLang> = const { Cell::new(UiLang::En) };
}

/// Thread-local UI language for the config wizard (`cli_wizard::run` sets this).
pub fn set_ui_lang(lang: UiLang) {
    UI_LANG.with(|c| c.set(lang));
}

pub fn ui_lang() -> UiLang {
    UI_LANG.with(|c| c.get())
}

/// Wizard strings using the thread-local language from [`set_ui_lang`].
pub fn tr(key: &str) -> String {
    lookup(ui_lang(), key)
}

pub fn tr_fmt(key: &str, pairs: &[(&str, &str)]) -> String {
    lookup_fmt(ui_lang(), key, pairs)
}
