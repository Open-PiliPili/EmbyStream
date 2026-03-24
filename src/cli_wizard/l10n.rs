//! Wizard UI language (delegates to `crate::i18n` for thread-local locale).

pub use crate::i18n::{set_ui_lang as set_wizard_lang, tr};

pub fn empty_display() -> String {
    tr("wizard.display.empty")
}

pub fn auto_generated_display() -> String {
    tr("wizard.display.auto_generated")
}

pub fn secret_masked_display() -> String {
    tr("wizard.display.secret_masked")
}
