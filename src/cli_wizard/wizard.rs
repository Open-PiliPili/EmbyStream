//! Interactive config wizard and `config show`.

use std::{env, fs, io::Write, path::Path};

use anyhow::{Result, anyhow};
use chrono::Utc;
use dialoguer::{Input, Select};
use rand::Rng;

use crate::{
    cli::ConfigArgs,
    config::{
        backend::{
            Backend, BackendNode, direct::DirectLink, disk::Disk,
            openlist::OpenList, webdav::WebDavConfig,
        },
        core::{finish_raw_config, parse_raw_config_str},
        frontend::Frontend,
        general::{Emby, General, Log, StreamMode, UserAgent},
        http2::Http2,
        types::{
            AntiReverseProxyConfig, FallbackConfig, PathRewriteConfig,
            RawConfig,
        },
    },
    core::backend::constants::STREAM_RELAY_BACKEND_TYPE,
    core::backend::webdav::BACKEND_TYPE,
};

use super::{
    discover::{DiscoveredConfig, discover_configs},
    emit::emit_wizard_config_toml,
    mask::mask_toml_secrets,
    persist::{path_exists, safe_join_cwd, write_atomic},
    regex_lab::{prompt_regex_until_ok, regex_playground, try_compile_regex},
    template_payload::build_template_raw,
    terminal::{
        WIZ_DIALOG_LINES_BELOW_QUESTION, print_error, print_field_input_tip,
        print_field_intro_line, print_field_name_line,
        print_field_result_separator, print_field_value_line,
        print_field_value_line_compact, print_hint, print_ok,
        print_select_file_list_tip, print_subsection_title, print_table_header,
        print_title, print_welcome_banner, print_yes_no_result,
        rewrite_default_prompt_as_checkmark,
    },
    wizard_input_theme::{WIZ_INPUT_THEME, WizardInputTheme},
};

fn theme() -> dialoguer::theme::ColorfulTheme {
    dialoguer::theme::ColorfulTheme::default()
}

fn input_theme() -> &'static WizardInputTheme {
    &WIZ_INPUT_THEME
}

/// `intro()` prints `===> field (…, Default: … / Example: …)`; no `===> ? …` preview — use `wiz_input_*` after.
const WIZARD_INPUT_PROMPT: &str = "";

/// Yes/No via arrow-key selection (default option highlighted).
fn confirm_yes_no(prompt: &str, default_yes: bool) -> Result<bool> {
    const ITEMS: &[&str] = &["Yes", "No"];
    let default = if default_yes { 0usize } else { 1 };
    print_field_intro_line(prompt, "", None, None);
    let i = Select::with_theme(&theme())
        .with_prompt("")
        .items(ITEMS)
        .default(default)
        .report(false)
        .interact()
        .map_err(|e| anyhow!(e.to_string()))?;
    let yes = i == 0;
    print_yes_no_result(if yes { "Yes" } else { "No" });
    Ok(yes)
}

fn wiz_input_string(
    default: Option<String>,
    allow_empty: bool,
) -> Result<String> {
    let mut first_tip = true;
    loop {
        let previewed_default = default.is_some();
        if default.is_some() {
            print_field_input_tip();
        } else if first_tip {
            print_field_input_tip();
            first_tip = false;
        } else {
            print_field_input_tip();
        }
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            if let Some(ref d) = default {
                let disp = if d.trim().is_empty() {
                    "(empty)"
                } else {
                    d.trim()
                };
                rewrite_default_prompt_as_checkmark(
                    disp,
                    WIZ_DIALOG_LINES_BELOW_QUESTION,
                    None,
                );
                return Ok(d.clone());
            }
            if allow_empty {
                print_field_value_line("(empty)");
                return Ok(String::new());
            }
            print_error("A value is required.");
            continue;
        }
        if previewed_default {
            rewrite_default_prompt_as_checkmark(
                trimmed,
                WIZ_DIALOG_LINES_BELOW_QUESTION,
                None,
            );
        } else {
            print_field_value_line(trimmed);
        }
        return Ok(trimmed.to_string());
    }
}

fn wiz_input_string_no_echo(
    default: Option<String>,
    allow_empty: bool,
) -> Result<String> {
    let mut first_tip = true;
    loop {
        if default.is_some() {
            print_field_input_tip();
        } else if first_tip {
            print_field_input_tip();
            first_tip = false;
        } else {
            print_field_input_tip();
        }
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            if let Some(d) = default {
                return Ok(d);
            }
            if allow_empty {
                return Ok(String::new());
            }
            print_error("A value is required.");
            continue;
        }
        return Ok(trimmed.to_string());
    }
}

fn wiz_input_u16(default: u16) -> Result<u16> {
    loop {
        print_field_input_tip();
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.trim().is_empty() {
            rewrite_default_prompt_as_checkmark(
                &default.to_string(),
                WIZ_DIALOG_LINES_BELOW_QUESTION,
                None,
            );
            return Ok(default);
        }
        match s.trim().parse::<u16>() {
            Ok(v) => {
                rewrite_default_prompt_as_checkmark(
                    &v.to_string(),
                    WIZ_DIALOG_LINES_BELOW_QUESTION,
                    None,
                );
                return Ok(v);
            }
            Err(_) => print_error("Enter a number between 0 and 65535."),
        }
    }
}

fn wiz_input_i32(default: i32) -> Result<i32> {
    loop {
        print_field_input_tip();
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.trim().is_empty() {
            rewrite_default_prompt_as_checkmark(
                &default.to_string(),
                WIZ_DIALOG_LINES_BELOW_QUESTION,
                None,
            );
            return Ok(default);
        }
        match s.trim().parse::<i32>() {
            Ok(v) => {
                rewrite_default_prompt_as_checkmark(
                    &v.to_string(),
                    WIZ_DIALOG_LINES_BELOW_QUESTION,
                    None,
                );
                return Ok(v);
            }
            Err(_) => print_error("Enter a valid integer."),
        }
    }
}

fn wiz_input_u64(default: u64) -> Result<u64> {
    loop {
        print_field_input_tip();
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.trim().is_empty() {
            rewrite_default_prompt_as_checkmark(
                &default.to_string(),
                WIZ_DIALOG_LINES_BELOW_QUESTION,
                None,
            );
            return Ok(default);
        }
        match s.trim().parse::<u64>() {
            Ok(v) => {
                rewrite_default_prompt_as_checkmark(
                    &v.to_string(),
                    WIZ_DIALOG_LINES_BELOW_QUESTION,
                    None,
                );
                return Ok(v);
            }
            Err(_) => print_error("Enter a valid non-negative integer."),
        }
    }
}

/// Entry from `main`.
pub fn run(args: &ConfigArgs) -> Result<()> {
    let cwd = env::current_dir()?;
    match args.sub {
        None => run_main_menu(&cwd),
        Some(crate::cli::ConfigSubcommand::Show) => run_show_flow(&cwd),
        Some(crate::cli::ConfigSubcommand::Template) => run_template_flow(&cwd),
    }
}

fn run_show_flow(cwd: &Path) -> Result<()> {
    let list = discover_configs(cwd)?;
    print_discovered_table(&list);
    if list.is_empty() {
        print_hint("No valid EmbyStream TOML files in this directory.");
        return Ok(());
    }
    print_field_intro_line("Select file to display", "", None, None);
    print_select_file_list_tip();
    let idx = Select::with_theme(&theme())
        .with_prompt("")
        .items(
            list.iter()
                .map(|d| {
                    format!(
                        "{}  (stream_mode={})",
                        d.path.display(),
                        d.stream_mode
                    )
                })
                .collect::<Vec<_>>(),
        )
        .default(0)
        .report(false)
        .interact_opt()
        .map_err(|e| anyhow!(e.to_string()))?;
    let Some(idx) = idx else {
        return Ok(());
    };
    let content = fs::read_to_string(&list[idx].path)?;
    let mut masked = true;
    if confirm_yes_no(
        "Show secrets in plain text? (unsafe if others can see your screen)",
        false,
    )? {
        masked = false;
    }
    print_field_result_separator();
    if masked {
        print_title("Masked content");
        print!("{}", mask_toml_secrets(&content));
    } else {
        print_title("File content");
        print!("{content}");
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn default_template_filename(mode: StreamMode) -> String {
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    format!("{ts}_{mode}_template.toml")
}

fn run_template_flow(cwd: &Path) -> Result<()> {
    print_title("Configuration template");
    print_hint(
        "Choose stream_mode. A comment-free starter TOML is built in a temp file, \
         then written atomically to the name you choose.",
    );
    let mode = select_stream_mode()?;
    let default_name = default_template_filename(mode);
    let fname: String = {
        print_field_intro_line(
            "file_name",
            "Write under the current directory (bare file name or relative path).",
            Some(default_name.as_str()),
            None,
        );
        print_field_input_tip();
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.trim().is_empty() {
            default_name
        } else {
            s.trim().to_string()
        }
    };
    rewrite_default_prompt_as_checkmark(
        &fname,
        WIZ_DIALOG_LINES_BELOW_QUESTION,
        None,
    );
    let dest = safe_join_cwd(cwd, fname.trim())
        .ok_or_else(|| anyhow!("invalid file name"))?;
    if path_exists(&dest) {
        print_error("That file already exists. Choose another name.");
        return Ok(());
    }
    let raw = build_template_raw(mode);
    finish_raw_config(dest.clone(), raw.clone()).map_err(|e| anyhow!("{e}"))?;
    let toml = emit_wizard_config_toml(&raw)?;
    write_atomic(&dest, &toml).map_err(|e| anyhow!("{e}"))?;
    print_ok(&format!("Wrote template {}", dest.display()));
    Ok(())
}

fn run_main_menu(cwd: &Path) -> Result<()> {
    let mut first_menu = true;
    loop {
        if first_menu {
            print_welcome_banner();
            first_menu = false;
        }
        let list = discover_configs(cwd)?;
        print_discovered_table(&list);
        let items = vec![
            "New configuration file",
            "Edit existing",
            "Delete",
            "Rename",
            "Copy",
            "Quit",
        ];
        print_field_intro_line(
            "Main menu",
            "Pick an action (↑ / ↓ and Enter).",
            None,
            None,
        );
        let sel = Select::with_theme(&theme())
            .with_prompt("")
            .items(&items)
            .default(0)
            .report(false)
            .interact()
            .map_err(|e| anyhow!(e.to_string()))?;
        match sel {
            0 => run_new_flow(cwd)?,
            1 => {
                if list.is_empty() {
                    print_error("No config files to edit.");
                    continue;
                }
                let Some(i) = pick_discovered(&list, "Select file to edit")?
                else {
                    continue;
                };
                let path = list[i].path.clone();
                let raw = parse_raw_config_str(&fs::read_to_string(&path)?)?;
                let mut updated = run_edit_loop(raw)?;
                save_config_file(&path, &mut updated)?;
                print_ok("Saved.");
            }
            2 => {
                if list.is_empty() {
                    print_error("No config files to delete.");
                    continue;
                }
                let Some(i) = pick_discovered(&list, "Select file to delete")?
                else {
                    continue;
                };
                let p = &list[i].path;
                if confirm_yes_no(
                    &format!("Permanently delete {}?", p.display()),
                    false,
                )? {
                    fs::remove_file(p)?;
                    print_ok("Deleted.");
                }
            }
            3 => {
                if list.is_empty() {
                    print_error("No config files to rename.");
                    continue;
                }
                let Some(i) = pick_discovered(&list, "Select file to rename")?
                else {
                    continue;
                };
                print_field_input_tip();
                let new_name: String = Input::with_theme(input_theme())
                    .with_prompt("New file name (e.g. my.toml)")
                    .report(false)
                    .interact_text()
                    .map_err(|e| anyhow!(e.to_string()))?;
                let dest = safe_join_cwd(cwd, &new_name)
                    .ok_or_else(|| anyhow!("invalid file name"))?;
                if path_exists(&dest) {
                    print_error("Target already exists.");
                    continue;
                }
                fs::rename(&list[i].path, &dest)?;
                print_ok("Renamed.");
            }
            4 => {
                if list.is_empty() {
                    print_error("No config files to copy.");
                    continue;
                }
                let Some(i) = pick_discovered(&list, "Select file to copy")?
                else {
                    continue;
                };
                print_field_input_tip();
                let new_name: String = Input::with_theme(input_theme())
                    .with_prompt("New file name")
                    .report(false)
                    .interact_text()
                    .map_err(|e| anyhow!(e.to_string()))?;
                let dest = safe_join_cwd(cwd, &new_name)
                    .ok_or_else(|| anyhow!("invalid file name"))?;
                if path_exists(&dest) {
                    print_error("Target already exists.");
                    continue;
                }
                fs::copy(&list[i].path, &dest)?;
                print_ok("Copied.");
            }
            5 => break,
            _ => break,
        }
    }
    Ok(())
}

fn pick_discovered(
    list: &[DiscoveredConfig],
    prompt: &str,
) -> Result<Option<usize>> {
    print_field_intro_line(prompt, "", None, None);
    print_select_file_list_tip();
    Select::with_theme(&theme())
        .with_prompt("")
        .items(
            list.iter()
                .map(|d| format!("{}  ({})", d.path.display(), d.stream_mode))
                .collect::<Vec<_>>(),
        )
        .default(0)
        .report(false)
        .interact_opt()
        .map_err(|e| anyhow!(e.to_string()))
}

fn print_discovered_table(list: &[DiscoveredConfig]) {
    print_title("Configurations in current directory");
    if list.is_empty() {
        print_hint("(none)");
        print_field_result_separator();
        return;
    }
    print_table_header("idx", "file", "stream_mode");
    for (i, d) in list.iter().enumerate() {
        let name = d.path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        println!("  {:<4}  {:<38}  {}", i, name, d.stream_mode);
    }
    print_field_result_separator();
}

fn run_new_flow(cwd: &Path) -> Result<()> {
    let mode = select_stream_mode()?;
    let default_name = default_filename(mode);
    let fname: String = {
        print_field_intro_line(
            "file_name",
            "Write under the current directory (bare file name or relative path).",
            Some(default_name.as_str()),
            None,
        );
        print_field_input_tip();
        let s: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.trim().is_empty() {
            default_name
        } else {
            s.trim().to_string()
        }
    };
    rewrite_default_prompt_as_checkmark(
        &fname,
        WIZ_DIALOG_LINES_BELOW_QUESTION,
        None,
    );
    let dest = safe_join_cwd(cwd, fname.trim())
        .ok_or_else(|| anyhow!("invalid file name"))?;
    if path_exists(&dest) {
        print_error("That file already exists. Choose another name.");
        return Ok(());
    }

    let mut raw = build_new_raw_skeleton(mode)?;
    prompt_shared_sections(&mut raw)?;
    match mode {
        StreamMode::Frontend => {
            raw.frontend = Some(prompt_frontend_section()?);
        }
        StreamMode::Backend => {
            raw.backend = Some(prompt_backend_section()?);
            raw.backend_nodes = Some(prompt_backend_nodes_loop()?);
        }
        StreamMode::Dual => {
            raw.frontend = Some(prompt_frontend_section()?);
            raw.backend = Some(prompt_backend_section()?);
            resolve_dual_listen_ports(&mut raw)?;
            raw.backend_nodes = Some(prompt_backend_nodes_loop()?);
        }
    }

    validate_and_preview(&mut raw, &dest)?;
    if !confirm_yes_no("Write this configuration to disk?", true)? {
        print_hint("Discarded.");
        return Ok(());
    }
    save_config_file(&dest, &mut raw)?;
    print_ok(&format!("Wrote {}", dest.display()));

    if confirm_yes_no("Create another new configuration?", false)? {
        run_new_flow(cwd)?;
    }
    Ok(())
}

/// If dual mode ports collide, prompt until they differ.
fn resolve_dual_listen_ports(raw: &mut RawConfig) -> Result<()> {
    if raw.general.stream_mode != StreamMode::Dual {
        return Ok(());
    }
    loop {
        let (Some(fe), Some(be)) = (&raw.frontend, &raw.backend) else {
            return Ok(());
        };
        if fe.listen_port != be.listen_port {
            return Ok(());
        }
        print_error(&format!(
            "Dual mode: frontend listen_port {} cannot equal backend listen_port. Change one of them.",
            fe.listen_port
        ));
        let items =
            ["Change Frontend.listen_port", "Change Backend.listen_port"];
        print_field_intro_line(
            "Which port to change",
            "Pick Frontend or Backend listen_port to edit.",
            None,
            None,
        );
        let sel = Select::with_theme(&theme())
            .with_prompt("")
            .items(items)
            .default(0)
            .report(false)
            .interact()
            .map_err(|e| anyhow!(e.to_string()))?;
        match sel {
            0 => {
                if let Some(fe_mut) = raw.frontend.as_mut() {
                    let cur = fe_mut.listen_port;
                    let cur_s = cur.to_string();
                    intro(
                        "listen_port",
                        "Must differ from Backend.listen_port in dual mode.",
                        Some(cur_s.as_str()),
                        None,
                    );
                    fe_mut.listen_port = wiz_input_u16(cur)?;
                }
            }
            _ => {
                if let Some(be_mut) = raw.backend.as_mut() {
                    let cur = be_mut.listen_port;
                    let cur_s = cur.to_string();
                    intro(
                        "listen_port",
                        "Must differ from Frontend.listen_port in dual mode.",
                        Some(cur_s.as_str()),
                        None,
                    );
                    be_mut.listen_port = wiz_input_u16(cur)?;
                }
            }
        }
    }
}

fn default_filename(mode: StreamMode) -> String {
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    match mode {
        StreamMode::Frontend => format!("{ts}_frontend.toml"),
        StreamMode::Backend => format!("{ts}_backend.toml"),
        StreamMode::Dual => format!("{ts}_dual.toml"),
    }
}

fn select_stream_mode() -> Result<StreamMode> {
    print_title("Stream mode");
    print_hint("frontend: proxy clients to Emby only.");
    print_hint("backend: signed stream gateway + storage nodes.");
    print_hint(
        "dual: both; frontend and backend must use different listen_port.",
    );
    print_field_intro_line(
        "stream_mode",
        "Choose frontend, backend, or dual (↑ / ↓ and Enter).",
        None,
        None,
    );
    let items = vec!["frontend", "backend", "dual"];
    let i = Select::with_theme(&theme())
        .with_prompt("")
        .items(&items)
        .default(0)
        .report(false)
        .interact()
        .map_err(|e| anyhow!(e.to_string()))?;
    let mode = match i {
        1 => StreamMode::Backend,
        2 => StreamMode::Dual,
        _ => StreamMode::Frontend,
    };
    print_field_name_line("stream_mode");
    print_field_value_line(match mode {
        StreamMode::Frontend => "frontend",
        StreamMode::Backend => "backend",
        StreamMode::Dual => "dual",
    });
    Ok(mode)
}

fn build_new_raw_skeleton(mode: StreamMode) -> Result<RawConfig> {
    Ok(RawConfig {
        general: General {
            memory_mode: "middle".into(),
            stream_mode: mode,
            encipher_key: String::new(),
            encipher_iv: String::new(),
        },
        log: Log {
            level: "info".into(),
            prefix: String::new(),
            root_path: "./logs".into(),
        },
        emby: Emby {
            url: "http://127.0.0.1".into(),
            port: "8096".into(),
            token: String::new(),
        },
        user_agent: UserAgent {
            mode: "allow".into(),
            allow_ua: vec![],
            deny_ua: vec![],
        },
        http2: None,
        frontend: None,
        backend: None,
        backend_nodes: None,
        disk: None,
        open_list: None,
        direct_link: None,
        fallback: FallbackConfig::default(),
    })
}

fn intro(
    field: &str,
    purpose: &str,
    default_hint: Option<&str>,
    example: Option<&str>,
) {
    print_field_intro_line(field, purpose, default_hint, example);
}

fn input_text_w_echo(
    default: Option<String>,
    allow_empty: bool,
) -> Result<String> {
    wiz_input_string(default, allow_empty)
}

fn input_secret_w_echo(allow_empty: bool) -> Result<String> {
    print_field_input_tip();
    let s: String = Input::with_theme(input_theme())
        .with_prompt(WIZARD_INPUT_PROMPT)
        .allow_empty(allow_empty)
        .report(false)
        .interact_text()
        .map_err(|e| anyhow!(e.to_string()))?;
    let disp = if s.trim().is_empty() {
        "(empty)"
    } else {
        "· · ·"
    };
    print_field_value_line(disp);
    Ok(s)
}

fn prompt_shared_sections(raw: &mut RawConfig) -> Result<()> {
    print_title("Log");
    let def_level = raw.log.level.clone();
    intro(
        "level",
        "tracing filter for the app (info/warn/debug/error)",
        Some(def_level.as_str()),
        None,
    );
    raw.log.level = wiz_input_string(Some(def_level), false)?;
    let def_root = raw.log.root_path.clone();
    let root_disp = if def_root.trim().is_empty() {
        "(empty)"
    } else {
        def_root.trim()
    };
    intro(
        "root_path",
        "directory for log files (created on run if possible).",
        Some(root_disp),
        None,
    );
    raw.log.root_path = wiz_input_string(Some(def_root), false)?;
    let def_prefix = raw.log.prefix.clone();
    let prefix_disp = if def_prefix.trim().is_empty() {
        "(empty)"
    } else {
        def_prefix.trim()
    };
    intro(
        "prefix",
        "optional prefix for log file names.",
        Some(prefix_disp),
        None,
    );
    raw.log.prefix = wiz_input_string(Some(def_prefix), true)?;

    print_title("General");
    raw.general.memory_mode = prompt_memory_mode(&raw.general.memory_mode)?;
    intro(
        "encipher_key",
        "AES key material for stream signing (keep secret). \
         Press Enter to auto-generate 16 random letters and digits (mixed case); \
         or type your own value.",
        None,
        Some("(auto-generate on Enter)"),
    );
    let key_in: String = wiz_input_string_no_echo(None, true)?;
    raw.general.encipher_key = if key_in.trim().is_empty() {
        let v = random_alnum(16);
        print_field_value_line("(auto-generated)");
        v
    } else {
        print_field_value_line(key_in.trim());
        key_in
    };

    intro(
        "encipher_iv",
        "AES IV for stream signing (keep secret). \
         Press Enter to auto-generate 16 random letters and digits (mixed case); \
         or type your own value.",
        None,
        Some("(auto-generate on Enter)"),
    );
    let iv_in: String = wiz_input_string_no_echo(None::<String>, true)?;
    raw.general.encipher_iv = if iv_in.trim().is_empty() {
        let v = random_alnum(16);
        print_field_value_line("(auto-generated)");
        v
    } else {
        print_field_value_line(iv_in.trim());
        iv_in
    };

    print_title("Emby");
    let def_url = raw.emby.url.clone();
    let url_disp = if def_url.trim().is_empty() {
        "(empty)"
    } else {
        def_url.trim()
    };
    intro(
        "url",
        "Base URL of your Emby server (no trailing path). \
         Press Enter for http://127.0.0.1. You may omit the scheme (e.g. 127.0.0.1); http:// is added automatically.",
        Some(url_disp),
        None,
    );
    let url_in: String = wiz_input_string_no_echo(Some(def_url), true)?;
    raw.emby.url = normalize_emby_url(&url_in);
    rewrite_default_prompt_as_checkmark(
        raw.emby.url.as_str(),
        WIZ_DIALOG_LINES_BELOW_QUESTION,
        None,
    );
    let def_emby_port = raw.emby.port.clone();
    intro(
        "port",
        "Emby HTTP port (omit if url already includes port).",
        Some(def_emby_port.as_str()),
        None,
    );
    raw.emby.port = wiz_input_string(Some(def_emby_port), false)?;
    intro(
        "token",
        "Emby API access token from dashboard.",
        None,
        Some("paste_token_here"),
    );
    raw.emby.token = wiz_input_string(None, false)?;

    print_title("UserAgent");
    raw.user_agent.mode = prompt_user_agent_mode(&raw.user_agent.mode)?;
    raw.user_agent.allow_ua = prompt_ua_token_list(
        "allow_ua",
        "Tokens matched as substrings of the client User-Agent; matching is case-insensitive. \
         In allow mode, only matching clients pass.",
        false,
    )?;
    raw.user_agent.deny_ua = prompt_ua_token_list(
        "deny_ua",
        "Tokens matched as substrings of the client User-Agent; matching is case-insensitive. \
         In deny mode, matching clients are blocked.",
        true,
    )?;

    print_title("Http2 (TLS cert paths, optional)");
    intro(
        "ssl_cert_file",
        "PEM certificate path for backend HTTPS (empty = default layout next to config).",
        None,
        None,
    );
    let cert: String = wiz_input_string(None, true)?;
    intro("ssl_key_file", "PEM private key path.", None, None);
    let key: String = wiz_input_string(None, true)?;
    if !cert.is_empty() || !key.is_empty() {
        raw.http2 = Some(Http2 {
            ssl_cert_file: cert,
            ssl_key_file: key,
        });
    }

    print_title("Fallback");
    intro(
        "video_missing_path",
        "Local file served when a video resource is missing (empty to disable).",
        None,
        Some("/mnt/media/fallback.mp4"),
    );
    raw.fallback.video_missing_path = wiz_input_string(None, true)?;

    Ok(())
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn normalize_emby_url(raw_input: &str) -> String {
    let t = raw_input.trim();
    if t.is_empty() {
        return "http://127.0.0.1".into();
    }
    if t.contains("://") {
        t.to_string()
    } else {
        format!("http://{t}")
    }
}

fn prompt_user_agent_mode(current: &str) -> Result<String> {
    const VALUES: &[&str] = &["allow", "deny"];
    const LABELS: &[&str] = &[
        "allow — only listed User-Agent tokens pass",
        "deny — listed User-Agent tokens are blocked",
    ];
    let t = current.trim();
    let def_disp = VALUES
        .iter()
        .copied()
        .find(|v| v.eq_ignore_ascii_case(t))
        .unwrap_or("allow");
    intro(
        "mode",
        "allow: only listed User-Agent tokens pass; deny: listed tokens are blocked.",
        Some(def_disp),
        None,
    );
    let idx = VALUES
        .iter()
        .position(|v| v.eq_ignore_ascii_case(t))
        .unwrap_or(0);
    let i = Select::with_theme(&theme())
        .with_prompt("")
        .items(LABELS)
        .default(idx)
        .report(false)
        .interact()
        .map_err(|e| anyhow!(e.to_string()))?;
    let v = VALUES[i].to_string();
    print_field_value_line(&v);
    Ok(v)
}

fn prompt_memory_mode(current: &str) -> Result<String> {
    intro(
        "memory_mode",
        "cache footprint hint used by the app (low / middle / high).",
        Some(current),
        None,
    );
    const VALUES: &[&str] = &["low", "middle", "high"];
    const LABELS: &[&str] = &[
        "low — minimal caching",
        "middle — balanced (default)",
        "high — more aggressive caching",
    ];
    if let Some(idx) = VALUES.iter().position(|&v| v == current) {
        let i = Select::with_theme(&theme())
            .with_prompt("")
            .items(LABELS)
            .default(idx)
            .report(false)
            .interact()
            .map_err(|e| anyhow!(e.to_string()))?;
        let v = VALUES[i].to_string();
        print_field_value_line(&v);
        return Ok(v);
    }
    let s = wiz_input_string(Some(current.to_string()), false)?;
    Ok(s)
}

/// `skip_leading_input_tip`: set true for the second of back-to-back UA lists (`deny_ua` after `allow_ua`)
/// so the same `Tip:` line is not printed twice in a row.
fn prompt_ua_token_list(
    field_label: &str,
    purpose: &str,
    skip_leading_input_tip: bool,
) -> Result<Vec<String>> {
    intro(field_label, purpose, None, Some("Mozilla/5.0"));
    if !skip_leading_input_tip {
        print_field_input_tip();
    }
    print_hint("One token per line; empty line finishes the list.");
    let mut out = Vec::new();
    loop {
        let line: String = Input::with_theme(input_theme())
            .with_prompt(WIZARD_INPUT_PROMPT)
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        print_field_value_line_compact(t);
        out.push(t.to_string());
    }
    print_field_input_tip();
    print_field_result_separator();
    Ok(out)
}

fn random_alnum(len: usize) -> String {
    const CHARS: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

fn prompt_frontend_section() -> Result<Frontend> {
    print_title("Frontend");
    intro(
        "listen_port",
        "TCP port this process listens on for frontend clients.",
        Some("60001"),
        None,
    );
    let listen_port: u16 = wiz_input_u16(60001)?;
    const CHECK_DEFAULT: bool = true;
    intro(
        "check_file_existence",
        "When true, verify the resource exists on Emby before streaming.",
        Some(if CHECK_DEFAULT { "Yes" } else { "No" }),
        None,
    );
    let check = confirm_yes_no("Probe Emby before streaming?", CHECK_DEFAULT)?;
    let path_rewrites = prompt_path_rewrites(
        "PathRewrite",
        "Rewrite outgoing paths to Emby/CDN.",
    )?;
    let anti = prompt_anti_reverse("AntiReverseProxy")?;
    Ok(Frontend {
        listen_port,
        check_file_existence: check,
        path_rewrites,
        anti_reverse_proxy: anti,
    })
}

fn prompt_backend_section() -> Result<Backend> {
    print_title("Backend");
    intro(
        "listen_port",
        "TCP port for backend gateway (HTTPS if certs configured).",
        Some("60001"),
        None,
    );
    let listen_port: u16 = wiz_input_u16(60001)?;
    intro(
        "base_url",
        "Public base URL clients use to reach this backend.",
        None,
        Some("https://stream.example.com"),
    );
    let base_url: String = wiz_input_string(None, false)?;
    intro(
        "port",
        "Port embedded in published URLs (often 443).",
        Some("443"),
        None,
    );
    let port: String = wiz_input_string(Some("443".into()), false)?;
    intro(
        "path",
        "URL path prefix for stream routes (e.g. stream).",
        None,
        Some("stream"),
    );
    let path: String = wiz_input_string(None, true)?;
    intro(
        "problematic_clients",
        "Comma-separated substrings of User-Agent to treat specially.",
        Some("yamby, hills, embytolocalplayer, Emby/"),
        None,
    );
    let pc: String = wiz_input_string(
        Some("yamby, hills, embytolocalplayer, Emby/".into()),
        true,
    )?;
    let problematic_clients = split_csv(&pc);
    Ok(Backend {
        listen_port,
        base_url,
        port,
        path,
        problematic_clients,
    })
}

fn prompt_anti_reverse(ctx: &str) -> Result<AntiReverseProxyConfig> {
    intro(
        ctx,
        "Reject requests whose Host header does not match trusted host when enabled.",
        None,
        Some("host = \"stream.example.com\""),
    );
    let enable = confirm_yes_no(
        &format!("Enable {ctx}? (reject requests when Host ≠ trusted host)"),
        false,
    )?;
    let host: String = if enable {
        intro(
            "host",
            "Trusted Host header when anti-reverse-proxy is enabled.",
            None,
            Some("stream.example.com"),
        );
        let h: String = wiz_input_string_no_echo(None, false)?;
        let disp = if h.trim().is_empty() {
            "(empty)"
        } else {
            h.trim()
        };
        print_field_value_line(disp);
        h
    } else {
        String::new()
    };
    Ok(AntiReverseProxyConfig {
        enable,
        trusted_host: host,
    })
}

fn prompt_path_rewrites(
    ctx: &str,
    purpose: &str,
) -> Result<Vec<PathRewriteConfig>> {
    let mut out = vec![];
    print_field_intro_line(ctx, purpose, None, None);
    while confirm_yes_no("Add a PathRewrite entry?", false)? {
        let enable = confirm_yes_no("Enable this PathRewrite rule?", false)?;
        intro(
            "pattern",
            "Rust regex applied to the path when enable=true.",
            None,
            Some("^/media(/.*)$"),
        );
        let pattern: String = if enable {
            let Some(p) = prompt_regex_until_ok()? else {
                print_error("Skipped entry (empty pattern).");
                continue;
            };
            if confirm_yes_no(
                "Open regex match playground for this pattern?",
                true,
            )? {
                if let Some(re) = try_compile_regex(&p) {
                    regex_playground(&re)?;
                }
            }
            p
        } else {
            let p: String = wiz_input_string_no_echo(None, true)?;
            let disp = if p.trim().is_empty() {
                "(empty)"
            } else {
                p.trim()
            };
            print_field_value_line(disp);
            p
        };
        intro(
            "replacement",
            "Replacement string (supports capture groups).",
            None,
            Some("$1"),
        );
        let replacement: String = wiz_input_string(None, true)?;
        out.push(PathRewriteConfig {
            enable,
            pattern,
            replacement,
        });
    }
    Ok(out)
}

fn backend_type_labels() -> Vec<&'static str> {
    vec![
        "Disk — local filesystem root",
        "OpenList — AList/OpenList HTTP API",
        "DirectLink — plain HTTP fetch with custom User-Agent",
        "WebDav — WebDAV upstream",
        "StreamRelay — 307 relay signed /stream to another backend",
    ]
}

fn prompt_backend_nodes_loop() -> Result<Vec<BackendNode>> {
    let mut nodes = vec![];
    loop {
        print_title("BackendNode");
        let prompt = if nodes.is_empty() {
            "Add a BackendNode?"
        } else {
            "Add another BackendNode?"
        };
        let default_first = nodes.is_empty();
        if !confirm_yes_no(prompt, default_first)? {
            break;
        }
        nodes.push(prompt_one_backend_node()?);
    }
    Ok(nodes)
}

fn prompt_one_backend_node() -> Result<BackendNode> {
    intro(
        "name",
        "Short label for logs (unique recommended).",
        None,
        Some("MyOpenList"),
    );
    let name: String = wiz_input_string(None, false)?;

    intro(
        "type",
        "Storage or upstream kind for this node.",
        None,
        None,
    );
    let labels = backend_type_labels();
    let tidx = Select::with_theme(&theme())
        .with_prompt("")
        .items(&labels)
        .default(0)
        .report(false)
        .interact()
        .map_err(|e| anyhow!(e.to_string()))?;

    let backend_type = match tidx {
        0 => "Disk",
        1 => "OpenList",
        2 => "DirectLink",
        3 => BACKEND_TYPE,
        4 => STREAM_RELAY_BACKEND_TYPE,
        _ => "Disk",
    }
    .to_string();
    print_field_value_line(&backend_type);

    intro(
        "pattern",
        "Rust regex on request path; empty uses path-prefix fallback rules.",
        None,
        Some("/openlist/.*"),
    );
    let pattern: String = wiz_input_string(None, true)?;
    if !pattern.is_empty() {
        if try_compile_regex(&pattern).is_none() {
            return Err(anyhow!("invalid regex pattern"));
        }
        if confirm_yes_no("Open regex match playground?", true)? {
            if let Some(re) = try_compile_regex(&pattern) {
                regex_playground(&re)?;
            }
        }
    }

    intro(
        "base_url",
        "Upstream origin for this node (scheme + host).",
        None,
        Some("http://127.0.0.1"),
    );
    let base_url: String = wiz_input_string(None, true)?;
    intro(
        "port",
        "Upstream port (empty if not needed).",
        None,
        Some("5244"),
    );
    let port: String = wiz_input_string(None, true)?;
    intro(
        "path",
        "Path segment appended to base (no leading slash required).",
        None,
        Some("openlist"),
    );
    let path: String = wiz_input_string(None, true)?;

    intro(
        "proxy_mode",
        "redirect: client follows Location; proxy: server fetches upstream.",
        None,
        None,
    );
    let proxy_items = vec![
        "redirect — client follows Location",
        "proxy — server fetches upstream",
    ];
    let pidx = Select::with_theme(&theme())
        .with_prompt("")
        .items(&proxy_items)
        .default(0)
        .report(false)
        .interact()
        .map_err(|e| anyhow!(e.to_string()))?;
    let proxy_mode = if pidx == 1 { "proxy" } else { "redirect" }.to_string();
    print_field_value_line(&proxy_mode);

    intro(
        "priority",
        "Lower runs earlier when multiple nodes match.",
        Some("0"),
        None,
    );
    let priority: i32 = wiz_input_i32(0)?;
    intro(
        "client_speed_limit_kbs",
        "Per-client speed limit in KiB/s (0 = unlimited).",
        Some("0"),
        None,
    );
    let client_speed_limit_kbs: u64 = wiz_input_u64(0)?;
    intro(
        "client_burst_speed_kbs",
        "Burst allowance in KiB/s (0 = default / none).",
        Some("0"),
        None,
    );
    let client_burst_speed_kbs: u64 = wiz_input_u64(0)?;

    let path_rewrites = prompt_path_rewrites(
        "PathRewrite",
        "Rewrite path before hitting upstream.",
    )?;
    let anti_reverse_proxy = prompt_anti_reverse("AntiReverseProxy")?;

    let (disk, open_list, direct_link, webdav) = match backend_type.as_str() {
        "Disk" => {
            intro(
                "description",
                "Optional human note (not used for routing).",
                None,
                Some("local NAS"),
            );
            let description: String = wiz_input_string(None, true)?;
            (Some(Disk { description }), None, None, None)
        }
        "OpenList" => {
            intro(
                "base_url",
                "AList base URL.",
                None,
                Some("http://127.0.0.1"),
            );
            let b: String = wiz_input_string(None, false)?;
            intro("port", "AList port if not in URL.", None, Some("5244"));
            let p: String = wiz_input_string(None, true)?;
            intro("token", "AList API token.", None, None);
            let tok: String = wiz_input_string(None, false)?;
            (
                None,
                Some(OpenList {
                    base_url: b,
                    port: p,
                    token: tok,
                }),
                None,
                None,
            )
        }
        "DirectLink" => {
            intro(
                "user_agent",
                "User-Agent header for upstream fetch.",
                None,
                Some("Mozilla/5.0"),
            );
            let ua: String = wiz_input_string(None, false)?;
            (None, None, Some(DirectLink { user_agent: ua }), None)
        }
        t if t.eq_ignore_ascii_case(BACKEND_TYPE) => {
            print_subsection_title("BackendNode.WebDav");
            intro(
                "url_mode",
                "path_join | query_path | url_template",
                Some("path_join"),
                None,
            );
            let url_mode: String =
                input_text_w_echo(Some("path_join".into()), false)?;
            intro(
                "query_param",
                "Query key when url_mode=query_path.",
                Some("path"),
                None,
            );
            let query_param: String =
                input_text_w_echo(Some("path".into()), false)?;
            intro(
                "url_template",
                "When url_mode=url_template, URL with {file_path} placeholder.",
                None,
                None,
            );
            let url_template: String = input_text_w_echo(None, true)?;
            intro("username", "Optional HTTP basic user.", None, None);
            let username: String = input_text_w_echo(None, true)?;
            intro("password", "Optional HTTP basic password.", None, None);
            let password: String = input_secret_w_echo(true)?;
            intro("user_agent", "Optional custom User-Agent.", None, None);
            let user_agent: String = input_text_w_echo(None, true)?;
            (
                None,
                None,
                None,
                Some(WebDavConfig {
                    url_mode,
                    query_param,
                    url_template,
                    username,
                    password,
                    user_agent,
                }),
            )
        }
        _ => (None, None, None, None),
    };

    Ok(BackendNode {
        name,
        backend_type,
        pattern,
        pattern_regex: None,
        base_url,
        port,
        path,
        priority,
        proxy_mode,
        client_speed_limit_kbs,
        client_burst_speed_kbs,
        path_rewrites,
        anti_reverse_proxy,
        path_rewriter_cache: vec![],
        uuid: String::new(),
        disk,
        open_list,
        direct_link,
        webdav,
    })
}

fn validate_and_preview(raw: &mut RawConfig, dest: &Path) -> Result<()> {
    resolve_dual_listen_ports(raw)?;
    finish_raw_config(dest.to_path_buf(), raw.clone())
        .map_err(|e| anyhow!("{e}"))?;
    let toml = emit_wizard_config_toml(raw)?;
    print_title("Preview (comment-free TOML)");
    print!("{toml}");
    std::io::stdout().flush()?;
    Ok(())
}

fn save_config_file(dest: &Path, raw: &mut RawConfig) -> Result<()> {
    resolve_dual_listen_ports(raw)?;
    finish_raw_config(dest.to_path_buf(), raw.clone())
        .map_err(|e| anyhow!("{e}"))?;
    let toml = emit_wizard_config_toml(raw)?;
    write_atomic(dest, &toml).map_err(|e| anyhow!("{e}"))
}

fn run_edit_loop(mut raw: RawConfig) -> Result<RawConfig> {
    loop {
        let mut opts: Vec<&'static str> = Vec::new();
        opts.push("Log / General / Emby / UserAgent / Http2 / Fallback");
        if raw.frontend.is_some() {
            opts.push("Frontend section");
        }
        if raw.backend.is_some() {
            opts.push("Backend section");
            opts.push("Backend nodes");
        }
        opts.push("Done (save)");
        print_field_intro_line(
            "Edit which part",
            "Choose a config section to change (↑ / ↓ and Enter).",
            None,
            None,
        );
        let sel = Select::with_theme(&theme())
            .with_prompt("")
            .items(&opts)
            .report(false)
            .interact()
            .map_err(|e| anyhow!(e.to_string()))?;

        match opts[sel] {
            "Log / General / Emby / UserAgent / Http2 / Fallback" => {
                prompt_shared_sections(&mut raw)?;
            }
            "Frontend section" => {
                raw.frontend = Some(prompt_frontend_section()?);
            }
            "Backend section" => {
                raw.backend = Some(prompt_backend_section()?);
            }
            "Backend nodes" => {
                raw.backend_nodes = Some(prompt_backend_nodes_loop()?);
            }
            "Done (save)" => break,
            _ => break,
        }
    }
    Ok(raw)
}

#[cfg(test)]
mod normalize_emby_url_tests {
    use super::normalize_emby_url;

    #[test]
    fn empty_and_whitespace_default() {
        assert_eq!(normalize_emby_url(""), "http://127.0.0.1");
        assert_eq!(normalize_emby_url("   "), "http://127.0.0.1");
    }

    #[test]
    fn prepends_http_without_scheme() {
        assert_eq!(normalize_emby_url("127.0.0.1"), "http://127.0.0.1");
        assert_eq!(
            normalize_emby_url("192.168.1.1:8096"),
            "http://192.168.1.1:8096"
        );
    }

    #[test]
    fn keeps_explicit_scheme() {
        assert_eq!(
            normalize_emby_url("https://emby.example.com"),
            "https://emby.example.com"
        );
    }
}
