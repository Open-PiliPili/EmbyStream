//! Validate regex and optional match playground.

use anyhow::{Result, anyhow};
use dialoguer::Input;
use regex::Regex;

use super::terminal::{
    print_error, print_field_input_tip, print_field_intro_line,
    print_field_value_line, print_step_dim_line,
};
use super::wizard_input_theme::WIZ_INPUT_THEME;
use crate::i18n::{tr, tr_fmt};

/// Compile pattern or print localized error and return None.
pub fn try_compile_regex(pattern: &str) -> Option<Regex> {
    match Regex::new(pattern) {
        Ok(re) => Some(re),
        Err(e) => {
            let msg =
                tr_fmt("regex.error.invalid", &[("detail", &e.to_string())]);
            print_error(&msg);
            None
        }
    }
}

/// Loop until user enters a valid regex (non-empty) or empty to cancel (returns None).
/// Caller must print `intro` for `pattern` first; prompt line stays blank to avoid duplicate labels.
pub fn prompt_regex_until_ok() -> Result<Option<String>> {
    loop {
        print_field_input_tip();
        let s: String = Input::with_theme(&WIZ_INPUT_THEME)
            .with_prompt("")
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if s.is_empty() {
            return Ok(None);
        }
        if try_compile_regex(&s).is_some() {
            print_field_value_line(s.trim());
            return Ok(Some(s));
        }
    }
}

/// After a valid pattern, optionally test matches until empty line.
pub fn regex_playground(re: &Regex) -> Result<()> {
    print_field_intro_line(
        "test_path",
        tr("wizard.msg.prefix.wrote_template"),
        None,
        None,
    );
    let mut first_input = true;
    loop {
        if first_input {
            print_field_input_tip();
            first_input = false;
        }
        let line: String = Input::with_theme(&WIZ_INPUT_THEME)
            .with_prompt("")
            .allow_empty(true)
            .report(false)
            .interact_text()
            .map_err(|e| anyhow!(e.to_string()))?;
        if line.is_empty() {
            break;
        }
        let m = re.is_match(&line);
        let msg = tr_fmt("regex.output.matches", &[("value", &format!("{m}"))]);
        print_step_dim_line(&msg);
        if let Some(caps) = re.captures(&line) {
            for (i, g) in caps.iter().enumerate() {
                if let Some(g) = g {
                    let msg = tr_fmt(
                        "regex.output.group",
                        &[
                            ("idx", &i.to_string()),
                            ("value", &format!("{:?}", g.as_str())),
                        ],
                    );
                    print_step_dim_line(&msg);
                }
            }
        }
    }
    Ok(())
}
