//! Unit tests: validation, emit round-trip, discovery helpers.

use std::fs;

use tempfile::tempdir;

use crate::cli_wizard::{
    discover::discover_configs,
    emit::{compact_emit_test::emit_raw_config_toml, emit_wizard_config_toml},
    mask::mask_toml_secrets,
    persist::{safe_join_cwd, write_atomic},
};
use crate::config::core::{
    finish_raw_config, parse_raw_config_str, validate_raw_regexes,
    validate_raw_structure,
};
use crate::config::general::StreamMode;
use crate::config::types::RawConfig;

const MIN_FRONTEND_TOML: &str = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "frontend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Frontend]
listen_port = 60001

[Frontend.AntiReverseProxy]
enable = false
host = ""
"#;

#[test]
fn parse_validate_min_frontend() {
    let raw: RawConfig =
        parse_raw_config_str(MIN_FRONTEND_TOML).expect("fixture TOML");
    validate_raw_structure(&raw).expect("structure");
    validate_raw_regexes(&raw).expect("regexes");
}

#[test]
fn finish_raw_min_frontend() {
    let raw: RawConfig =
        parse_raw_config_str(MIN_FRONTEND_TOML).expect("fixture TOML");
    let cfg = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect("finish raw");
    assert_eq!(cfg.general.stream_mode, StreamMode::Frontend);
}

#[test]
fn emit_and_reparse_min_frontend() {
    let raw: RawConfig =
        parse_raw_config_str(MIN_FRONTEND_TOML).expect("fixture TOML");
    let s = emit_raw_config_toml(&raw).expect("emit");
    let raw2: RawConfig = parse_raw_config_str(&s).expect("re-parse");
    validate_raw_structure(&raw2).expect("structure");
    validate_raw_regexes(&raw2).expect("regexes");
}

#[test]
fn emit_wizard_keeps_core_defaults_visible() {
    let raw: RawConfig =
        parse_raw_config_str(MIN_FRONTEND_TOML).expect("fixture TOML");
    let s = emit_wizard_config_toml(&raw).expect("emit wizard");
    assert!(s.contains("memory_mode = \"middle\""));
    assert!(s.contains("allow_ua = []"));
    assert!(
        !s.contains("[Frontend.AntiReverseProxy]"),
        "disabled empty AntiReverseProxy should be omitted"
    );
    let raw2: RawConfig = parse_raw_config_str(&s).expect("re-parse");
    validate_raw_structure(&raw2).expect("structure");
    validate_raw_regexes(&raw2).expect("regexes");
}

/// Wizard emit drops empty AntiReverseProxy and default-only WebDav (runtime uses same defaults).
const BACKEND_WEBDAV_TRIM_TOML: &str = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "node"
type = "WebDav"
pattern = "/mnt/webdav/*"
base_url = "http://127.0.0.1"
port = "6222"
path = ""
priority = 0
proxy_mode = "redirect"
client_speed_limit_kbs = 0
client_burst_speed_kbs = 0

[BackendNode.AntiReverseProxy]
enable = false
host = ""

[BackendNode.WebDav]
url_mode = "path_join"
query_param = "path"
url_template = ""
username = ""
password = ""
user_agent = ""
"#;

#[test]
fn emit_wizard_trims_backend_node_webdav_and_anti_defaults() {
    let raw: RawConfig =
        parse_raw_config_str(BACKEND_WEBDAV_TRIM_TOML).expect("fixture TOML");
    validate_raw_structure(&raw).expect("structure");
    let s = emit_wizard_config_toml(&raw).expect("emit wizard");
    assert!(
        !s.contains("[BackendNode.AntiReverseProxy]"),
        "expected empty anti table omitted: {s}"
    );
    assert!(
        !s.contains("[BackendNode.WebDav]"),
        "expected default path_join WebDav omitted: {s}"
    );
    let raw2: RawConfig = parse_raw_config_str(&s).expect("re-parse");
    validate_raw_structure(&raw2).expect("structure2");
    validate_raw_regexes(&raw2).expect("regexes2");
    let node = raw2
        .backend_nodes
        .as_ref()
        .expect("nodes")
        .first()
        .expect("one");
    assert!(node.webdav.is_none());
    assert!(!node.anti_reverse_proxy.enable);
}

#[test]
fn invalid_regex_in_node_rejected() {
    let mut raw: RawConfig =
        parse_raw_config_str(MIN_FRONTEND_TOML).expect("fixture TOML");
    raw.backend = Some(crate::config::backend::Backend {
        listen_port: 1,
        base_url: "http://x".into(),
        port: "80".into(),
        path: "".into(),
        problematic_clients: vec![],
    });
    raw.general.stream_mode = StreamMode::Backend;
    raw.frontend = None;
    raw.backend_nodes = Some(vec![crate::config::backend::BackendNode {
        name: "n".into(),
        backend_type: "Disk".into(),
        pattern: "(".into(),
        pattern_regex: None,
        base_url: "".into(),
        port: "".into(),
        path: "".into(),
        priority: 0,
        proxy_mode: "redirect".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![],
        anti_reverse_proxy: Default::default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: Some(crate::config::backend::disk::Disk {
            description: String::new(),
        }),
        open_list: None,
        direct_link: None,
        google_drive: None,
        webdav: None,
    }]);
    assert!(validate_raw_regexes(&raw).is_err());
}

#[test]
fn finish_raw_rejects_google_drive_without_node_uuid() {
    let toml = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "gdrive"
type = "googleDrive"
pattern = "/mnt/media/.*"
base_url = "https://www.googleapis.com"
port = "443"
path = ""
priority = 0
proxy_mode = "proxy"

[BackendNode.GoogleDrive]
client_id = "cid"
client_secret = "csecret"
refresh_token = "refresh"
"#;

    let raw: RawConfig = parse_raw_config_str(toml).expect("fixture TOML");
    let err = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect_err("missing node_uuid must fail");
    assert!(err.to_string().contains("node_uuid"));
}

#[test]
fn finish_raw_rejects_google_drive_without_refresh_token() {
    let toml = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "gdrive"
type = "googleDrive"
pattern = "/mnt/media/.*"
base_url = "https://www.googleapis.com"
port = "443"
path = ""
priority = 0
proxy_mode = "proxy"

[BackendNode.GoogleDrive]
node_uuid = "google_drive_a"
client_id = "cid"
client_secret = "csecret"
"#;

    let raw: RawConfig = parse_raw_config_str(toml).expect("fixture TOML");
    let err = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect_err("missing refresh_token must fail");
    assert!(err.to_string().contains("refresh_token"));
}

#[test]
fn finish_raw_rejects_duplicate_google_drive_node_uuid() {
    let toml = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "gdrive-a"
type = "googleDrive"
pattern = "/mnt/a/.*"
base_url = "https://www.googleapis.com"
port = "443"
path = ""
priority = 0
proxy_mode = "proxy"

[BackendNode.GoogleDrive]
node_uuid = "dup_google_drive"
client_id = "cid"
client_secret = "csecret"
refresh_token = "refresh-a"

[[BackendNode]]
name = "gdrive-b"
type = "googleDrive"
pattern = "/mnt/b/.*"
base_url = "https://www.googleapis.com"
port = "443"
path = ""
priority = 0
proxy_mode = "redirect"

[BackendNode.GoogleDrive]
node_uuid = "dup_google_drive"
client_id = "cid"
client_secret = "csecret"
refresh_token = "refresh-b"
"#;

    let raw: RawConfig = parse_raw_config_str(toml).expect("fixture TOML");
    let err = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect_err("duplicate node_uuid must fail");
    assert!(err.to_string().contains("Duplicate"));
}

#[test]
fn finish_raw_rejects_webdav_accel_redirect_without_node_uuid() {
    let toml = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "node"
type = "WebDav"
pattern = "/mnt/webdav/.*"
base_url = "http://127.0.0.1"
port = "6222"
path = ""
priority = 0
proxy_mode = "accel_redirect"

[BackendNode.WebDav]
url_mode = "path_join"
"#;

    let raw: RawConfig = parse_raw_config_str(toml).expect("fixture TOML");
    let err = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect_err("missing node_uuid must fail");
    assert!(err.to_string().contains("node_uuid"));
}

#[test]
fn finish_raw_rejects_duplicate_webdav_accel_redirect_node_uuid() {
    let toml = r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "backend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]

[Backend]
listen_port = 60002
base_url = "https://b.example"
port = "443"
path = "stream"
problematic_clients = []

[[BackendNode]]
name = "node-a"
type = "WebDav"
pattern = "/mnt/a/.*"
base_url = "http://127.0.0.1"
port = "6222"
path = ""
priority = 0
proxy_mode = "accel_redirect"

[BackendNode.WebDav]
node_uuid = "dup_node"
url_mode = "path_join"

[[BackendNode]]
name = "node-b"
type = "WebDav"
pattern = "/mnt/b/.*"
base_url = "http://127.0.0.1"
port = "6333"
path = ""
priority = 0
proxy_mode = "accel_redirect"

[BackendNode.WebDav]
node_uuid = "dup_node"
url_mode = "path_join"
"#;

    let raw: RawConfig = parse_raw_config_str(toml).expect("fixture TOML");
    let err = finish_raw_config(std::path::PathBuf::from("x.toml"), raw)
        .expect_err("duplicate node_uuid must fail");
    assert!(err.to_string().contains("Duplicate"));
}

#[test]
fn discover_finds_valid_file() {
    let dir = tempdir().expect("tempdir");
    let p = dir.path().join("a.toml");
    fs::write(&p, MIN_FRONTEND_TOML).expect("write");
    let list = discover_configs(dir.path()).expect("discover");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].path.file_name().expect("file name"), "a.toml");
}

#[test]
fn discover_sorts_lexicographically() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("z.toml"), MIN_FRONTEND_TOML).expect("write");
    fs::write(dir.path().join("m.toml"), MIN_FRONTEND_TOML).expect("write");
    let list = discover_configs(dir.path()).expect("discover");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].path.file_name().expect("file name"), "m.toml");
}

#[test]
fn safe_join_rejects_path_traversal() {
    assert!(
        safe_join_cwd(std::path::Path::new("/tmp"), "../etc/passwd").is_none()
    );
}

#[test]
fn atomic_write_readable() {
    let dir = tempdir().expect("tempdir");
    let dest = dir.path().join("out.toml");
    write_atomic(&dest, "k = 1\n").expect("atomic write");
    assert_eq!(fs::read_to_string(&dest).expect("read").trim(), "k = 1");
}

#[test]
fn mask_secrets_line() {
    let m = mask_toml_secrets("token = \"abc123secret\"\n");
    assert!(m.contains("***"));
    assert!(!m.contains("abc123secret"));
}
