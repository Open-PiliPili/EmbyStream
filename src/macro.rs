/// Internal macro used by all log level macros (trace_log!, info_log!, etc.).
/// Automatically formats log output as: [DOMAIN] formatted_message
/// Supports both literal and dynamic format strings.
/// Not intended to be called directly by external users.
#[macro_export]
macro_rules! _log_internal {
    // Case 1: No domain, just format + args (e.g., `info_log!("{}", x)`)
    ($level:ident, $fmt:literal $(, $arg:expr)* $(,)? ) => {
        tracing::$level!(
            "{}",
            format_args!(
                "[{}] {}",
                $crate::domain::DEFAULT_LOGGER_DOMAIN,
                format_args!($fmt $(, $arg)*)
            )
        );
    };

    // Case 2: No domain, single value (e.g., `info_log!(x)`)
    ($level:ident, $value:expr $(,)? ) => {
        tracing::$level!(
            "{}",
            format_args!(
                "[{}] {}",
                $crate::domain::DEFAULT_LOGGER_DOMAIN,
                $value
            )
        );
    };

    // Case 3: With domain, format + args (e.g., `info_log!("DOMAIN", "{}", x)`)
    ($level:ident, $domain:expr, $fmt:literal $(, $arg:expr)* $(,)? ) => {
        tracing::$level!(
            "{}",
            format_args!("[{}] {}", $domain, format_args!($fmt $(, $arg)*))
        );
    };

    // Case 4: With domain, single value (e.g., `info_log!("DOMAIN", x)`)
    ($level:ident, $domain:expr, $value:expr $(,)? ) => {
        tracing::$level!(
            "{}",
            format_args!("[{}] {}", $domain, $value)
        );
    };
}

/// Logs a message at TRACE level, with optional domain support.
/// Examples:
///   trace_log!("Init done.");
///   trace_log!("Domain", "Loaded config: {:?}", config);
#[macro_export]
macro_rules! trace_log {
    ($($args:tt)*) => {
        $crate::_log_internal!(trace, $($args)*)
    };
}

/// Logs a message at DEBUG level, with optional domain support.
#[macro_export]
macro_rules! debug_log {
    ($($args:tt)*) => {
        $crate::_log_internal!(debug, $($args)*)
    };
}

/// Logs a message at INFO level, with optional domain support.
#[macro_export]
macro_rules! info_log {
    ($($args:tt)*) => {
        $crate::_log_internal!(info, $($args)*)
    };
}

/// Logs a message at WARN level, with optional domain support.
#[macro_export]
macro_rules! warn_log {
    ($($args:tt)*) => {
        $crate::_log_internal!(warn, $($args)*)
    };
}

/// Logs a message at ERROR level, with optional domain support.
#[macro_export]
macro_rules! error_log {
    ($($args:tt)*) => {
        $crate::_log_internal!(error, $($args)*)
    };
}
