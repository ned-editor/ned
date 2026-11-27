//! zero-overhead logging stubs replacing the external log crate.

/// no-op trace logging macro.
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {};
}

/// no-op warning logging macro.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {};
}

pub(crate) use log_trace as trace;
pub(crate) use log_warn as warn;
