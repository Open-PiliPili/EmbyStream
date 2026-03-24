//! Static `RawConfig` values for `embystream config template`.

use crate::config::{
    backend::{
        Backend, BackendNode, direct::DirectLink, disk::Disk,
        openlist::OpenList,
    },
    frontend::Frontend,
    general::{Emby, General, Log, StreamMode, UserAgent},
    types::{
        AntiReverseProxyConfig, FallbackConfig, PathRewriteConfig, RawConfig,
    },
};

fn log_template() -> Log {
    Log {
        level: "info".into(),
        prefix: String::new(),
        root_path: "./logs".into(),
    }
}

fn general_template(mode: StreamMode, memory_mode: &str) -> General {
    General {
        memory_mode: memory_mode.into(),
        stream_mode: mode,
        encipher_key: "Q4eCbawEp3sCvDvx".into(),
        encipher_iv: "a3cH2abhxnu9hGo5".into(),
    }
}

fn emby_template() -> Emby {
    Emby {
        url: "http://127.0.0.1".into(),
        port: "8096".into(),
        token: "replace_with_emby_api_token".into(),
    }
}

fn user_agent_template() -> UserAgent {
    UserAgent {
        mode: "deny".into(),
        allow_ua: vec![],
        deny_ua: vec![
            "curl".into(),
            "wget".into(),
            "python".into(),
            "fimily".into(),
            "infuse-library".into(),
        ],
    }
}

fn fallback_template() -> FallbackConfig {
    FallbackConfig {
        video_missing_path: "/mnt/anime/fallback/video_missing.mp4".into(),
    }
}

fn anti_rev_default() -> AntiReverseProxyConfig {
    AntiReverseProxyConfig {
        enable: false,
        trusted_host: String::new(),
    }
}

fn frontend_path_rewrites_full() -> Vec<PathRewriteConfig> {
    vec![
        PathRewriteConfig {
            enable: false,
            pattern: "^(/.*)$".into(),
            replacement: "https://my-cdn.com$1".into(),
        },
        PathRewriteConfig {
            enable: false,
            pattern: "^/media(/.*)$".into(),
            replacement: "$1".into(),
        },
        PathRewriteConfig {
            enable: false,
            pattern: "^/stream(/.*)$".into(),
            replacement: "/proxy$1".into(),
        },
    ]
}

fn frontend_section_full() -> Frontend {
    Frontend {
        listen_port: 60001,
        check_file_existence: true,
        path_rewrites: frontend_path_rewrites_full(),
        anti_reverse_proxy: anti_rev_default(),
    }
}

fn frontend_section_dual() -> Frontend {
    Frontend {
        listen_port: 60001,
        check_file_existence: true,
        path_rewrites: vec![PathRewriteConfig {
            enable: false,
            pattern: "^(/.*)$".into(),
            replacement: "https://my-cdn.com$1".into(),
        }],
        anti_reverse_proxy: anti_rev_default(),
    }
}

/// Valid `RawConfig` scaffold for docs / starting point (comment-free when emitted).
pub(crate) fn build_template_raw(mode: StreamMode) -> RawConfig {
    let base = RawConfig {
        log: log_template(),
        general: general_template(
            mode,
            match mode {
                StreamMode::Frontend => "middle",
                StreamMode::Backend => "high",
                StreamMode::Dual => "middle",
            },
        ),
        emby: emby_template(),
        user_agent: user_agent_template(),
        http2: None,
        disk: None,
        open_list: None,
        direct_link: None,
        fallback: fallback_template(),
        frontend: None,
        backend: None,
        backend_nodes: None,
    };

    match mode {
        StreamMode::Frontend => RawConfig {
            frontend: Some(frontend_section_full()),
            ..base
        },
        StreamMode::Backend => RawConfig {
            backend: Some(Backend {
                listen_port: 60001,
                base_url: "https://backend.example.com".into(),
                port: "443".into(),
                path: "stream".into(),
                problematic_clients: vec![
                    "yamby".into(),
                    "hills".into(),
                    "embytolocalplayer".into(),
                    "Emby/".into(),
                ],
            }),
            backend_nodes: Some(vec![
                openlist_example_node(),
                directlink_example_node(),
            ]),
            ..base
        },
        StreamMode::Dual => RawConfig {
            frontend: Some(frontend_section_dual()),
            backend: Some(Backend {
                listen_port: 3000,
                base_url: "http://127.0.0.1".into(),
                port: "3000".into(),
                path: String::new(),
                problematic_clients: vec![
                    "yamby".into(),
                    "hills".into(),
                    "embytolocalplayer".into(),
                    "Emby/".into(),
                ],
            }),
            backend_nodes: Some(vec![
                disk_example_node(),
                openlist_node_dual(),
                directlink_node_dual(),
            ]),
            ..base
        },
    }
}

fn openlist_example_node() -> BackendNode {
    BackendNode {
        name: "MyOpenList".into(),
        backend_type: "OpenList".into(),
        pattern: "/openlist/.*".into(),
        pattern_regex: None,
        base_url: "http://alist.example.com".into(),
        port: "5244".into(),
        path: "/openlist".into(),
        priority: 0,
        proxy_mode: "redirect".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![PathRewriteConfig {
            enable: false,
            pattern: "^/openlist(/.*)$".into(),
            replacement: "$1".into(),
        }],
        anti_reverse_proxy: anti_rev_default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: None,
        open_list: Some(OpenList {
            base_url: "http://alist.example.com".into(),
            port: String::new(),
            token: "replace_openlist_token".into(),
        }),
        direct_link: None,
        webdav: None,
    }
}

fn directlink_example_node() -> BackendNode {
    BackendNode {
        name: "CloudDrive".into(),
        backend_type: "DirectLink".into(),
        pattern: "/cloud/.*".into(),
        pattern_regex: None,
        base_url: "https://cloud.example.com".into(),
        port: "443".into(),
        path: "/cloud".into(),
        priority: 0,
        proxy_mode: "redirect".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![PathRewriteConfig {
            enable: false,
            pattern: "^/cloud(/.*)$".into(),
            replacement: "https://cdn.example.com$1".into(),
        }],
        anti_reverse_proxy: anti_rev_default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: None,
        open_list: None,
        direct_link: Some(DirectLink {
            user_agent: "Mozilla/5.0 (MockClient)".into(),
        }),
        webdav: None,
    }
}

fn disk_example_node() -> BackendNode {
    BackendNode {
        name: "LocalDisk".into(),
        backend_type: "Disk".into(),
        pattern: "/mnt/media/.*".into(),
        pattern_regex: None,
        base_url: "http://127.0.0.1".into(),
        port: "3000".into(),
        path: "/mnt/media".into(),
        priority: 0,
        proxy_mode: "proxy".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![
            PathRewriteConfig {
                enable: false,
                pattern: "^/mnt/media(/.*)$".into(),
                replacement: "/media$1".into(),
            },
            PathRewriteConfig {
                enable: false,
                pattern: r"^(.*)\.mkv$".into(),
                replacement: "$1.mp4".into(),
            },
        ],
        anti_reverse_proxy: anti_rev_default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: Some(Disk {
            description: String::new(),
        }),
        open_list: None,
        direct_link: None,
        webdav: None,
    }
}

fn openlist_node_dual() -> BackendNode {
    BackendNode {
        name: "MyOpenList".into(),
        backend_type: "OpenList".into(),
        pattern: "/openlist/.*".into(),
        pattern_regex: None,
        base_url: "http://alist.example.com".into(),
        port: "5244".into(),
        path: "/openlist".into(),
        priority: 0,
        proxy_mode: "redirect".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![PathRewriteConfig {
            enable: false,
            pattern: "^/openlist(/.*)$".into(),
            replacement: "$1".into(),
        }],
        anti_reverse_proxy: anti_rev_default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: None,
        open_list: Some(OpenList {
            base_url: "http://alist.example.com".into(),
            port: String::new(),
            token: "replace_openlist_token".into(),
        }),
        direct_link: None,
        webdav: None,
    }
}

fn directlink_node_dual() -> BackendNode {
    BackendNode {
        name: "CloudDrive".into(),
        backend_type: "DirectLink".into(),
        pattern: "/cloud/.*".into(),
        pattern_regex: None,
        base_url: "https://cloud.example.com".into(),
        port: "443".into(),
        path: "/cloud".into(),
        priority: 0,
        proxy_mode: "redirect".into(),
        client_speed_limit_kbs: 0,
        client_burst_speed_kbs: 0,
        path_rewrites: vec![PathRewriteConfig {
            enable: false,
            pattern: "^/cloud(/.*)$".into(),
            replacement: "https://cdn.example.com$1".into(),
        }],
        anti_reverse_proxy: anti_rev_default(),
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk: None,
        open_list: None,
        direct_link: Some(DirectLink {
            user_agent: "Mozilla/5.0 (MockClient)".into(),
        }),
        webdav: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_wizard::emit::compact_emit_test::emit_raw_config_toml;
    use crate::config::core::{
        finish_raw_config, parse_raw_config_str, validate_raw_regexes,
        validate_raw_structure,
    };

    #[test]
    fn template_frontend_roundtrip() {
        let raw = build_template_raw(StreamMode::Frontend);
        validate_raw_structure(&raw).expect("structure");
        validate_raw_regexes(&raw).expect("regex");
        let s = emit_raw_config_toml(&raw).expect("emit");
        let p = parse_raw_config_str(&s).expect("parse");
        validate_raw_structure(&p).expect("structure2");
        finish_raw_config(std::path::PathBuf::from("t.toml"), p)
            .expect("finish");
    }

    #[test]
    fn template_backend_roundtrip() {
        let raw = build_template_raw(StreamMode::Backend);
        validate_raw_structure(&raw).expect("structure");
        validate_raw_regexes(&raw).expect("regex");
        let s = emit_raw_config_toml(&raw).expect("emit");
        let p = parse_raw_config_str(&s).expect("parse");
        finish_raw_config(std::path::PathBuf::from("t.toml"), p)
            .expect("finish");
    }

    #[test]
    fn template_dual_roundtrip_and_ports() {
        let raw = build_template_raw(StreamMode::Dual);
        validate_raw_structure(&raw).expect("structure");
        validate_raw_regexes(&raw).expect("regex");
        let fe = raw.frontend.as_ref().expect("fe");
        let be = raw.backend.as_ref().expect("be");
        assert_ne!(fe.listen_port, be.listen_port);
        let s = emit_raw_config_toml(&raw).expect("emit");
        let p = parse_raw_config_str(&s).expect("parse");
        finish_raw_config(std::path::PathBuf::from("t.toml"), p)
            .expect("finish");
    }
}
