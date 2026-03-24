//! Input prompt styling: `===> ` prefix (dim) instead of a bare `: `.

use std::fmt;

use console::Style;
use dialoguer::theme::{SimpleTheme, Theme};

/// Same prefix as `terminal::STEP`, dimmed like the step arrow.
const STEP_DIM: &str = "===> ";

/// Delegates to `SimpleTheme` except text `Input` prompts use `===> …` (no trailing colon).
pub struct WizardInputTheme;

pub static WIZ_INPUT_THEME: WizardInputTheme = WizardInputTheme;

impl Theme for WizardInputTheme {
    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        write!(f, "{}", Style::new().dim().apply_to(STEP_DIM))?;
        if !prompt.is_empty() {
            write!(f, "{prompt} ")?;
        }
        if let Some(d) = default {
            if !d.is_empty() {
                write!(f, "[{d}] ")?;
            }
        }
        Ok(())
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        Theme::format_input_prompt_selection(&SimpleTheme, f, prompt, sel)
    }

    fn format_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_prompt(&SimpleTheme, f, prompt)
    }

    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        Theme::format_error(&SimpleTheme, f, err)
    }

    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        Theme::format_confirm_prompt(&SimpleTheme, f, prompt, default)
    }

    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        Theme::format_confirm_prompt_selection(
            &SimpleTheme,
            f,
            prompt,
            selection,
        )
    }

    fn format_password_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_password_prompt(&SimpleTheme, f, prompt)
    }

    fn format_password_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_password_prompt_selection(&SimpleTheme, f, prompt)
    }

    fn format_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_select_prompt(&SimpleTheme, f, prompt)
    }

    fn format_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        Theme::format_select_prompt_selection(&SimpleTheme, f, prompt, sel)
    }

    fn format_multi_select_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_multi_select_prompt(&SimpleTheme, f, prompt)
    }

    fn format_sort_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
    ) -> fmt::Result {
        Theme::format_sort_prompt(&SimpleTheme, f, prompt)
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        Theme::format_multi_select_prompt_selection(
            &SimpleTheme,
            f,
            prompt,
            selections,
        )
    }

    fn format_sort_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        Theme::format_sort_prompt_selection(&SimpleTheme, f, prompt, selections)
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        Theme::format_select_prompt_item(&SimpleTheme, f, text, active)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        Theme::format_multi_select_prompt_item(
            &SimpleTheme,
            f,
            text,
            checked,
            active,
        )
    }

    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        Theme::format_sort_prompt_item(&SimpleTheme, f, text, picked, active)
    }
}
