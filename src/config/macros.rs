#[macro_export]
macro_rules! config_log {
    ($level:expr, $domain:expr, $($arg:tt)+) => {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%6f");
        let colored_now = format!("\x1b[2m{}\x1b[0m", now);

        let level_padded = format!("{:>5}", $level);
        let colored_level = match $level {
            "ERROR" => format!("\x1b[31m{}\x1b[0m", level_padded),
            "WARN" => format!("\x1b[33m{}\x1b[0m", level_padded),
            "DEBUG" => format!("\x1b[33m{}\x1b[0m", level_padded),
            _ => format!("\x1b[32m{}\x1b[0m", level_padded),
        };

        let msg = format!($($arg)+);

        println!("{} {} [{}] {}", colored_now, colored_level, $domain, msg);
    };
}

#[macro_export]
macro_rules! config_info_log {
    ($domain:expr, $($arg:tt)+) => {
        $crate::config_log!("INFO", $domain, $($arg)+);
    };
}

#[macro_export]
macro_rules! config_error_log {
    ($domain:expr, $($arg:tt)+) => {
        $crate::config_log!("ERROR", $domain, $($arg)+);
    };
}

#[macro_export]
macro_rules! config_debug_log {
    ($domain:expr, $($arg:tt)+) => {
        $crate::config_log!("DEBUG", $domain, $($arg)+);
    };
}

#[macro_export]
macro_rules! config_warn_log {
    ($domain:expr, $($arg:tt)+) => {
        $crate::config_log!("WARN", $domain, $($arg)+);
    };
}
