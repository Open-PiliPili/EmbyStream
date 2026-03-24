//! CLI `--lang` handling and localized `--help` text (zh vs default en).

use clap::Command;

use crate::cli::UiLang;
use crate::i18n::lookup;

/// Scan `std::env::args()` so `--help` can be localized before full parse.
pub fn detect_lang_from_env_early() -> UiLang {
    let mut it = std::env::args();
    while let Some(a) = it.next() {
        if a == "--lang" {
            if let Some(v) = it.next() {
                if v.eq_ignore_ascii_case("zh") {
                    return UiLang::Zh;
                }
            }
        } else if let Some(rest) = a.strip_prefix("--lang=") {
            if rest.eq_ignore_ascii_case("zh") {
                return UiLang::Zh;
            }
        }
    }
    UiLang::En
}

/// Replace top-level and nested `about` / `help` when `lang` is Chinese.
pub fn localize_cli_command(cmd: &mut Command, lang: UiLang) {
    if lang != UiLang::Zh {
        return;
    }

    *cmd = std::mem::take(cmd)
        .about(lookup(lang, "cli.about"))
        .mut_arg("lang", |a| a.help(lookup(lang, "cli.arg.lang")));

    if let Some(h) = cmd.find_subcommand_mut("help") {
        *h = std::mem::take(h).about(lookup(lang, "cli.sub.help.about"));
    }

    if let Some(run) = cmd.find_subcommand_mut("run") {
        *run = std::mem::take(run)
            .about(lookup(lang, "cli.run.about"))
            .mut_arg("config", |a| a.help(lookup(lang, "cli.run.arg.config")))
            .mut_arg("ssl_cert_file", |a| {
                a.help(lookup(lang, "cli.run.arg.ssl_cert_file"))
            })
            .mut_arg("ssl_key_file", |a| {
                a.help(lookup(lang, "cli.run.arg.ssl_key_file"))
            });
    }

    if let Some(cfg) = cmd.find_subcommand_mut("config") {
        *cfg = std::mem::take(cfg).about(lookup(lang, "cli.config.about"));
        if let Some(show) = cfg.find_subcommand_mut("show") {
            *show = std::mem::take(show)
                .about(lookup(lang, "cli.config.show.about"));
        }
        if let Some(tpl) = cfg.find_subcommand_mut("template") {
            *tpl = std::mem::take(tpl)
                .about(lookup(lang, "cli.config.template.about"));
        }
    }
}
