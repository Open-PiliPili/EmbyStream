//! Styled terminal output (English only).

use std::io::Write;

use console::Style;

/// Full-width rule for primary sections (titles, banners).
const RULE_FULL: usize = 60;

/// Slightly inset rule for nested subsections (e.g. PathRewrite under BackendNode).
const RULE_SUB: usize = 56;

/// Step prefix at column 0 (cargo-style arrow).
const STEP: &str = "===> ";

fn rule(len: usize) -> String {
    "─".repeat(len)
}

fn style_step() -> String {
    Style::new().dim().apply_to(STEP).to_string()
}

fn purpose_one_line(purpose: &str) -> String {
    purpose
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Short banner when opening the config wizard menu (once per session feel).
pub fn print_welcome_banner() {
    println!();
    println!("{}", Style::new().dim().apply_to(rule(RULE_FULL)));
    println!(
        "  {}",
        Style::new()
            .cyan()
            .bold()
            .apply_to("EmbyStream · configuration")
    );
    println!(
        "  {}",
        Style::new()
            .dim()
            .apply_to("Create or edit TOML files in the current directory.")
    );
    println!("{}", Style::new().dim().apply_to(rule(RULE_FULL)));
}

/// Section banner: rules with the title flush left (no extra blank lines).
pub fn print_title(s: &str) {
    println!("{}", Style::new().dim().apply_to(rule(RULE_FULL)));
    println!("{}", Style::new().cyan().bold().apply_to(s));
    println!("{}", Style::new().dim().apply_to(rule(RULE_FULL)));
}

/// Nested block — same geometry as `print_title`, different accent color.
pub fn print_subsection_title(s: &str) {
    println!("{}", Style::new().dim().apply_to(rule(RULE_SUB)));
    println!("{}", Style::new().magenta().bold().apply_to(s));
    println!("{}", Style::new().dim().apply_to(rule(RULE_SUB)));
}

/// Dim hint lines with the same step prefix as prompts (no bare empty lines).
pub fn print_hint(s: &str) {
    for line in s.lines() {
        let t = line.trim();
        if !t.is_empty() {
            println!("{}{}", style_step(), Style::new().dim().apply_to(t));
        }
    }
}

pub fn print_error(s: &str) {
    eprintln!("{}", Style::new().red().bold().apply_to(format!("  ✗ {s}")));
}

pub fn print_ok(s: &str) {
    println!(
        "{}",
        Style::new().green().bold().apply_to(format!("  ✓ {s}"))
    );
}

/// Short label line: `===> stream_mode` (before `===> ✔ …` for clarity).
pub fn print_field_name_line(name: &str) {
    println!("{}{}", style_step(), Style::new().bold().apply_to(name));
}

/// One line before `Input`: `===> level (…, Default: info)` and/or `Example: …`.
/// Section title gives context; `name` is the field only. Omit `? value` preview — defaults live here.
pub fn print_field_intro_line(
    name: &str,
    purpose: &str,
    default_hint: Option<&str>,
    example: Option<&str>,
) {
    let body = purpose_one_line(purpose);
    print!("{}", style_step());
    print!("{}", Style::new().bold().apply_to(name));
    let has_any =
        !body.is_empty() || default_hint.is_some() || example.is_some();
    if !has_any {
        let _ = std::io::stdout().flush();
        println!();
        return;
    }
    print!("{}", Style::new().dim().apply_to(" ("));
    let mut need_sep = false;
    if !body.is_empty() {
        print!("{}", Style::new().dim().apply_to(&body));
        need_sep = true;
    }
    if let Some(d) = default_hint.filter(|s| !s.is_empty()) {
        if need_sep {
            print!("{}", Style::new().dim().apply_to(", "));
        }
        print!("{}", Style::new().dim().apply_to("Default: "));
        print!("{}", Style::new().magenta().dim().apply_to(d));
        need_sep = true;
    }
    if let Some(ex) = example.filter(|s| !s.is_empty()) {
        if need_sep {
            print!("{}", Style::new().dim().apply_to(", "));
        }
        print!("{}", Style::new().dim().apply_to("Example: "));
        print!("{}", Style::new().magenta().dim().apply_to(ex));
    }
    print!("{}", Style::new().dim().apply_to(")"));
    let _ = std::io::stdout().flush();
    println!();
}

/// Dim `===>` on its own line after a field result — separates `✔ …` from the next `intro`.
pub fn print_field_result_separator() {
    println!("{}", style_step());
}

/// Rows below the field intro line: dim `Tip:` line + `===>` input row (no `===> ? …` preview).
pub const WIZ_DIALOG_LINES_BELOW_QUESTION: usize = 2;

/// Clears only the lines **below** the field intro (`Tip:` + `===>` input). The intro line
/// (`===> name (…, Default: …)`) stays visible; prints `===> ✔ …` on the row below it.
///
/// `result_label`: optional extra `===> label` line before the checkmark (use `None` when the
/// intro already names the field).
/// Prints a dim trailing `===>` line so the next field intro is visually separated.
pub fn rewrite_default_prompt_as_checkmark(
    display: &str,
    dialog_lines_below_question: usize,
    result_label: Option<&str>,
) {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    for _ in 0..dialog_lines_below_question {
        print!("\r\x1b[2K");
        print!("\x1b[1A");
    }
    // Cursor is on the intro line; do not erase it.
    print!("\x1b[1B\r\x1b[2K");
    if let Some(label) = result_label {
        println!("{}{}", style_step(), Style::new().bold().apply_to(label));
    }
    print!(
        "{}{} {}",
        style_step(),
        Style::new().green().bold().apply_to("✔"),
        Style::new().bold().apply_to(display)
    );
    println!();
    print_field_result_separator();
}

/// After Yes/No: `===> ✔ Yes`, then dim `===>` on the next line (no blank line between).
pub fn print_yes_no_result(answer: &str) {
    println!(
        "{}{} {}",
        style_step(),
        Style::new().green().bold().apply_to("✔"),
        Style::new().bold().apply_to(answer)
    );
    print_field_result_separator();
}

/// After text input when there was **no** `===> ? …` preview line (e.g. selects, secrets).
/// `===> ✔ value`, or `===> ? ›` when `display` is `(empty)`.
pub fn print_field_value_line(display: &str) {
    print_field_value_line_inner(display, true);
}

/// Same as [`print_field_value_line`] but does not print [`print_field_result_separator`] after —
/// for multi-line token lists so consecutive `✔` rows stay visually grouped.
pub fn print_field_value_line_compact(display: &str) {
    print_field_value_line_inner(display, false);
}

fn print_field_value_line_inner(display: &str, separator_after: bool) {
    if display == "(empty)" {
        println!(
            "{}{} {}",
            style_step(),
            Style::new().yellow().bold().apply_to("?"),
            Style::new().dim().apply_to("›")
        );
    } else {
        println!(
            "{}{} {}",
            style_step(),
            Style::new().green().bold().apply_to("✔"),
            Style::new().bold().apply_to(display)
        );
    }
    if separator_after {
        print_field_result_separator();
    }
}

/// Before file-list `Select` prompts that use `interact_opt` (Esc / q cancels).
pub fn print_select_file_list_tip() {
    println!(
        "{}{}",
        style_step(),
        Style::new()
            .dim()
            .apply_to("Tip: ↑ / ↓ and Enter to choose · Esc or q to go back",)
    );
}

/// Shown before wizard `Input` (dim line with `===>` prefix).
pub const FIELD_INPUT_TIP: &str = "Tip: Press Enter to keep the default from the field line (Default: …), or type a new value and press Enter.";

pub fn print_field_input_tip() {
    println!(
        "{}{}",
        style_step(),
        Style::new().dim().apply_to(FIELD_INPUT_TIP)
    );
}

/// Single dim line prefixed with `===>` (e.g. regex playground match output).
pub fn print_step_dim_line(msg: &str) {
    println!("{}{}", style_step(), Style::new().dim().apply_to(msg));
}

/// Table header row styling for discovered configs.
pub fn print_table_header(c1: &str, c2: &str, c3: &str) {
    println!(
        "  {}  {:<38}  {}",
        Style::new().bold().dim().apply_to(c1),
        Style::new().bold().apply_to(c2),
        Style::new().bold().dim().apply_to(c3)
    );
    println!("  {}", Style::new().dim().apply_to("─".repeat(50)));
}
