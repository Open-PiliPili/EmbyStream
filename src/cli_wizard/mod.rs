//! Interactive `embystream config` wizard (English prompts).

mod discover;
pub(crate) mod emit;
mod mask;
mod persist;
mod regex_lab;
pub(crate) mod template_payload;
mod terminal;
mod wizard;
mod wizard_input_theme;

pub use wizard::run;

#[cfg(test)]
mod tests_wizard;
