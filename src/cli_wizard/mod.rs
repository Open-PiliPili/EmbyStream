//! Interactive `embystream config` wizard (prompts follow `--lang`).

mod discover;
pub(crate) mod emit;
mod l10n;
mod mask;
mod persist;
mod regex_lab;
pub(crate) mod template_payload;
mod terminal;
mod wizard;
mod wizard_input_theme;

use anyhow::Result;

use crate::cli::{ConfigArgs, UiLang};

pub fn run(args: &ConfigArgs, lang: UiLang) -> Result<()> {
    l10n::set_wizard_lang(lang);
    wizard::run(args)
}

#[cfg(test)]
mod tests_wizard;
